# Velkar native CUDA miner

The CUDA backend is intended for NVIDIA GPUs. AMD GPUs must continue using
the OpenCL backend.

## Build on Windows

Requirements:

- Rust 1.88 or newer (MSVC toolchain)
- Visual Studio 2022 C++ build tools
- NVIDIA CUDA Toolkit 13.x
- A current NVIDIA display driver on the mining computer

```powershell
$env:CUDA_PATH="C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.3"
$env:PATH="$env:CUDA_PATH\bin;$env:PATH"
cargo build --release --features cuda
```

## Solo mining with CUDA

```powershell
.\target\release\velkar-cpuminer.exe `
  --velkard-address 127.0.0.1 `
  --port 26110 `
  --mining-address velkar:YOUR_ADDRESS `
  --cuda-enable `
  --cuda-device 0
```

## Pool mining with CUDA

```powershell
.\target\release\velkar-cpuminer.exe `
  --velkard-address stratum+tcp://pool.liquidpool.net:4001 `
  --mining-address YOUR_WORKER `
  --cuda-enable `
  --cuda-device 0
```

The default CUDA workload is 128 nonces (about 1 GiB of VRAM). The maximum is
calculated dynamically from half of the detected VRAM (8 MiB per job), with an
absolute safety ceiling of 4096 jobs. Test progressively with `--cuda-workload`;
the best value depends on GPU memory bandwidth and pool job frequency. At
startup the miner compares the complete CUDA result with the CPU
consensus implementation. Mining stops if that validation does not match.

The fixed 1% GPU devfund scheduler is shared by OpenCL and CUDA: 99 batches
are sent to the selected worker and one batch to the project worker.
