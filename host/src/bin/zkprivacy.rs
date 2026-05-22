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
#[path = "../zkprivacy_cli.rs"]
mod zkprivacy_cli;
#[path = "../zkprivacy_commands.rs"]
mod zkprivacy_commands;
#[path = "../zkprivacy_config.rs"]
mod zkprivacy_config;
#[path = "../zkprivacy_notes.rs"]
mod zkprivacy_notes;
#[path = "../zkprivacy_utils.rs"]
mod zkprivacy_utils;

use anyhow::Result;
use clap::Parser;
use zkprivacy_cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    zkprivacy_commands::run(Cli::parse()).await
}
