use serde::{Deserialize, Serialize};

// ─── Private Input (chỉ host & guest biết) ───────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionInput {
    pub sender_address: [u8; 20],
    pub receiver_address: [u8; 20],
    pub amount: u64,
    pub sender_balance: u64,
    pub nonce: [u8; 32],
}

// ─── Public Output (commit ra journal, ai cũng đọc được) ─────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionOutput {
    pub sender_commitment: [u8; 32],
    pub receiver_commitment: [u8; 32],
    pub amount_commitment: [u8; 32],
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
}

impl ChainConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            rpc_url: std::env::var("SEPOLIA_RPC_URL")
                .unwrap_or_else(|_| "https://rpc.sepolia.org".to_string()),
            private_key: std::env::var("PRIVATE_KEY")
                .unwrap_or_default(),
            contract_address: std::env::var("CONTRACT_ADDRESS")
                .unwrap_or_default(),
        })
    }

    pub fn is_configured(&self) -> bool {
        !self.private_key.is_empty() && !self.contract_address.is_empty()
    }
}
