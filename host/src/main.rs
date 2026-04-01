mod chain;
mod display;
mod executor;
mod prover;
mod types;

use anyhow::Result;
use clap::Parser;

use types::ChainConfig;

// ─── CLI Arguments ───────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "zk-privacy-host",
    about = "🔐 ZK Privacy Transaction Host – RISC Zero zkVM + Sepolia"
)]
struct Cli {
    /// Số tiền giao dịch (mặc định: 500)
    #[arg(long, default_value_t = 500)]
    amount: u64,

    /// Số dư người gửi (mặc định: 1000)
    #[arg(long, default_value_t = 1000)]
    balance: u64,

    /// Địa chỉ người gửi (hex, mặc định: demo address)
    #[arg(long)]
    sender: Option<String>,

    /// Địa chỉ người nhận (hex, mặc định: demo address)
    #[arg(long)]
    receiver: Option<String>,

    /// Gửi proof lên Sepolia testnet
    #[arg(long, default_value_t = false)]
    chain: bool,

    /// Nén proof thành Groth16 (yêu cầu RAM cao)
    #[arg(long, default_value_t = false)]
    groth16: bool,

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

    // ── 2. Build TransactionInput ────────────────────────────
    let input = match (&cli.sender, &cli.receiver) {
        (Some(sender), Some(receiver)) => {
            executor::build_custom_input(sender, receiver, cli.amount, cli.balance)?
        }
        _ => {
            let mut demo = executor::build_demo_input();
            demo.amount = cli.amount;
            demo.sender_balance = cli.balance;
            demo
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

            let chain_result = chain::submit_proof(&result.receipt, &config).await?;

            display::print_chain_result(
                &chain_result.explorer_url,
                chain_result.block_number,
                chain_result.gas_used,
            );
        }
    } else {
        display::print_chain_skipped();
    }

    // ── 6. Summary ───────────────────────────────────────────
    display::print_summary();

    Ok(())
}
