use anyhow::{Context, Result};
use serde_json::json;
use std::{fs, path::PathBuf};

use crate::{
    chain, executor, prover, zkprivacy_chain as cli_chain,
    zkprivacy_cli::*,
    zkprivacy_config::{self, AppConfig},
    zkprivacy_notes::{self, Note, NoteStore},
    zkprivacy_utils as utils,
};

pub struct GlobalOptions {
    pub json: bool,
    pub verbose: bool,
    pub dry_run: bool,
}

pub async fn run(cli: Cli) -> Result<()> {
    let opts = GlobalOptions {
        json: cli.json,
        verbose: cli.verbose,
        dry_run: cli.dry_run,
    };
    match cli.command {
        Command::Init => init(&opts),
        Command::Config(ConfigCommand::Show) => config_show(&opts),
        Command::Config(ConfigCommand::Set(args)) => config_set(args, &opts),
        Command::Deposit(args) => deposit(args, &opts).await,
        Command::Notes(NotesCommand::List) => notes_list(&opts),
        Command::Notes(NotesCommand::Show(args)) => notes_show(args, &opts),
        Command::Notes(NotesCommand::Export(args)) => notes_export(args, &opts),
        Command::Notes(NotesCommand::Import(args)) => notes_import(args, &opts),
        Command::Prove(args) => prove(args, &opts).await,
        Command::Withdraw(args) => withdraw(args, &opts).await,
        Command::Status => status(&opts),
        Command::Balance => balance(&opts).await,
        Command::Nullifier(NullifierCommand::Check { nullifier }) => {
            nullifier_check(&nullifier, &opts).await
        }
    }
}

fn init(opts: &GlobalOptions) -> Result<()> {
    step(opts, "[1/2] Creating .env if needed");
    let created = zkprivacy_config::init_env()?;
    step(opts, "[2/2] Initializing local note store");
    NoteStore::load()?.save()?;
    if opts.json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({"env_created": created, "notes_path": zkprivacy_notes::notes_path()})
            )?
        );
    } else {
        println!("Initialized zkprivacy project.");
        println!("Notes are stored in {}. WARNING: notes contain plaintext secrets; back them up securely.", zkprivacy_notes::notes_path());
    }
    Ok(())
}

fn config_show(opts: &GlobalOptions) -> Result<()> {
    let config = AppConfig::load()?;
    if opts.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "rpc_url": config.rpc_url,
                "private_key": zkprivacy_config::masked(&config.private_key),
                "contract_address": config.contract_address,
                "deploy_block": config.deploy_block,
                "network": config.network,
                "chain_ready": config.is_chain_ready(),
            }))?
        );
    } else {
        println!("Config");
        println!(
            "  RPC URL        : {}",
            config.rpc_url.unwrap_or_else(|| "<missing>".to_string())
        );
        println!(
            "  Private key    : {}",
            zkprivacy_config::masked(&config.private_key)
        );
        println!(
            "  Contract       : {}",
            config
                .contract_address
                .unwrap_or_else(|| "<missing>".to_string())
        );
        println!(
            "  Deploy block   : {}",
            config
                .deploy_block
                .map(|v| v.to_string())
                .unwrap_or_else(|| "<missing>".to_string())
        );
        println!("  Network        : {}", config.network);
    }
    Ok(())
}

fn config_set(args: ConfigSetArgs, opts: &GlobalOptions) -> Result<()> {
    if let Some(address) = &args.contract {
        utils::validate_address(address)?;
    }
    if opts.dry_run {
        println!("Dry run: config values validated; .env not modified.");
        return Ok(());
    }
    zkprivacy_config::update_env(
        args.rpc_url,
        args.private_key,
        args.contract,
        args.deploy_block,
    )?;
    println!("Config updated. PRIVATE_KEY was not printed.");
    Ok(())
}

async fn deposit(args: DepositArgs, opts: &GlobalOptions) -> Result<()> {
    step(opts, "[1/4] Loading config");
    let config = AppConfig::load()?;
    let amount = utils::parse_amount(&args.amount)?;
    let secret = match args.secret {
        Some(secret) => utils::parse_hex32(&secret)?,
        None => utils::random_secret(),
    };

    step(opts, "[2/4] Building commitment");
    let commitment = executor::compute_leaf(&secret, amount);
    let secret_hex = utils::hex(&secret);
    let commitment_hex = utils::hex(&commitment);

    let mut tx_hash = None;
    if config.is_chain_ready() && !opts.dry_run {
        step(opts, "[3/4] Sending deposit transaction");
        let chain_config = config.require_chain()?;
        let tx = cli_chain::deposit(commitment, amount, &chain_config).await?;
        tx_hash = Some(format!("0x{}", alloy::hex::encode(tx.tx_hash)));
        println!("Deposit tx: {}", tx_hash.as_ref().unwrap());
        if let Some(gas) = tx.gas_used {
            println!("Gas used  : {}", gas);
        }
    } else {
        step(opts, "[3/4] Skipping chain transaction");
        if opts.dry_run {
            println!("Dry run: transaction not sent.");
        } else {
            println!("Missing chain config; commitment generated but transaction not sent. Run `zkprivacy config set ...`.");
        }
    }

    step(opts, "[4/4] Saving note");
    if !opts.dry_run {
        let mut store = NoteStore::load()?;
        let note = store.add(Note {
            id: String::new(),
            amount,
            secret: secret_hex,
            commitment: commitment_hex.clone(),
            tx_hash,
            timestamp: zkprivacy_notes::now(),
            network: config.network,
            spent: false,
        })?;
        print_note_created(&note, opts)?;
    } else if opts.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({"amount": amount, "commitment": commitment_hex}))?
        );
    } else {
        println!("Commitment: {}", commitment_hex);
    }
    Ok(())
}

fn notes_list(opts: &GlobalOptions) -> Result<()> {
    let store = NoteStore::load()?;
    if opts.json {
        let safe_notes: Vec<_> = store.notes.iter().map(|n| json!({
            "id": n.id, "amount": n.amount, "commitment": n.commitment, "tx_hash": n.tx_hash, "timestamp": n.timestamp, "network": n.network, "spent": n.spent
        })).collect();
        println!("{}", serde_json::to_string_pretty(&safe_notes)?);
    } else {
        println!("Notes (secret hidden)");
        println!("| ID | Amount | Network | Spent | Tx | Commitment |");
        println!("|---|---:|---|---|---|---|");
        for n in store.notes {
            println!(
                "| {} | {} | {} | {} | {} | {} |",
                n.id,
                n.amount,
                n.network,
                n.spent,
                n.tx_hash.unwrap_or_else(|| "-".to_string()),
                n.commitment
            );
        }
    }
    Ok(())
}

fn notes_show(args: NoteShowArgs, opts: &GlobalOptions) -> Result<()> {
    let note = NoteStore::load()?.get(&args.note_id)?;
    if opts.json {
        let mut value = serde_json::to_value(&note)?;
        if !args.show_secret {
            value["secret"] = json!("<hidden>");
        }
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("Note {}", note.id);
        println!("  Amount     : {}", note.amount);
        println!("  Commitment : {}", note.commitment);
        println!(
            "  Tx hash    : {}",
            note.tx_hash.unwrap_or_else(|| "-".to_string())
        );
        println!("  Network    : {}", note.network);
        println!("  Spent      : {}", note.spent);
        println!(
            "  Secret     : {}",
            if args.show_secret {
                note.secret
            } else {
                "<hidden; use --show-secret>".to_string()
            }
        );
    }
    Ok(())
}

fn notes_export(args: NoteExportArgs, _opts: &GlobalOptions) -> Result<()> {
    let note = NoteStore::load()?.get(&args.note_id)?;
    fs::write(&args.output, serde_json::to_string_pretty(&note)?)?;
    println!(
        "Note exported to {}. WARNING: exported file contains plaintext secret.",
        args.output.display()
    );
    Ok(())
}

fn notes_import(args: NoteImportArgs, _opts: &GlobalOptions) -> Result<()> {
    let json = fs::read_to_string(&args.input)
        .with_context(|| format!("Proof/note file not found: {}", args.input.display()))?;
    let note: Note = serde_json::from_str(&json).context("Invalid note JSON")?;
    let mut store = NoteStore::load()?;
    store.import(note)?;
    println!("Note imported. Keep local notes backed up securely.");
    Ok(())
}

async fn prove(args: ProveArgs, opts: &GlobalOptions) -> Result<()> {
    let (amount, secret, recipient) =
        proof_inputs(args.note, args.amount, args.secret, args.recipient)?;
    generate_proof(amount, &secret, &recipient, args.output, args.groth16, opts).await
}

async fn withdraw(args: WithdrawArgs, opts: &GlobalOptions) -> Result<()> {
    let proof_path = if let Some(path) = args.proof {
        path
    } else {
        let note = args
            .note
            .context("Use --proof <file> or --note <note-id>")?;
        let recipient = args
            .recipient
            .context("--recipient is required with --note")?;
        let note = NoteStore::load()?.get(&note)?;
        generate_proof(
            note.amount,
            &note.secret,
            &recipient,
            args.output.clone(),
            args.groth16,
            opts,
        )
        .await?;
        args.output
    };

    let config = AppConfig::load()?.require_chain()?;
    let json = fs::read_to_string(&proof_path)
        .with_context(|| format!("Proof file not found: {}", proof_path.display()))?;
    let result: crate::types::ProofResult =
        serde_json::from_str(&json).context("Invalid proof JSON")?;
    prover::verify_receipt(&result.receipt)?;

    step(opts, "[1/3] Checking nullifier");
    let used = cli_chain::is_nullifier_used(result.output.nullifier_hash, &config).await?;
    anyhow::ensure!(!used, "Nullifier already used; withdraw would fail.");

    if opts.dry_run {
        println!("Dry run: proof verified and nullifier is unused; transaction not sent.");
        return Ok(());
    }

    step(opts, "[2/3] Submitting proof");
    let chain_result = chain::submit_proof(&result.receipt, &result.output, &config).await?;
    step(opts, "[3/3] Confirmed");
    println!(
        "Withdraw tx: 0x{}",
        alloy::hex::encode(chain_result.tx_hash)
    );
    if let Some(gas) = chain_result.gas_used {
        println!("Gas used   : {}", gas);
    }
    Ok(())
}

fn proof_inputs(
    note_id: Option<String>,
    amount: Option<String>,
    secret: Option<String>,
    recipient: String,
) -> Result<(u64, String, String)> {
    utils::validate_address(&recipient)?;
    if let Some(note_id) = note_id {
        let note = NoteStore::load()?.get(&note_id)?;
        Ok((note.amount, note.secret, recipient))
    } else {
        let amount = utils::parse_amount(&amount.context("Use --note or --amount + --secret")?)?;
        let secret = secret.context("--secret is required with --amount")?;
        utils::parse_hex32(&secret)?;
        Ok((amount, secret, recipient))
    }
}

async fn generate_proof(
    amount: u64,
    secret: &str,
    recipient: &str,
    output: PathBuf,
    groth16: bool,
    opts: &GlobalOptions,
) -> Result<()> {
    step(opts, "[1/4] Loading config");
    let config = AppConfig::load()?.require_chain()?;
    step(opts, "[2/4] Building Merkle input from chain");
    let input = executor::build_custom_input_on_chain(secret, amount, recipient, &config).await?;
    step(opts, "[3/4] Running RISC Zero prover");
    let result = prover::prove_transaction(&input, groth16).context("Proof generation failed. If using --groth16, check Docker is running and enough RAM is available.")?;
    prover::verify_receipt(&result.receipt)?;
    step(opts, "[4/4] Writing proof JSON");
    if !opts.dry_run {
        fs::write(&output, serde_json::to_string_pretty(&result)?)?;
        println!("Proof written: {}", output.display());
    } else {
        println!("Dry run: proof generated and verified; file not written.");
    }
    Ok(())
}

fn status(opts: &GlobalOptions) -> Result<()> {
    let config = AppConfig::load()?;
    let store = NoteStore::load()?;
    if opts.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "chain_ready": config.is_chain_ready(), "notes": store.notes.len(), "notes_path": zkprivacy_notes::notes_path(), "network": config.network
            }))?
        );
    } else {
        println!("ZK Privacy status");
        println!(
            "  Chain config : {}",
            if config.is_chain_ready() {
                "ready"
            } else {
                "incomplete"
            }
        );
        println!("  Notes        : {}", store.notes.len());
        println!("  Notes path   : {}", zkprivacy_notes::notes_path());
        println!("  Network      : {}", config.network);
    }
    Ok(())
}

async fn balance(opts: &GlobalOptions) -> Result<()> {
    let config = AppConfig::load()?.require_chain()?;
    let balance = cli_chain::balance(&config).await?;
    if opts.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({"balance_wei": balance.to_string()}))?
        );
    } else {
        println!("Wallet balance: {} wei", balance);
    }
    Ok(())
}

async fn nullifier_check(nullifier: &str, opts: &GlobalOptions) -> Result<()> {
    let config = AppConfig::load()?.require_chain()?;
    let nullifier = utils::parse_hex32(nullifier)?;
    let used = cli_chain::is_nullifier_used(nullifier, &config).await?;
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&json!({"used": used}))?);
    } else {
        println!("Nullifier status: {}", if used { "used" } else { "unused" });
    }
    Ok(())
}

fn print_note_created(note: &Note, opts: &GlobalOptions) -> Result<()> {
    if opts.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": note.id, "amount": note.amount, "commitment": note.commitment, "tx_hash": note.tx_hash, "network": note.network
            }))?
        );
    } else {
        println!("Note saved: {}", note.id);
        println!("Commitment: {}", note.commitment);
        println!("WARNING: this note contains a plaintext secret in the local note store. Back it up securely.");
    }
    Ok(())
}

fn step(opts: &GlobalOptions, message: &str) {
    if !opts.json {
        println!("{}", message);
    } else if opts.verbose {
        eprintln!("{}", message);
    }
}
