use anyhow::Result;
use host::zk_auth::compare_latest_results;

fn main() -> Result<()> {
    let summary = compare_latest_results()?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
