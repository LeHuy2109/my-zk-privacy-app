use anyhow::Result;
use clap::Parser;
use host::zk_auth::{load_config, run_availability_benchmark, write_json_result};

#[derive(Parser, Debug)]
#[command(name = "zk_auth_availability_benchmark")]
struct Cli {
    #[arg(long, default_value_t = 10)]
    count: u64,
    #[arg(long, default_value_t = 0)]
    max_retries: u64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config()?;
    let result = run_availability_benchmark(&config, cli.count, cli.max_retries)?;
    let output = write_json_result("availability", &result)?;

    println!("ZK auth availability benchmark");
    println!(
        "Traditional success rate: {}%",
        result.traditional.success_rate_percent
    );
    println!(
        "ZK auth success rate: {}%",
        result.zk_auth.success_rate_percent
    );
    println!("Result file: {}", output.display());

    Ok(())
}
