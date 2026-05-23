use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;

use host::zk_auth;

#[derive(Parser, Debug)]
#[command(name = "zk_auth_demo", about = "ZK-STARK authentication record demo")]
struct Cli {
    #[arg(long)]
    recipient: Option<String>,

    #[arg(long)]
    secret: Option<String>,

    #[arg(long)]
    nonce: Option<String>,

    #[arg(long, default_value_t = false)]
    groth16: bool,
}

#[derive(Serialize)]
struct ZkAuthResult {
    mode: String,
    payload: String,
    payload_hash: String,
    identity_commitment: String,
    nullifier_hash: String,
    intent_hash: String,
    journal_hash: String,
    proof_hash: String,
    artifact_ref: String,
    proof_generation_seconds: f64,
    proof_verify_seconds: f64,
    seal_size_bytes: usize,
    journal_size_bytes: usize,
    guest_execution_seconds: Option<f64>,
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
    let cli = Cli::parse();
    zk_auth::ensure_dirs()?;

    let config = zk_auth::ZkAuthConfig::from_env()?;
    anyhow::ensure!(
        config.is_configured(),
        "Thiếu PRIVATE_KEY hoặc ZK_AUTH_CONTRACT_ADDRESS trong .env"
    );

    let signer = config.signer()?;
    let recipient = match cli.recipient {
        Some(value) => value.parse().context("Parse recipient thất bại")?,
        None => signer.address(),
    };
    let secret = cli
        .secret
        .as_deref()
        .map(zk_auth::parse_bytes32_hex)
        .transpose()?;
    let nonce = cli
        .nonce
        .as_deref()
        .map(zk_auth::parse_bytes32_hex)
        .transpose()?;

    let timestamp = zk_auth::now_unix();
    let payload = format!("Hello from zk-auth proof demo at {timestamp}");
    let payload_hash = zk_auth::payload_hash(&payload);
    let (chain_id, contract) = zk_auth::chain_context(&config).await?;
    let input = zk_auth::build_input(payload_hash, recipient, chain_id, contract, secret, nonce);

    let proof = zk_auth::prove(&input, cli.groth16).context("Tạo ZK-auth proof thất bại")?;
    let journal = proof.receipt.journal.bytes.clone();
    let journal_hash = zk_auth::sha256_bytes(&journal);
    let proof_hash = zk_auth::sha256_bytes(&proof.seal);
    let artifact_path = zk_auth::save_artifact(timestamp, &proof.receipt, &input)?;
    let artifact_ref = artifact_path.display().to_string();

    let metrics = zk_auth::store_with_proof(
        &config,
        journal.clone(),
        proof.seal.clone(),
        payload_hash,
        &artifact_ref,
    )
    .await
    .context("storeRecordWithProof thất bại")?;

    let result = ZkAuthResult {
        mode: "zk-auth".to_string(),
        payload,
        payload_hash: zk_auth::hex0x(payload_hash),
        identity_commitment: zk_auth::hex0x(proof.output.identity_commitment),
        nullifier_hash: zk_auth::hex0x(proof.output.nullifier_hash),
        intent_hash: zk_auth::hex0x(proof.output.intent_hash),
        journal_hash: zk_auth::hex0x(journal_hash),
        proof_hash: zk_auth::hex0x(proof_hash),
        artifact_ref,
        proof_generation_seconds: proof.proof_generation_seconds,
        proof_verify_seconds: proof.proof_verify_seconds,
        seal_size_bytes: proof.seal.len(),
        journal_size_bytes: journal.len(),
        guest_execution_seconds: None,
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

    let path = format!("{}/zk_auth_{timestamp}.json", zk_auth::RESULTS_DIR);
    zk_auth::write_json(&path, &result)?;
    println!("Wrote {path}");

    Ok(())
}
