# Velkar CPU Miner (Testnets)

## Installation

### From Binaries
The [release page] includes precompiled binaries for Linux, macOS and Windows.

# Usage
To start mining you need to run [velkard] and have an address to send the rewards to.
See the Rusty Velkar testnet docs for running a full node and generating addresses: 

### Help:
```
A Velkar high performance CPU miner

Usage: velkar-miner [OPTIONS] --mining-address <MINING_ADDRESS>

Options:
  -a, --mining-address <MINING_ADDRESS>
          The Velkar address for the miner reward
  -s, --velkard-address <VELKARD_ADDRESS>
          The IP of the velkard instance [default: 127.0.0.1]
  -p, --port <PORT>
          Velkard port [default: Mainnet = 16110, Testnet = 16210]
  -d, --debug
          Enable debug logging level
      --testnet
          Use testnet instead of mainnet [default: false]
  -t, --threads <NUM_THREADS>
          Amount of miner threads to launch [default: number of logical cpus]
      --devfund <DEVFUND_ADDRESS>
          Devfund address [default: Velkar core fee wallet]
      --devfund-percent <DEVFUND_PERCENT>
          The percentage of blocks to send to the devfund [default: 1, minimum: 1]
      --mine-when-not-synced
          Mine even when velkard says it is not synced, only useful when passing `--allow-submit-block-when-not-synced` to velkard  [default: false]
      --throttle <THROTTLE>
          Throttle (milliseconds) between each VelkarHash generation (used for development testing)
      --altlogs
          Output logs in alternative format (same as velkard)
  -h, --help
          Print help
  -V, --version
          Print version
```

### Running

`./velkar-miner --testnet --mining-address velkar:XXXXX`

This will run the miner on all the available CPU cores. Requires a testnet Velkard on localhost.

### Docker

`docker run --rm velkarnet/cpuminer --testnet -s 123.123.123.123 -a velkar:XXXXX`

Supply a valid testnet node with an open GRPC port to the -s parameter.

### Docker Compose

Create docker-compose.yaml:
```yaml
services:

  velkar_miner_testnet_10:
    container_name: velkar_miner_testnet_10
    image: velkarnet/cpuminer
    restart: unless-stopped
    cpus: 0.1 # Increase if necessary, remove to use all cores
    command: --testnet -s 123.123.123.123 -a velkar:XXXXX

  velkar_miner_testnet_12:
    container_name: velkar_miner_testnet_12
    image: velkarnet/cpuminer
    restart: unless-stopped
    cpus: 0.1 # Increase if necessary, remove to use all cores
    command: --testnet -s 321.321.321.321 -a velkar:XXXXX
```

Run in same directory:
`docker compose up -d`
