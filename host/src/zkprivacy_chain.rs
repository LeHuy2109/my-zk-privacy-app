use alloy::{
    network::TransactionBuilder,
    primitives::{Address, Bytes, FixedBytes, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
    sol_types::{sol, SolCall},
};
use anyhow::{Context, Result};

use crate::types::ChainConfig;

#[derive(Debug, Clone)]
pub struct TxResult {
    pub tx_hash: FixedBytes<32>,
    pub block_number: Option<u64>,
    pub gas_used: Option<u128>,
}

pub async fn deposit(commitment: [u8; 32], amount: u64, config: &ChainConfig) -> Result<TxResult> {
    let provider = wallet_provider(config)?;
    let contract_addr: Address = config
        .contract_address
        .parse()
        .context("Invalid CONTRACT_ADDRESS")?;
    let call = IPrivacyVerifier::depositCall {
        commitment: commitment.into(),
    };
    let tx = TransactionRequest::default()
        .with_to(contract_addr)
        .with_value(U256::from(amount))
        .with_input(Bytes::from(call.abi_encode()));
    send(provider, tx).await
}

pub async fn is_nullifier_used(nullifier: [u8; 32], config: &ChainConfig) -> Result<bool> {
    let rpc_url = config.rpc_url.parse().context("Invalid SEPOLIA_RPC_URL")?;
    let provider = ProviderBuilder::new().connect_http(rpc_url);
    let contract_addr: Address = config
        .contract_address
        .parse()
        .context("Invalid CONTRACT_ADDRESS")?;
    let call = IPrivacyVerifier::isNullifierUsedCall {
        nullifier: nullifier.into(),
    };
    let tx = TransactionRequest::default()
        .with_to(contract_addr)
        .with_input(Bytes::from(call.abi_encode()));
    let bytes = provider
        .call(tx)
        .await
        .context("Failed to call isNullifierUsed")?;
    let decoded = IPrivacyVerifier::isNullifierUsedCall::abi_decode_returns(&bytes)?;
    Ok(decoded)
}

pub async fn balance(config: &ChainConfig) -> Result<U256> {
    let signer: PrivateKeySigner = config.private_key.parse().context("Invalid PRIVATE_KEY")?;
    let rpc_url = config.rpc_url.parse().context("Invalid SEPOLIA_RPC_URL")?;
    let provider = ProviderBuilder::new().connect_http(rpc_url);
    provider
        .get_balance(signer.address())
        .await
        .context("Failed to read wallet balance")
}

fn wallet_provider(config: &ChainConfig) -> Result<impl Provider + Clone> {
    let signer: PrivateKeySigner = config.private_key.parse().context("Invalid PRIVATE_KEY")?;
    let rpc_url = config.rpc_url.parse().context("Invalid SEPOLIA_RPC_URL")?;
    Ok(ProviderBuilder::new().wallet(signer).connect_http(rpc_url))
}

async fn send(provider: impl Provider + Clone, tx: TransactionRequest) -> Result<TxResult> {
    let pending = provider
        .send_transaction(tx)
        .await
        .context("Failed to send transaction")?;
    let tx_hash = *pending.tx_hash();
    let receipt = pending
        .get_receipt()
        .await
        .context("Failed while waiting for transaction receipt")?;
    Ok(TxResult {
        tx_hash,
        block_number: receipt.block_number,
        gas_used: Some(receipt.gas_used as u128),
    })
}

sol! {
    interface IPrivacyVerifier {
        function deposit(bytes32 commitment) external payable;
        function isNullifierUsed(bytes32 nullifier) external view returns (bool);
    }
}
