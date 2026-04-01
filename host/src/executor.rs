use anyhow::{Context, Result};
use rand::RngCore;
use risc0_zkvm::ExecutorEnv;

use crate::types::TransactionInput;

// ─── Tạo TransactionInput demo ───────────────────────────────

pub fn build_demo_input() -> TransactionInput {
    TransactionInput {
        sender_address: [
            0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89,
            0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89,
            0xAB, 0xCD, 0x12, 0x34,
        ],
        receiver_address: [
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
            0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00,
            0x11, 0x22, 0xAA, 0xBB,
        ],
        amount: 500,
        sender_balance: 1000,
        nonce: generate_nonce(),
    }
}

// ─── Tạo TransactionInput custom từ CLI args ─────────────────

pub fn build_custom_input(
    sender_hex: &str,
    receiver_hex: &str,
    amount: u64,
    balance: u64,
) -> Result<TransactionInput> {
    let sender_address = parse_address(sender_hex)
        .context("Địa chỉ sender không hợp lệ")?;
    let receiver_address = parse_address(receiver_hex)
        .context("Địa chỉ receiver không hợp lệ")?;

    Ok(TransactionInput {
        sender_address,
        receiver_address,
        amount,
        sender_balance: balance,
        nonce: generate_nonce(),
    })
}

// ─── Tạo ExecutorEnv chứa private input ──────────────────────

pub fn create_executor_env(input: &TransactionInput) -> Result<ExecutorEnv<'static>> {
    let env = ExecutorEnv::builder()
        .write(input)
        .context("Ghi TransactionInput vào env thất bại")?
        .build()
        .context("Tạo ExecutorEnv thất bại")?;

    Ok(env)
}

// ─── Helpers ──────────────────────────────────────────────────

fn generate_nonce() -> [u8; 32] {
    let mut nonce = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce);
    nonce
}

fn parse_address(hex_str: &str) -> Result<[u8; 20]> {
    let cleaned = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let bytes = hex::decode(cleaned)
        .context("Hex decode thất bại")?;

    if bytes.len() != 20 {
        anyhow::bail!("Địa chỉ phải đúng 20 bytes, nhận được {} bytes", bytes.len());
    }

    let mut addr = [0u8; 20];
    addr.copy_from_slice(&bytes);
    Ok(addr)
}
