use anyhow::Result;
use clap::Parser;
use host::zk_auth::{
    load_config, run_zk_auth_demo, write_json_result, zk_auth_image_id_hex, ZkAuthDemoOptions,
    DEFAULT_ACTION_TYPE,
};

#[derive(Parser, Debug)]
#[command(name = "zk_auth_demo")]
struct Cli {
    #[arg(long)]
    payload: Option<String>,
    #[arg(long)]
    secret: Option<String>,
    #[arg(long)]
    nonce: Option<String>,
    #[arg(long)]
    recipient: Option<String>,
    #[arg(long, default_value_t = DEFAULT_ACTION_TYPE)]
    action_type: u32,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = load_config()?;
    let result = run_zk_auth_demo(
        &config,
        ZkAuthDemoOptions {
            payload: cli.payload,
            secret_hex: cli.secret,
            nonce_hex: cli.nonce,
            recipient: cli.recipient,
            action_type: cli.action_type,
            groth16: true,
        },
    )?;
    let output = write_json_result("zk_auth", &result)?;

    println!("ZK auth demo");
    println!("Image ID: {}", zk_auth_image_id_hex());
    println!("Tx hash: {}", result.tx_hash);
    println!("Artifact: {}", result.artifact_ref);
    println!("Result file: {}", output.display());

    Ok(())
}
