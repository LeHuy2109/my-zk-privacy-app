use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "zkprivacy",
    about = "Professional CLI for the ZK Privacy App",
    version
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        default_value_t = false,
        help = "Print machine-readable JSON output"
    )]
    pub json: bool,
    #[arg(
        long,
        global = true,
        default_value_t = false,
        help = "Print detailed progress and diagnostics"
    )]
    pub verbose: bool,
    #[arg(
        long,
        global = true,
        default_value_t = false,
        help = "Validate and preview without sending transactions or writing sensitive outputs"
    )]
    pub dry_run: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(about = "Initialize local .env and note store")]
    Init,
    #[command(subcommand, about = "Show or update CLI configuration")]
    Config(ConfigCommand),
    #[command(about = "Deposit ETH and save a local note")]
    Deposit(DepositArgs),
    #[command(subcommand, about = "Manage local notes")]
    Notes(NotesCommand),
    #[command(about = "Generate a ZK withdraw proof")]
    Prove(ProveArgs),
    #[command(about = "Submit an existing proof or prove+withdraw from a note")]
    Withdraw(WithdrawArgs),
    #[command(about = "Show CLI, config, note, and chain status")]
    Status,
    #[command(about = "Show configured wallet balance")]
    Balance,
    #[command(subcommand, about = "Check nullifier status")]
    Nullifier(NullifierCommand),
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    Show,
    Set(ConfigSetArgs),
}

#[derive(Args, Debug)]
pub struct ConfigSetArgs {
    #[arg(long)]
    pub rpc_url: Option<String>,
    #[arg(long)]
    pub private_key: Option<String>,
    #[arg(long)]
    pub contract: Option<String>,
    #[arg(long)]
    pub deploy_block: Option<u64>,
}

#[derive(Args, Debug)]
pub struct DepositArgs {
    #[arg(long)]
    pub amount: String,
    #[arg(long)]
    pub secret: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum NotesCommand {
    List,
    Show(NoteShowArgs),
    Export(NoteExportArgs),
    Import(NoteImportArgs),
}

#[derive(Args, Debug)]
pub struct NoteShowArgs {
    pub note_id: String,
    #[arg(long, default_value_t = false)]
    pub show_secret: bool,
}

#[derive(Args, Debug)]
pub struct NoteExportArgs {
    pub note_id: String,
    #[arg(long)]
    pub output: PathBuf,
}

#[derive(Args, Debug)]
pub struct NoteImportArgs {
    pub input: PathBuf,
}

#[derive(Args, Debug)]
pub struct ProveArgs {
    #[arg(long, conflicts_with = "amount")]
    pub note: Option<String>,
    #[arg(long, requires = "secret")]
    pub amount: Option<String>,
    #[arg(long, requires = "amount")]
    pub secret: Option<String>,
    #[arg(long)]
    pub recipient: String,
    #[arg(long, default_value = "proof.json")]
    pub output: PathBuf,
    #[arg(long, default_value_t = false)]
    pub groth16: bool,
}

#[derive(Args, Debug)]
pub struct WithdrawArgs {
    #[arg(long, conflicts_with = "note")]
    pub proof: Option<PathBuf>,
    #[arg(long, conflicts_with = "proof")]
    pub note: Option<String>,
    #[arg(long, requires = "note")]
    pub recipient: Option<String>,
    #[arg(long, default_value_t = false)]
    pub groth16: bool,
    #[arg(long, default_value = "proof.json")]
    pub output: PathBuf,
}

#[derive(Subcommand, Debug)]
pub enum NullifierCommand {
    Check { nullifier: String },
}
