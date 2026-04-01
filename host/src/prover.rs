use anyhow::{Context, Result};
use methods::{METHOD_ELF, METHOD_ID};
use risc0_zkvm::{default_prover, Receipt};
use std::time::Instant;

use crate::executor;
use crate::types::{ProofResult, TransactionInput, TransactionOutput};

// ─── Chạy toàn bộ pipeline: Executor → Prover → Verify ──────

pub fn prove_transaction(input: &TransactionInput, groth16: bool) -> Result<ProofResult> {
    let env = executor::create_executor_env(input)?;

    let start = Instant::now();
    let prover = default_prover();
    
    // Tạo initial proof (STARK)
    let prove_info = prover
        .prove(env, METHOD_ELF)
        .context("Tạo ZK proof thất bại")?;
    
    let mut receipt = prove_info.receipt;

    // Nếu người dùng yêu cầu Groth16 nén (Localize)
    if groth16 {
        println!("⏳ Đang thực hiện nén STARK sang Groth16 SNARK (Local Proving)...");
        receipt = prover
            .compress(&receipt)
            .context("Nén Groth16 thất bại - Máy có đủ RAM và Proving Key chưa?")?;
    }

    let proving_time_ms = start.elapsed().as_millis();
    let output = extract_output(&receipt)?;

    Ok(ProofResult {
        receipt,
        output,
        proving_time_ms,
    })
}

// ─── Xác minh Receipt (locally) ──────────────────────────────

pub fn verify_receipt(receipt: &Receipt) -> Result<()> {
    receipt
        .verify(METHOD_ID)
        .context("Xác minh ZK proof thất bại – proof không hợp lệ")?;
    Ok(())
}

// ─── Decode TransactionOutput từ journal ─────────────────────

pub fn extract_output(receipt: &Receipt) -> Result<TransactionOutput> {
    receipt
        .journal
        .decode()
        .context("Giải mã journal thất bại")
}
