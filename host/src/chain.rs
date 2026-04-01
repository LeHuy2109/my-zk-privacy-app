use alloy::{
    network::TransactionBuilder,
    primitives::{Address, Bytes, FixedBytes, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use anyhow::{Context, Result};
use risc0_zkvm::Receipt;

use crate::types::ChainConfig;

// ─── Kết quả gửi proof lên chain ─────────────────────────────

#[derive(Debug, Clone)]
pub struct ChainSubmitResult {
    pub tx_hash: FixedBytes<32>,
    pub block_number: Option<u64>,
    pub gas_used: Option<u128>,
    pub explorer_url: String,
}

// ─── Submit proof lên Sepolia ─────────────────────────────────

pub async fn submit_proof(
    receipt: &Receipt,
    config: &ChainConfig,
) -> Result<ChainSubmitResult> {
    let signer: PrivateKeySigner = config
        .private_key
        .parse()
        .context("Parse private key thất bại")?;

    let rpc_url = config
        .rpc_url
        .parse()
        .context("Parse RPC URL thất bại")?;

    let provider = ProviderBuilder::new()
        .wallet(signer.clone())
        .connect_http(rpc_url);

    // Encode proof data thành calldata
    let journal_bytes = receipt.journal.bytes.clone();
    let seal_bytes = serde_json::to_vec(&receipt.inner)
        .context("Serialize receipt seal thất bại")?;

    let calldata = encode_submit_proof_calldata(&journal_bytes, &seal_bytes);

    let contract_addr: Address = config
        .contract_address
        .parse()
        .context("Parse contract address thất bại")?;

    let tx = TransactionRequest::default()
        .with_to(contract_addr)
        .with_input(calldata);

    let pending = provider
        .send_transaction(tx)
        .await
        .context("Gửi transaction lên Sepolia thất bại")?;

    let tx_hash = *pending.tx_hash();

    let tx_receipt = pending
        .get_receipt()
        .await
        .context("Chờ receipt thất bại")?;

    let block_number = tx_receipt.block_number;
    let gas_used = Some(tx_receipt.gas_used as u128);

    Ok(ChainSubmitResult {
        tx_hash,
        block_number,
        gas_used,
        explorer_url: format!(
            "https://sepolia.etherscan.io/tx/0x{}",
            alloy::hex::encode(tx_hash)
        ),
    })
}

// ─── Kiểm tra balance ví trên Sepolia ─────────────────────────

pub async fn get_wallet_balance(config: &ChainConfig) -> Result<U256> {
    let signer: PrivateKeySigner = config
        .private_key
        .parse()
        .context("Parse private key thất bại")?;

    let rpc_url = config
        .rpc_url
        .parse()
        .context("Parse RPC URL thất bại")?;

    let provider = ProviderBuilder::new()
        .connect_http(rpc_url);

    let balance = provider
        .get_balance(signer.address())
        .await
        .context("Lấy balance thất bại")?;

    Ok(balance)
}

// ─── Helpers ──────────────────────────────────────────────────

fn encode_submit_proof_calldata(journal: &[u8], seal: &[u8]) -> Bytes {
    // Encode: submitProof(bytes journal, bytes seal)
    // Function selector (placeholder – thay bằng selector thực từ contract ABI)
    let selector: [u8; 4] = [0xc0, 0x4b, 0x8d, 0x59];

    let mut calldata = Vec::new();
    calldata.extend_from_slice(&selector);

    // ABI encode 2 dynamic bytes params
    let journal_offset: U256 = U256::from(64);
    let seal_offset: U256 = U256::from(64 + 32 + pad32(journal.len()));

    calldata.extend_from_slice(&journal_offset.to_be_bytes::<32>());
    calldata.extend_from_slice(&seal_offset.to_be_bytes::<32>());

    // Journal bytes
    calldata.extend_from_slice(&U256::from(journal.len()).to_be_bytes::<32>());
    calldata.extend_from_slice(journal);
    let pad_len = pad32(journal.len()) - journal.len();
    calldata.extend(vec![0u8; pad_len]);

    // Seal bytes
    calldata.extend_from_slice(&U256::from(seal.len()).to_be_bytes::<32>());
    calldata.extend_from_slice(seal);
    let pad_len = pad32(seal.len()) - seal.len();
    calldata.extend(vec![0u8; pad_len]);

    Bytes::from(calldata)
}

fn pad32(len: usize) -> usize {
    ((len + 31) / 32) * 32
}
