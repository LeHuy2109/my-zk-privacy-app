use crate::types::{ProofResult, TransactionInput, TransactionOutput};

// ─── Banner ──────────────────────────────────────────────────

pub fn print_banner() {
    println!("ZK Privacy App");
}

// ─── Input summary ───────────────────────────────────────────

pub fn print_input_summary(input: &TransactionInput) {
    println!("INPUT:");
    println!("  Secret:       [PRIVATE]");
    println!("  Amount:       {} token", input.amount);
    println!("  Recipient:    0x{}", hex_encode(&input.recipient));
    println!("  Merkle Root:  {}", hex_encode(&input.merkle_root));
    println!("  Tree Depth:   {} tang", input.merkle_path.len());
    println!();
}

// ─── Proof result ────────────────────────────────────────────

pub fn print_proof_result(result: &ProofResult) {
    let o = &result.output;
    println!("Kết quả Proof:");
    println!("  Status:      {}", if o.is_valid { "OK" } else { "FAIL" });
    println!("  Time:       {} ms", result.proving_time_ms);
    println!("  Merkle Root:     {}", hex_encode(&o.merkle_root));
    println!("  Nullifier Hash:  {}", hex_encode(&o.nullifier_hash));
    println!("  Recipient:       0x{}", hex_encode(&o.recipient));
    println!("  Amount:          {} token", o.amount);
    println!();
}

// ─── Verification ────────────────────────────────────────────

pub fn print_verification_success() {
    println!("XÁC MINH PROOF thành công:");
    println!();
}

// ─── Chain result ────────────────────────────────────────────

pub fn print_chain_result(tx_hash: &[u8], block: Option<u64>, gas: Option<u128>, explorer_url: &str) {
    println!("SEPOLIA TESTNET:");
    println!("  Tx:       0x{}", hex_encode(tx_hash));
    if let Some(b) = block { println!("  Block:    {}", b); }
    if let Some(g) = gas   { println!("  Gas:      {}", g); }
    println!("  Explorer: {}", explorer_url);
    println!();
}

pub fn print_chain_skipped() {
    println!("SEPOLIA: bỏ qua (cần --chain + cấu hình .env)");
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
    println!("Tóm tắt:");
    println!("- Cấu hình Sepolia trong file .env (RPC URL, private key, contract address).");
    println!("- Chạy `generate-proof` để tạo ZK proof cho giao dịch.");
    println!("- Chạy `submit-proof` để gửi proof lên Sepolia testnet.");
    println!();
}

// ─── Helpers ─────────────────────────────────────────────────

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}
