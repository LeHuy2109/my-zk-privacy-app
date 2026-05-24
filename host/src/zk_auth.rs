use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use alloy::{
    network::EthereumWallet,
    primitives::{keccak256, Address, Bytes, FixedBytes, U256},
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
    sol,
    sol_types::SolType,
};
use anyhow::{bail, Context, Result};
use methods::{ZK_AUTH_METHOD_ELF, ZK_AUTH_METHOD_ID};
use risc0_zkvm::{default_prover, Receipt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::groth16_docker;

pub const DEFAULT_ACTION_TYPE: u32 = 1;
const DEFAULT_VERIFIER_ROUTER: &str = "0x925d8331ddc0a1F0d96E68CF073DFE1d92b69187";

sol! {
    struct ZkAuthJournal {
        bytes32 payload_hash;
        bytes32 identity_commitment;
        bytes32 nullifier_hash;
        address recipient;
        uint64 chain_id;
        address contract_address;
        uint32 action_type;
        bytes32 intent_hash;
        bool is_valid;
    }

    struct ZkAuthRecord {
        bytes32 payloadHash;
        bytes32 journalHash;
        bytes32 proofHash;
        bytes32 nullifierHash;
        bytes32 identityCommitment;
        address recipient;
        string mode;
        string artifactRef;
        uint256 timestamp;
        bool verified;
    }

    #[sol(rpc)]
    interface IZkAuthDemo {
        function storeRecordTraditional(bytes32 payloadHash, string calldata mode) external;
        function storeRecordWithProof(
            bytes calldata journal,
            bytes calldata seal,
            bytes32 expectedPayloadHash,
            string calldata artifactRef
        ) external;
        function getRecord(uint256 recordId) external view returns (ZkAuthRecord memory);
        function recordCount() external view returns (uint256);
        function usedNullifiers(bytes32 nullifierHash) external view returns (bool);
        function imageId() external view returns (bytes32);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkAuthInput {
    pub secret: [u8; 32],
    pub payload_hash: [u8; 32],
    pub recipient: [u8; 20],
    pub chain_id: u64,
    pub contract_address: [u8; 20],
    pub nonce: [u8; 32],
    pub action_type: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkAuthOutput {
    pub payload_hash: [u8; 32],
    pub identity_commitment: [u8; 32],
    pub nullifier_hash: [u8; 32],
    pub recipient: [u8; 20],
    pub chain_id: u64,
    pub contract_address: [u8; 20],
    pub action_type: u32,
    pub intent_hash: [u8; 32],
    pub is_valid: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub mode: String,
    pub payload: String,
    pub payload_hash: String,
    pub identity_commitment: String,
    pub nullifier_hash: String,
    pub intent_hash: String,
    pub journal_hash: String,
    pub proof_hash: String,
    pub recipient: String,
    pub chain_id: u64,
    pub contract_address: String,
    pub action_type: u32,
    pub journal_path: String,
    pub seal_path: String,
    pub receipt_path: String,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraditionalDemoResult {
    pub mode: String,
    pub payload: String,
    pub payload_hash: String,
    pub record_id: u64,
    pub tx_hash: String,
    pub gas_used: u128,
    pub tx_build_seconds: Option<f64>,
    pub ecdsa_sign_seconds: Option<f64>,
    pub send_and_confirm_seconds: Option<f64>,
    pub raw_tx_size_bytes: Option<usize>,
    pub calldata_size_bytes: Option<usize>,
    pub total_latency_seconds: f64,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkAuthDemoResult {
    pub mode: String,
    pub payload: String,
    pub payload_hash: String,
    pub identity_commitment: String,
    pub nullifier_hash: String,
    pub intent_hash: String,
    pub journal_hash: String,
    pub proof_hash: String,
    pub artifact_ref: String,
    pub record_id: u64,
    pub tx_hash: String,
    pub gas_used: u128,
    pub proof_generation_seconds: f64,
    pub proof_verify_seconds: Option<f64>,
    pub seal_size_bytes: usize,
    pub journal_size_bytes: usize,
    pub guest_execution_seconds: Option<f64>,
    pub artifact_size_bytes: usize,
    pub tx_build_seconds: Option<f64>,
    pub ecdsa_sign_seconds: Option<f64>,
    pub send_and_confirm_seconds: Option<f64>,
    pub raw_tx_size_bytes: Option<usize>,
    pub calldata_size_bytes: Option<usize>,
    pub total_latency_seconds: f64,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifyE2EResult {
    pub record_id: u64,
    pub artifact_ref: String,
    pub payload_hash_match: bool,
    pub journal_hash_match: bool,
    pub proof_hash_match: bool,
    pub local_proof_verify_result: Option<bool>,
    pub e2e_result: bool,
    pub e2e_verification_latency_seconds: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrityCaseResult {
    pub case_name: String,
    pub expected_result: String,
    pub actual_result: String,
    pub passed: bool,
    pub revert_reason: Option<String>,
    pub gas_used_if_any: Option<u128>,
    pub latency_seconds: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub setup_record_id: Option<u64>,
    pub setup_tx_hash: Option<String>,
    pub timestamp: u64,
    pub cases: Vec<IntegrityCaseResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AvailabilityModeSummary {
    pub mode: String,
    pub success_count: u64,
    pub failure_count: u64,
    pub success_rate_percent: f64,
    pub average_latency_seconds: Option<f64>,
    pub p50_latency_seconds: Option<f64>,
    pub p95_latency_seconds: Option<f64>,
    pub p99_latency_seconds: Option<f64>,
    pub throughput_tx_per_second: Option<f64>,
    pub average_gas_used: Option<f64>,
    pub retry_count: u64,
    pub error_breakdown: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AvailabilityReport {
    pub timestamp: u64,
    pub count_per_mode: u64,
    pub max_retries: u64,
    pub traditional: AvailabilityModeSummary,
    pub zk_auth: AvailabilityModeSummary,
}

#[derive(Clone, Debug)]
pub struct ZkAuthConfig {
    pub rpc_url: String,
    pub private_key: String,
    pub contract_address: String,
    pub verifier_address: String,
    pub wait_timeout_seconds: u64,
}

#[derive(Clone, Debug)]
pub struct ZkAuthDemoOptions {
    pub payload: Option<String>,
    pub secret_hex: Option<String>,
    pub nonce_hex: Option<String>,
    pub recipient: Option<String>,
    pub action_type: u32,
    pub groth16: bool,
}

#[derive(Clone, Debug)]
struct StoredArtifact {
    artifact_ref: String,
    artifact_size_bytes: usize,
}

#[derive(Clone, Debug)]
struct PreparedProof {
    receipt: Receipt,
    output: ZkAuthOutput,
    journal: Vec<u8>,
    seal: Vec<u8>,
    journal_hash: [u8; 32],
    proof_hash: [u8; 32],
    proof_generation_seconds: f64,
    proof_verify_seconds: Option<f64>,
}

#[derive(Clone, Debug)]
struct SubmitResult {
    record_id: u64,
    tx_hash: String,
    gas_used: u128,
    tx_build_seconds: Option<f64>,
    ecdsa_sign_seconds: Option<f64>,
    send_and_confirm_seconds: Option<f64>,
    raw_tx_size_bytes: Option<usize>,
    calldata_size_bytes: Option<usize>,
}

pub fn load_env() {
    let _ = dotenv::dotenv();
    let env_path = repo_root().join(".env");
    let _ = dotenv::from_path(env_path);
}

pub fn zk_auth_image_id() -> [u8; 32] {
    image_id_from_words(&ZK_AUTH_METHOD_ID)
}

pub fn zk_auth_image_id_hex() -> String {
    format!("0x{}", hex::encode(zk_auth_image_id()))
}

pub fn load_config() -> Result<ZkAuthConfig> {
    load_env();

    let rpc_url = std::env::var("SEPOLIA_RPC_URL")
        .or_else(|_| std::env::var("RPC_URL"))
        .unwrap_or_else(|_| "https://ethereum-sepolia-rpc.publicnode.com".to_string());
    let private_key = std::env::var("PRIVATE_KEY").unwrap_or_default();
    let contract_address = std::env::var("ZK_AUTH_CONTRACT_ADDRESS").unwrap_or_default();
    let verifier_address = std::env::var("RISC0_VERIFIER_ADDRESS")
        .unwrap_or_else(|_| DEFAULT_VERIFIER_ROUTER.to_string());
    let wait_timeout_seconds = std::env::var("TX_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(120);

    if private_key.is_empty() {
        bail!("Missing PRIVATE_KEY in environment");
    }
    if contract_address.is_empty() {
        bail!("Missing ZK_AUTH_CONTRACT_ADDRESS in environment");
    }

    Ok(ZkAuthConfig {
        rpc_url,
        private_key,
        contract_address,
        verifier_address,
        wait_timeout_seconds,
    })
}

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("host crate should sit under repo root")
        .to_path_buf()
}

pub fn results_dir() -> Result<PathBuf> {
    let path = repo_root().join("results").join("zk-auth");
    fs::create_dir_all(&path)?;
    Ok(path)
}

pub fn offchain_store_dir() -> Result<PathBuf> {
    let path = repo_root()
        .join("shared")
        .join("offchain_store")
        .join("zk-auth");
    fs::create_dir_all(&path)?;
    Ok(path)
}

pub fn write_json_result<T: Serialize>(prefix: &str, payload: &T) -> Result<PathBuf> {
    let timestamp = unix_timestamp();
    let output_path = results_dir()?.join(format!("{prefix}_{timestamp}.json"));
    fs::write(&output_path, serde_json::to_vec_pretty(payload)?)?;
    Ok(output_path)
}

pub fn latest_result_file(prefix: &str) -> Result<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(results_dir()?)?
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(prefix) && name.ends_with(".json"))
                .unwrap_or(false)
        })
        .collect();

    files.sort();
    files.pop().context("No matching result files found")
}

pub fn run_traditional_demo(
    config: &ZkAuthConfig,
    payload: Option<String>,
) -> Result<TraditionalDemoResult> {
    let timestamp = unix_timestamp();
    let payload = payload
        .unwrap_or_else(|| format!("Hello from traditional zk-auth baseline at {timestamp}"));
    let payload_hash = keccak256(payload.as_bytes());

    let start = Instant::now();
    let submit = submit_traditional(config, payload_hash, "traditional")?;

    Ok(TraditionalDemoResult {
        mode: "traditional".to_string(),
        payload,
        payload_hash: to_hex32(&payload_hash),
        record_id: submit.record_id,
        tx_hash: submit.tx_hash,
        gas_used: submit.gas_used,
        tx_build_seconds: submit.tx_build_seconds,
        ecdsa_sign_seconds: submit.ecdsa_sign_seconds,
        send_and_confirm_seconds: submit.send_and_confirm_seconds,
        raw_tx_size_bytes: submit.raw_tx_size_bytes,
        calldata_size_bytes: submit.calldata_size_bytes,
        total_latency_seconds: seconds(start.elapsed().as_secs_f64()),
        timestamp,
    })
}

pub fn run_zk_auth_demo(
    config: &ZkAuthConfig,
    options: ZkAuthDemoOptions,
) -> Result<ZkAuthDemoResult> {
    if !options.groth16 {
        bail!("zk-auth demo requires Groth16 compression for the on-chain verifier");
    }

    let timestamp = unix_timestamp();
    let payload = options
        .payload
        .unwrap_or_else(|| format!("Hello from ZK auth demo at {timestamp}"));
    let payload_hash = keccak256(payload.as_bytes());
    let recipient = resolve_recipient(&options.recipient, &config.private_key)?;
    let contract_address = parse_address(&config.contract_address)?;
    let chain_id = current_chain_id(config)?;
    let proof = prepare_proof(
        &payload_hash,
        recipient,
        contract_address,
        chain_id,
        options.secret_hex.as_deref(),
        options.nonce_hex.as_deref(),
        options.action_type,
        options.groth16,
    )?;

    let artifact = store_artifact(
        timestamp,
        &payload,
        &proof.output,
        &proof.receipt,
        &proof.journal,
        &proof.seal,
        proof.journal_hash,
        proof.proof_hash,
    )?;

    let start = Instant::now();
    let submit = submit_with_proof(
        config,
        &proof.journal,
        &proof.seal,
        payload_hash,
        &artifact.artifact_ref,
    )?;

    Ok(ZkAuthDemoResult {
        mode: "zk_auth".to_string(),
        payload,
        payload_hash: to_hex32(&payload_hash),
        identity_commitment: to_hex32(&proof.output.identity_commitment),
        nullifier_hash: to_hex32(&proof.output.nullifier_hash),
        intent_hash: to_hex32(&proof.output.intent_hash),
        journal_hash: to_hex32(&proof.journal_hash),
        proof_hash: to_hex32(&proof.proof_hash),
        artifact_ref: artifact.artifact_ref,
        record_id: submit.record_id,
        tx_hash: submit.tx_hash,
        gas_used: submit.gas_used,
        proof_generation_seconds: proof.proof_generation_seconds,
        proof_verify_seconds: proof.proof_verify_seconds,
        seal_size_bytes: proof.seal.len(),
        journal_size_bytes: proof.journal.len(),
        guest_execution_seconds: None,
        artifact_size_bytes: artifact.artifact_size_bytes,
        tx_build_seconds: submit.tx_build_seconds,
        ecdsa_sign_seconds: submit.ecdsa_sign_seconds,
        send_and_confirm_seconds: submit.send_and_confirm_seconds,
        raw_tx_size_bytes: submit.raw_tx_size_bytes,
        calldata_size_bytes: submit.calldata_size_bytes,
        total_latency_seconds: seconds(
            proof.proof_generation_seconds
                + proof.proof_verify_seconds.unwrap_or(0.0)
                + submit.tx_build_seconds.unwrap_or(0.0)
                + submit.ecdsa_sign_seconds.unwrap_or(0.0)
                + submit.send_and_confirm_seconds.unwrap_or(0.0)
                + start.elapsed().as_secs_f64(),
        ),
        timestamp,
    })
}

pub async fn resolve_latest_record_id(config: &ZkAuthConfig) -> Result<u64> {
    let provider =
        ProviderBuilder::new().connect_http(config.rpc_url.parse().context("Invalid RPC URL")?);
    let contract = IZkAuthDemo::new(parse_address(&config.contract_address)?, provider);
    let latest = contract.recordCount().call().await?;
    Ok(latest.to::<u64>())
}

pub async fn fetch_record(config: &ZkAuthConfig, record_id: u64) -> Result<ZkAuthRecord> {
    let provider =
        ProviderBuilder::new().connect_http(config.rpc_url.parse().context("Invalid RPC URL")?);
    let contract = IZkAuthDemo::new(parse_address(&config.contract_address)?, provider);
    let result = contract.getRecord(U256::from(record_id)).call().await?;
    Ok(result)
}

pub fn verify_e2e_from_record(record_id: u64, record: &ZkAuthRecord) -> Result<VerifyE2EResult> {
    let started = Instant::now();
    let metadata_path = resolve_repo_relative(&record.artifactRef);
    let metadata: ArtifactMetadata = serde_json::from_slice(
        &fs::read(&metadata_path)
            .with_context(|| format!("Failed to read {}", metadata_path.display()))?,
    )?;

    let journal_path = resolve_repo_relative(&metadata.journal_path);
    let seal_path = resolve_repo_relative(&metadata.seal_path);
    let receipt_path = resolve_repo_relative(&metadata.receipt_path);

    let journal = fs::read(&journal_path)?;
    let seal = fs::read(&seal_path)?;
    let receipt: Receipt = serde_json::from_slice(&fs::read(&receipt_path)?)?;

    let payload_hash = keccak256(metadata.payload.as_bytes());
    let payload_hash_match = payload_hash == record.payloadHash.0;
    let journal_hash_match = sha256_one(&journal) == record.journalHash.0;
    let proof_hash_match = sha256_one(&seal) == record.proofHash.0;

    let seal_matches_receipt = encode_seal(&receipt)? == seal;
    let journal_matches_receipt = receipt.journal.bytes.as_slice() == journal.as_slice();
    let local_proof_verify_result = Some(
        receipt.verify(ZK_AUTH_METHOD_ID).is_ok()
            && seal_matches_receipt
            && journal_matches_receipt,
    );
    let e2e_result = payload_hash_match
        && journal_hash_match
        && proof_hash_match
        && local_proof_verify_result.unwrap_or(false);

    Ok(VerifyE2EResult {
        record_id,
        artifact_ref: record.artifactRef.clone(),
        payload_hash_match,
        journal_hash_match,
        proof_hash_match,
        local_proof_verify_result,
        e2e_result,
        e2e_verification_latency_seconds: seconds(started.elapsed().as_secs_f64()),
    })
}

pub fn run_integrity_cases(config: &ZkAuthConfig) -> Result<IntegrityReport> {
    let timestamp = unix_timestamp();
    let payload = format!("Hello from ZK auth integrity setup at {timestamp}");
    let payload_hash = keccak256(payload.as_bytes());
    let recipient = resolve_recipient(&None, &config.private_key)?;
    let contract_address = parse_address(&config.contract_address)?;
    let actual_chain_id = current_chain_id(config)?;

    let valid = prepare_proof(
        &payload_hash,
        recipient,
        contract_address,
        actual_chain_id,
        None,
        None,
        DEFAULT_ACTION_TYPE,
        true,
    )?;

    let artifact = store_artifact(
        timestamp,
        &payload,
        &valid.output,
        &valid.receipt,
        &valid.journal,
        &valid.seal,
        valid.journal_hash,
        valid.proof_hash,
    )?;

    let setup_submit = submit_with_proof(
        config,
        &valid.journal,
        &valid.seal,
        payload_hash,
        &artifact.artifact_ref,
    )?;

    let wrong_chain = prepare_proof(
        &payload_hash,
        recipient,
        contract_address,
        actual_chain_id.saturating_add(1),
        None,
        None,
        DEFAULT_ACTION_TYPE,
        true,
    )?;
    let wrong_contract = prepare_proof(
        &payload_hash,
        recipient,
        parse_address("0x1111111111111111111111111111111111111111")?,
        actual_chain_id,
        None,
        None,
        DEFAULT_ACTION_TYPE,
        true,
    )?;
    let wrong_recipient = prepare_proof(
        &payload_hash,
        Address::ZERO,
        contract_address,
        actual_chain_id,
        None,
        None,
        DEFAULT_ACTION_TYPE,
        true,
    )?;

    let cases = vec![
        evaluate_integrity_case(
            config,
            "tampered_payload_hash",
            "reverted",
            valid.journal.clone(),
            valid.seal.clone(),
            mutate_32(*payload_hash),
            artifact.artifact_ref.clone(),
        )?,
        evaluate_integrity_case(
            config,
            "tampered_journal",
            "reverted",
            mutate_bytes(&valid.journal),
            valid.seal.clone(),
            *payload_hash,
            artifact.artifact_ref.clone(),
        )?,
        evaluate_integrity_case(
            config,
            "tampered_seal",
            "reverted",
            valid.journal.clone(),
            mutate_bytes(&valid.seal),
            *payload_hash,
            artifact.artifact_ref.clone(),
        )?,
        evaluate_integrity_case(
            config,
            "reused_nullifier",
            "reverted",
            valid.journal.clone(),
            valid.seal.clone(),
            *payload_hash,
            artifact.artifact_ref.clone(),
        )?,
        evaluate_integrity_case(
            config,
            "wrong_chain_id",
            "reverted",
            wrong_chain.journal,
            wrong_chain.seal,
            *payload_hash,
            artifact.artifact_ref.clone(),
        )?,
        evaluate_integrity_case(
            config,
            "wrong_contract_address",
            "reverted",
            wrong_contract.journal,
            wrong_contract.seal,
            *payload_hash,
            artifact.artifact_ref.clone(),
        )?,
        evaluate_integrity_case(
            config,
            "wrong_recipient",
            "reverted",
            wrong_recipient.journal,
            wrong_recipient.seal,
            *payload_hash,
            artifact.artifact_ref.clone(),
        )?,
    ];

    Ok(IntegrityReport {
        setup_record_id: Some(setup_submit.record_id),
        setup_tx_hash: Some(setup_submit.tx_hash),
        timestamp,
        cases,
    })
}

pub fn run_availability_benchmark(
    config: &ZkAuthConfig,
    count: u64,
    max_retries: u64,
) -> Result<AvailabilityReport> {
    let traditional = benchmark_mode(config, count, max_retries, "traditional")?;
    let zk_auth = benchmark_mode(config, count, max_retries, "zk_auth")?;

    Ok(AvailabilityReport {
        timestamp: unix_timestamp(),
        count_per_mode: count,
        max_retries,
        traditional,
        zk_auth,
    })
}

fn benchmark_mode(
    config: &ZkAuthConfig,
    count: u64,
    max_retries: u64,
    mode: &str,
) -> Result<AvailabilityModeSummary> {
    let mut success_count = 0u64;
    let mut failure_count = 0u64;
    let mut retry_count = 0u64;
    let mut error_breakdown = BTreeMap::new();
    let mut latencies = Vec::new();
    let mut gas_values = Vec::new();
    let wall_start = Instant::now();

    for _ in 0..count {
        let mut attempts = 0u64;
        let mut success = false;
        while attempts <= max_retries {
            attempts += 1;
            let run = match mode {
                "traditional" => run_traditional_demo(config, None)
                    .map(|result| (result.total_latency_seconds, result.gas_used)),
                "zk_auth" => run_zk_auth_demo(
                    config,
                    ZkAuthDemoOptions {
                        payload: None,
                        secret_hex: None,
                        nonce_hex: None,
                        recipient: None,
                        action_type: DEFAULT_ACTION_TYPE,
                        groth16: true,
                    },
                )
                .map(|result| (result.total_latency_seconds, result.gas_used)),
                _ => bail!("Unsupported availability mode: {mode}"),
            };

            match run {
                Ok((latency, gas_used)) => {
                    latencies.push(latency);
                    gas_values.push(gas_used as f64);
                    success_count += 1;
                    retry_count += attempts.saturating_sub(1);
                    success = true;
                    break;
                }
                Err(error) => {
                    if attempts > max_retries {
                        failure_count += 1;
                        let key = classify_error(&error);
                        *error_breakdown.entry(key).or_insert(0) += 1;
                    }
                }
            }
        }

        if !success && max_retries == 0 {
            retry_count += 0;
        }
    }

    latencies.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));

    let total_wall = wall_start.elapsed().as_secs_f64();
    let total_runs = success_count + failure_count;
    let success_rate_percent = if total_runs == 0 {
        0.0
    } else {
        (success_count as f64 / total_runs as f64) * 100.0
    };

    Ok(AvailabilityModeSummary {
        mode: mode.to_string(),
        success_count,
        failure_count,
        success_rate_percent: seconds(success_rate_percent),
        average_latency_seconds: average(&latencies),
        p50_latency_seconds: percentile(&latencies, 0.50),
        p95_latency_seconds: percentile(&latencies, 0.95),
        p99_latency_seconds: percentile(&latencies, 0.99),
        throughput_tx_per_second: if total_wall > 0.0 {
            Some(seconds(success_count as f64 / total_wall))
        } else {
            None
        },
        average_gas_used: average(&gas_values),
        retry_count,
        error_breakdown,
    })
}

pub fn compare_latest_results() -> Result<Value> {
    let traditional: Value =
        serde_json::from_slice(&fs::read(latest_result_file("traditional")?)?)?;
    let zk_auth: Value = serde_json::from_slice(&fs::read(latest_result_file("zk_auth")?)?)?;
    let availability: Value =
        serde_json::from_slice(&fs::read(latest_result_file("availability")?)?)?;
    let integrity: Value = serde_json::from_slice(&fs::read(latest_result_file("integrity")?)?)?;

    let tamper_cases = integrity["cases"].as_array().cloned().unwrap_or_default();
    let tamper_detection_rate = if tamper_cases.is_empty() {
        None
    } else {
        let passed = tamper_cases
            .iter()
            .filter(|case| case["passed"].as_bool().unwrap_or(false))
            .count() as f64;
        Some(seconds((passed / tamper_cases.len() as f64) * 100.0))
    };
    let replay_rejection_rate = tamper_cases
        .iter()
        .find(|case| case["case_name"].as_str() == Some("reused_nullifier"))
        .map(|case| {
            if case["passed"].as_bool().unwrap_or(false) {
                100.0
            } else {
                0.0
            }
        });

    Ok(json!({
        "traditional": {
            "gas_used": traditional["gas_used"],
            "proof_generation_seconds": Value::Null,
            "proof_verify_seconds": Value::Null,
            "seal_size_bytes": Value::Null,
            "journal_size_bytes": Value::Null,
            "artifact_size_bytes": Value::Null,
            "raw_tx_size_bytes": traditional["raw_tx_size_bytes"],
            "calldata_size_bytes": traditional["calldata_size_bytes"],
            "send_and_confirm_seconds": traditional["send_and_confirm_seconds"],
            "total_latency_seconds": traditional["total_latency_seconds"],
            "success_rate_percent": availability["traditional"]["success_rate_percent"],
            "tamper_detection_rate": Value::Null,
            "replay_rejection_rate": Value::Null,
        },
        "zk_auth": {
            "gas_used": zk_auth["gas_used"],
            "proof_generation_seconds": zk_auth["proof_generation_seconds"],
            "proof_verify_seconds": zk_auth["proof_verify_seconds"],
            "seal_size_bytes": zk_auth["seal_size_bytes"],
            "journal_size_bytes": zk_auth["journal_size_bytes"],
            "artifact_size_bytes": zk_auth["artifact_size_bytes"],
            "raw_tx_size_bytes": zk_auth["raw_tx_size_bytes"],
            "calldata_size_bytes": zk_auth["calldata_size_bytes"],
            "send_and_confirm_seconds": zk_auth["send_and_confirm_seconds"],
            "total_latency_seconds": zk_auth["total_latency_seconds"],
            "success_rate_percent": availability["zk_auth"]["success_rate_percent"],
            "tamper_detection_rate": tamper_detection_rate,
            "replay_rejection_rate": replay_rejection_rate,
        },
        "zk_auth_offchain_artifact": {
            "gas_used": zk_auth["gas_used"],
            "proof_generation_seconds": zk_auth["proof_generation_seconds"],
            "proof_verify_seconds": zk_auth["proof_verify_seconds"],
            "seal_size_bytes": zk_auth["seal_size_bytes"],
            "journal_size_bytes": zk_auth["journal_size_bytes"],
            "artifact_size_bytes": zk_auth["artifact_size_bytes"],
            "raw_tx_size_bytes": zk_auth["raw_tx_size_bytes"],
            "calldata_size_bytes": zk_auth["calldata_size_bytes"],
            "send_and_confirm_seconds": zk_auth["send_and_confirm_seconds"],
            "total_latency_seconds": zk_auth["total_latency_seconds"],
            "success_rate_percent": availability["zk_auth"]["success_rate_percent"],
            "tamper_detection_rate": tamper_detection_rate,
            "replay_rejection_rate": replay_rejection_rate,
        }
    }))
}

fn submit_traditional(
    config: &ZkAuthConfig,
    payload_hash: FixedBytes<32>,
    mode: &str,
) -> Result<SubmitResult> {
    let signer: PrivateKeySigner = config.private_key.parse().context("Invalid PRIVATE_KEY")?;
    let wallet = EthereumWallet::from(signer.clone());
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_http(config.rpc_url.parse().context("Invalid RPC URL")?);
    let contract = IZkAuthDemo::new(parse_address(&config.contract_address)?, provider);

    let build_start = Instant::now();
    let call = contract
        .storeRecordTraditional(payload_hash.into(), mode.to_string())
        .from(signer.address());
    let calldata_size_bytes = Some(call.calldata().len());
    let tx_build_seconds = Some(seconds(build_start.elapsed().as_secs_f64()));

    let send_start = Instant::now();
    let (tx_hash, receipt, latest) = futures_executor::block_on(async {
        let pending = call.send().await?;
        let tx_hash = pending.tx_hash().to_string();
        let receipt = pending.get_receipt().await?;
        let latest = contract.recordCount().call().await?;
        Ok::<_, anyhow::Error>((tx_hash, receipt, latest))
    })
    .context("Traditional transaction flow failed")?;
    let send_and_confirm_seconds = Some(seconds(send_start.elapsed().as_secs_f64()));

    Ok(SubmitResult {
        record_id: latest.to::<u64>(),
        tx_hash,
        gas_used: receipt.gas_used as u128,
        tx_build_seconds,
        ecdsa_sign_seconds: None,
        send_and_confirm_seconds,
        raw_tx_size_bytes: None,
        calldata_size_bytes,
    })
}

fn submit_with_proof(
    config: &ZkAuthConfig,
    journal: &[u8],
    seal: &[u8],
    payload_hash: FixedBytes<32>,
    artifact_ref: &str,
) -> Result<SubmitResult> {
    let signer: PrivateKeySigner = config.private_key.parse().context("Invalid PRIVATE_KEY")?;
    let wallet = EthereumWallet::from(signer.clone());
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_http(config.rpc_url.parse().context("Invalid RPC URL")?);
    let contract = IZkAuthDemo::new(parse_address(&config.contract_address)?, provider);

    let build_start = Instant::now();
    let call = contract
        .storeRecordWithProof(
            Bytes::copy_from_slice(journal),
            Bytes::copy_from_slice(seal),
            payload_hash.into(),
            artifact_ref.to_string(),
        )
        .from(signer.address());
    let calldata_size_bytes = Some(call.calldata().len());
    let tx_build_seconds = Some(seconds(build_start.elapsed().as_secs_f64()));

    let send_start = Instant::now();
    let (tx_hash, receipt, latest) = futures_executor::block_on(async {
        let pending = call.send().await?;
        let tx_hash = pending.tx_hash().to_string();
        let receipt = pending.get_receipt().await?;
        let latest = contract.recordCount().call().await?;
        Ok::<_, anyhow::Error>((tx_hash, receipt, latest))
    })
    .context("zk-auth transaction flow failed")?;
    let send_and_confirm_seconds = Some(seconds(send_start.elapsed().as_secs_f64()));

    Ok(SubmitResult {
        record_id: latest.to::<u64>(),
        tx_hash,
        gas_used: receipt.gas_used as u128,
        tx_build_seconds,
        ecdsa_sign_seconds: None,
        send_and_confirm_seconds,
        raw_tx_size_bytes: None,
        calldata_size_bytes,
    })
}

fn prepare_proof(
    payload_hash: &[u8; 32],
    recipient: Address,
    contract_address: Address,
    chain_id: u64,
    secret_hex: Option<&str>,
    nonce_hex: Option<&str>,
    action_type: u32,
    groth16: bool,
) -> Result<PreparedProof> {
    if groth16 {
        groth16_docker::prepare().with_context(|| {
            format!(
                "Groth16 Docker prover is not ready ({})",
                groth16_docker::image_name()
            )
        })?;
    }

    let input = ZkAuthInput {
        secret: parse_or_random_32(secret_hex)?,
        payload_hash: *payload_hash,
        recipient: recipient.into_array(),
        chain_id,
        contract_address: contract_address.into_array(),
        nonce: parse_or_random_32(nonce_hex)?,
        action_type,
    };

    let env = risc0_zkvm::ExecutorEnv::builder()
        .write(&input)
        .context("Failed to write zk-auth input into executor env")?
        .build()
        .context("Failed to build zk-auth executor env")?;

    let started = Instant::now();
    let prover = default_prover();
    let mut receipt = prover
        .prove(env, ZK_AUTH_METHOD_ELF)
        .context("Failed to generate zk-auth proof")?
        .receipt;

    if groth16 {
        receipt = prover
            .compress(&risc0_zkvm::ProverOpts::groth16(), &receipt)
            .context("Failed to compress zk-auth receipt into Groth16")?;
    }

    let proof_generation_seconds = seconds(started.elapsed().as_secs_f64());
    let verify_started = Instant::now();
    receipt
        .verify(ZK_AUTH_METHOD_ID)
        .context("Local zk-auth receipt verification failed")?;
    let proof_verify_seconds = Some(seconds(verify_started.elapsed().as_secs_f64()));

    let journal = receipt.journal.bytes.clone().to_vec();
    let output = decode_output(&journal)?;
    let seal = encode_seal(&receipt)?;
    let journal_hash = sha256_one(&journal);
    let proof_hash = sha256_one(&seal);

    Ok(PreparedProof {
        receipt,
        output,
        journal,
        seal,
        journal_hash,
        proof_hash,
        proof_generation_seconds,
        proof_verify_seconds,
    })
}

fn current_chain_id(config: &ZkAuthConfig) -> Result<u64> {
    let provider =
        ProviderBuilder::new().connect_http(config.rpc_url.parse().context("Invalid RPC URL")?);
    futures_executor::block_on(provider.get_chain_id()).context("Failed to read chain ID")
}

fn decode_output(journal: &[u8]) -> Result<ZkAuthOutput> {
    let decoded = ZkAuthJournal::abi_decode(journal).context("Failed to decode zk-auth journal")?;
    Ok(ZkAuthOutput {
        payload_hash: decoded.payload_hash.0,
        identity_commitment: decoded.identity_commitment.0,
        nullifier_hash: decoded.nullifier_hash.0,
        recipient: decoded.recipient.0 .0,
        chain_id: decoded.chain_id,
        contract_address: decoded.contract_address.0 .0,
        action_type: decoded.action_type,
        intent_hash: decoded.intent_hash.0,
        is_valid: decoded.is_valid,
    })
}

fn store_artifact(
    timestamp: u64,
    payload: &str,
    output: &ZkAuthOutput,
    receipt: &Receipt,
    journal: &[u8],
    seal: &[u8],
    journal_hash: [u8; 32],
    proof_hash: [u8; 32],
) -> Result<StoredArtifact> {
    let artifact_dir = offchain_store_dir()?.join(format!(
        "{timestamp}_{}",
        &to_hex32(&output.nullifier_hash)[2..18]
    ));
    fs::create_dir_all(&artifact_dir)?;

    let journal_path = artifact_dir.join("journal.bin");
    let seal_path = artifact_dir.join("seal.bin");
    let receipt_path = artifact_dir.join("receipt.json");
    let metadata_path = artifact_dir.join("metadata.json");

    fs::write(&journal_path, journal)?;
    fs::write(&seal_path, seal)?;
    fs::write(&receipt_path, serde_json::to_vec_pretty(receipt)?)?;

    let metadata = ArtifactMetadata {
        mode: "zk_auth".to_string(),
        payload: payload.to_string(),
        payload_hash: to_hex32(&output.payload_hash),
        identity_commitment: to_hex32(&output.identity_commitment),
        nullifier_hash: to_hex32(&output.nullifier_hash),
        intent_hash: to_hex32(&output.intent_hash),
        journal_hash: to_hex32(&journal_hash),
        proof_hash: to_hex32(&proof_hash),
        recipient: address_to_string(&output.recipient),
        chain_id: output.chain_id,
        contract_address: address_to_string(&output.contract_address),
        action_type: output.action_type,
        journal_path: repo_relative(&journal_path)?,
        seal_path: repo_relative(&seal_path)?,
        receipt_path: repo_relative(&receipt_path)?,
        timestamp,
    };

    fs::write(&metadata_path, serde_json::to_vec_pretty(&metadata)?)?;

    Ok(StoredArtifact {
        artifact_ref: repo_relative(&metadata_path)?,
        artifact_size_bytes: fs::metadata(&journal_path)?.len() as usize
            + fs::metadata(&seal_path)?.len() as usize
            + fs::metadata(&receipt_path)?.len() as usize
            + fs::metadata(&metadata_path)?.len() as usize,
    })
}

fn evaluate_integrity_case(
    config: &ZkAuthConfig,
    case_name: &str,
    expected_result: &str,
    journal: Vec<u8>,
    seal: Vec<u8>,
    payload_hash: [u8; 32],
    artifact_ref: String,
) -> Result<IntegrityCaseResult> {
    let signer: PrivateKeySigner = config.private_key.parse().context("Invalid PRIVATE_KEY")?;
    let wallet = EthereumWallet::from(signer.clone());
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_http(config.rpc_url.parse().context("Invalid RPC URL")?);
    let contract = IZkAuthDemo::new(parse_address(&config.contract_address)?, provider);

    let started = Instant::now();
    let call = contract
        .storeRecordWithProof(
            Bytes::from(journal),
            Bytes::from(seal),
            payload_hash.into(),
            artifact_ref,
        )
        .from(signer.address());

    let result = futures_executor::block_on(async { call.call().await });
    let latency_seconds = seconds(started.elapsed().as_secs_f64());

    let (actual_result, revert_reason): (String, Option<String>) = match result {
        Ok(_) => ("accepted".to_string(), None),
        Err(error) => ("reverted".to_string(), Some(error.to_string())),
    };

    Ok(IntegrityCaseResult {
        case_name: case_name.to_string(),
        expected_result: expected_result.to_string(),
        actual_result: actual_result.clone(),
        passed: actual_result == expected_result,
        revert_reason,
        gas_used_if_any: None,
        latency_seconds,
    })
}

fn encode_seal(receipt: &Receipt) -> Result<Vec<u8>> {
    match receipt.inner.groth16() {
        Ok(groth16) => {
            let selector = &groth16.verifier_parameters.as_bytes()[..4];
            let mut encoded = Vec::with_capacity(selector.len() + groth16.seal.len());
            encoded.extend_from_slice(selector);
            encoded.extend_from_slice(groth16.seal.as_ref());
            Ok(encoded)
        }
        Err(_) => {
            let json =
                serde_json::to_vec(&receipt.inner).context("Failed to serialize receipt inner")?;
            Ok(json)
        }
    }
}

fn resolve_recipient(recipient: &Option<String>, private_key: &str) -> Result<Address> {
    if let Some(value) = recipient {
        return parse_address(value);
    }

    let signer: PrivateKeySigner = private_key.parse().context("Invalid PRIVATE_KEY")?;
    Ok(signer.address())
}

fn parse_address(value: &str) -> Result<Address> {
    value
        .parse()
        .with_context(|| format!("Invalid address: {value}"))
}

fn parse_or_random_32(value: Option<&str>) -> Result<[u8; 32]> {
    match value {
        Some(hex_value) => parse_hex_32(hex_value),
        None => {
            let mut bytes = [0u8; 32];
            getrandom::getrandom(&mut bytes).context("Failed to generate random bytes")?;
            Ok(bytes)
        }
    }
}

fn parse_hex_32(value: &str) -> Result<[u8; 32]> {
    let cleaned = value.strip_prefix("0x").unwrap_or(value);
    let decoded = hex::decode(cleaned).with_context(|| format!("Invalid hex input: {value}"))?;
    if decoded.len() != 32 {
        bail!("Expected 32 bytes, got {}", decoded.len());
    }
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&decoded);
    Ok(bytes)
}

fn sha256_one(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

fn image_id_from_words(words: &[u32; 8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    for (index, word) in words.iter().enumerate() {
        output[index * 4..(index + 1) * 4].copy_from_slice(&word.to_le_bytes());
    }
    output
}

fn to_hex32(value: &[u8; 32]) -> String {
    format!("0x{}", hex::encode(value))
}

fn address_to_string(bytes: &[u8; 20]) -> String {
    format!("0x{}", hex::encode(bytes))
}

fn repo_relative(path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(repo_root())
        .with_context(|| format!("Path {} is not under repo root", path.display()))?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn resolve_repo_relative(path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else {
        repo_root().join(candidate)
    }
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_secs()
}

fn mutate_bytes(bytes: &[u8]) -> Vec<u8> {
    if bytes.is_empty() {
        return vec![1];
    }

    let mut output = bytes.to_vec();
    let index = if output.len() > 4 { 4 } else { 0 };
    output[index] ^= 0x01;
    output
}

fn mutate_32(value: [u8; 32]) -> [u8; 32] {
    let mut output = value;
    output[0] ^= 0x01;
    output
}

fn average(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(seconds(values.iter().sum::<f64>() / values.len() as f64))
    }
}

fn percentile(values: &[f64], ratio: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    if values.len() == 1 {
        return Some(seconds(values[0]));
    }

    let index = ratio * (values.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;
    let fraction = index - lower as f64;
    let interpolated = values[lower] + (values[upper] - values[lower]) * fraction;
    Some(seconds(interpolated))
}

fn classify_error(error: &anyhow::Error) -> String {
    let message = error.to_string().to_lowercase();
    if message.contains("docker") || message.contains("groth16") {
        "groth16_error".to_string()
    } else if message.contains("revert") {
        "contract_revert".to_string()
    } else if message.contains("rpc") || message.contains("http") {
        "rpc_error".to_string()
    } else {
        "unknown_error".to_string()
    }
}

fn seconds(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}
