use crate::types::{ProofResult, TransactionInput, TransactionOutput};

// ─── Banner ──────────────────────────────────────────────────

pub fn print_banner() {
    println!();
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║       🔐  ZK PRIVACY TRANSACTION – RISC ZERO  🔐        ║");
    println!("║        Giao dịch bảo mật Zero-Knowledge Proof            ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
}

// ─── Hiển thị input summary ──────────────────────────────────

pub fn print_input_summary(input: &TransactionInput) {
    println!("┌──────────────────────────────────────────────────────────┐");
    println!("│      THÔNG TIN GIAO DỊCH (Private – chỉ host biết)       │");
    println!("├──────────────────────────────────────────────────────────┤");
    println!(
        "│  Người gửi:   0x{}...{}",
        hex_short(&input.sender_address[..3]),
        hex_short(&input.sender_address[17..])
    );
    println!(
        "│  Người nhận:  0x{}...{}",
        hex_short(&input.receiver_address[..3]),
        hex_short(&input.receiver_address[17..])
    );
    println!("│  Số tiền:     {} token", input.amount);
    println!("│  Số dư gửi:   {} token", input.sender_balance);
    println!(
        "│  Nonce:       {}...",
        hex_short(&input.nonce[..8])
    );
    println!("└──────────────────────────────────────────────────────────┘");
    println!();
}

// ─── Hiển thị kết quả proving ────────────────────────────────

pub fn print_proof_result(result: &ProofResult) {
    let output = &result.output;

    println!("┌──────────────────────────────────────────────────────────┐");
    println!("│      KẾT QUẢ EXECUTOR & PROVER                           │");
    println!("├──────────────────────────────────────────────────────────┤");
    println!("│  Trạng thái:  {}", status_icon(output.is_valid));
    println!("│  Thời gian:   {} ms", result.proving_time_ms);
    println!("├──────────────────────────────────────────────────────────┤");
    println!("│       COMMITMENT HASH (Public – Journal)                 │");
    println!("├──────────────────────────────────────────────────────────┤");
    println!(
        "│  Sender:    {}",
        hex_encode(&output.sender_commitment)
    );
    println!(
        "│  Receiver:  {}",
        hex_encode(&output.receiver_commitment)
    );
    println!(
        "│  Amount:    {}",
        hex_encode(&output.amount_commitment)
    );
    println!("└──────────────────────────────────────────────────────────┘");
    println!();
}

// ─── Hiển thị kết quả verify ─────────────────────────────────

pub fn print_verification_success() {
    println!("┌──────────────────────────────────────────────────────────┐");
    println!("│       XÁC MINH PROOF (Local Verification)                │");
    println!("├──────────────────────────────────────────────────────────┤");
    println!("│   ZK Proof hợp lệ!                                       │");
    println!("│   Receipt khớp với METHOD_ID                             │");
    println!("│   Journal chưa bị thay đổi                               │");
    println!("└──────────────────────────────────────────────────────────┘");
    println!();
}

// ─── Hiển thị kết quả Sepolia ────────────────────────────────

pub fn print_chain_result(tx_hash: &str, block: Option<u64>, gas: Option<u128>) {
    println!("┌──────────────────────────────────────────────────────────┐");
    println!("│       KẾT QUẢ SEPOLIA TESTNET                            │");
    println!("├──────────────────────────────────────────────────────────┤");
    println!("│  Tx Hash:   {}", tx_hash);
    if let Some(b) = block {
        println!("│  Block:     #{}", b);
    }
    if let Some(g) = gas {
        println!("│  Gas Used:  {}", g);
    }
    println!("│  Explorer:  https://sepolia.etherscan.io/tx/{}", tx_hash);
    println!("└──────────────────────────────────────────────────────────┘");
    println!();
}

// ─── Hiển thị khi skip chain ─────────────────────────────────

pub fn print_chain_skipped() {
    println!("┌──────────────────────────────────────────────────────────┐");
    println!("│     SEPOLIA TESTNET: BỎ QUA                              │");
    println!("├──────────────────────────────────────────────────────────┤");
    println!("│  Chưa cấu hình .env (PRIVATE_KEY, CONTRACT_ADDRESS).     │");
    println!("│  Dùng --chain flag và cấu hình .env để gửi on-chain.     │");
    println!("└──────────────────────────────────────────────────────────┘");
    println!();
}

// ─── JSON output ─────────────────────────────────────────────

pub fn print_json(input: &TransactionInput, output: &TransactionOutput, proving_time_ms: u128) {
    let json = serde_json::json!({
        "transaction": {
            "sender_commitment": hex_encode(&output.sender_commitment),
            "receiver_commitment": hex_encode(&output.receiver_commitment),
            "amount_commitment": hex_encode(&output.amount_commitment),
            "is_valid": output.is_valid,
        },
        "private_input": {
            "amount": input.amount,
            "sender_balance": input.sender_balance,
        },
        "proving_time_ms": proving_time_ms,
    });

    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}

// ─── Summary cho báo cáo ─────────────────────────────────────

pub fn print_summary() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         TÓM TẮT                                          ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║  • Dữ liệu private (sender, receiver, amount) KHÔNG      ║");
    println!("║    bao giờ xuất hiện trên blockchain.                    ║");
    println!("║  • Blockchain chỉ lưu commitment hash + ZK proof.        ║");
    println!("║  • Bất kỳ ai cũng verify được proof mà không biết        ║");
    println!("║    thông tin gốc.                                        ║");
    println!("╚══════════════════════════════════════════════════════════╝");
}

// ─── Helpers ──────────────────────────────────────────────────

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_short(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn status_icon(valid: bool) -> &'static str {
    if valid {
        "Giao dịch HỢP LỆ"
    } else {
        "Giao dịch KHÔNG HỢP LỆ"
    }
}
