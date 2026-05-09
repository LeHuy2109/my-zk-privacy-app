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

    // Lấy seal bytes từ Groth16 (hoặc fallback JSON nếu chưa nén)
    let seal_bytes = match receipt.inner.groth16() {
        Ok(groth16) => {
            // Selector 4-byte EVM từ verifier_parameters
            let selector = &groth16.verifier_parameters.as_bytes()[..4];
            let mut encoded = Vec::with_capacity(selector.len() + groth16.seal.len());
            encoded.extend_from_slice(selector);
            encoded.extend_from_slice(groth16.seal.as_ref());
            println!(
                "Groth16 Seal (kèm EVM Selector, {} bytes).",
                encoded.len()
            );
            encoded
        }
        Err(_) => {
            let json_bytes = serde_json::to_vec(&receipt.inner)
                .context("Serialize receipt seal thất bại")?;
            println!(
                "Proof KHÔNG phải Groth16 - gửi dạng JSON ({} bytes). \
                 Contract sẽ từ chối nếu verifier yêu cầu Groth16!",
                json_bytes.len()
            );
            json_bytes
        }
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

// ─── Query Deposit Events (paginated) ──────────────────────────

/// Query tất cả Deposit events từ contract để lấy danh sách commitments.
///
/// Chia thành nhiều batch 40,000 blocks để tránh giới hạn của RPC provider
/// (Alchemy free tier giới hạn 50,000 blocks mỗi eth_getLogs request).
pub async fn query_deposit_events(config: &ChainConfig) -> Result<Vec<[u8; 32]>> {
    use alloy::primitives::keccak256;

    const BATCH_SIZE: u64 = 40_000;

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

    // Lấy block mới nhất
    let latest_block = provider
        .get_block_number()
        .await
        .context("Lấy block number mới nhất thất bại")?;

    let from_block = config.deploy_block;

    if from_block > latest_block {
        anyhow::bail!(
            "DEPLOY_BLOCK ({}) lớn hơn block hiện tại ({}). \
             Kiểm tra lại giá trị DEPLOY_BLOCK trong .env",
            from_block,
            latest_block
        );
    }

    let total_blocks = latest_block - from_block + 1;
    let num_batches = (total_blocks + BATCH_SIZE - 1) / BATCH_SIZE;
    println!(
        "   Quét {} blocks ({} → {}) trong {} batch...",
        total_blocks, from_block, latest_block, num_batches
    );

    let mut commitments: Vec<[u8; 32]> = Vec::new();
    let mut current = from_block;
    let mut batch_num = 1u64;

    while current <= latest_block {
        let end = (current + BATCH_SIZE - 1).min(latest_block);

        let filter = alloy::rpc::types::Filter::new()
            .address(contract_addr)
            .event_signature(deposit_event_sig)
            .from_block(current)
            .to_block(end);

        let logs = provider
            .get_logs(&filter)
            .await
            .with_context(|| {
                format!(
                    "Query Deposit events thất bại (batch {}/{}, blocks {}-{})",
                    batch_num, num_batches, current, end
                )
            })?;

        let count = logs.len();
        if count > 0 {
            println!(
                "   Batch {}/{}: blocks {}–{} → {} event(s)",
                batch_num, num_batches, current, end, count
            );
        }

        for log in logs {
            let topics = log.topics();
            if topics.len() >= 2 {
                // topic[0] = event signature (Deposit)
                // topic[1] = commitment (indexed param)
                let commitment: [u8; 32] = topics[1].into();
                commitments.push(commitment);
            }
        }

        current = end + 1;
        batch_num += 1;
    }

    println!(
        "Tổng cộng {} commitment(s) từ contract.",
        commitments.len()
    );

    Ok(commitments)
}

// ─── Kiểm tra balance ví trên Sepolia ─────────────────────────

/// Kiểm tra balance ví trên Sepolia.
#[allow(dead_code)]
pub async fn get_wallet_balance(config: &ChainConfig) -> Result<U256> {
    let signer: PrivateKeySigner = config
        .private_key
        .parse()
        .context("Parse private key thất bại")?;

    let rpc_url = config
        .rpc_url
        .parse()
        .context("Parse RPC URL thất bại")?;

    let provider = ProviderBuilder::new().connect_http(rpc_url);

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
