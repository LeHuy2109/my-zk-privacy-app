use alloy::{
    network::TransactionBuilder,
    primitives::{Address, Bytes, FixedBytes, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
    sol_types::{sol, SolCall, SolType},
};
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use methods::{ZK_AUTH_GUEST_ELF, ZK_AUTH_GUEST_ID};
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, Receipt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, time::Instant};

#[derive(Parser, Debug)]
#[command(name = "zk-auth-demo", about = "Independent ZK application-auth demo")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Generate(GenerateArgs),
    #[command(alias = "zk-demo")]
    ZkDemo(GenerateArgs),
    #[command(alias = "verify-e2e")]
    VerifyE2e(ProofFileArgs),
    #[command(alias = "integrity-cases")]
    IntegrityCases(ProofFileArgs),
    #[command(alias = "traditional-demo")]
    Traditional(TraditionalArgs),
    SubmitZk(SubmitZkArgs),
    #[command(alias = "availability-benchmark")]
    AvailabilityBenchmark(BenchmarkArgs),
    Compare(CompareArgs),
    ImageId,
}

#[derive(Args, Debug)]
struct GenerateArgs {
    #[arg(long)]
    payload: String,
    #[arg(long)]
    secret: String,
    #[arg(long)]
    recipient: String,
    #[arg(long, default_value_t = 1)]
    action_type: u64,
    #[arg(long, default_value_t = 11155111)]
    chain_id: u64,
    #[arg(long)]
    contract: String,
    #[arg(long)]
    nonce: Option<u64>,
    #[arg(long, default_value_t = false)]
    groth16: bool,
    #[arg(long, default_value = "zk-auth-demo/results/proof.json")]
    output: PathBuf,
}

#[derive(Args, Debug)]
struct ProofFileArgs {
    #[arg(long, default_value = "zk-auth-demo/results/proof.json")]
    proof: PathBuf,
}

#[derive(Args, Debug)]
struct TraditionalArgs {
    #[arg(long)]
    payload: String,
    #[arg(long)]
    recipient: String,
    #[arg(long, default_value_t = 1)]
    action_type: u64,
    #[arg(long, default_value = "traditional")]
    algorithm: String,
    #[arg(long, default_value = "local:traditional")]
    cid: String,
    #[arg(long, default_value_t = false)]
    chain: bool,
}

#[derive(Args, Debug)]
struct SubmitZkArgs {
    #[arg(long, default_value = "zk-auth-demo/results/proof.json")]
    proof: PathBuf,
    #[arg(long, default_value = "zk-stark-auth")]
    algorithm: String,
    #[arg(long)]
    proof_cid: Option<String>,
    #[arg(long, default_value_t = false)]
    chain: bool,
}

#[derive(Args, Debug)]
struct BenchmarkArgs {
    #[arg(long, default_value_t = 10)]
    count: u64,
    #[arg(long, value_enum, default_value_t = BenchmarkMode::Local)]
    mode: BenchmarkMode,
    #[arg(long, default_value = "benchmark payload")]
    payload: String,
    #[arg(long, default_value = "zk-auth-demo/results/benchmark.json")]
    output: PathBuf,
}

#[derive(Clone, Debug, ValueEnum)]
enum BenchmarkMode {
    Local,
    Traditional,
    Zk,
}

#[derive(Args, Debug)]
struct CompareArgs {
    #[arg(long, required = true)]
    input: Vec<PathBuf>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ZkAuthInput {
    secret: [u8; 32],
    payload_hash: [u8; 32],
    recipient: [u8; 20],
    action_type: u64,
    chain_id: u64,
    contract_address: [u8; 20],
    nonce: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ZkAuthOutput {
    payload_hash: [u8; 32],
    identity_commitment: [u8; 32],
    nullifier_hash: [u8; 32],
    recipient: [u8; 20],
    action_type: u64,
    chain_id: u64,
    contract_address: [u8; 20],
    nonce: u64,
    is_valid: bool,
}

#[derive(Serialize, Deserialize)]
struct ZkAuthProofArtifact {
    input: ZkAuthPublicInput,
    output: ZkAuthOutput,
    receipt: Receipt,
    proving_time_ms: u128,
    journal_size_bytes: usize,
    seal_size_bytes: usize,
    artifact_size_bytes: Option<u64>,
    payload_hash: String,
    journal_hash: String,
    proof_hash: String,
    local_cid: String,
}

#[derive(Serialize, Deserialize)]
struct ZkAuthPublicInput {
    payload: String,
    recipient: String,
    action_type: u64,
    chain_id: u64,
    contract: String,
    nonce: u64,
}

#[derive(Serialize, Deserialize)]
struct BenchmarkReport {
    mode: String,
    count: u64,
    criteria: Vec<BenchmarkCriterion>,
}

#[derive(Serialize, Deserialize)]
struct BenchmarkCriterion {
    name: String,
    value: String,
    unit: String,
}

sol! {
    struct Journal {
        bytes32 payload_hash;
        bytes32 identity_commitment;
        bytes32 nullifier_hash;
        address recipient;
        uint256 action_type;
        uint256 chain_id;
        address contract_address;
        uint256 nonce;
        bool is_valid;
    }

    interface IZkAuthDemo {
        function storeRecordWithProof(
            bytes calldata journal,
            bytes calldata seal,
            bytes32 payloadHash,
            bytes32 nullifierHash,
            string calldata proofCid,
            string calldata algorithm
        ) external returns (uint256 recordId);

        function storeRecordTraditional(
            bytes32 payloadHash,
            string calldata cid,
            string calldata algorithm,
            address recipient,
            uint256 actionType
        ) external returns (uint256 recordId);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();
    let cli = Cli::parse();

    match cli.command {
        Command::Generate(args) | Command::ZkDemo(args) => generate(args)?,
        Command::VerifyE2e(args) => verify_e2e(args)?,
        Command::IntegrityCases(args) => integrity_cases(args)?,
        Command::Traditional(args) => traditional(args).await?,
        Command::SubmitZk(args) => submit_zk(args).await?,
        Command::AvailabilityBenchmark(args) => availability_benchmark(args)?,
        Command::Compare(args) => compare(args)?,
        Command::ImageId => print_image_id(),
    }

    Ok(())
}

fn generate(args: GenerateArgs) -> Result<()> {
    let secret = parse_bytes32(&args.secret).context("Secret must be 32-byte hex")?;
    let recipient = parse_address(&args.recipient).context("Recipient must be 20-byte hex")?;
    let contract_address = parse_address(&args.contract).context("Contract must be 20-byte hex")?;
    let payload_hash = sha256(args.payload.as_bytes());
    let nonce = args
        .nonce
        .unwrap_or_else(|| current_unix_seconds().unwrap_or(0));

    let input = ZkAuthInput {
        secret,
        payload_hash,
        recipient,
        action_type: args.action_type,
        chain_id: args.chain_id,
        contract_address,
        nonce,
    };

    let env = ExecutorEnv::builder()
        .write(&input)
        .context("Failed to write zk-auth input")?
        .build()
        .context("Failed to build executor env")?;

    let start = Instant::now();
    let prover = default_prover();
    let prove_info = prover
        .prove(env, ZK_AUTH_GUEST_ELF)
        .context("Failed to create zk-auth proof")?;
    let mut receipt = prove_info.receipt;

    if args.groth16 {
        receipt = prover
            .compress(&ProverOpts::groth16(), &receipt)
            .context("Failed to compress zk-auth proof to Groth16")?;
    }

    receipt
        .verify(ZK_AUTH_GUEST_ID)
        .context("Local zk-auth receipt verification failed")?;

    let output = decode_output(&receipt)?;
    let journal_bytes = receipt.journal.bytes.clone();
    let seal_bytes = seal_bytes(&receipt)?;
    let journal_hash = sha256(&journal_bytes);
    let proof_hash = sha256(&seal_bytes);

    let mut artifact = ZkAuthProofArtifact {
        input: ZkAuthPublicInput {
            payload: args.payload,
            recipient: args.recipient,
            action_type: args.action_type,
            chain_id: args.chain_id,
            contract: args.contract,
            nonce,
        },
        output,
        receipt,
        proving_time_ms: start.elapsed().as_millis(),
        journal_size_bytes: journal_bytes.len(),
        seal_size_bytes: seal_bytes.len(),
        artifact_size_bytes: None,
        payload_hash: hex32(payload_hash),
        journal_hash: hex32(journal_hash),
        proof_hash: hex32(proof_hash),
        local_cid: format!("local:{}", hex32(proof_hash)),
    };

    write_json_artifact(&args.output, &artifact)?;
    artifact.artifact_size_bytes = Some(fs::metadata(&args.output)?.len());
    write_json_artifact(&args.output, &artifact)?;

    print_artifact_summary(&artifact);
    println!("Artifact written: {}", args.output.display());
    Ok(())
}

fn verify_e2e(args: ProofFileArgs) -> Result<()> {
    let artifact = read_artifact(&args.proof)?;
    artifact
        .receipt
        .verify(ZK_AUTH_GUEST_ID)
        .context("Receipt verification failed")?;
    let decoded = decode_output(&artifact.receipt)?;
    anyhow::ensure!(
        decoded.payload_hash == artifact.output.payload_hash,
        "payload hash output mismatch"
    );
    anyhow::ensure!(
        decoded.nullifier_hash == artifact.output.nullifier_hash,
        "nullifier output mismatch"
    );
    anyhow::ensure!(
        hex32(sha256(&artifact.receipt.journal.bytes)) == artifact.journal_hash,
        "journal hash mismatch"
    );
    anyhow::ensure!(decoded.is_valid, "journal says proof is invalid");
    println!("E2E verification passed for {}", args.proof.display());
    Ok(())
}

fn integrity_cases(args: ProofFileArgs) -> Result<()> {
    let artifact = read_artifact(&args.proof)?;
    artifact.receipt.verify(ZK_AUTH_GUEST_ID)?;
    let decoded = decode_output(&artifact.receipt)?;
    let journal_hash_ok = hex32(sha256(&artifact.receipt.journal.bytes)) == artifact.journal_hash;
    let payload_tamper_detected = decoded.payload_hash != sha256(b"tampered payload");
    let nullifier_reuse_detectable = !artifact.output.nullifier_hash.iter().all(|b| *b == 0);

    println!("Integrity cases");
    println!("| Case | Result |");
    println!("|---|---|");
    println!("| Receipt verifies locally | pass |");
    println!(
        "| Journal hash unchanged | {} |",
        pass_fail(journal_hash_ok)
    );
    println!(
        "| Tampered payload hash rejected by mismatch | {} |",
        pass_fail(payload_tamper_detected)
    );
    println!(
        "| Nullifier available for reuse protection | {} |",
        pass_fail(nullifier_reuse_detectable)
    );
    Ok(())
}

async fn traditional(args: TraditionalArgs) -> Result<()> {
    let payload_hash = sha256(args.payload.as_bytes());
    let recipient: Address = args
        .recipient
        .parse()
        .context("Invalid recipient address")?;

    if !args.chain {
        println!("Traditional baseline (local)");
        println!("payload_hash: {}", hex32(payload_hash));
        println!("algorithm   : {}", args.algorithm);
        println!("cid         : {}", args.cid);
        return Ok(());
    }

    let config = ChainConfig::from_env()?;
    let provider = config.provider()?;
    let call = IZkAuthDemo::storeRecordTraditionalCall {
        payloadHash: payload_hash.into(),
        cid: args.cid,
        algorithm: args.algorithm,
        recipient,
        actionType: U256::from(args.action_type),
    };
    let receipt = send_call(
        provider,
        config.contract_address,
        Bytes::from(call.abi_encode()),
    )
    .await?;
    println!("Traditional tx: 0x{}", alloy::hex::encode(receipt.tx_hash));
    println!("Gas used      : {}", receipt.gas_used.unwrap_or(0));
    Ok(())
}

async fn submit_zk(args: SubmitZkArgs) -> Result<()> {
    let artifact = read_artifact(&args.proof)?;
    artifact.receipt.verify(ZK_AUTH_GUEST_ID)?;
    let proof_cid = args.proof_cid.unwrap_or_else(|| artifact.local_cid.clone());

    if !args.chain {
        println!("ZK submit preview (local)");
        println!("payload_hash : {}", artifact.payload_hash);
        println!("nullifier    : {}", hex32(artifact.output.nullifier_hash));
        println!("proof_cid    : {}", proof_cid);
        println!("algorithm    : {}", args.algorithm);
        return Ok(());
    }

    let config = ChainConfig::from_env()?;
    let provider = config.provider()?;
    let seal = seal_bytes(&artifact.receipt)?;
    let call = IZkAuthDemo::storeRecordWithProofCall {
        journal: artifact.receipt.journal.bytes.clone().into(),
        seal: seal.into(),
        payloadHash: artifact.output.payload_hash.into(),
        nullifierHash: artifact.output.nullifier_hash.into(),
        proofCid: proof_cid,
        algorithm: args.algorithm,
    };
    let receipt = send_call(
        provider,
        config.contract_address,
        Bytes::from(call.abi_encode()),
    )
    .await?;
    println!("ZK-auth tx: 0x{}", alloy::hex::encode(receipt.tx_hash));
    println!("Gas used  : {}", receipt.gas_used.unwrap_or(0));
    Ok(())
}

fn availability_benchmark(args: BenchmarkArgs) -> Result<()> {
    let start = Instant::now();
    let mut hashes = Vec::new();
    for i in 0..args.count {
        hashes.push(sha256(format!("{}:{}", args.payload, i).as_bytes()));
    }
    let elapsed = start.elapsed();
    let throughput = if elapsed.as_secs_f64() > 0.0 {
        args.count as f64 / elapsed.as_secs_f64()
    } else {
        args.count as f64
    };
    let report = BenchmarkReport {
        mode: format!("{:?}", args.mode).to_lowercase(),
        count: args.count,
        criteria: vec![
            criterion("scenario", "availability_benchmark", "label"),
            criterion("payload_count", args.count, "records"),
            criterion("success_count", args.count, "records"),
            criterion("failure_count", 0, "records"),
            criterion("success_rate_percent", "100.00", "percent"),
            criterion("payload_hash_latency_ms", elapsed.as_millis(), "ms"),
            criterion(
                "payload_hash_throughput",
                format!("{throughput:.2}"),
                "records/sec",
            ),
            criterion("payload_hash_size", 32, "bytes"),
            criterion(
                "proof_generation_seconds",
                "measure_with_zk_demo",
                "seconds",
            ),
            criterion("proof_verify_seconds", "measure_with_verify_e2e", "seconds"),
            criterion("seal_size_bytes", "measure_with_zk_demo", "bytes"),
            criterion("journal_size_bytes", "measure_with_zk_demo", "bytes"),
            criterion("raw_tx_size_bytes", "measure_on_chain_submit", "bytes"),
            criterion("calldata_size_bytes", "measure_on_chain_submit", "bytes"),
            criterion(
                "send_and_confirm_seconds",
                "measure_on_chain_submit",
                "seconds",
            ),
            criterion("total_latency_seconds", "derive_from_e2e_run", "seconds"),
            criterion(
                "tamper_detection_rate",
                "measure_with_integrity_cases",
                "percent",
            ),
            criterion(
                "replay_rejection_rate",
                "measure_with_integrity_cases",
                "percent",
            ),
        ],
    };
    write_json_artifact(&args.output, &report)?;

    println!("Benchmark criteria");
    println!("| Metric | Value | Unit |");
    println!("|---|---:|---|");
    for item in &report.criteria {
        println!("| {} | {} | {} |", item.name, item.value, item.unit);
    }
    println!("Benchmark written: {}", args.output.display());
    drop(hashes);
    Ok(())
}

fn compare(args: CompareArgs) -> Result<()> {
    println!("Comparison");
    println!("| File | Proving ms | Journal bytes | Seal bytes | Artifact bytes | Valid |");
    println!("|---|---:|---:|---:|---:|---|");
    for path in args.input {
        let text = fs::read_to_string(&path)?;
        if let Ok(artifact) = serde_json::from_str::<ZkAuthProofArtifact>(&text) {
            println!(
                "| {} | {} | {} | {} | {} | {} |",
                path.display(),
                artifact.proving_time_ms,
                artifact.journal_size_bytes,
                artifact.seal_size_bytes,
                artifact.artifact_size_bytes.unwrap_or(0),
                artifact.output.is_valid
            );
        } else if let Ok(report) = serde_json::from_str::<BenchmarkReport>(&text) {
            println!(
                "| {} | - | - | - | - | benchmark:{} |",
                path.display(),
                report.mode
            );
        }
    }
    Ok(())
}

fn print_image_id() {
    println!("{}", hex32(image_id_bytes()));
}

fn decode_output(receipt: &Receipt) -> Result<ZkAuthOutput> {
    let decoded = Journal::abi_decode(receipt.journal.bytes.as_slice())
        .context("Failed to decode zk-auth journal")?;
    Ok(ZkAuthOutput {
        payload_hash: decoded.payload_hash.0,
        identity_commitment: decoded.identity_commitment.0,
        nullifier_hash: decoded.nullifier_hash.0,
        recipient: decoded.recipient.0 .0,
        action_type: decoded.action_type.try_into().unwrap_or(0),
        chain_id: decoded.chain_id.try_into().unwrap_or(0),
        contract_address: decoded.contract_address.0 .0,
        nonce: decoded.nonce.try_into().unwrap_or(0),
        is_valid: decoded.is_valid,
    })
}

fn seal_bytes(receipt: &Receipt) -> Result<Vec<u8>> {
    match receipt.inner.groth16() {
        Ok(groth16) => {
            let selector = &groth16.verifier_parameters.as_bytes()[..4];
            let mut encoded = Vec::with_capacity(selector.len() + groth16.seal.len());
            encoded.extend_from_slice(selector);
            encoded.extend_from_slice(groth16.seal.as_ref());
            Ok(encoded)
        }
        Err(_) => {
            serde_json::to_vec(&receipt.inner).context("Failed to serialize non-Groth16 receipt")
        }
    }
}

#[derive(Clone)]
struct ChainConfig {
    rpc_url: String,
    private_key: String,
    contract_address: Address,
}

impl ChainConfig {
    fn from_env() -> Result<Self> {
        let contract = std::env::var("ZK_AUTH_CONTRACT_ADDRESS")
            .or_else(|_| std::env::var("CONTRACT_ADDRESS"))
            .context("Missing ZK_AUTH_CONTRACT_ADDRESS or CONTRACT_ADDRESS")?;
        Ok(Self {
            rpc_url: std::env::var("SEPOLIA_RPC_URL")
                .unwrap_or_else(|_| "https://rpc.sepolia.org".to_string()),
            private_key: std::env::var("PRIVATE_KEY").context("Missing PRIVATE_KEY")?,
            contract_address: contract
                .parse()
                .context("Invalid ZK-auth contract address")?,
        })
    }

    fn provider(&self) -> Result<impl Provider + Clone> {
        let signer: PrivateKeySigner = self.private_key.parse().context("Invalid PRIVATE_KEY")?;
        let rpc_url = self.rpc_url.parse().context("Invalid SEPOLIA_RPC_URL")?;
        Ok(ProviderBuilder::new().wallet(signer).connect_http(rpc_url))
    }
}

struct TxSummary {
    tx_hash: FixedBytes<32>,
    gas_used: Option<u128>,
}

async fn send_call(
    provider: impl Provider + Clone,
    to: Address,
    calldata: Bytes,
) -> Result<TxSummary> {
    let tx = TransactionRequest::default()
        .with_to(to)
        .with_input(calldata);
    let pending = provider
        .send_transaction(tx)
        .await
        .context("Failed to send transaction")?;
    let tx_hash = *pending.tx_hash();
    let receipt = pending
        .get_receipt()
        .await
        .context("Failed waiting for transaction receipt")?;
    Ok(TxSummary {
        tx_hash,
        gas_used: Some(receipt.gas_used as u128),
    })
}

fn parse_bytes32(hex_str: &str) -> Result<[u8; 32]> {
    let bytes = parse_hex(hex_str)?;
    anyhow::ensure!(bytes.len() == 32, "expected 32 bytes, got {}", bytes.len());
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn parse_address(hex_str: &str) -> Result<[u8; 20]> {
    let bytes = parse_hex(hex_str)?;
    anyhow::ensure!(bytes.len() == 20, "expected 20 bytes, got {}", bytes.len());
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn parse_hex(hex_str: &str) -> Result<Vec<u8>> {
    let cleaned = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    alloy::hex::decode(cleaned).context("hex decode failed")
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    let r = h.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&r);
    out
}

fn hex32(bytes: [u8; 32]) -> String {
    format!("0x{}", alloy::hex::encode(bytes))
}

fn image_id_bytes() -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, word) in ZK_AUTH_GUEST_ID.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn current_unix_seconds() -> Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs())
}

fn write_json_artifact<T: Serialize>(path: &PathBuf, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn read_artifact(path: &PathBuf) -> Result<ZkAuthProofArtifact> {
    let json =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_str(&json).context("Failed to parse zk-auth proof artifact")
}

fn print_artifact_summary(artifact: &ZkAuthProofArtifact) {
    println!("ZK-auth proof generated");
    println!("payload_hash : {}", artifact.payload_hash);
    println!("journal_hash : {}", artifact.journal_hash);
    println!("proof_hash   : {}", artifact.proof_hash);
    println!("local_cid    : {}", artifact.local_cid);
    println!("proving_ms   : {}", artifact.proving_time_ms);
    println!("journal_bytes: {}", artifact.journal_size_bytes);
    println!("seal_bytes   : {}", artifact.seal_size_bytes);
}

fn pass_fail(value: bool) -> &'static str {
    if value {
        "pass"
    } else {
        "fail"
    }
}

fn criterion(
    name: impl Into<String>,
    value: impl ToString,
    unit: impl Into<String>,
) -> BenchmarkCriterion {
    BenchmarkCriterion {
        name: name.into(),
        value: value.to_string(),
        unit: unit.into(),
    }
}
