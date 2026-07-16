use crate::{miner::StratumMinerManager, target, Error, ShutdownHandler};
use log::{info, warn};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::mpsc::{self, error::SendError, Sender},
};

#[derive(Debug)]
pub enum StratumCommand {
    SubmitShare { job_id: String, nonce: String },
}

pub struct StratumHandler {
    writer: Sender<String>,
    submit_tx: Sender<StratumCommand>,
    worker_name: String,
    request_id: AtomicU64,
    current_difficulty: Option<String>,
}

impl StratumHandler {
    pub async fn connect(
        address: String,
        mining_address: String,
        user_agent_suffix: Option<String>,
    ) -> Result<
        (Self, tokio::io::Lines<BufReader<tokio::net::tcp::OwnedReadHalf>>, mpsc::Receiver<StratumCommand>),
        Error,
    > {
        let stream = TcpStream::connect(&address).await?;
        let (reader, writer) = stream.into_split();
        let (writer_tx, mut writer_rx) = mpsc::channel::<String>(64);
        let (submit_tx, submit_rx) = mpsc::channel::<StratumCommand>(64);
        let writer_tx_clone = writer_tx.clone();

        let mut writer = writer;
        tokio::spawn(async move {
            while let Some(line) = writer_rx.recv().await {
                if writer.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                if writer.write_all(b"\n").await.is_err() {
                    break;
                }
            }
        });

        let handler = Self {
            writer: writer_tx_clone,
            submit_tx,
            worker_name: mining_address.clone(),
            request_id: AtomicU64::new(1),
            current_difficulty: None,
        };

        let user_agent = match user_agent_suffix {
            Some(suffix) => format!("{}/{}", env!("CARGO_PKG_VERSION"), suffix),
            None => env!("CARGO_PKG_VERSION").to_string(),
        };
        info!("Using user agent: {}, specify --user-agent-suffix to customize", user_agent);

        handler
            .send_raw(json!({
                "id": handler.next_id(),
                "method": "mining.subscribe",
                "params": [user_agent]
            }))
            .await?;

        handler
            .send_raw(json!({
                "id": handler.next_id(),
                "method": "mining.authorize",
                "params": [handler.worker_name, "x"]
            }))
            .await?;

        Ok((handler, BufReader::new(reader).lines(), submit_rx))
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::Relaxed)
    }

    pub async fn send_command(&self, cmd: StratumCommand) -> Result<(), SendError<String>> {
        match cmd {
            StratumCommand::SubmitShare { job_id, nonce } => {
                let req = json!({
                    "id": self.next_id(),
                    "method": "mining.submit",
                    "params": [self.worker_name, job_id, nonce]
                });
                self.writer.send(req.to_string()).await
            }
        }
    }

    async fn send_raw(&self, value: Value) -> Result<(), SendError<String>> {
        self.writer.send(value.to_string()).await
    }

    pub fn submit_channel(&self) -> Sender<StratumCommand> {
        self.submit_tx.clone()
    }

    pub async fn listen(
        &mut self,
        lines: &mut tokio::io::Lines<BufReader<tokio::net::tcp::OwnedReadHalf>>,
        miner: &mut StratumMinerManager,
        submit_rx: &mut mpsc::Receiver<StratumCommand>,
        shutdown: ShutdownHandler,
    ) -> Result<(), Error> {
        loop {
            tokio::select! {
                maybe_cmd = submit_rx.recv() => {
                    match maybe_cmd {
                        Some(cmd) => {
                            self.send_command(cmd).await?;
                        }
                        None => break,
                    }
                }
                maybe_line = lines.next_line() => {
                    let Some(line) = maybe_line? else {
                        break;
                    };
                    self.handle_line(&line, miner).await?;
                }
            }

            if shutdown.is_shutdown() {
                break;
            }
        }

        Ok(())
    }

    async fn handle_line(&mut self, line: &str, miner: &mut StratumMinerManager) -> Result<(), Error> {
        let value: Value = serde_json::from_str(line)?;

        if let Some(method) = value.get("method").and_then(Value::as_str) {
            match method {
                "mining.set_difficulty" => {
                    if let Some(diff_value) = value.get("params").and_then(Value::as_array).and_then(|x| x.first()) {
                        let diff = diff_value.to_string();
                        if let Some(share_target) = target::u256_from_stratum_difficulty_str(&diff) {
                            info!("Pool difficulty set to {} (share_target={})", diff, share_target.to_be_hex());
                        } else {
                            warn!("Pool difficulty set to {}, but the miner could not convert it to a target", diff);
                        }
                        self.current_difficulty = Some(diff.clone());
                    }
                }
                "set_extranonce" => {
                    info!("Received extranonce parameters from stratum");
                }
                "mining.notify" => {
                    let Some(params) = value.get("params").and_then(Value::as_array) else {
                        return Ok(());
                    };

                    let Some(job_id) = params.first().and_then(Value::as_str) else {
                        return Ok(());
                    };

                    let Some(diff) = self.current_difficulty.as_ref() else {
                        warn!("Ignoring job {} because pool difficulty has not been set yet", job_id);
                        return Ok(());
                    };

                    let fallback_share_target = match target::u256_from_stratum_difficulty_str(diff) {
                        Some(target) => target,
                        None => {
                            warn!("Failed to parse pool difficulty {}", diff);
                            return Ok(());
                        }
                    };

                    let (pre_pow_hash, timestamp, block_target, job_share_target) =
                        if let Some(words) = params.get(1).and_then(Value::as_array) {
                            if words.len() != 4 {
                                warn!("Unsupported job payload length for {}", job_id);
                                return Ok(());
                            }

                            let mut bytes = [0u8; 32];
                            for (i, word) in words.iter().enumerate() {
                                let value = word.as_u64().ok_or("invalid job word")?;
                                bytes[i * 8..(i + 1) * 8].copy_from_slice(&value.to_le_bytes());
                            }
                            let ts = params.get(2).and_then(Value::as_i64).ok_or("missing timestamp")? as u64;
                            let block_target = parse_target_hex(params.get(3)).ok_or("missing block target")?;
                            let job_share_target = parse_target_hex(params.get(4));
                            (crate::target::Uint256::from_le_bytes(bytes), ts, block_target, job_share_target)
                        } else if let Some(large_job) = params.get(1).and_then(Value::as_str) {
                            if large_job.len() < 80 {
                                warn!("Unsupported large-job payload for {}", job_id);
                                return Ok(());
                            }

                            let hash_hex = &large_job[..64];
                            let ts_hex = &large_job[64..80];
                            let mut bytes = [0u8; 32];
                            for i in 0..32 {
                                let pair = &hash_hex[(i * 2)..(i * 2 + 2)];
                                bytes[i] = u8::from_str_radix(pair, 16)?;
                            }
                            let ts = u64::from_str_radix(ts_hex, 16)?;
                            let block_target = parse_target_hex(params.get(2)).ok_or("missing block target")?;
                            let job_share_target = parse_target_hex(params.get(3));
                            (crate::target::Uint256::from_le_bytes(bytes), ts, block_target, job_share_target)
                        } else {
                            warn!("Unsupported job format for {}", job_id);
                            return Ok(());
                        };
                    let share_target = job_share_target.unwrap_or(fallback_share_target);

                    info!(
                        "Stratum job {}: diff={} pre_pow_hash={:x} timestamp={} share_target={} block_target={} share_target_source={}",
                        job_id,
                        diff,
                        pre_pow_hash,
                        timestamp,
                        share_target.to_be_hex(),
                        block_target.to_be_hex(),
                        if job_share_target.is_some() { "job" } else { "difficulty" }
                    );

                    miner.process_job(job_id.to_string(), pre_pow_hash, timestamp, block_target, share_target);
                }
                _ => {}
            }
        } else if value.get("error").is_some() && !value.get("error").unwrap().is_null() {
            warn!("Stratum error: {}", value["error"]);
        } else if value.get("result").is_some() {
            let result = &value["result"];
            if result == true {
                info!("Share accepted by stratum pool");
            }
        }

        Ok(())
    }
}

fn parse_target_hex(value: Option<&Value>) -> Option<crate::target::Uint256> {
    let text = value?.as_str()?.trim();
    let hex = text.strip_prefix("0x").unwrap_or(text);
    if hex.is_empty() || hex.len() > 64 {
        return None;
    }

    let padded = format!("{hex:0>64}");
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        let pair = &padded[(i * 2)..(i * 2 + 2)];
        bytes[31 - i] = u8::from_str_radix(pair, 16).ok()?;
    }

    Some(crate::target::Uint256::from_le_bytes(bytes))
}
