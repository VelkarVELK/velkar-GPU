use crate::{pow, target::Uint256, Error, Hash};
use argon2::{Algorithm, Argon2, Block, Params, Version};
use log::{debug, info, warn};
use opencl3::{
    command_queue::CommandQueue,
    context::Context,
    device::{Device, CL_DEVICE_TYPE_GPU},
    kernel::Kernel,
    memory::{Buffer, CL_MEM_READ_WRITE},
    platform::get_platforms,
    program::Program,
    types::{CL_BLOCKING},
};
use std::{cmp::max, mem, ptr};

#[cfg(feature = "cuda")]
use crate::cuda::CudaSearcher;

const ARGON_MEMORY_KIB: u32 = 8 * 1024;
const ARGON_TIME_COST: u32 = 1;
const ARGON_LANES: u32 = 1;
const ARGON2_BLOCK_BYTES: usize = 1024;
const ARGON2_SYNC_POINTS: u32 = 4;
const THREADS_PER_LANE: usize = 32;
const JOB_INPUT_BYTES: usize = 72 + 16 + 8 + 32 + 32 + 32;
const CONSTANTS_BYTES: usize = 72 + 64 * 64 * 2;

#[derive(Clone, Debug)]
pub struct GpuConfig {
    pub backend: GpuBackend,
    pub platform_index: Option<u16>,
    pub device_index: Option<u16>,
    pub batch_size: usize,
    pub jobs_per_block: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
pub struct GpuHit {
    pub nonce: u64,
    pub pow: Uint256,
}

pub struct OpenClSearcher {
    queue: CommandQueue,
    argon_kernel: Kernel,
    prepare_kernel: Kernel,
    init_kernel: Kernel,
    finalize_kernel: Kernel,
    export_kernel: Kernel,
    memory: Buffer<u8>,
    constants_buffer: Buffer<u8>,
    job_inputs_buffer: Buffer<u8>,
    found_nonces_buffer: Buffer<u64>,
    stage4_buffer: Buffer<u8>,
    nonce_out_buffer: Buffer<u64>,
    refs_buffer: Buffer<u8>,
    memory_per_job: usize,
    jobs_per_block: usize,
    batch_size: usize,
    initial_validation_done: bool,
    full_validation_done: bool,
}

impl OpenClSearcher {
    pub fn new(config: &GpuConfig) -> Result<Self, Error> {
        let platforms = get_platforms().map_err(|e| format!("OpenCL platform enumeration failed: {e}"))?;
        if platforms.is_empty() {
            return Err("No OpenCL platforms found".into());
        }

        for (i, platform) in platforms.iter().enumerate() {
            let vendor = platform.vendor().unwrap_or_else(|_| "Unknown".into());
            let name = platform.name().unwrap_or_else(|_| "Unknown".into());
            info!("OpenCL platform #{i}: {vendor} / {name}");
        }

        let platform = match config.platform_index {
            Some(idx) => platforms.get(idx as usize).ok_or("Invalid OpenCL platform index")?,
            None => &platforms[0],
        };

        let devices = platform.get_devices(CL_DEVICE_TYPE_GPU).unwrap_or_default();
        if devices.is_empty() {
            return Err(format!(
                "No OpenCL GPU devices found on platform {}",
                platform.name().unwrap_or_else(|_| "Unknown".into())
            )
            .into());
        }

        let device_id = match config.device_index {
            Some(idx) => *devices.get(idx as usize).ok_or("Invalid OpenCL device index")?,
            None => devices[0],
        };

        let device = Device::new(device_id);
        let device_name = device.name().unwrap_or_else(|_| "Unknown".into());
        let vendor = device.vendor().unwrap_or_else(|_| "Unknown".into());
        let compute_units = device.max_compute_units().unwrap_or(1);
        let max_wg = device.max_work_group_size().unwrap_or(64) as usize;
        info!("OpenCL GPU selected: {vendor} / {device_name} ({compute_units} CUs, WG max {max_wg})");

        let context = Context::from_device(&device).map_err(|e| format!("OpenCL context creation failed: {e}"))?;
        let program = Program::create_and_build_from_source(
            &context,
            include_str!("argon2_kernel.cl"),
            "-DARGON2_TYPE=2 -DARGON2_VERSION=0x13",
        )
        .map_err(|e| format!("OpenCL program build failed: {e}"))?;

        let argon_kernel = Kernel::create(&program, "argon2_kernel_oneshot_precompute")
            .map_err(|e| format!("OpenCL Argon2 kernel creation failed: {e}"))?;
        let prepare_kernel = Kernel::create(&program, "velkar_prepare_jobs")
            .map_err(|e| format!("OpenCL prepare kernel creation failed: {e}"))?;
        let init_kernel = Kernel::create(&program, "velkar_init_first_blocks")
            .map_err(|e| format!("OpenCL init kernel creation failed: {e}"))?;
        let finalize_kernel = Kernel::create(&program, "velkar_finalize_candidates")
            .map_err(|e| format!("OpenCL finalize kernel creation failed: {e}"))?;
        let export_kernel = Kernel::create(&program, "velkar_export_stage4")
            .map_err(|e| format!("OpenCL stage4 export kernel creation failed: {e}"))?;

        let queue = unsafe { CommandQueue::create_with_properties(&context, device.id(), 0, 0) }
            .map_err(|e| e.to_string())?;

        let memory_per_job = memory_per_job_bytes();
        let max_jobs_by_mem = {
            let max_alloc = device.max_mem_alloc_size().unwrap_or(128 * 1024 * 1024) as usize;
            let global_mem = device.global_mem_size().unwrap_or(512 * 1024 * 1024) as usize;
            let cap = max_alloc.min(global_mem / 2);
            // Scale with the GPU while respecting both the driver's maximum
            // single allocation and half of total VRAM. The old 256-job cap
            // was only intended for initial compatibility testing.
            (cap / memory_per_job).clamp(1, 4096)
        };
        let requested = max(1, config.batch_size);
        let batch_size = requested.min(max_jobs_by_mem);
        let max_jobs_per_block = (max_wg / THREADS_PER_LANE).clamp(1, 32).min(batch_size);
        // One Argon2 job per workgroup avoids barrier contention and was the
        // fastest stable setting across the validated AMD benchmark matrix.
        let mut jobs_per_block = config.jobs_per_block.unwrap_or(1).clamp(1, max_jobs_per_block);
        while jobs_per_block > 1 && batch_size % jobs_per_block != 0 {
            jobs_per_block -= 1;
        }

        let memory = unsafe {
            Buffer::<u8>::create(&context, CL_MEM_READ_WRITE, memory_per_job * batch_size, ptr::null_mut())
        }
        .map_err(|e| e.to_string())?;
        let constants_buffer = unsafe {
            Buffer::<u8>::create(&context, CL_MEM_READ_WRITE, CONSTANTS_BYTES, ptr::null_mut())
        }
        .map_err(|e| e.to_string())?;
        let job_inputs_buffer = unsafe {
            Buffer::<u8>::create(&context, CL_MEM_READ_WRITE, JOB_INPUT_BYTES * batch_size, ptr::null_mut())
        }
        .map_err(|e| e.to_string())?;
        let found_nonces_buffer = unsafe {
            Buffer::<u64>::create(&context, CL_MEM_READ_WRITE, batch_size, ptr::null_mut())
        }
        .map_err(|e| e.to_string())?;
        let stage4_buffer = unsafe {
            Buffer::<u8>::create(&context, CL_MEM_READ_WRITE, 32 * batch_size, ptr::null_mut())
        }
        .map_err(|e| e.to_string())?;
        let nonce_out_buffer = unsafe {
            Buffer::<u64>::create(&context, CL_MEM_READ_WRITE, batch_size, ptr::null_mut())
        }
        .map_err(|e| e.to_string())?;
        let refs_buffer = unsafe {
            Buffer::<u8>::create(&context, CL_MEM_READ_WRITE, refs_buffer_bytes(), ptr::null_mut())
        }
        .map_err(|e| e.to_string())?;

        precompute_argon2_refs(&queue, &program, &refs_buffer)?;

        // Run Argon2 slice-by-slice. Kernel boundaries provide a hard sync point
        // and make GPU results easier to compare with the CPU consensus path.

        info!(
            "{device_name}: full OpenCL Velkar pipeline active: batch={} jobs, {} MiB/job, {} MiB total, jobs/block={}",
            batch_size,
            memory_per_job / (1024 * 1024),
            (memory_per_job * batch_size) / (1024 * 1024),
            jobs_per_block
        );

        Ok(Self {
            queue,
            argon_kernel,
            prepare_kernel,
            init_kernel,
            finalize_kernel,
            export_kernel,
            memory,
            constants_buffer,
            job_inputs_buffer,
            found_nonces_buffer,
            stage4_buffer,
            nonce_out_buffer,
            refs_buffer,
            memory_per_job,
            jobs_per_block,
            batch_size,
            initial_validation_done: false,
            full_validation_done: false,
        })
    }

    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    pub fn search(&mut self, state: &pow::State, nonce_base: u64, check_target: Uint256) -> Result<Vec<GpuHit>, Error> {
        self.upload_constants(state)?;
        self.prepare_jobs(state, nonce_base)?;
        self.run_init_kernel()?;
        self.validate_initial_blocks_once()?;
        self.run_argon_kernel()?;
        self.export_and_filter_candidates(state, check_target)
    }

    fn upload_constants(&mut self, state: &pow::State) -> Result<(), Error> {
        let mut constants = vec![0u8; CONSTANTS_BYTES];
        constants[..72].copy_from_slice(&state.gpu_hash_header());
        constants[72..].copy_from_slice(&state.gpu_matrix_u16_le_bytes());
        unsafe {
            self.queue
                .enqueue_write_buffer(&mut self.constants_buffer, CL_BLOCKING, 0, &constants, &[])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn prepare_jobs(&mut self, state: &pow::State, nonce_base: u64) -> Result<(), Error> {
        unsafe {
            let input_per_job = JOB_INPUT_BYTES as u64;
            let nonce_mask = u64::MAX;
            let nonce_fixed = 0u64;
            let target = state.block_target().to_le_u64();
            self.prepare_kernel.set_arg(0, &self.job_inputs_buffer).map_err(|e| e.to_string())?;
            self.prepare_kernel.set_arg(1, &self.constants_buffer).map_err(|e| e.to_string())?;
            self.prepare_kernel.set_arg(2, &input_per_job).map_err(|e| e.to_string())?;
            self.prepare_kernel.set_arg(3, &nonce_base).map_err(|e| e.to_string())?;
            self.prepare_kernel.set_arg(4, &nonce_mask).map_err(|e| e.to_string())?;
            self.prepare_kernel.set_arg(5, &nonce_fixed).map_err(|e| e.to_string())?;
            self.prepare_kernel.set_arg(6, &target[0]).map_err(|e| e.to_string())?;
            self.prepare_kernel.set_arg(7, &target[1]).map_err(|e| e.to_string())?;
            self.prepare_kernel.set_arg(8, &target[2]).map_err(|e| e.to_string())?;
            self.prepare_kernel.set_arg(9, &target[3]).map_err(|e| e.to_string())?;
            let global_work_sizes = [self.batch_size];
            self.queue
                .enqueue_nd_range_kernel(
                    self.prepare_kernel.get(),
                    1,
                    ptr::null(),
                    global_work_sizes.as_ptr(),
                    ptr::null(),
                    &[],
                )
                .map_err(|e| e.to_string())?;
        }
        // The command queue is in-order. Keep preparation and initialization
        // queued so the GPU can execute the complete batch without host gaps.
        Ok(())
    }

    fn run_init_kernel(&mut self) -> Result<(), Error> {
        unsafe {
            let memory_per_job = self.memory_per_job as u64;
            let input_per_job = JOB_INPUT_BYTES as u64;
            self.init_kernel.set_arg(0, &self.memory).map_err(|e| e.to_string())?;
            self.init_kernel.set_arg(1, &self.job_inputs_buffer).map_err(|e| e.to_string())?;
            self.init_kernel.set_arg(2, &memory_per_job).map_err(|e| e.to_string())?;
            self.init_kernel.set_arg(3, &input_per_job).map_err(|e| e.to_string())?;
            let global_work_sizes = [self.batch_size];
            self.queue
                .enqueue_nd_range_kernel(self.init_kernel.get(), 1, ptr::null(), global_work_sizes.as_ptr(), ptr::null(), &[])
                .map_err(|e| e.to_string())?;
        }
        // The following Argon2 dispatch depends on this kernel through the
        // same in-order queue, so an intermediate host-side wait is redundant.
        Ok(())
    }

    fn run_argon_kernel(&mut self) -> Result<(), Error> {
        unsafe {
            let lanes_per_block = 1usize;
            let shmem_bytes = THREADS_PER_LANE * lanes_per_block * self.jobs_per_block * mem::size_of::<u32>() * 2;
            self.argon_kernel.set_arg_local_buffer(0, shmem_bytes).map_err(|e| e.to_string())?;
            self.argon_kernel.set_arg(1, &self.memory).map_err(|e| e.to_string())?;
            self.argon_kernel.set_arg(2, &self.refs_buffer).map_err(|e| e.to_string())?;
            let passes = ARGON_TIME_COST;
            let lanes = ARGON_LANES;
            let segment_blocks = ARGON_MEMORY_KIB / (ARGON2_SYNC_POINTS * lanes);
            self.argon_kernel.set_arg(3, &passes).map_err(|e| e.to_string())?;
            self.argon_kernel.set_arg(4, &lanes).map_err(|e| e.to_string())?;
            self.argon_kernel.set_arg(5, &segment_blocks).map_err(|e| e.to_string())?;
            let global_work_sizes = [THREADS_PER_LANE * lanes_per_block, self.batch_size];
            let local_work_sizes = [THREADS_PER_LANE * lanes_per_block, self.jobs_per_block];
            self.queue
                .enqueue_nd_range_kernel(
                    self.argon_kernel.get(),
                    2,
                    ptr::null(),
                    global_work_sizes.as_ptr(),
                    local_work_sizes.as_ptr(),
                    &[],
                )
                .map_err(|e| e.to_string())?;
            self.queue.finish().map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn validate_initial_blocks_once(&mut self) -> Result<(), Error> {
        if self.initial_validation_done {
            return Ok(());
        }
        let mut input = vec![0u8; JOB_INPUT_BYTES];
        let mut gpu_first_blocks = vec![0u8; ARGON2_BLOCK_BYTES * 2];
        unsafe {
            self.queue
                .enqueue_read_buffer(&self.job_inputs_buffer, CL_BLOCKING, 0, &mut input, &[])
                .map_err(|e| e.to_string())?;
            self.queue
                .enqueue_read_buffer(&self.memory, CL_BLOCKING, 0, &mut gpu_first_blocks, &[])
                .map_err(|e| e.to_string())?;
        }

        let mut cpu_first_blocks = vec![0u8; ARGON2_BLOCK_BYTES * 2];
        fill_first_blocks_cpu(&mut cpu_first_blocks, &input[..72], input[72..88].try_into().expect("salt length"));
        if gpu_first_blocks == cpu_first_blocks {
            info!("OpenCL Argon2 initial blocks match CPU reference");
        } else {
            warn!(
                "OpenCL Argon2 initial block mismatch gpu_b0={} cpu_b0={} gpu_b1={} cpu_b1={}",
                hex32(&gpu_first_blocks[..32]),
                hex32(&cpu_first_blocks[..32]),
                hex32(&gpu_first_blocks[ARGON2_BLOCK_BYTES..ARGON2_BLOCK_BYTES + 32]),
                hex32(&cpu_first_blocks[ARGON2_BLOCK_BYTES..ARGON2_BLOCK_BYTES + 32])
            );
        }
        self.initial_validation_done = true;
        Ok(())
    }

    fn finalize_candidates(&mut self, state: &pow::State, check_target: Uint256) -> Result<Vec<GpuHit>, Error> {
        let zeroes = vec![0u64; self.batch_size];
        unsafe {
            self.queue
                .enqueue_write_buffer(&mut self.found_nonces_buffer, CL_BLOCKING, 0, &zeroes, &[])
                .map_err(|e| e.to_string())?;

            let memory_per_job = self.memory_per_job as u64;
            let input_per_job = JOB_INPUT_BYTES as u64;
            let target = state.block_target().to_le_u64();
            let check = check_target.to_le_u64();
            self.finalize_kernel.set_arg(0, &self.memory).map_err(|e| e.to_string())?;
            self.finalize_kernel.set_arg(1, &self.job_inputs_buffer).map_err(|e| e.to_string())?;
            self.finalize_kernel.set_arg(2, &self.found_nonces_buffer).map_err(|e| e.to_string())?;
            self.finalize_kernel.set_arg(3, &memory_per_job).map_err(|e| e.to_string())?;
            self.finalize_kernel.set_arg(4, &input_per_job).map_err(|e| e.to_string())?;
            self.finalize_kernel.set_arg(5, &target[0]).map_err(|e| e.to_string())?;
            self.finalize_kernel.set_arg(6, &target[1]).map_err(|e| e.to_string())?;
            self.finalize_kernel.set_arg(7, &target[2]).map_err(|e| e.to_string())?;
            self.finalize_kernel.set_arg(8, &target[3]).map_err(|e| e.to_string())?;
            self.finalize_kernel.set_arg(9, &check[0]).map_err(|e| e.to_string())?;
            self.finalize_kernel.set_arg(10, &check[1]).map_err(|e| e.to_string())?;
            self.finalize_kernel.set_arg(11, &check[2]).map_err(|e| e.to_string())?;
            self.finalize_kernel.set_arg(12, &check[3]).map_err(|e| e.to_string())?;
            let global_work_sizes = [self.batch_size];
            self.queue
                .enqueue_nd_range_kernel(
                    self.finalize_kernel.get(),
                    1,
                    ptr::null(),
                    global_work_sizes.as_ptr(),
                    ptr::null(),
                    &[],
                )
                .map_err(|e| e.to_string())?;
        }
        self.queue.finish().map_err(|e| e.to_string())?;

        let mut found = vec![0u64; self.batch_size];
        unsafe {
            self.queue
                .enqueue_read_buffer(&self.found_nonces_buffer, CL_BLOCKING, 0, &mut found, &[])
                .map_err(|e| e.to_string())?;
        }
        let hits = found.into_iter().filter(|nonce| *nonce != 0).map(|nonce| GpuHit { nonce, pow: Uint256::default() }).collect::<Vec<_>>();
        if !hits.is_empty() {
            debug!("OpenCL found {} matching candidate(s)", hits.len());
        }
        Ok(hits)
    }

    fn export_and_filter_candidates(&mut self, state: &pow::State, check_target: Uint256) -> Result<Vec<GpuHit>, Error> {
        unsafe {
            let memory_per_job = self.memory_per_job as u64;
            let input_per_job = JOB_INPUT_BYTES as u64;
            self.export_kernel.set_arg(0, &self.memory).map_err(|e| e.to_string())?;
            self.export_kernel.set_arg(1, &self.job_inputs_buffer).map_err(|e| e.to_string())?;
            self.export_kernel.set_arg(2, &self.stage4_buffer).map_err(|e| e.to_string())?;
            self.export_kernel.set_arg(3, &self.nonce_out_buffer).map_err(|e| e.to_string())?;
            self.export_kernel.set_arg(4, &memory_per_job).map_err(|e| e.to_string())?;
            self.export_kernel.set_arg(5, &input_per_job).map_err(|e| e.to_string())?;
            let global_work_sizes = [self.batch_size];
            self.queue
                .enqueue_nd_range_kernel(
                    self.export_kernel.get(),
                    1,
                    ptr::null(),
                    global_work_sizes.as_ptr(),
                    ptr::null(),
                    &[],
                )
                .map_err(|e| e.to_string())?;
        }
        self.queue.finish().map_err(|e| e.to_string())?;

        let mut job_inputs = vec![0u8; JOB_INPUT_BYTES * self.batch_size];
        let mut stage4 = vec![0u8; 32 * self.batch_size];
        let mut nonces = vec![0u64; self.batch_size];
        unsafe {
            self.queue
                .enqueue_read_buffer(&self.job_inputs_buffer, CL_BLOCKING, 0, &mut job_inputs, &[])
                .map_err(|e| e.to_string())?;
            self.queue
                .enqueue_read_buffer(&self.stage4_buffer, CL_BLOCKING, 0, &mut stage4, &[])
                .map_err(|e| e.to_string())?;
            self.queue
                .enqueue_read_buffer(&self.nonce_out_buffer, CL_BLOCKING, 0, &mut nonces, &[])
                .map_err(|e| e.to_string())?;
        }

        let mut hits = Vec::new();
        for (i, nonce) in nonces.into_iter().enumerate() {
            let input = &job_inputs[i * JOB_INPUT_BYTES..(i + 1) * JOB_INPUT_BYTES];
            let stage1 = Hash::from_le_bytes(input[96..128].try_into().expect("stage1 length"));
            let stage2 = Hash::from_le_bytes(input[128..160].try_into().expect("stage2 length"));
            let stage3 = Hash::from_le_bytes(input[160..192].try_into().expect("stage3 length"));
            let stage4 = Hash::from_le_bytes(stage4[i * 32..(i + 1) * 32].try_into().expect("stage4 length"));
            if i == 0 && !self.full_validation_done {
                let cpu_stage1 = state.stage1_for_nonce(nonce);
                let cpu_stage2 = state.stage2_for_stage1(cpu_stage1);
                let cpu_stage3 = state.stage3_for_stage2(cpu_stage2);
                let cpu_stage4 = state.cpu_stage4_from_parts(cpu_stage1, cpu_stage3, nonce);
                let mut gpu_last_block = vec![0u8; ARGON2_BLOCK_BYTES];
                unsafe {
                    self.queue
                        .enqueue_read_buffer(
                            &self.memory,
                            CL_BLOCKING,
                            self.memory_per_job - ARGON2_BLOCK_BYTES,
                            &mut gpu_last_block,
                            &[],
                        )
                        .map_err(|e| e.to_string())?;
                }
                let mut cpu_digest_of_gpu_last = [0u8; 32];
                digest_long_cpu(&mut cpu_digest_of_gpu_last, &gpu_last_block);
                let cpu_digest_of_gpu_last = Hash::from_le_bytes(cpu_digest_of_gpu_last);
                self.diagnose_argon_memory(input)?;
                if stage1 == cpu_stage1 && stage2 == cpu_stage2 && stage3 == cpu_stage3 && stage4 == cpu_stage4 {
                    info!("OpenCL Velkar full pipeline validation passed for nonce {nonce:016x}");
                } else {
                    warn!(
                        "OpenCL Velkar validation mismatch nonce={:016x} stage1={} cpu_stage1={} stage2={} cpu_stage2={} stage3={} cpu_stage3={} gpu_stage4={} cpu_digest_gpu_last={} cpu_stage4={}",
                        nonce,
                        stage1.to_be_hex(),
                        cpu_stage1.to_be_hex(),
                        stage2.to_be_hex(),
                        cpu_stage2.to_be_hex(),
                        stage3.to_be_hex(),
                        cpu_stage3.to_be_hex(),
                        stage4.to_be_hex(),
                        cpu_digest_of_gpu_last.to_be_hex(),
                        cpu_stage4.to_be_hex()
                    );
                }
                self.full_validation_done = true;
            }
            let pow = state.calculate_pow_from_gpu_stage4(stage1, stage2, stage3, stage4, nonce);
            if pow <= check_target {
                hits.push(GpuHit { nonce, pow });
            }
        }

        if !hits.is_empty() {
            debug!("OpenCL/CPU-final found {} valid share candidate(s)", hits.len());
        }
        Ok(hits)
    }

    fn diagnose_argon_memory(&self, input: &[u8]) -> Result<(), Error> {
        let mut gpu_memory = vec![0u8; self.memory_per_job];
        unsafe {
            self.queue
                .enqueue_read_buffer(&self.memory, CL_BLOCKING, 0, &mut gpu_memory, &[])
                .map_err(|e| e.to_string())?;
        }

        let params = Params::new(ARGON_MEMORY_KIB, ARGON_TIME_COST, ARGON_LANES, Some(32))
            .map_err(|e| e.to_string())?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut cpu_memory = vec![Block::default(); ARGON_MEMORY_KIB as usize];
        let mut cpu_output = [0u8; 32];
        argon2
            .hash_password_into_with_memory(&input[..72], &input[72..88], &mut cpu_output, &mut cpu_memory)
            .map_err(|e| e.to_string())?;

        for (block_index, cpu_block) in cpu_memory.iter().enumerate() {
            for (word_index, cpu_word) in cpu_block.as_ref().iter().enumerate() {
                let byte_offset = block_index * ARGON2_BLOCK_BYTES + word_index * mem::size_of::<u64>();
                let gpu_word = u64::from_le_bytes(
                    gpu_memory[byte_offset..byte_offset + mem::size_of::<u64>()]
                        .try_into()
                        .expect("Argon2 word length"),
                );
                if gpu_word != *cpu_word {
                    warn!(
                        "OpenCL Argon2 first memory divergence: block={} word={} gpu={:016x} cpu={:016x}",
                        block_index, word_index, gpu_word, cpu_word
                    );
                    return Ok(());
                }
            }
        }

        info!("OpenCL Argon2 memory matches CPU reference");
        Ok(())
    }

}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuBackend {
    OpenCl,
    #[cfg(feature = "cuda")]
    Cuda,
}

pub enum GpuSearcher {
    OpenCl(OpenClSearcher),
    #[cfg(feature = "cuda")]
    Cuda(CudaSearcher),
}

impl GpuSearcher {
    pub fn new(config: &GpuConfig) -> Result<Self, Error> {
        match config.backend {
            GpuBackend::OpenCl => Ok(Self::OpenCl(OpenClSearcher::new(config)?)),
            #[cfg(feature = "cuda")]
            GpuBackend::Cuda => Ok(Self::Cuda(CudaSearcher::new(config)?)),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::OpenCl(_) => "OpenCL",
            #[cfg(feature = "cuda")]
            Self::Cuda(_) => "CUDA",
        }
    }

    pub fn batch_size(&self) -> usize {
        match self {
            Self::OpenCl(searcher) => searcher.batch_size(),
            #[cfg(feature = "cuda")]
            Self::Cuda(searcher) => searcher.batch_size(),
        }
    }

    pub fn search(&mut self, state: &pow::State, nonce_base: u64, target: Uint256) -> Result<Vec<GpuHit>, Error> {
        match self {
            Self::OpenCl(searcher) => searcher.search(state, nonce_base, target),
            #[cfg(feature = "cuda")]
            Self::Cuda(searcher) => searcher.search(state, nonce_base, target),
        }
    }
}

fn memory_per_job_bytes() -> usize {
    let segment_blocks = (ARGON_MEMORY_KIB / (ARGON2_SYNC_POINTS * ARGON_LANES)) as usize;
    segment_blocks * ARGON2_SYNC_POINTS as usize * ARGON2_BLOCK_BYTES
}

fn refs_buffer_bytes() -> usize {
    const ARGON2_REFS_PER_BLOCK: usize = ARGON2_BLOCK_BYTES / (2 * mem::size_of::<u32>());
    let segment_blocks = (ARGON_MEMORY_KIB / (ARGON2_SYNC_POINTS * ARGON_LANES)) as usize;
    let segment_addr_blocks = (segment_blocks + ARGON2_REFS_PER_BLOCK - 1) / ARGON2_REFS_PER_BLOCK;
    let segments = ARGON_LANES as usize * (ARGON2_SYNC_POINTS as usize / 2);
    segments * segment_addr_blocks * ARGON2_REFS_PER_BLOCK * 2 * mem::size_of::<u32>()
}

fn precompute_argon2_refs(queue: &CommandQueue, program: &Program, refs_buffer: &Buffer<u8>) -> Result<(), Error> {
    let precompute_kernel = Kernel::create(program, "argon2_precompute_kernel").map_err(|e| e.to_string())?;
    let passes = ARGON_TIME_COST;
    let lanes = ARGON_LANES;
    let segment_blocks = ARGON_MEMORY_KIB / (ARGON2_SYNC_POINTS * lanes);
    let segment_addr_blocks =
        (segment_blocks as usize + (ARGON2_BLOCK_BYTES / (2 * mem::size_of::<u32>())) - 1)
            / (ARGON2_BLOCK_BYTES / (2 * mem::size_of::<u32>()));
    let segments = lanes as usize * (ARGON2_SYNC_POINTS as usize / 2);
    let shmem_bytes = THREADS_PER_LANE * mem::size_of::<u32>() * 2;

    unsafe {
        precompute_kernel.set_arg_local_buffer(0, shmem_bytes).map_err(|e| e.to_string())?;
        precompute_kernel.set_arg(1, refs_buffer).map_err(|e| e.to_string())?;
        precompute_kernel.set_arg(2, &passes).map_err(|e| e.to_string())?;
        precompute_kernel.set_arg(3, &lanes).map_err(|e| e.to_string())?;
        precompute_kernel.set_arg(4, &segment_blocks).map_err(|e| e.to_string())?;
        let global_work_sizes = [THREADS_PER_LANE * segments * segment_addr_blocks];
        let local_work_sizes = [THREADS_PER_LANE];
        queue
            .enqueue_nd_range_kernel(
                precompute_kernel.get(),
                1,
                ptr::null(),
                global_work_sizes.as_ptr(),
                local_work_sizes.as_ptr(),
                &[],
            )
            .map_err(|e| e.to_string())?;
    }
    queue.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn fill_first_blocks_cpu(memory: &mut [u8], password: &[u8], salt: &[u8; 16]) {
    let mut init_hash = [0u8; 72];
    initial_hash_cpu(&mut init_hash[..64], password, salt);

    init_hash[64..68].copy_from_slice(&0u32.to_le_bytes());
    init_hash[68..72].copy_from_slice(&0u32.to_le_bytes());
    digest_long_cpu(&mut memory[..ARGON2_BLOCK_BYTES], &init_hash);

    init_hash[64..68].copy_from_slice(&1u32.to_le_bytes());
    digest_long_cpu(&mut memory[ARGON2_BLOCK_BYTES..2 * ARGON2_BLOCK_BYTES], &init_hash);
}

fn initial_hash_cpu(out: &mut [u8], password: &[u8], salt: &[u8; 16]) {
    let mut data = Vec::with_capacity(128);
    data.extend_from_slice(&ARGON_LANES.to_le_bytes());
    data.extend_from_slice(&32u32.to_le_bytes());
    data.extend_from_slice(&ARGON_MEMORY_KIB.to_le_bytes());
    data.extend_from_slice(&ARGON_TIME_COST.to_le_bytes());
    data.extend_from_slice(&0x13u32.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&(password.len() as u32).to_le_bytes());
    data.extend_from_slice(password);
    data.extend_from_slice(&(salt.len() as u32).to_le_bytes());
    data.extend_from_slice(salt);
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());

    let digest = blake2b_simd::Params::new().hash_length(out.len()).to_state().update(&data).finalize();
    out.copy_from_slice(digest.as_bytes());
}

fn digest_long_cpu(out: &mut [u8], input: &[u8]) {
    let mut data = Vec::with_capacity(4 + input.len());
    data.extend_from_slice(&(out.len() as u32).to_le_bytes());
    data.extend_from_slice(input);

    if out.len() <= 64 {
        let digest = blake2b_simd::Params::new().hash_length(out.len()).to_state().update(&data).finalize();
        out.copy_from_slice(digest.as_bytes());
        return;
    }

    let mut buffer = blake2b_simd::Params::new().hash_length(64).to_state().update(&data).finalize().as_bytes().to_vec();
    let mut produced = 0usize;
    out[..32].copy_from_slice(&buffer[..32]);
    produced += 32;

    while out.len() - produced > 64 {
        buffer = blake2b_simd::Params::new().hash_length(64).to_state().update(&buffer).finalize().as_bytes().to_vec();
        out[produced..produced + 32].copy_from_slice(&buffer[..32]);
        produced += 32;
    }

    let remaining = out.len() - produced;
    let final_digest = blake2b_simd::Params::new().hash_length(remaining).to_state().update(&buffer).finalize();
    out[produced..].copy_from_slice(final_digest.as_bytes());
}

fn hex32(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}
