mod chain;
mod display;
mod executor;
mod prover;
mod types;

use anyhow::{Result, Context};
use clap::Parser;

use types::ChainConfig;

// ─── CLI Arguments ───────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "zk-privacy-host",
    about = "🔐 ZK Privacy – Merkle Inclusion Proof + Nullifier (RISC Zero + Sepolia)"
)]
struct Cli {
    /// Số tiền giao dịch (mặc định: 500)
    #[arg(long, default_value_t = 500)]
    amount: u64,

    /// Địa chỉ người nhận (hex 20-byte, bỏ trống để dùng demo address)
    #[arg(long)]
    recipient: Option<String>,

    /// Secret (Bí mật) người dùng sinh ra lúc deposit (Bắt buộc khi chạy --chain)
    #[arg(long)]
    secret: Option<String>,

    /// Gửi proof lên Sepolia testnet
    #[arg(long, default_value_t = false)]
    chain: bool,

    /// Nén proof sang Groth16 SNARK local (yêu cầu RAM ~16GB+)
    #[arg(long, default_value_t = false)]
    groth16: bool,

    /// Công cụ tính Commitment để nạp tiền (Deposit)
    #[arg(long, default_value_t = false)]
    deposit: bool,

    /// Xuất kết quả dạng JSON
    #[arg(long, default_value_t = false)]
    json: bool,
}

// ─── Main ────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let _ = dotenv::dotenv();
    let cli = Cli::parse();
    
    // ── 1. Banner ────────────────────────────────────────────
    if !cli.json {
        display::print_banner();
    }

    if cli.deposit {
        let secret_hex = cli.secret.as_deref().context("Cần truyền --secret khi chạy --deposit")?;
        let cleaned = secret_hex.strip_prefix("0x").unwrap_or(secret_hex);
        let secret_bytes = hex::decode(cleaned).context("Lỗi decode hex secret")?;
        anyhow::ensure!(secret_bytes.len() == 32, "Secret phải là 32 bytes hex");
        
        let mut s = [0u8; 32];
        s.copy_from_slice(&secret_bytes);
        
        let commitment = executor::compute_leaf(&s, cli.amount);
        
        println!("🚀 TÍNH TOÁN LỆNH DEPOSIT (NẠP TIỀN)");
        println!("────────────────────────────────────────────────────────────");
        println!("🔑 Secret Input : {}", secret_hex);
        println!("💰 Lượng (Amount): {}", cli.amount);
        println!("🔒 Commitment   : 0x{}", alloy::hex::encode(commitment));
        println!("────────────────────────────────────────────────────────────");
        println!("💡 HƯỚNG DẪN:");
        println!("1. Dùng ví có tiền mạng Sepolia.");
        println!("2. Gọi thử qua giao diện Remix, hoặc Cast:");
        println!("   cast send <CONTRACT_ADDRESS> \"deposit(bytes32)\" 0x{} --value {} --rpc-url <YOUR_RPC_URL> --private-key <YOUR_PK>", 
            alloy::hex::encode(commitment), cli.amount);
        
        return Ok(());
    }

    // ── 2. Build TransactionInput (Hỗ trợ On-chain hoặc Offline) ──
    let input = if cli.chain {
        let config = ChainConfig::from_env()?;
        if !config.is_configured() {
            anyhow::bail!("Thiếu cấu hình Sepolia (.env). Vui lòng cấu hình hợp lệ khi sử dụng --chain");
        }
        let secret = cli.secret.as_deref().context("Cần truyền --secret khi chạy --chain")?;
        let recipient = cli.recipient.as_deref().context("Cần truyền --recipient khi chạy --chain")?;
        
        println!("⏳ Đang tải dữ liệu Merkle Tree từ Smart Contract (Sepolia)...");
        executor::build_custom_input_on_chain(secret, cli.amount, recipient, &config).await?
    } else {
        match &cli.recipient {
            // Khi chạy demo off-chain
            Some(recipient) => {
                executor::build_custom_input(
                    recipient, 
                    cli.amount, 
                    cli.secret.as_deref()
                )?
            },
            None => executor::build_demo_input(cli.amount),
        }
    };

    if !cli.json {
        display::print_input_summary(&input);
    }

    // ── 3. Executor + Prover: tạo ZK proof ──────────────────
    if !cli.json {
        println!("⏳ Đang chạy Executor & Prover (tạo ZK proof)...\n");
    }

    let result = prover::prove_transaction(&input, cli.groth16)?;

    if cli.json {
        display::print_json(&input, &result.output, result.proving_time_ms);
        return Ok(());
    }

    display::print_proof_result(&result);

    // ── 4. Verify locally ────────────────────────────────────
    prover::verify_receipt(&result.receipt)?;
    display::print_verification_success();

    // ── 5. Submit to Sepolia (nếu --chain) ───────────────────
    if cli.chain {
        let config = ChainConfig::from_env()?;
        if !config.is_configured() {
            display::print_chain_skipped();
        } else {
            println!("⏳ Đang gửi proof lên Sepolia testnet...\n");
            let chain_result = chain::submit_proof(&result.receipt, &result.output, &config).await?;
            display::print_chain_result(
                chain_result.tx_hash.as_ref(),
                chain_result.block_number,
                chain_result.gas_used,
                &chain_result.explorer_url,
            );
        }
    } else {
        display::print_chain_skipped();
    }

    // ── 6. Summary ───────────────────────────────────────────
    display::print_summary();

    Ok(())
}
