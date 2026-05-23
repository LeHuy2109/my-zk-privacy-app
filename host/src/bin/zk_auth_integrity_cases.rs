use anyhow::{Context, Result};
use clap::Parser;
use serde::Serialize;
use std::time::Instant;

use host::zk_auth::{self, CaseResult};

#[derive(Parser, Debug)]
#[command(
    name = "zk_auth_integrity_cases",
    about = "Negative integrity cases for ZK-auth records"
)]
struct Cli {
    #[arg(long)]
    recipient: Option<String>,

    #[arg(long, default_value_t = false)]
    groth16: bool,
}

#[derive(Serialize)]
struct IntegrityReport {
    mode: String,
    timestamp: u64,
    cases: Vec<CaseResult>,
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
    let timestamp = zk_auth::now_unix();
    let payload = format!("Integrity case payload at {timestamp}");
    let payload_hash = zk_auth::payload_hash(&payload);
    let (chain_id, contract) = zk_auth::chain_context(&config).await?;

    let base_input = zk_auth::build_input(payload_hash, recipient, chain_id, contract, None, None);
    let base_proof = zk_auth::prove(&base_input, cli.groth16)?;
    let base_journal = base_proof.receipt.journal.bytes.clone();
    let base_seal = base_proof.seal.clone();

    let mut cases = Vec::new();

    cases.push(
        expect_revert(
            "tampered_payload_hash",
            zk_auth::store_with_proof(
                &config,
                base_journal.clone(),
                base_seal.clone(),
                zk_auth::sha256_bytes(b"tampered"),
                "integrity://payload-mismatch",
            ),
        )
        .await,
    );

    let mut tampered_journal = base_journal.clone();
    if let Some(byte) = tampered_journal.get_mut(0) {
        *byte ^= 0x01;
    }
    cases.push(
        expect_revert(
            "tampered_journal_byte",
            zk_auth::store_with_proof(
                &config,
                tampered_journal,
                base_seal.clone(),
                payload_hash,
                "integrity://journal-tamper",
            ),
        )
        .await,
    );

    let mut tampered_seal = base_seal.clone();
    if let Some(byte) = tampered_seal.last_mut() {
        *byte ^= 0x01;
    }
    cases.push(
        expect_revert(
            "tampered_seal_byte",
            zk_auth::store_with_proof(
                &config,
                base_journal.clone(),
                tampered_seal,
                payload_hash,
                "integrity://seal-tamper",
            ),
        )
        .await,
    );

    let wrong_chain_input = zk_auth::build_input(
        payload_hash,
        recipient,
        chain_id.saturating_add(1),
        contract,
        None,
        None,
    );
    let wrong_chain_proof = zk_auth::prove(&wrong_chain_input, cli.groth16)?;
    cases.push(
        expect_revert(
            "wrong_chain_id",
            zk_auth::store_with_proof(
                &config,
                wrong_chain_proof.receipt.journal.bytes.clone(),
                wrong_chain_proof.seal.clone(),
                payload_hash,
                "integrity://wrong-chain",
            ),
        )
        .await,
    );

    let wrong_contract_input = zk_auth::build_input(
        payload_hash,
        recipient,
        chain_id,
        "0x000000000000000000000000000000000000dEaD".parse()?,
        None,
        None,
    );
    let wrong_contract_proof = zk_auth::prove(&wrong_contract_input, cli.groth16)?;
    cases.push(
        expect_revert(
            "wrong_contract_address",
            zk_auth::store_with_proof(
                &config,
                wrong_contract_proof.receipt.journal.bytes.clone(),
                wrong_contract_proof.seal.clone(),
                payload_hash,
                "integrity://wrong-contract",
            ),
        )
        .await,
    );

    let zero_recipient_input = zk_auth::build_input(
        payload_hash,
        "0x0000000000000000000000000000000000000000".parse()?,
        chain_id,
        contract,
        None,
        None,
    );
    let zero_recipient_proof = zk_auth::prove(&zero_recipient_input, cli.groth16)?;
    cases.push(
        expect_revert(
            "wrong_recipient_zero_address",
            zk_auth::store_with_proof(
                &config,
                zero_recipient_proof.receipt.journal.bytes.clone(),
                zero_recipient_proof.seal.clone(),
                payload_hash,
                "integrity://wrong-recipient",
            ),
        )
        .await,
    );

    let replay_artifact = "integrity://valid-then-replay";
    zk_auth::store_with_proof(
        &config,
        base_journal.clone(),
        base_seal.clone(),
        payload_hash,
        replay_artifact,
    )
    .await
    .context("Valid setup transaction for replay case failed")?;

    cases.push(
        expect_revert(
            "reused_nullifier",
            zk_auth::store_with_proof(
                &config,
                base_journal,
                base_seal,
                payload_hash,
                replay_artifact,
            ),
        )
        .await,
    );

    let report = IntegrityReport {
        mode: "zk-auth-integrity".to_string(),
        timestamp,
        cases,
    };
    let path = format!("{}/integrity_{timestamp}.json", zk_auth::RESULTS_DIR);
    zk_auth::write_json(&path, &report)?;
    println!("Wrote {path}");

    Ok(())
}

async fn expect_revert(
    case_name: &str,
    fut: impl std::future::Future<Output = Result<zk_auth::ChainTxMetrics>>,
) -> CaseResult {
    let start = Instant::now();
    match fut.await {
        Ok(metrics) => CaseResult {
            case_name: case_name.to_string(),
            expected_result: "revert".to_string(),
            actual_result: "success".to_string(),
            passed: false,
            revert_reason: None,
            gas_used_if_any: metrics.gas_used,
            latency_seconds: start.elapsed().as_secs_f64(),
        },
        Err(err) => CaseResult {
            case_name: case_name.to_string(),
            expected_result: "revert".to_string(),
            actual_result: "revert".to_string(),
            passed: true,
            revert_reason: Some(format!("{err:#}")),
            gas_used_if_any: None,
            latency_seconds: start.elapsed().as_secs_f64(),
        },
    }
}
