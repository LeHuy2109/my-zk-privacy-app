use serde::{Deserialize, Serialize};

// ─── Private Input (chỉ host & guest biết) ───────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionInput {
    pub secret: [u8; 32],
    pub amount: u64,
    pub merkle_path: Vec<[u8; 32]>,
    pub merkle_indices: Vec<bool>,
    pub merkle_root: [u8; 32],
    pub recipient: [u8; 20],
}

// ─── Public Output (commit ra journal) ───────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionOutput {
    pub merkle_root: [u8; 32],
    pub nullifier_hash: [u8; 32],
    pub recipient: [u8; 20],
    pub amount: u64,
    pub is_valid: bool,
}

// ─── Kết quả sau khi Executor + Prover hoàn tất ──────────────

pub struct ProofResult {
    pub receipt: risc0_zkvm::Receipt,
    pub output: TransactionOutput,
    pub proving_time_ms: u128,
}

// ─── Cấu hình kết nối Sepolia ────────────────────────────────

#[derive(Clone, Debug)]
pub struct ChainConfig {
    pub rpc_url: String,
    pub private_key: String,
    pub contract_address: String,
    /// Block number mà contract được deploy, dùng để bắt đầu query Deposit events
    pub deploy_block: u64,
}

impl ChainConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        // Block mà contract được deploy – dùng để bắt đầu paginate eth_getLogs
        let deploy_block = std::env::var("DEPLOY_BLOCK")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(10625000); // fallback: block deploy contract trên Sepolia

        Ok(Self {
            rpc_url: std::env::var("SEPOLIA_RPC_URL")
                .unwrap_or_else(|_| "https://rpc.sepolia.org".to_string()),
            private_key: std::env::var("PRIVATE_KEY").unwrap_or_default(),
            contract_address: std::env::var("CONTRACT_ADDRESS").unwrap_or_default(),
            deploy_block,
        })
    }

    pub fn is_configured(&self) -> bool {
        !self.private_key.is_empty() && !self.contract_address.is_empty()
    }
}
