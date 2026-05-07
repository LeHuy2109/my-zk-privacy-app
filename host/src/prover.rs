use anyhow::{Context, Result};
use methods::{METHOD_ELF, METHOD_ID};
use risc0_zkvm::{default_prover, ProverOpts, Receipt};
use std::time::Instant;

use crate::executor;
use crate::groth16_docker;
use crate::types::{ProofResult, TransactionInput, TransactionOutput};

// ─── Chạy toàn bộ pipeline: Executor → Prover → (Groth16) ───

pub fn prove_transaction(input: &TransactionInput, groth16: bool) -> Result<ProofResult> {
    if groth16 {
        groth16_docker::prepare().with_context(|| {
            format!(
                "Groth16 Docker prover is not ready (image: {})",
                groth16_docker::image_name()
            )
        })?;
    }

    let env = executor::create_executor_env(input)?;

    let start = Instant::now();
    let prover = default_prover();

    // ── Bước 1: Tạo STARK proof ──────────────────────────────
    let prove_info = prover
        .prove(env, METHOD_ELF)
        .context("Tạo ZK proof thất bại")?;

    let mut receipt = prove_info.receipt;

    // ── Bước 2 (tuỳ chọn): Nén STARK → Groth16 SNARK ────────
    if groth16 {
        println!("Nén STARK thành Groth16 SNARK (cần ít nhất 16GB RAM)\n");
        receipt = prover
            .compress(&ProverOpts::groth16(), &receipt)
            .with_context(|| {
                format!(
                    "Nen Groth16 that bai. Docker image: {}. RISC0_WORK_DIR: {}. \
                     Neu Docker bao exit code 127, hay xoa image cu/hong roi chay lai: \
                     docker image rm {}",
                    groth16_docker::image_name(),
                    std::env::var("RISC0_WORK_DIR").unwrap_or_else(|_| "<temp>".to_string()),
                    groth16_docker::image_name()
                )
            })?;
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
        .context("Xác minh ZK proof thất bại - proof không hợp lệ")?;
    Ok(())
}

// ─── Decode TransactionOutput từ journal ─────────────────────

use alloy::sol_types::{sol, SolType};

sol! {
    struct Journal {
        bytes32 merkle_root;
        bytes32 nullifier_hash;
        address recipient;
        uint256 amount;
        bool is_valid;
    }
}

pub fn extract_output(receipt: &Receipt) -> Result<TransactionOutput> {
    let bytes = receipt.journal.bytes.as_slice();
    let decoded = Journal::abi_decode(bytes).context("Giải mã ABI journal thất bại")?;

    Ok(TransactionOutput {
        merkle_root: decoded.merkle_root.0,
        nullifier_hash: decoded.nullifier_hash.0,
        recipient: decoded.recipient.0.0,
        amount: decoded.amount.try_into().unwrap_or(0),
        is_valid: decoded.is_valid,
    })
}
