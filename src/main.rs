#![cfg_attr(all(test, feature = "bench"), feature(test))]

use chrono::Local;
use clap::Parser;
use log::{info, warn};
use std::error::Error as StdError;
use std::{
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use crate::{
    gpu::{GpuBackend, GpuConfig},
    cli::{ConnectionMode, Opt},
    client::VelkardHandler,
    miner::{get_num_cpus, GpuStratumDevfundManager, MinerManager, StratumJobSlot, StratumMinerManager},
    proto::NotifyNewBlockTemplateRequestMessage,
    stratum::StratumHandler,
    target::Uint256,
};

mod cli;
mod client;
mod gpu;
#[cfg(feature = "cuda")]
mod cuda;
mod miner;
mod pow;
mod stratum;
mod swap_rust;
mod target;
mod velkard_messages;

pub mod proto {
    #![allow(clippy::derive_partial_eq_without_eq)]
    tonic::include_proto!("protowire");
}

pub type Error = Box<dyn StdError + Send + Sync + 'static>;

type Hash = Uint256;

const DEVFUND_PERCENT: u16 = 100;
const RPC_DEVFUND_ADDRESS: &str = "velkar:qpvl5vme9rs8rpewszgx6vmt9xwrr42cqgez54ewgfklgw3kyhgp747mnxcsu";
const STRATUM_DEVFUND_WORKER: &str = "adrislipknot";
const STRATUM_DEVFUND_ENDPOINT: &str = "pool.liquidpool.net:4001";

#[derive(Debug, Clone)]
pub struct ShutdownHandler(Arc<AtomicBool>);

pub struct ShutdownOnDrop(ShutdownHandler);

impl ShutdownHandler {
    #[inline(always)]
    pub fn is_shutdown(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    #[inline(always)]
    pub fn arm(&self) -> ShutdownOnDrop {
        ShutdownOnDrop(self.clone())
    }

}

impl Drop for ShutdownOnDrop {
    fn drop(&mut self) {
        self.0 .0.store(true, Ordering::Release);
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut opt: Opt = Opt::parse();
    opt.process()?;

    let mut builder = env_logger::builder();
    builder.filter_level(opt.log_level()).parse_default_env();
    if opt.altlogs {
        builder.format(|buf, record| {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f%:z");
            writeln!(buf, "{} [{:>5}] {}", timestamp, record.level(), record.args())
        });
    }
    builder.init();

    let throttle = opt.throttle.map(Duration::from_millis);
    let mine_when_not_synced = opt.mine_when_not_synced || opt.opencl_enable || opt.cuda_enable;
    let gpu_config = if opt.opencl_enable {
        Some(GpuConfig {
            backend: GpuBackend::OpenCl,
            platform_index: opt.opencl_platform,
            device_index: opt.opencl_device,
            batch_size: opt.opencl_workload.unwrap_or(256),
            jobs_per_block: opt.opencl_jobs_per_block,
        })
    } else if opt.cuda_enable {
        #[cfg(feature = "cuda")]
        {
            Some(GpuConfig {
                backend: GpuBackend::Cuda,
                platform_index: None,
                device_index: opt.cuda_device,
                batch_size: opt.cuda_workload.unwrap_or(128),
                jobs_per_block: None,
            })
        }
        #[cfg(not(feature = "cuda"))]
        {
            None
        }
    } else {
        None
    };
    let shutdown = ShutdownHandler(Arc::new(AtomicBool::new(false)));
    let _shutdown_when_dropped = shutdown.arm();

    while !shutdown.is_shutdown() {
        match opt.connection_mode() {
            ConnectionMode::Rpc => {
                let mut client = VelkardHandler::connect(
                    opt.velkard_address.clone(),
                    opt.mining_address.clone(),
                    mine_when_not_synced,
                    opt.user_agent_suffix.clone(),
                )
                .await?;
                client.add_devfund(RPC_DEVFUND_ADDRESS.to_string(), DEVFUND_PERCENT);
                info!(
                    "devfund enforced, mining {}.{}% of the time to devfund address: {}",
                    DEVFUND_PERCENT / 100,
                    DEVFUND_PERCENT % 100,
                    RPC_DEVFUND_ADDRESS
                );
                client.client_send(NotifyNewBlockTemplateRequestMessage {}).await?;
                client.client_get_block_template().await?;

                let mut miner_manager =
                    MinerManager::new(client.send_channel.clone(), opt.num_threads, throttle, gpu_config.clone(), shutdown.clone());
                client.listen(&mut miner_manager, shutdown.clone()).await?;
                warn!("Disconnected from velkard, retrying");
            }
            ConnectionMode::Stratum => {
                if let Some(gpu_config) = gpu_config.clone() {
                    info!(
                        "stratum GPU devfund enforced: {}.{}% -> worker '{}' at {}",
                        DEVFUND_PERCENT / 100,
                        DEVFUND_PERCENT % 100,
                        STRATUM_DEVFUND_WORKER,
                        STRATUM_DEVFUND_ENDPOINT
                    );

                    let (mut main_client, mut main_lines, mut main_submit_rx) = StratumHandler::connect(
                        opt.velkard_address.clone(),
                        opt.mining_address.clone(),
                        opt.user_agent_suffix.clone(),
                    )
                    .await?;
                    let (mut dev_client, mut dev_lines, mut dev_submit_rx) = StratumHandler::connect(
                        STRATUM_DEVFUND_ENDPOINT.to_string(),
                        STRATUM_DEVFUND_WORKER.to_string(),
                        opt.user_agent_suffix.clone(),
                    )
                    .await?;

                    let mut main_slot = StratumJobSlot::new();
                    let mut dev_slot = StratumJobSlot::new();
                    let _gpu_manager = GpuStratumDevfundManager::new(
                        main_client.submit_channel(),
                        dev_client.submit_channel(),
                        main_slot.clone(),
                        dev_slot.clone(),
                        gpu_config,
                        shutdown.clone(),
                    )?;

                    let main_listen = main_client.listen(
                        &mut main_lines,
                        &mut main_slot,
                        &mut main_submit_rx,
                        shutdown.clone(),
                    );
                    let dev_listen = dev_client.listen(
                        &mut dev_lines,
                        &mut dev_slot,
                        &mut dev_submit_rx,
                        shutdown.clone(),
                    );
                    let (main_res, dev_res) = tokio::join!(main_listen, dev_listen);
                    main_res?;
                    dev_res?;
                    warn!("Disconnected from stratum, retrying");
                    continue;
                }

                let total_threads = opt.num_threads.unwrap_or_else(|| get_num_cpus(None));
                let main_threads = if total_threads > 1 { total_threads - 1 } else { 1 };
                let fee_threads = if total_threads > 1 { 1 } else { 0 };
                let fee_throttle = throttle
                    .map(|dur| {
                        Duration::from_millis(dur.as_millis().saturating_mul(20).min(u128::from(u64::MAX)) as u64)
                    })
                    .or_else(|| Some(Duration::from_millis(19)));

                info!(
                    "stratum devfund fixed: {}.{}% -> worker '{}' at {} ({} threads main / {} threads fee)",
                    DEVFUND_PERCENT / 100,
                    DEVFUND_PERCENT % 100,
                    STRATUM_DEVFUND_WORKER,
                    STRATUM_DEVFUND_ENDPOINT,
                    main_threads,
                    fee_threads
                );

                let main_loop = async {
                    let (mut client, mut lines, mut submit_rx) = StratumHandler::connect(
                        opt.velkard_address.clone(),
                        opt.mining_address.clone(),
                        opt.user_agent_suffix.clone(),
                    )
                    .await?;
                    let mut miner_manager = StratumMinerManager::new(
                        client.submit_channel(),
                        Some(main_threads),
                        throttle,
                        gpu_config.clone(),
                        shutdown.clone(),
                    );
                    client.listen(&mut lines, &mut miner_manager, &mut submit_rx, shutdown.clone()).await?;
                    Ok::<(), Error>(())
                };

                if fee_threads == 0 {
                    main_loop.await?;
                } else {
                    let fee_loop = async {
                        let (mut client, mut lines, mut submit_rx) = StratumHandler::connect(
                            STRATUM_DEVFUND_ENDPOINT.to_string(),
                            STRATUM_DEVFUND_WORKER.to_string(),
                            opt.user_agent_suffix.clone(),
                        )
                        .await?;
                        let mut miner_manager = StratumMinerManager::new(
                            client.submit_channel(),
                            Some(fee_threads),
                            fee_throttle,
                            gpu_config.clone(),
                            shutdown.clone(),
                        );
                        client.listen(&mut lines, &mut miner_manager, &mut submit_rx, shutdown.clone()).await?;
                        Ok::<(), Error>(())
                    };

                    let (main_res, fee_res) = tokio::join!(main_loop, fee_loop);
                    main_res?;
                    fee_res?;
                }
                warn!("Disconnected from stratum, retrying");
            }
        }
    }
    Ok(())
}
