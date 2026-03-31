// ============================================================
// RISC0 Host Program: ZK Privacy Transaction Demo
//
// Host gửi TransactionInput (private) vào guest,
// nhận proof và đọc TransactionOutput (public) từ journal.
// ============================================================

use methods::{METHOD_ELF, METHOD_ID};
use risc0_zkvm::{default_prover, ExecutorEnv};
use serde::{Deserialize, Serialize};

// -------------------------------------------------------
// Phải định nghĩa lại cùng struct với guest để serialize/deserialize
// (Trong dự án thực tế, nên dùng một crate shared chung)
// -------------------------------------------------------
#[derive(Serialize, Deserialize)]
pub struct TransactionInput {
    pub sender_address: [u8; 20],
    pub receiver_address: [u8; 20],
    pub amount: u64,
    pub sender_balance: u64,
    pub nonce: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionOutput {
    pub sender_commitment: [u8; 32],
    pub receiver_commitment: [u8; 32],
    pub amount_commitment: [u8; 32],
    pub is_valid: bool,
}

fn main() {
    // Khởi tạo logging (RUST_LOG=info cargo run để xem log)
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    // -------------------------------------------------------
    // Tạo dữ liệu giao dịch riêng tư (chỉ host & guest biết)
    // Trong ứng dụng thực tế, các giá trị này đến từ ví/người dùng
    // -------------------------------------------------------
    let tx_input = TransactionInput {
        // Địa chỉ người gửi: 0xABCD...1234 (20 bytes)
        sender_address: [
            0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89,
            0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89,
            0xAB, 0xCD, 0x12, 0x34,
        ],
        // Địa chỉ người nhận: 0x1122...AABB (20 bytes)
        receiver_address: [
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
            0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00,
            0x11, 0x22, 0xAA, 0xBB,
        ],
        // Số tiền: 500 đơn vị token
        amount: 500,
        // Số dư người gửi: 1000 đơn vị token (đủ để giao dịch)
        sender_balance: 1000,
        // Nonce ngẫu nhiên 32 bytes (chống replay attack)
        nonce: [
            0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
        ],
    };

    println!("=== ZK Privacy Transaction Demo ===");
    println!("[HOST] Gửi giao dịch vào ZK prover (dữ liệu được giữ bí mật)...");
    println!("[HOST] Số tiền: {} token", tx_input.amount);
    println!("[HOST] Số dư gửi: {} token", tx_input.sender_balance);

    // -------------------------------------------------------
    // Tạo ExecutorEnv và ghi private input vào
    // -------------------------------------------------------
    let env = ExecutorEnv::builder()
        .write(&tx_input)
        .unwrap()
        .build()
        .unwrap();

    // Lấy prover mặc định (mock prover khi dev, BonsaiProver khi production)
    let prover = default_prover();

    println!("[HOST] Đang tạo ZK proof...");
    let prove_info = prover
        .prove(env, METHOD_ELF)
        .expect("Tạo proof thất bại");

    let receipt = prove_info.receipt;

    // -------------------------------------------------------
    // Đọc public output từ journal
    // Journal chứa các commitment hash – KHÔNG có thông tin thật
    // -------------------------------------------------------
    let output: TransactionOutput = receipt
        .journal
        .decode()
        .expect("Giải mã journal thất bại");

    println!("\n=== KẾT QUẢ PUBLIC (Journal) ===");
    println!("[PUBLIC] Giao dịch hợp lệ: {}", output.is_valid);
    println!(
        "[PUBLIC] Commitment người gửi:  {}",
        hex_encode(&output.sender_commitment)
    );
    println!(
        "[PUBLIC] Commitment người nhận: {}",
        hex_encode(&output.receiver_commitment)
    );
    println!(
        "[PUBLIC] Commitment số tiền:    {}",
        hex_encode(&output.amount_commitment)
    );
    println!("\n[Lưu ý] Địa chỉ thật và số tiền thật KHÔNG xuất hiện ở trên.");

    // -------------------------------------------------------
    // Xác minh proof (bên thứ 3 / blockchain có thể làm điều này)
    // -------------------------------------------------------
    receipt
        .verify(METHOD_ID)
        .expect("Xác minh proof thất bại");

    println!("\n[VERIFIER] ✓ ZK Proof hợp lệ! Giao dịch được chấp nhận.");
    println!("[VERIFIER] Blockchain ghi nhận commitment, không biết sender/receiver/amount thật.");
}

/// Chuyển byte array thành chuỗi hex để hiển thị
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
