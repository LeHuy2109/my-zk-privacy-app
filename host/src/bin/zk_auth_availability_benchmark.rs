use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;
use std::{collections::BTreeMap, time::Instant};

use host::zk_auth;

#[derive(Parser, Debug)]
#[command(
    name = "zk_auth_availability_benchmark",
    about = "Availability benchmark for traditional and ZK-auth record writes"
)]
struct Cli {
    #[arg(long, env = "ZK_AUTH_BENCH_N", default_value_t = 10)]
    n: usize,

    #[arg(long, default_value_t = false)]
    groth16: bool,

    #[arg(long, default_value = "both")]
    mode: String,
}

#[derive(Serialize, Clone, Debug)]
struct AvailabilityMetrics {
    mode: String,
    success_count: usize,
    failure_count: usize,
    success_rate_percent: f64,
    average_latency_seconds: Option<f64>,
    p50_latency_seconds: Option<f64>,
    p95_latency_seconds: Option<f64>,
    p99_latency_seconds: Option<f64>,
    throughput_tx_per_second: Option<f64>,
    average_gas_used: Option<f64>,
    retry_count: usize,
    error_breakdown: BTreeMap<String, usize>,
}

#[derive(Serialize)]
struct AvailabilityReport {
    timestamp: u64,
    iterations: usize,
    results: Vec<AvailabilityMetrics>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();
    let cli = Cli::parse();
    zk_auth::ensure_dirs()?;
    anyhow::ensure!(cli.n > 0, "n phải lớn hơn 0");

    let config = zk_auth::ZkAuthConfig::from_env()?;
    anyhow::ensure!(
        config.is_configured(),
        "Thiếu PRIVATE_KEY hoặc ZK_AUTH_CONTRACT_ADDRESS trong .env"
    );

    let timestamp = zk_auth::now_unix();
    let mut results = Vec::new();
    let mode = cli.mode.to_ascii_lowercase();
    anyhow::ensure!(
        matches!(mode.as_str(), "traditional" | "zk" | "zk-auth" | "both"),
        "--mode phải là traditional, zk, zk-auth hoặc both"
    );

    if mode == "traditional" || mode == "both" {
        results.push(run_traditional(&config, cli.n, timestamp).await);
    }
    if mode == "zk" || mode == "zk-auth" || mode == "both" {
        results.push(run_zk(&config, cli.n, timestamp, cli.groth16).await?);
    }

    let report = AvailabilityReport {
        timestamp,
        iterations: cli.n,
        results,
    };
    let path = format!("{}/availability_{timestamp}.json", zk_auth::RESULTS_DIR);
    zk_auth::write_json(&path, &report)?;
    println!("Wrote {path}");

    Ok(())
}

async fn run_traditional(
    config: &zk_auth::ZkAuthConfig,
    n: usize,
    timestamp: u64,
) -> AvailabilityMetrics {
    let mut latencies = Vec::new();
    let mut gas = Vec::new();
    let mut errors = BTreeMap::new();
    let total_start = Instant::now();

    for i in 0..n {
        let payload = format!("Traditional availability payload {timestamp}-{i}");
        let payload_hash = zk_auth::payload_hash(&payload);
        match zk_auth::store_traditional(config, payload_hash, "traditional-availability").await {
            Ok(metrics) => {
                latencies.push(metrics.total_latency_seconds);
                if let Some(value) = metrics.gas_used {
                    gas.push(value as f64);
                }
            }
            Err(err) => {
                *errors.entry(short_error(&err)).or_insert(0) += 1;
            }
        }
    }

    summarize(
        "traditional-ecdsa",
        n,
        latencies,
        gas,
        errors,
        total_start.elapsed().as_secs_f64(),
    )
}

async fn run_zk(
    config: &zk_auth::ZkAuthConfig,
    n: usize,
    timestamp: u64,
    groth16: bool,
) -> Result<AvailabilityMetrics> {
    let signer = config.signer()?;
    let recipient = signer.address();
    let (chain_id, contract) = zk_auth::chain_context(config).await?;
    let mut latencies = Vec::new();
    let mut gas = Vec::new();
    let mut errors = BTreeMap::new();
    let total_start = Instant::now();

    for i in 0..n {
        let loop_start = Instant::now();
        let payload = format!("ZK-auth availability payload {timestamp}-{i}");
        let payload_hash = zk_auth::payload_hash(&payload);
        let input = zk_auth::build_input(payload_hash, recipient, chain_id, contract, None, None);
        let result = async {
            let proof = zk_auth::prove(&input, groth16).context("prove failed")?;
            let artifact_path =
                zk_auth::save_artifact(zk_auth::now_unix(), &proof.receipt, &input)?;
            zk_auth::store_with_proof(
                config,
                proof.receipt.journal.bytes.clone(),
                proof.seal,
                payload_hash,
                &artifact_path.display().to_string(),
            )
            .await
        }
        .await;

        match result {
            Ok(metrics) => {
                latencies.push(loop_start.elapsed().as_secs_f64());
                if let Some(value) = metrics.gas_used {
                    gas.push(value as f64);
                }
            }
            Err(err) => {
                *errors.entry(short_error(&err)).or_insert(0) += 1;
            }
        }
    }

    Ok(summarize(
        "zk-auth",
        n,
        latencies,
        gas,
        errors,
        total_start.elapsed().as_secs_f64(),
    ))
}

fn summarize(
    mode: &str,
    n: usize,
    mut latencies: Vec<f64>,
    gas: Vec<f64>,
    error_breakdown: BTreeMap<String, usize>,
    wall_seconds: f64,
) -> AvailabilityMetrics {
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let success_count = latencies.len();
    let failure_count = n.saturating_sub(success_count);
    let average_latency_seconds = average(&latencies);
    let throughput_tx_per_second = if wall_seconds > 0.0 {
        Some(success_count as f64 / wall_seconds)
    } else {
        None
    };

    AvailabilityMetrics {
        mode: mode.to_string(),
        success_count,
        failure_count,
        success_rate_percent: success_count as f64 * 100.0 / n as f64,
        average_latency_seconds,
        p50_latency_seconds: percentile(&latencies, 0.50),
        p95_latency_seconds: percentile(&latencies, 0.95),
        p99_latency_seconds: percentile(&latencies, 0.99),
        throughput_tx_per_second,
        average_gas_used: average(&gas),
        retry_count: 0,
        error_breakdown,
    }
}

fn average(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn percentile(values: &[f64], percentile: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let idx = ((values.len() - 1) as f64 * percentile).ceil() as usize;
    values.get(idx).copied()
}

fn short_error(err: &anyhow::Error) -> String {
    let value = format!("{err:#}");
    value
        .lines()
        .next()
        .unwrap_or("unknown error")
        .chars()
        .take(160)
        .collect()
}
