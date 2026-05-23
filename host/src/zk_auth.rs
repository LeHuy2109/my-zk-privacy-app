use alloy::{
    network::TransactionBuilder,
    primitives::{Address, Bytes, FixedBytes},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use alloy_sol_types::{sol, SolCall, SolType};
use anyhow::{Context, Result};
use methods::{ZK_AUTH_METHOD_ELF, ZK_AUTH_METHOD_ID};
use rand::RngCore;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, Receipt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::groth16_docker;

pub const RESULTS_DIR: &str = "results/zk-auth";
pub const ARTIFACT_DIR: &str = "results/zk-auth/artifacts";
pub const ACTION_STORE_RECORD: u32 = 1;

#[derive(Clone, Debug)]
pub struct ZkAuthConfig {
    pub rpc_url: String,
    pub private_key: String,
    pub contract_address: String,
}

impl ZkAuthConfig {
    pub fn from_env() -> Result<Self> {
        let rpc_url = std::env::var("SEPOLIA_RPC_URL")
            .unwrap_or_else(|_| "https://ethereum-sepolia-rpc.publicnode.com".to_string());
        let private_key = normalize_private_key(&std::env::var("PRIVATE_KEY").unwrap_or_default());
        let contract_address = std::env::var("ZK_AUTH_CONTRACT_ADDRESS").unwrap_or_default();

        Ok(Self {
            rpc_url,
            private_key,
            contract_address,
        })
    }

    pub fn is_configured(&self) -> bool {
        !self.private_key.is_empty() && !self.contract_address.is_empty()
    }

    pub fn signer(&self) -> Result<PrivateKeySigner> {
        self.private_key
            .parse()
            .context("Parse PRIVATE_KEY thất bại")
    }

    pub fn contract(&self) -> Result<Address> {
        self.contract_address
            .parse()
            .context("Parse ZK_AUTH_CONTRACT_ADDRESS thất bại")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ZkAuthInput {
    pub secret: [u8; 32],
    pub payload_hash: [u8; 32],
    pub recipient: [u8; 20],
    pub chain_id: u64,
    pub contract_address: [u8; 20],
    pub nonce: [u8; 32],
    pub action_type: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize)]
pub struct ZkAuthProofResult {
    pub receipt: Receipt,
    pub output: ZkAuthOutput,
    pub proof_generation_seconds: f64,
    pub proof_verify_seconds: f64,
    pub seal: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChainTxMetrics {
    pub tx_hash: String,
    pub gas_used: Option<u128>,
    pub tx_build_seconds: f64,
    pub ecdsa_sign_seconds: Option<f64>,
    pub send_and_confirm_seconds: f64,
    pub raw_tx_size_bytes: Option<usize>,
    pub calldata_size_bytes: Option<usize>,
    pub total_latency_seconds: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CaseResult {
    pub case_name: String,
    pub expected_result: String,
    pub actual_result: String,
    pub passed: bool,
    pub revert_reason: Option<String>,
    pub gas_used_if_any: Option<u128>,
    pub latency_seconds: f64,
}

sol! {
    struct Journal {
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

    interface IZkAuthDemo {
        function storeRecordTraditional(bytes32 payloadHash, string calldata mode) external returns (uint256);
        function storeRecordWithProof(bytes calldata journal, bytes calldata seal, bytes32 expectedPayloadHash, string calldata artifactRef) external returns (uint256);
    }
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn ensure_dirs() -> Result<()> {
    fs::create_dir_all(RESULTS_DIR)?;
    fs::create_dir_all(ARTIFACT_DIR)?;
    Ok(())
}

pub fn write_json<T: Serialize>(path: impl AsRef<Path>, value: &T) -> Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(value)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn hex0x(bytes: impl AsRef<[u8]>) -> String {
    format!("0x{}", hex::encode(bytes))
}

pub fn payload_hash(payload: &str) -> [u8; 32] {
    sha256_bytes(payload.as_bytes())
}

pub fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

pub fn random_bytes32() -> [u8; 32] {
    let mut out = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut out);
    out
}

pub fn parse_bytes32_hex(value: &str) -> Result<[u8; 32]> {
    let cleaned = value.strip_prefix("0x").unwrap_or(value);
    let bytes = hex::decode(cleaned).context("Decode bytes32 hex thất bại")?;
    anyhow::ensure!(bytes.len() == 32, "bytes32 phải đúng 32 bytes");
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

pub fn address_to_array(address: Address) -> [u8; 20] {
    address.0 .0
}

pub async fn chain_context(config: &ZkAuthConfig) -> Result<(u64, Address)> {
    let rpc_url = config.rpc_url.parse().context("Parse RPC URL thất bại")?;
    let provider = ProviderBuilder::new().connect_http(rpc_url);
    let chain_id = provider
        .get_chain_id()
        .await
        .context("Lấy chain_id thất bại")?;
    Ok((chain_id, config.contract()?))
}

pub fn build_input(
    payload_hash: [u8; 32],
    recipient: Address,
    chain_id: u64,
    contract: Address,
    secret: Option<[u8; 32]>,
    nonce: Option<[u8; 32]>,
) -> ZkAuthInput {
    ZkAuthInput {
        secret: secret.unwrap_or_else(random_bytes32),
        payload_hash,
        recipient: address_to_array(recipient),
        chain_id,
        contract_address: address_to_array(contract),
        nonce: nonce.unwrap_or_else(random_bytes32),
        action_type: ACTION_STORE_RECORD,
    }
}

pub fn prove(input: &ZkAuthInput, groth16: bool) -> Result<ZkAuthProofResult> {
    if groth16 {
        groth16_docker::prepare().with_context(|| {
            format!(
                "Groth16 Docker prover is not ready (image: {})",
                groth16_docker::image_name()
            )
        })?;
    }

    let env = ExecutorEnv::builder()
        .write(input)
        .context("Ghi ZkAuthInput vào env thất bại")?
        .build()
        .context("Tạo ExecutorEnv thất bại")?;

    let start = Instant::now();
    let prover = default_prover();
    let prove_info = prover
        .prove(env, ZK_AUTH_METHOD_ELF)
        .context("Tạo ZK-auth proof thất bại")?;
    let mut receipt = prove_info.receipt;

    if groth16 {
        receipt = prover
            .compress(&ProverOpts::groth16(), &receipt)
            .context("Nén ZK-auth STARK thành Groth16 thất bại")?;
    }

    let proof_generation_seconds = start.elapsed().as_secs_f64();

    let verify_start = Instant::now();
    receipt
        .verify(ZK_AUTH_METHOD_ID)
        .context("Verify local ZK-auth receipt thất bại")?;
    let proof_verify_seconds = verify_start.elapsed().as_secs_f64();

    let output = decode_output(&receipt)?;
    let seal = evm_seal(&receipt)?;

    Ok(ZkAuthProofResult {
        receipt,
        output,
        proof_generation_seconds,
        proof_verify_seconds,
        seal,
    })
}

pub fn decode_output(receipt: &Receipt) -> Result<ZkAuthOutput> {
    decode_journal(receipt.journal.bytes.as_slice())
}

pub fn decode_journal(bytes: &[u8]) -> Result<ZkAuthOutput> {
    let decoded = Journal::abi_decode(bytes).context("Giải mã ZK-auth journal thất bại")?;

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

pub fn evm_seal(receipt: &Receipt) -> Result<Vec<u8>> {
    match receipt.inner.groth16() {
        Ok(groth16) => {
            let selector = &groth16.verifier_parameters.as_bytes()[..4];
            let mut encoded = Vec::with_capacity(selector.len() + groth16.seal.len());
            encoded.extend_from_slice(selector);
            encoded.extend_from_slice(groth16.seal.as_ref());
            Ok(encoded)
        }
        Err(_) => {
            serde_json::to_vec(&receipt.inner).context("Serialize non-Groth16 receipt thất bại")
        }
    }
}

pub async fn store_traditional(
    config: &ZkAuthConfig,
    payload_hash: [u8; 32],
    mode: &str,
) -> Result<ChainTxMetrics> {
    let build_start = Instant::now();
    let signer = config.signer()?;
    let rpc_url = config.rpc_url.parse().context("Parse RPC URL thất bại")?;
    let provider = ProviderBuilder::new().wallet(signer).connect_http(rpc_url);
    let contract = config.contract()?;

    let call = IZkAuthDemo::storeRecordTraditionalCall {
        payloadHash: payload_hash.into(),
        mode: mode.to_string(),
    };
    let calldata = Bytes::from(call.abi_encode());
    let tx = TransactionRequest::default()
        .with_to(contract)
        .with_input(calldata.clone());
    let tx_build_seconds = build_start.elapsed().as_secs_f64();

    send_and_measure(provider, tx, Some(calldata.len()), tx_build_seconds).await
}

pub async fn store_with_proof(
    config: &ZkAuthConfig,
    journal: Vec<u8>,
    seal: Vec<u8>,
    expected_payload_hash: [u8; 32],
    artifact_ref: &str,
) -> Result<ChainTxMetrics> {
    let build_start = Instant::now();
    let signer = config.signer()?;
    let rpc_url = config.rpc_url.parse().context("Parse RPC URL thất bại")?;
    let provider = ProviderBuilder::new().wallet(signer).connect_http(rpc_url);
    let contract = config.contract()?;

    let call = IZkAuthDemo::storeRecordWithProofCall {
        journal: journal.into(),
        seal: seal.into(),
        expectedPayloadHash: expected_payload_hash.into(),
        artifactRef: artifact_ref.to_string(),
    };
    let calldata = Bytes::from(call.abi_encode());
    let tx = TransactionRequest::default()
        .with_to(contract)
        .with_input(calldata.clone());
    let tx_build_seconds = build_start.elapsed().as_secs_f64();

    send_and_measure(provider, tx, Some(calldata.len()), tx_build_seconds).await
}

async fn send_and_measure(
    provider: impl Provider + Clone,
    tx: TransactionRequest,
    calldata_size_bytes: Option<usize>,
    tx_build_seconds: f64,
) -> Result<ChainTxMetrics> {
    let send_start = Instant::now();
    let pending = provider
        .send_transaction(tx)
        .await
        .context("Gửi transaction thất bại")?;
    let tx_hash: FixedBytes<32> = *pending.tx_hash();
    let receipt = pending
        .get_receipt()
        .await
        .context("Chờ transaction receipt thất bại")?;
    let send_and_confirm_seconds = send_start.elapsed().as_secs_f64();

    Ok(ChainTxMetrics {
        tx_hash: hex0x(tx_hash),
        gas_used: Some(receipt.gas_used as u128),
        tx_build_seconds,
        ecdsa_sign_seconds: None,
        send_and_confirm_seconds,
        raw_tx_size_bytes: None,
        calldata_size_bytes,
        total_latency_seconds: tx_build_seconds + send_and_confirm_seconds,
    })
}

pub fn save_artifact(timestamp: u64, receipt: &Receipt, input: &ZkAuthInput) -> Result<PathBuf> {
    ensure_dirs()?;
    let path = PathBuf::from(ARTIFACT_DIR).join(format!("zk_auth_artifact_{timestamp}.json"));
    let artifact = serde_json::json!({
        "input": input,
        "receipt": receipt,
        "journal_hex": hex0x(&receipt.journal.bytes),
        "journal_hash": hex0x(sha256_bytes(&receipt.journal.bytes)),
    });
    write_json(&path, &artifact)?;
    Ok(path)
}

pub fn latest_file_with_prefix(prefix: &str) -> Result<Option<PathBuf>> {
    let dir = Path::new(RESULTS_DIR);
    if !dir.exists() {
        return Ok(None);
    }

    let mut entries = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| name.starts_with(prefix) && name.ends_with(".json"))
                .unwrap_or(false)
        })
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, entry.path()))
        })
        .collect::<Vec<_>>();

    entries.sort_by_key(|(modified, _)| *modified);
    Ok(entries.pop().map(|(_, path)| path))
}

fn normalize_private_key(value: &str) -> String {
    if value.is_empty() || value.starts_with("0x") || value.starts_with("0X") {
        value.to_string()
    } else {
        format!("0x{value}")
    }
}
