use anyhow::{Context, Result};
use serde::Serialize;

use host::zk_auth;

#[derive(Serialize)]
struct TraditionalResult {
    mode: String,
    payload: String,
    payload_hash: String,
    tx_hash: String,
    gas_used: Option<u128>,
    tx_build_seconds: f64,
    ecdsa_sign_seconds: Option<f64>,
    send_and_confirm_seconds: f64,
    raw_tx_size_bytes: Option<usize>,
    calldata_size_bytes: Option<usize>,
    total_latency_seconds: f64,
    timestamp: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();
    zk_auth::ensure_dirs()?;

    let config = zk_auth::ZkAuthConfig::from_env()?;
    anyhow::ensure!(
        config.is_configured(),
        "Thiếu PRIVATE_KEY hoặc ZK_AUTH_CONTRACT_ADDRESS trong .env"
    );

    let timestamp = zk_auth::now_unix();
    let payload = format!("Hello from traditional zk-auth baseline at {timestamp}");
    let payload_hash = zk_auth::payload_hash(&payload);

    let metrics = zk_auth::store_traditional(&config, payload_hash, "traditional-ecdsa")
        .await
        .context("storeRecordTraditional thất bại")?;

    let result = TraditionalResult {
        mode: "traditional-ecdsa".to_string(),
        payload,
        payload_hash: zk_auth::hex0x(payload_hash),
        tx_hash: metrics.tx_hash,
        gas_used: metrics.gas_used,
        tx_build_seconds: metrics.tx_build_seconds,
        ecdsa_sign_seconds: metrics.ecdsa_sign_seconds,
        send_and_confirm_seconds: metrics.send_and_confirm_seconds,
        raw_tx_size_bytes: metrics.raw_tx_size_bytes,
        calldata_size_bytes: metrics.calldata_size_bytes,
        total_latency_seconds: metrics.total_latency_seconds,
        timestamp,
    };

    let path = format!("{}/traditional_{timestamp}.json", zk_auth::RESULTS_DIR);
    zk_auth::write_json(&path, &result)?;
    println!("Wrote {path}");

    Ok(())
}
