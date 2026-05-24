use anyhow::Result;
use clap::Parser;
use host::zk_auth::{
    fetch_record, load_config, resolve_latest_record_id, verify_e2e_from_record, write_json_result,
};

#[derive(Parser, Debug)]
#[command(name = "zk_auth_verify_e2e")]
struct Cli {
    #[arg(long)]
    record_id: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config()?;
    let record_id = match cli.record_id {
        Some(record_id) => record_id,
        None => resolve_latest_record_id(&config).await?,
    };
    let record = fetch_record(&config, record_id).await?;
    let result = verify_e2e_from_record(record_id, &record)?;
    let output = write_json_result("verify_e2e", &result)?;

    println!("ZK auth end-to-end verification");
    println!("Record ID: {}", result.record_id);
    println!("Verification passed: {}", result.e2e_result);
    println!("Result file: {}", output.display());

    Ok(())
}
