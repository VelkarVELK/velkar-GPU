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
    cli::{ConnectionMode, Opt},
    client::VelkardHandler,
    miner::{get_num_cpus, MinerManager, StratumMinerManager},
    proto::NotifyNewBlockTemplateRequestMessage,
    stratum::StratumHandler,
    target::Uint256,
};

mod cli;
mod client;
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
const STRATUM_DEVFUND_ENDPOINT: &str = "stratum+tcp://pool.liquidpool.net:4001";

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
    let shutdown = ShutdownHandler(Arc::new(AtomicBool::new(false)));
    let _shutdown_when_dropped = shutdown.arm();

    while !shutdown.is_shutdown() {
        match opt.connection_mode() {
            ConnectionMode::Rpc => {
                let mut client = VelkardHandler::connect(
                    opt.velkard_address.clone(),
                    opt.mining_address.clone(),
                    opt.mine_when_not_synced,
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
                    MinerManager::new(client.send_channel.clone(), opt.num_threads, throttle, shutdown.clone());
                client.listen(&mut miner_manager, shutdown.clone()).await?;
                warn!("Disconnected from velkard, retrying");
            }
            ConnectionMode::Stratum => {
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
