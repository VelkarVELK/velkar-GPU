use crate::{
    gpu::{GpuConfig, GpuHit},
    pow,
    target::Uint256,
    Error, Hash,
};
use cust::{launch, prelude::*};
use log::{debug, info};

const ARGON_MEMORY_KIB: usize = 8 * 1024;
const ARGON2_BLOCK_BYTES: usize = 1024;
const MEMORY_PER_JOB: usize = ARGON_MEMORY_KIB * ARGON2_BLOCK_BYTES;
const JOB_INPUT_BYTES: usize = 72 + 16 + 8 + 32 + 32 + 32;
const CONSTANTS_BYTES: usize = 72 + 64 * 64 * 2;

pub struct CudaSearcher {
    _context: Context,
    module: Module,
    stream: Stream,
    memory: DeviceBuffer<u8>,
    constants: DeviceBuffer<u8>,
    job_inputs: DeviceBuffer<u8>,
    stage4: DeviceBuffer<u8>,
    nonces: DeviceBuffer<u64>,
    shuffle_scratch: DeviceBuffer<u8>,
    batch_size: usize,
    validation_done: bool,
}

impl CudaSearcher {
    pub fn new(config: &GpuConfig) -> Result<Self, Error> {
        cust::init(CudaFlags::empty()).map_err(|e| format!("CUDA driver initialization failed: {e}"))?;
        let device_index = config.device_index.unwrap_or(0) as u32;
        let device = Device::get_device(device_index)
            .map_err(|e| format!("CUDA device #{device_index} is not available: {e}"))?;
        let name = device.name().unwrap_or_else(|_| "Unknown NVIDIA GPU".into());
        let total_memory = device.total_memory().map_err(|e| e.to_string())?;
        info!(
            "CUDA GPU detected: {name} (device #{device_index}, {} MiB VRAM); creating context",
            total_memory / (1024 * 1024)
        );
        let context = Context::new(device).map_err(|e| format!("CUDA context creation failed: {e}"))?;
        info!("CUDA context created; loading embedded Velkar PTX");
        let module = Module::from_ptx(include_str!("velkar_cuda.ptx"), &[])
            .map_err(|e| format!("CUDA PTX loading failed: {e}"))?;
        let stream = Stream::new(StreamFlags::NON_BLOCKING, None)?;

        // Keep at least half of VRAM free for the driver, desktop and temporary allocations.
        // Scale with the card instead of imposing the initial 256-job test cap.
        // Half of VRAM remains available to the display driver and the system.
        let max_by_memory = (total_memory / 2 / MEMORY_PER_JOB).clamp(1, 4096);
        let batch_size = config.batch_size.max(1).min(max_by_memory);
        info!(
            "CUDA allocating {} MiB for {} Velkar jobs",
            MEMORY_PER_JOB * batch_size / (1024 * 1024),
            batch_size
        );
        let memory = DeviceBuffer::<u8>::zeroed(MEMORY_PER_JOB * batch_size)?;
        let constants = DeviceBuffer::<u8>::zeroed(CONSTANTS_BYTES)?;
        let job_inputs = DeviceBuffer::<u8>::zeroed(JOB_INPUT_BYTES * batch_size)?;
        let stage4 = DeviceBuffer::<u8>::zeroed(32 * batch_size)?;
        let nonces = DeviceBuffer::<u64>::zeroed(batch_size)?;
        // CUDA shuffle uses warp intrinsics. This pointer only preserves the shared
        // OpenCL/CUDA kernel ABI and is intentionally not read by the CUDA path.
        let shuffle_scratch = DeviceBuffer::<u8>::zeroed(256)?;

        info!(
            "CUDA GPU selected: {name} (device #{device_index}, {} MiB VRAM)",
            total_memory / (1024 * 1024)
        );
        info!(
            "{name}: native CUDA Velkar pipeline active: batch={} jobs, {} MiB/job, {} MiB total",
            batch_size,
            MEMORY_PER_JOB / (1024 * 1024),
            MEMORY_PER_JOB * batch_size / (1024 * 1024)
        );

        Ok(Self {
            _context: context,
            module,
            stream,
            memory,
            constants,
            job_inputs,
            stage4,
            nonces,
            shuffle_scratch,
            batch_size,
            validation_done: false,
        })
    }

    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    pub fn search(
        &mut self,
        state: &pow::State,
        nonce_base: u64,
        check_target: Uint256,
    ) -> Result<Vec<GpuHit>, Error> {
        let mut constants = vec![0u8; CONSTANTS_BYTES];
        constants[..72].copy_from_slice(&state.gpu_hash_header());
        constants[72..].copy_from_slice(&state.gpu_matrix_u16_le_bytes());
        self.constants.copy_from(&constants)?;

        let target = state.block_target().to_le_u64();
        let prepare = self.module.get_function("velkar_prepare_jobs")?;
        let init = self.module.get_function("velkar_init_first_blocks")?;
        let argon = self.module.get_function("argon2_kernel_oneshot")?;
        let export = self.module.get_function("velkar_export_stage4")?;
        // The OpenCL-derived helper kernels do not receive a batch-length
        // argument. Select an exact divisor so CUDA never launches an
        // out-of-bounds helper thread, even for workloads larger than 256.
        let mut helper_block = self.batch_size.min(256);
        while self.batch_size % helper_block != 0 {
            helper_block -= 1;
        }
        let grid_1d = ((self.batch_size / helper_block) as u32, 1, 1);
        let block_1d = (helper_block as u32, 1, 1);
        let stream = &self.stream;

        unsafe {
            launch!(prepare<<<grid_1d, block_1d, 0, stream>>>(
                self.job_inputs.as_device_ptr(),
                self.constants.as_device_ptr(),
                JOB_INPUT_BYTES as u64,
                nonce_base,
                u64::MAX,
                0u64,
                target[0], target[1], target[2], target[3]
            ))?;
            launch!(init<<<grid_1d, block_1d, 0, stream>>>(
                self.memory.as_device_ptr(),
                self.job_inputs.as_device_ptr(),
                MEMORY_PER_JOB as u64,
                JOB_INPUT_BYTES as u64
            ))?;
            // One 32-thread CUDA warp owns one Argon2 job. Grid Y selects the nonce.
            launch!(argon<<<(1, self.batch_size as u32, 1), (32, 1, 1), 0, stream>>>(
                self.shuffle_scratch.as_device_ptr(),
                self.memory.as_device_ptr(),
                1u32,
                1u32,
                (ARGON_MEMORY_KIB / 4) as u32
            ))?;
            launch!(export<<<grid_1d, block_1d, 0, stream>>>(
                self.memory.as_device_ptr(),
                self.job_inputs.as_device_ptr(),
                self.stage4.as_device_ptr(),
                self.nonces.as_device_ptr(),
                MEMORY_PER_JOB as u64,
                JOB_INPUT_BYTES as u64
            ))?;
        }
        self.stream.synchronize()?;

        let mut job_inputs = vec![0u8; JOB_INPUT_BYTES * self.batch_size];
        let mut stage4 = vec![0u8; 32 * self.batch_size];
        let mut nonces = vec![0u64; self.batch_size];
        self.job_inputs.copy_to(&mut job_inputs)?;
        self.stage4.copy_to(&mut stage4)?;
        self.nonces.copy_to(&mut nonces)?;

        let mut hits = Vec::new();
        for (i, nonce) in nonces.into_iter().enumerate() {
            let input = &job_inputs[i * JOB_INPUT_BYTES..(i + 1) * JOB_INPUT_BYTES];
            let stage1 = Hash::from_le_bytes(input[96..128].try_into().expect("stage1 length"));
            let stage2 = Hash::from_le_bytes(input[128..160].try_into().expect("stage2 length"));
            let stage3 = Hash::from_le_bytes(input[160..192].try_into().expect("stage3 length"));
            let gpu_stage4 = Hash::from_le_bytes(stage4[i * 32..(i + 1) * 32].try_into().expect("stage4 length"));

            if i == 0 && !self.validation_done {
                let cpu_stage1 = state.stage1_for_nonce(nonce);
                let cpu_stage2 = state.stage2_for_stage1(cpu_stage1);
                let cpu_stage3 = state.stage3_for_stage2(cpu_stage2);
                let cpu_stage4 = state.cpu_stage4_from_parts(cpu_stage1, cpu_stage3, nonce);
                if (stage1, stage2, stage3, gpu_stage4) != (cpu_stage1, cpu_stage2, cpu_stage3, cpu_stage4) {
                    return Err(format!(
                        "CUDA Velkar startup validation failed at nonce {nonce:016x}: GPU stage4={} CPU stage4={}",
                        gpu_stage4.to_be_hex(),
                        cpu_stage4.to_be_hex()
                    )
                    .into());
                }
                info!("CUDA Velkar full pipeline validation passed for nonce {nonce:016x}");
                self.validation_done = true;
            }

            let final_pow = state.calculate_pow_from_gpu_stage4(stage1, stage2, stage3, gpu_stage4, nonce);
            if final_pow <= check_target {
                hits.push(GpuHit { nonce, pow: final_pow });
            }
        }
        if !hits.is_empty() {
            debug!("CUDA/CPU-final found {} valid share candidate(s)", hits.len());
        }
        Ok(hits)
    }
}
