use crate::Error;
use clap::{ArgGroup, Parser};
use log::LevelFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionMode {
    Rpc,
    Stratum,
}

pub const MIN_DEVFUND_PERCENT: u16 = 100;
pub const DEFAULT_DEVFUND_ADDRESS: &str = "velkar:qpvl5vme9rs8rpewszgx6vmt9xwrr42cqgez54ewgfklgw3kyhgp747mnxcsu";

impl Default for ConnectionMode {
    fn default() -> Self {
        Self::Rpc
    }
}

#[derive(Debug, Parser)]
#[clap(about, version, author)]
#[clap(group(ArgGroup::new("required")))]
pub struct Opt {
    #[clap(short, long, display_order = 3)]
    /// Enable debug logging level
    pub debug: bool,
    #[clap(short = 'a', long = "mining-address", display_order = 0)]
    /// The Velkar address for the miner reward
    pub mining_address: String,
    #[clap(short = 's', long = "velkard-address", default_value = "127.0.0.1", display_order = 1)]
    /// The velkard IP or a stratum endpoint like stratum+tcp://127.0.0.1:4312
    pub velkard_address: String,

    #[clap(long = "devfund", default_value = DEFAULT_DEVFUND_ADDRESS, display_order = 6)]
    /// Devfund address [default: Velkar core fee wallet]
    pub devfund_address: Option<String>,

    #[clap(long = "devfund-percent", default_value = "1", display_order = 7, value_parser = parse_devfund_percent)]
    /// The percentage of blocks to send to the devfund [minimum: 1.00]
    pub devfund_percent: u16,

    #[clap(short, long, display_order = 2)]
    /// Velkard port [default: Mainnet = 26110, Testnet = 26210]
    port: Option<u16>,

    #[clap(long, display_order = 4)]
    /// Use testnet instead of mainnet [default: false]
    testnet: bool,
    #[clap(short = 't', long = "threads", display_order = 5)]
    /// Amount of miner threads to launch [default: number of logical cpus]
    pub num_threads: Option<u16>,
    #[clap(long = "mine-when-not-synced", display_order = 8)]
    /// Mine even when velkard says it is not synced, only useful when passing `--allow-submit-block-when-not-synced` to velkard  [default: false]
    pub mine_when_not_synced: bool,
    #[clap(long = "throttle", display_order = 9)]
    /// Throttle (milliseconds) between each VelkarHash generation (used for development testing)
    pub throttle: Option<u64>,
    #[clap(long, display_order = 10)]
    /// Output logs in alternative format (same as velkard)
    pub altlogs: bool,
    #[clap(long = "user-agent-suffix", display_order = 11)]
    /// Custom user agent suffix (max 20 characters)
    pub user_agent_suffix: Option<String>,

    #[clap(long = "opencl-enable", display_order = 12)]
    /// Enable the OpenCL GPU backend (AMD/NVIDIA)
    pub opencl_enable: bool,
    #[clap(long = "opencl-platform", display_order = 13)]
    /// Choose the OpenCL platform index
    pub opencl_platform: Option<u16>,
    #[clap(long = "opencl-device", display_order = 14)]
    /// Choose the OpenCL device index on the selected platform
    pub opencl_device: Option<u16>,
    #[clap(long = "opencl-workload", display_order = 15)]
    /// GPU batch size in nonces per OpenCL dispatch
    pub opencl_workload: Option<usize>,
    #[clap(long = "opencl-jobs-per-block", display_order = 16)]
    /// OpenCL jobs sharing one workgroup (advanced tuning)
    pub opencl_jobs_per_block: Option<usize>,

    #[clap(long = "cuda-enable", display_order = 17)]
    /// Enable the native NVIDIA CUDA backend
    pub cuda_enable: bool,
    #[clap(long = "cuda-device", display_order = 18)]
    /// Choose the CUDA device index
    pub cuda_device: Option<u16>,
    #[clap(long = "cuda-workload", display_order = 19)]
    /// GPU batch size in nonces per CUDA dispatch
    pub cuda_workload: Option<usize>,

    #[clap(skip)]
    mode: ConnectionMode,
}

fn parse_devfund_percent(s: &str) -> Result<u16, &'static str> {
    let err = "devfund-percent should be --devfund-percent=XX.YY up to 2 numbers after the dot";
    let mut splited = s.split('.');
    let prefix = splited.next().ok_or(err)?;
    // if there's no postfix then it's 0.
    let postfix = splited.next().ok_or(err).unwrap_or("0");
    // error if there's more than a single dot
    if splited.next().is_some() {
        return Err(err);
    };
    // error if there are more than 2 numbers before or after the dot
    if prefix.len() > 2 || postfix.len() > 2 {
        return Err(err);
    }
    let postfix: u16 = postfix.parse().map_err(|_| err)?;
    let prefix: u16 = prefix.parse().map_err(|_| err)?;
    // can't be more than 99.99%,
    if prefix >= 100 || postfix >= 100 {
        return Err(err);
    }
    let parsed = prefix * 100 + postfix;
    Ok(parsed.max(MIN_DEVFUND_PERCENT))
}

impl Opt {
    pub fn process(&mut self) -> Result<(), Error> {
        if self.opencl_enable && self.cuda_enable {
            return Err("Use either --opencl-enable or --cuda-enable, not both".into());
        }
        if self.cuda_enable && !cfg!(feature = "cuda") {
            return Err("This binary was built without CUDA support; rebuild with --features cuda".into());
        }
        if self.velkard_address.is_empty() {
            self.velkard_address = "127.0.0.1".to_string();
        }

        if self.velkard_address.starts_with("stratum+tcp://") {
            self.mode = ConnectionMode::Stratum;
            self.velkard_address = self.velkard_address.trim_start_matches("stratum+tcp://").to_string();
        } else if !self.velkard_address.starts_with("grpc://") {
            self.mode = ConnectionMode::Rpc;
            let port = self.port();
            self.velkard_address = format!("grpc://{}:{}", self.velkard_address, port);
        } else {
            self.mode = ConnectionMode::Rpc;
        }
        log::info!("Connection target: {}", self.velkard_address);

        if let Some(suffix) = &self.user_agent_suffix {
            if suffix.contains('/') {
                return Err("--user-agent-suffix cannot contain '/' characters".into());
            }
            if suffix.chars().count() > 20 {
                return Err("--user-agent-suffix must be at most 20 characters".into());
            }
        }

        Ok(())
    }

    fn port(&mut self) -> u16 {
        *self.port.get_or_insert(if self.testnet { 26210 } else { 26110 })
    }

    pub fn log_level(&self) -> LevelFilter {
        if self.debug {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        }
    }

    pub fn connection_mode(&self) -> ConnectionMode {
        self.mode
    }
}
