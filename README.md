# Velkar Miner

Official VelkarHash miner with CPU, AMD OpenCL and native NVIDIA CUDA support.

- Website: [velkar.org](https://velkar.org)
- Source: [github.com/VelkarVELK/velkar-GPU](https://github.com/VelkarVELK/velkar-GPU)

The miner supports:

- CPU mining on Windows and Linux.
- AMD GPU mining through OpenCL.
- NVIDIA GPU mining through native CUDA.
- Solo mining through the Velkar gRPC node interface.
- Pool mining through `stratum+tcp`.
- Automatic CPU verification of GPU results before a share is submitted.
- A fixed 1% project devfee for continued development.

> GPU backends are under active optimization. Always verify accepted shares and
> stability, not only the displayed hashrate.

## Release Binaries

Windows releases may contain two executables:

| File | Intended hardware |
| --- | --- |
| `velkar-cpuminer.exe` | CPU and AMD/OpenCL |
| `velkar-cudaminer.exe` | NVIDIA/CUDA |

The CUDA executable contains embedded Velkar PTX. Miners only need a compatible,
up-to-date NVIDIA display driver to run a prebuilt release.

## Build From Source

### Requirements

- Rust 1.88 or newer.
- Git.
- Protocol Buffers compiler (`protoc`).
- A C/C++ build toolchain.
- OpenCL runtime and GPU drivers for OpenCL mining.
- NVIDIA CUDA Toolkit for building the native CUDA variant.

### Windows: CPU and OpenCL

Install Rust using [rustup](https://rustup.rs/) and the Visual Studio 2022 C++
build tools, then run:

```powershell
git clone https://github.com/VelkarVELK/velkar-GPU.git
cd velkar-GPU
cargo build --release
```

Output:

```text
target\release\velkar-cpuminer.exe
```

### Windows: Native CUDA

Install the NVIDIA CUDA Toolkit and build with the `cuda` feature:

```powershell
$env:CUDA_PATH="C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.3"
$env:PATH="$env:CUDA_PATH\bin;$env:PATH"
cargo build --release --features cuda
```

The source build keeps the Cargo binary name `velkar-cpuminer.exe`; release
maintainers may rename this CUDA build to `velkar-cudaminer.exe` when packaging.

### Ubuntu/Linux: CPU and OpenCL

```bash
sudo apt update
sudo apt install -y build-essential clang pkg-config libssl-dev \
  protobuf-compiler ocl-icd-opencl-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

git clone https://github.com/VelkarVELK/velkar-GPU.git
cd velkar-GPU
cargo build --release
```

Output:

```text
target/release/velkar-cpuminer
```

## Pool Mining

The value passed to `--mining-address` is the pool username or worker name when
using Stratum.

### CPU

```powershell
velkar-cpuminer.exe `
  --velkard-address stratum+tcp://pool.liquidpool.net:4001 `
  --mining-address YOUR_POOL_USER
```

### AMD/OpenCL

Start with workload `256`:

```powershell
velkar-cpuminer.exe `
  --velkard-address stratum+tcp://pool.liquidpool.net:4001 `
  --mining-address YOUR_POOL_USER `
  --opencl-enable `
  --opencl-device 0 `
  --opencl-workload 256
```

If the computer has multiple OpenCL platforms, select one explicitly:

```text
--opencl-platform 0 --opencl-device 0
```

### NVIDIA/CUDA

Start with workload `256`:

```powershell
velkar-cudaminer.exe `
  --velkard-address stratum+tcp://pool.liquidpool.net:4001 `
  --mining-address YOUR_POOL_USER `
  --cuda-enable `
  --cuda-device 0 `
  --cuda-workload 256
```

At startup, CUDA performs a complete GPU/CPU consensus comparison. Do not mine
if this message is not shown:

```text
CUDA Velkar full pipeline validation passed
```

## Solo Mining

The Velkar node must expose its gRPC port, normally `26110` on mainnet. For solo
mining, `--mining-address` must be a valid Velkar address.

### CPU

```powershell
velkar-cpuminer.exe `
  --velkard-address 127.0.0.1 `
  --port 26110 `
  --mining-address velkar:YOUR_ADDRESS
```

### AMD/OpenCL

```powershell
velkar-cpuminer.exe `
  --velkard-address 127.0.0.1 `
  --port 26110 `
  --mining-address velkar:YOUR_ADDRESS `
  --opencl-enable `
  --opencl-device 0 `
  --opencl-workload 256
```

### NVIDIA/CUDA

```powershell
velkar-cudaminer.exe `
  --velkard-address 127.0.0.1 `
  --port 26110 `
  --mining-address velkar:YOUR_ADDRESS `
  --cuda-enable `
  --cuda-device 0 `
  --cuda-workload 256
```

`--mine-when-not-synced` is intended only for isolated development networks.
Do not use it on mainnet unless the node operator understands and explicitly
allows unsynced block submission.

## GPU Workload Tuning

VelkarHash uses approximately **8 MiB of VRAM per job**. Workload controls how
many nonces are processed in each GPU dispatch. It is not a difficulty setting.

Both GPU backends now scale dynamically instead of enforcing the old fixed
limit of 256 jobs:

- CUDA uses up to approximately half of detected VRAM.
- OpenCL uses the smaller of half the VRAM and the driver's maximum single
  allocation size.
- Both have an absolute safety ceiling of 4096 jobs.
- Values above the detected safe maximum are reduced automatically.

Approximate memory usage:

| Workload | Approximate GPU memory |
| ---: | ---: |
| 64 | 512 MiB |
| 128 | 1 GiB |
| 256 | 2 GiB |
| 512 | 4 GiB |
| 768 | 6 GiB |
| 1024 | 8 GiB |
| 1536 | 12 GiB |
| 2048 | 16 GiB |
| 4096 | 32 GiB |

### Recommended Test Procedure

1. Start with `256`.
2. Mine for at least three to five minutes.
3. Record stable hashrate, accepted shares, rejected shares and power usage.
4. Test `512`, then `768` or `1024` if enough VRAM is available.
5. Keep the fastest stable value, not necessarily the largest value.

Suggested starting points:

| GPU VRAM | Workloads to test |
| ---: | --- |
| 6-8 GB | `128`, `256`, `384` |
| 10-12 GB | `256`, `512`, `768` |
| 16 GB | `256`, `512`, `768`, `1024` |
| 24 GB | `512`, `768`, `1024`, `1536` |
| 32 GB or more | `512`, `1024`, `1536`, `2048` |

A very large workload can reduce effective pool performance because the GPU may
continue processing an old job after the pool publishes a new block template.
It can also make the desktop unresponsive or trigger a driver timeout. Reduce
the workload if jobs stall, the driver resets, shares become stale or hashrate
drops.

## Devfee

The miner includes a fixed 1% project devfee:

- Solo mining routes approximately 1% of work to the project address.
- Stratum GPU mining schedules 99 batches for the selected pool worker and one
  batch for the project worker.
- GPU devfee batches use the same OpenCL or CUDA backend selected by the miner.

## Troubleshooting

### No hashes are processed

- Confirm the node is synchronized or the pool is sending jobs.
- Update GPU drivers.
- Reduce workload to `64` or `128`.
- Verify the correct GPU device index.

### CUDA PTX loading fails

- Use the latest release binary.
- Update the NVIDIA display driver.
- Confirm the file checksum published with the release.

### OpenCL initialization fails

- Install the official AMD or NVIDIA driver rather than a generic Windows
  display driver.
- Use `--opencl-platform` and `--opencl-device` to select the correct device.

### Low-difficulty shares

Do not compensate by increasing workload. Share difficulty is assigned by the
pool and is independent from GPU workload. Check that the miner and pool are
using the same VelkarHash implementation and current network version.

## Security

- Download binaries only from the official Velkar repository or release page.
- Verify published SHA-256 checksums.
- Never enter a wallet seed phrase or private key into the miner.
- The miner only requires a public payout address or pool worker name.

## License

Licensed under MIT or Apache-2.0, as provided in this repository.
