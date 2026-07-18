# Velkar Miner Usage and Performance Guide

This guide covers CPU, AMD/OpenCL and NVIDIA/CUDA mining with the official
Velkar miner.

Official downloads: <https://github.com/VelkarVELK/velkar-GPU/releases/tag/miner>

## Important Safety Checks

- Download miners only from the official Velkar repository.
- Compare the SHA256 checksum before running an executable.
- Keep GPU drivers updated.
- GPU results are checked against the CPU consensus implementation before a
  share is submitted.
- Stop mining if the required GPU validation message does not appear.
- The miner contains a fixed 1% project devfee.

## Pool Mining

When using Stratum, `--mining-address` is the username, worker name or wallet
required by the selected pool.

### CPU

```powershell
velkar-amd.exe `
  --velkard-address stratum+tcp://POOL_HOST:PORT `
  --mining-address YOUR_POOL_USER
```

### AMD/OpenCL

```powershell
velkar-amd.exe `
  --velkard-address stratum+tcp://POOL_HOST:PORT `
  --mining-address YOUR_POOL_USER `
  --opencl-enable `
  --opencl-device 0 `
  --opencl-workload 256
```

If several OpenCL platforms are installed, select one explicitly:

```text
--opencl-platform 0 --opencl-device 0
```

Required startup messages include:

```text
OpenCL Argon2 initial blocks match CPU reference
OpenCL Argon2 memory matches CPU reference
OpenCL Velkar full pipeline validation passed
```

### NVIDIA/CUDA

Use a small workload for the first correctness test:

```powershell
velkar-cudaminer.exe `
  --velkard-address stratum+tcp://POOL_HOST:PORT `
  --mining-address YOUR_POOL_USER `
  --cuda-enable `
  --cuda-device 0 `
  --cuda-workload 16
```

Do not continue mining unless this message appears:

```text
CUDA Velkar full pipeline validation passed
```

After validation succeeds, tune the workload as described below.

## Solo Mining

The Velkar node must be synchronized and expose its gRPC port, normally
`26110` on mainnet. `--mining-address` must be a valid Velkar address.

### CPU

```powershell
velkar-amd.exe `
  --velkard-address 127.0.0.1 `
  --port 26110 `
  --mining-address velkar:YOUR_ADDRESS
```

### AMD/OpenCL

```powershell
velkar-amd.exe `
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
  --cuda-workload 128
```

Do not use `--mine-when-not-synced` on mainnet. It is intended only for
isolated development networks.

## Understanding Workload

Workload is the number of Velkar jobs processed in one GPU batch. It is not
pool difficulty and it does not directly control GPU utilization.

Each Velkar job uses approximately 8 MiB of VRAM:

| Workload | Approximate VRAM |
| ---: | ---: |
| 64 | 512 MiB |
| 128 | 1 GiB |
| 256 | 2 GiB |
| 512 | 4 GiB |
| 768 | 6 GiB |
| 1024 | 8 GiB |
| 1280 | 10 GiB |
| 1536 | 12 GiB |
| 2048 | 16 GiB |

Defaults when the option is omitted:

- AMD/OpenCL: `256`
- NVIDIA/CUDA: `128`

The miner automatically reduces a requested workload when it exceeds the
detected safe allocation. Read the startup log to see the effective value:

```text
CUDA allocating ... MiB for ... Velkar jobs
```

or:

```text
full OpenCL Velkar pipeline active: batch=... jobs
```

## Finding the Best Hashrate

Do not jump directly to the largest possible workload. A larger batch can
increase raw hashrate but also increase latency, stale work and driver resets.

Use this procedure:

1. Confirm that full GPU/CPU pipeline validation passes.
2. Close other GPU-intensive applications.
3. Test one workload for at least five minutes.
4. Record stable hashrate, accepted shares, rejected shares, power and
   temperature.
5. Increase only one step at a time.
6. Restart the miner between tests.
7. Keep the setting with the best stable accepted-share rate, not simply the
   highest momentary hashrate.

Suggested workload sequence:

| GPU VRAM | Values to test |
| ---: | --- |
| 6-8 GB | `128`, `256`, `384` |
| 10-12 GB | `256`, `512`, `768` |
| 16 GB | `256`, `512`, `768`, `1024` |
| 24 GB | `512`, `768`, `1024`, `1280` |
| 32 GB or more | `512`, `1024`, `1536`, `2048` |

Stop increasing the workload when any of these occurs:

- Stable hashrate decreases.
- Accepted shares per hour decrease.
- Rejected or stale shares increase.
- The pool changes jobs but the miner responds slowly.
- The display freezes or the driver resets.
- GPU memory allocation fails.

GPU utilization shown by Windows Task Manager may be misleading because it can
display the 3D engine instead of the Compute/CUDA/OpenCL engine. Use the GPU
vendor tools and accepted pool shares as additional measurements.

## Stratum Extranonce Support

The miner supports both `set_extranonce` and `mining.set_extranonce`. The pool's
assigned `extranonce1` is embedded in every submitted 64-bit nonce. A successful
configuration looks like:

```text
Stratum extranonce configured: extranonce1=fe93 extranonce2_size=6
```

The submitted nonce should then begin with the assigned prefix, for example
`fe93...`. If the pool reports `incorrect extranonce1`, update to the latest
official release and verify its SHA256 checksum.

## Troubleshooting

### No hashes are processed

- Verify that the pool is sending jobs or that the node is synchronized.
- Confirm the correct GPU device and platform indexes.
- Update the GPU driver.
- Reduce workload to `64` or `128`.

### CUDA validation fails

- Stop the miner; invalid GPU results must not be submitted.
- Retry with `--cuda-workload 16`.
- Verify that the latest official CUDA executable is being used.
- Send the complete startup log, GPU model, driver version and executable
  SHA256 to the Velkar developers.

### Low-difficulty shares

Do not compensate by increasing workload. Pool share difficulty and GPU
workload are independent. Verify the pool endpoint, current job, extranonce and
that the latest miner version is running.

### Too few shares

Shares are probabilistic. Compare results over a meaningful interval and check
the pool dashboard. A high displayed hashrate without accepted shares is not a
valid performance result.
