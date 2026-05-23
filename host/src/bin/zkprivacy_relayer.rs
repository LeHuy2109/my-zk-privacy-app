#[path = "../chain.rs"]
mod chain;
#[path = "../executor.rs"]
mod executor;
#[path = "../groth16_docker.rs"]
mod groth16_docker;
#[path = "../prover.rs"]
mod prover;
#[path = "../types.rs"]
mod types;
#[path = "../zkprivacy_chain.rs"]
mod zkprivacy_chain;
#[path = "../zkprivacy_relayer.rs"]
mod zkprivacy_relayer;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "zkprivacy-relayer",
    about = "HTTP relayer for zkprivacy withdraw proofs"
)]
struct Cli {
    #[arg(long, default_value = "127.0.0.1:8787")]
    bind: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    zkprivacy_relayer::serve(&cli.bind).await
}
