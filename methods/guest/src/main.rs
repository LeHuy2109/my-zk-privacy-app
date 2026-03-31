// ============================================================
// RISC0 Guest Program: ZK Privacy Transaction
//
// Mục đích: Chứng minh một giao dịch hợp lệ mà không tiết lộ
//   - Địa chỉ người gửi (sender)
//   - Địa chỉ người nhận (receiver)
//   - Số tiền giao dịch (amount)
//
// Dữ liệu private (chỉ guest biết, không lộ ra ngoài):
//   - sender_address, receiver_address, amount, sender_balance
//
// Dữ liệu public (commit ra journal, blockchain có thể xác minh):
//   - sender_commitment:   Hash(sender_address   || nonce)
//   - receiver_commitment: Hash(receiver_address || nonce)
//   - amount_commitment:   Hash(amount           || nonce)
//   - Cờ is_valid (bool): giao dịch có hợp lệ không
// ============================================================

#![no_main]

use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

risc0_zkvm::guest::entry!(main);

// -------------------------------------------------------
// Struct input (private – chỉ guest thấy, host gửi vào)
// -------------------------------------------------------
#[derive(Serialize, Deserialize)]
pub struct TransactionInput {
    /// Địa chỉ 20-byte của người gửi (dạng hex string hoặc bytes)
    pub sender_address: [u8; 20],
    /// Địa chỉ 20-byte của người nhận
    pub receiver_address: [u8; 20],
    /// Số tiền giao dịch (đơn vị: wei hoặc token nhỏ nhất)
    pub amount: u64,
    /// Số dư hiện tại của người gửi
    pub sender_balance: u64,
    /// Nonce ngẫu nhiên để tạo commitment riêng biệt, chống replay
    pub nonce: [u8; 32],
}

// -------------------------------------------------------
// Struct output (public – commit ra journal, ai cũng đọc được)
// -------------------------------------------------------
#[derive(Serialize, Deserialize)]
pub struct TransactionOutput {
    /// Commitment của địa chỉ người gửi: SHA256(sender || nonce)
    pub sender_commitment: [u8; 32],
    /// Commitment của địa chỉ người nhận: SHA256(receiver || nonce)
    pub receiver_commitment: [u8; 32],
    /// Commitment của số tiền: SHA256(amount_bytes || nonce)
    pub amount_commitment: [u8; 32],
    /// Giao dịch có hợp lệ không (amount > 0 và balance đủ)
    pub is_valid: bool,
}

// -------------------------------------------------------
// Hàm tạo commitment: SHA256(data || nonce)
// -------------------------------------------------------
fn compute_commitment(data: &[u8], nonce: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.update(nonce);
    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(&result);
    output
}

fn main() {
    // 1. Đọc private input từ host (không bao giờ lộ ra journal)
    let tx: TransactionInput = env::read();

    // 2. Kiểm tra các điều kiện hợp lệ (chạy trong ZK, không lộ giá trị)
    let is_valid = tx.amount > 0 && tx.sender_balance >= tx.amount;

    // 3. Tạo commitment cho từng trường nhạy cảm
    let sender_commitment = compute_commitment(&tx.sender_address, &tx.nonce);

    let receiver_commitment = compute_commitment(&tx.receiver_address, &tx.nonce);

    // Chuyển amount thành bytes để hash
    let amount_bytes = tx.amount.to_le_bytes();
    let amount_commitment = compute_commitment(&amount_bytes, &tx.nonce);

    // 4. Tạo output công khai
    let output = TransactionOutput {
        sender_commitment,
        receiver_commitment,
        amount_commitment,
        is_valid,
    };

    // 5. Commit ra journal – blockchain/verifier sẽ đọc phần này
    //    Đây là SỰ THẬT công khai: proof rằng giao dịch hợp lệ,
    //    mà không hề tiết lộ sender, receiver, hay amount thật sự.
    env::commit(&output);
}
