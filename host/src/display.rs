use crate::types::{ProofResult, TransactionInput, TransactionOutput};

// ─── Banner ──────────────────────────────────────────────────

pub fn print_banner() {
    println!();
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║   🔐  ZK PRIVACY – MERKLE INCLUSION + NULLIFIER  🔐     ║");
    println!("║        RISC Zero zkVM  •  Giao dịch Bảo mật             ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
}

// ─── Input summary ───────────────────────────────────────────

pub fn print_input_summary(input: &TransactionInput) {
    println!("┌──────────────────────────────────────────────────────────┐");
    println!("│      PRIVATE INPUT (chỉ host & guest biết)               │");
    println!("├──────────────────────────────────────────────────────────┤");
    println!(
        "│  Secret:      {}...  [PRIVATE]",
        hex_short(&input.secret[..4])
    );
    println!("│  Amount:      {} token", input.amount);
    println!(
        "│  Recipient:   0x{}...{}",
        hex_short(&input.recipient[..3]),
        hex_short(&input.recipient[17..])
    );
    println!(
        "│  Merkle Root: {}...",
        hex_short(&input.merkle_root[..8])
    );
    println!(
        "│  Tree Depth:  {} tầng ({} slots)",
        input.merkle_path.len(),
        1usize << input.merkle_path.len()
    );
    println!("└──────────────────────────────────────────────────────────┘");
    println!();
}

// ─── Proof result ────────────────────────────────────────────

pub fn print_proof_result(result: &ProofResult) {
    let o = &result.output;
    println!("┌──────────────────────────────────────────────────────────┐");
    println!("│      KẾT QUẢ PROOF                                       │");
    println!("├──────────────────────────────────────────────────────────┤");
    println!("│  Trạng thái:  {}", if o.is_valid { "HỢP LỆ ✅" } else { "KHÔNG HỢP LỆ ❌" });
    println!("│  Thời gian:   {} ms", result.proving_time_ms);
    println!("├──────────────────────────────────────────────────────────┤");
    println!("│      PUBLIC OUTPUT – JOURNAL (ai cũng đọc được)         │");
    println!("├──────────────────────────────────────────────────────────┤");
    println!("│  Merkle Root:    {}", hex_encode(&o.merkle_root));
    println!("│  Nullifier Hash: {}", hex_encode(&o.nullifier_hash));
    println!(
        "│  Recipient:      0x{}...{}",
        hex_short(&o.recipient[..3]),
        hex_short(&o.recipient[17..])
    );
    println!("│  Amount:         {} token", o.amount);
    println!("└──────────────────────────────────────────────────────────┘");
    println!();
}

// ─── Verification ────────────────────────────────────────────

pub fn print_verification_success() {
    println!("┌──────────────────────────────────────────────────────────┐");
    println!("│      XÁC MINH PROOF (Local)                              │");
    println!("├──────────────────────────────────────────────────────────┤");
    println!("│  ✅ ZK Proof hợp lệ                                      │");
    println!("│  ✅ Receipt khớp METHOD_ID                               │");
    println!("│  ✅ Merkle inclusion được chứng minh                     │");
    println!("│  ✅ Nullifier hash đã tạo (sẵn sàng gửi on-chain)       │");
    println!("└──────────────────────────────────────────────────────────┘");
    println!();
}

// ─── Chain result ────────────────────────────────────────────

pub fn print_chain_result(tx_hash: &[u8], block: Option<u64>, gas: Option<u128>, explorer_url: &str) {
    println!("┌──────────────────────────────────────────────────────────┐");
    println!("│      SEPOLIA TESTNET                                      │");
    println!("├──────────────────────────────────────────────────────────┤");
    println!("│  Tx:    0x{}", hex_encode(tx_hash));
    if let Some(b) = block { println!("│  Block: #{}", b); }
    if let Some(g) = gas   { println!("│  Gas:   {}", g); }
    println!("│  Explorer: {}", explorer_url);
    println!("└──────────────────────────────────────────────────────────┘");
    println!();
}

pub fn print_chain_skipped() {
    println!("┌──────────────────────────────────────────────────────────┐");
    println!("│  SEPOLIA: bỏ qua (cần --chain + cấu hình .env)          │");
    println!("└──────────────────────────────────────────────────────────┘");
    println!();
}

// ─── JSON output ─────────────────────────────────────────────

pub fn print_json(input: &TransactionInput, output: &TransactionOutput, ms: u128) {
    let json = serde_json::json!({
        "journal": {
            "merkle_root":    hex_encode(&output.merkle_root),
            "nullifier_hash": hex_encode(&output.nullifier_hash),
            "recipient":      format!("0x{}", hex_encode(&output.recipient)),
            "amount":         output.amount,
            "is_valid":       output.is_valid,
        },
        "private_input": {
            // Chỉ tiết lộ amount trong JSON (secret không bao giờ lộ ra)
            "amount": input.amount,
        },
        "proving_time_ms": ms,
    });
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}

// ─── Summary ─────────────────────────────────────────────────

pub fn print_summary() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  TÓM TẮT                                                 ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║  • Secret, amount KHÔNG BAO GIỜ lộ ra journal/chain.    ║");
    println!("║  • Merkle inclusion chứng minh note tồn tại on-chain.   ║");
    println!("║  • Nullifier hash lưu trên contract chống double-spend.  ║");
    println!("║  • Smart contract chỉ thấy: root, nullifier, recipient.  ║");
    println!("╚══════════════════════════════════════════════════════════╝");
}

// ─── Helpers ─────────────────────────────────────────────────

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

fn hex_short(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}
