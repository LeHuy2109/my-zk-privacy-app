use anyhow::Result;
use host::zk_auth::{load_config, run_integrity_cases, write_json_result};

fn main() -> Result<()> {
    let config = load_config()?;
    let result = run_integrity_cases(&config)?;
    let output = write_json_result("integrity", &result)?;

    println!("ZK auth integrity cases");
    println!("Cases: {}", result.cases.len());
    println!("Result file: {}", output.display());

    Ok(())
}
