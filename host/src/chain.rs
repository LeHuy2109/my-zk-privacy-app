use alloy::{
    network::TransactionBuilder,
    primitives::{Address, Bytes, FixedBytes, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
};
use anyhow::{Context, Result};
use risc0_zkvm::Receipt;

use crate::types::{ChainConfig, TransactionOutput};

// ─── Kết quả gửi proof lên chain ─────────────────────────────

#[derive(Debug, Clone)]
pub struct ChainSubmitResult {
    pub tx_hash: FixedBytes<32>,
    pub block_number: Option<u64>,
    pub gas_used: Option<u128>,
    pub explorer_url: String,
}

// ─── Submit proof lên Sepolia ─────────────────────────────────

/// Gửi proof lên smart contract Sepolia.
///
/// TODO(on-chain): Cần cập nhật khi có contract thật:
///   1. Thay `calldata` bằng ABI-encoded call từ contract ABI chính xác
///      (dùng alloy sol! macro hoặc alloy::sol_types)
///   2. Đường gọi: contract.withdraw(journal_bytes, seal_bytes, nullifier, recipient)
///   3. Kiểm tra gas estimate trước khi gửi để tránh out-of-gas
pub async fn submit_proof(
    receipt: &Receipt,
    output: &TransactionOutput,
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
    let seal_bytes = match receipt.inner.groth16() {
        Ok(groth16) => groth16.seal.clone(),
        Err(_) => serde_json::to_vec(&receipt.inner)
            .context("Serialize receipt seal thất bại")?,
    };
    let nullifier = output.nullifier_hash;
    let recipient = output.recipient;

    let recipient_addr: Address = recipient.into();

    let call = IPrivacyVerifier::withdrawCall {
        journal: journal_bytes.into(),
        seal: seal_bytes.into(),
        nullifier: nullifier.into(),
        recipient: recipient_addr,
    };
    let calldata = Bytes::from(call.abi_encode());

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

// ─── Query Deposit Events ───────────────────────────────────────

/// Query tất cả Deposit events từ contract để lấy danh sách commitments.
/// Được dùng để rebuild Merkle tree on-chain.
pub async fn query_deposit_events(config: &ChainConfig) -> Result<Vec<[u8; 32]>> {
    use alloy::primitives::keccak256;
    
    let rpc_url = config
        .rpc_url
        .parse()
        .context("Parse RPC URL thất bại")?;

    let provider = ProviderBuilder::new().connect_http(rpc_url);

    let contract_addr: Address = config
        .contract_address
        .parse()
        .context("Parse contract address thất bại")?;

    // Keccak256("Deposit(bytes32,uint256)") – event signature
    let deposit_event_sig = keccak256(b"Deposit(bytes32,uint256)");

    // Query logs từ contract với event signature
    // Alchemy free tier giới hạn dải block. 10620000 là block gần lúc deploy.
    let filter = alloy::rpc::types::Filter::new()
        .address(contract_addr)
        .event_signature(deposit_event_sig)
        .from_block(10620000);

    let logs = provider
        .get_logs(&filter)
        .await
        .context("Query Deposit events thất bại")?;

    let mut commitments = Vec::new();

    for log in logs {
        let topics = log.topics();
        if topics.len() >= 2 {
            // topic[0] = event signature
            // topic[1] = commitment (indexed param)
            let commitment_fixed = topics[1];
            let commitment: [u8; 32] = commitment_fixed.into();
            commitments.push(commitment);
        }
    }

    Ok(commitments)
}

// ─── Kiểm tra balance ví trên Sepolia ─────────────────────────

/// Kiểm tra balance ví trên Sepolia.
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

use alloy::sol_types::{sol, SolCall};

sol! {
    interface IPrivacyVerifier {
        function withdraw(
            bytes calldata journal,
            bytes calldata seal,
            bytes32 nullifier,
            address recipient
        ) external;
    }
}
