use anyhow::Result;
use host::zk_auth::{load_config, run_traditional_demo, write_json_result};

fn main() -> Result<()> {
    let config = load_config()?;
    let result = run_traditional_demo(&config, None)?;
    let output = write_json_result("traditional", &result)?;

    println!("Traditional zk-auth baseline");
    println!("Tx hash: {}", result.tx_hash);
    println!("Gas used: {}", result.gas_used);
    println!("Result file: {}", output.display());

    Ok(())
}
