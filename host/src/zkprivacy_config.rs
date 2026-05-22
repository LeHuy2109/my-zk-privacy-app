use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

pub const ENV_PATH: &str = ".env";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub rpc_url: Option<String>,
    pub private_key: Option<String>,
    pub contract_address: Option<String>,
    pub deploy_block: Option<u64>,
    pub network: String,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let _ = dotenv::dotenv();
        Ok(Self {
            rpc_url: std::env::var("SEPOLIA_RPC_URL").ok(),
            private_key: std::env::var("PRIVATE_KEY").ok(),
            contract_address: std::env::var("CONTRACT_ADDRESS").ok(),
            deploy_block: std::env::var("DEPLOY_BLOCK")
                .ok()
                .and_then(|v| v.parse().ok()),
            network: "sepolia".to_string(),
        })
    }

    pub fn is_chain_ready(&self) -> bool {
        self.rpc_url.as_ref().is_some_and(|v| !v.is_empty())
            && self.private_key.as_ref().is_some_and(|v| !v.is_empty())
            && self
                .contract_address
                .as_ref()
                .is_some_and(|v| !v.is_empty())
    }

    pub fn require_chain(&self) -> Result<crate::types::ChainConfig> {
        let rpc_url = self
            .rpc_url
            .clone()
            .context("Missing SEPOLIA_RPC_URL. Run `zkprivacy config set --rpc-url <url>`.")?;
        let private_key = self
            .private_key
            .clone()
            .context("Missing PRIVATE_KEY. Run `zkprivacy config set --private-key <key>`.")?;
        let contract_address = self.contract_address.clone().context(
            "Missing CONTRACT_ADDRESS. Run `zkprivacy config set --contract <address>`.",
        )?;
        Ok(crate::types::ChainConfig {
            rpc_url,
            private_key,
            contract_address,
            deploy_block: self.deploy_block.unwrap_or(0),
        })
    }
}

pub fn init_env() -> Result<bool> {
    if Path::new(ENV_PATH).exists() {
        return Ok(false);
    }
    let template = "SEPOLIA_RPC_URL=\nPRIVATE_KEY=\nCONTRACT_ADDRESS=\nDEPLOY_BLOCK=0\n";
    fs::write(ENV_PATH, template).context("Failed to create .env")?;
    Ok(true)
}

pub fn update_env(
    rpc_url: Option<String>,
    private_key: Option<String>,
    contract: Option<String>,
    deploy_block: Option<u64>,
) -> Result<()> {
    let mut values = read_env_file()?;
    if let Some(v) = rpc_url {
        values.insert("SEPOLIA_RPC_URL".to_string(), v);
    }
    if let Some(v) = private_key {
        values.insert("PRIVATE_KEY".to_string(), v);
    }
    if let Some(v) = contract {
        values.insert("CONTRACT_ADDRESS".to_string(), v);
    }
    if let Some(v) = deploy_block {
        values.insert("DEPLOY_BLOCK".to_string(), v.to_string());
    }
    write_env_file(&values)
}

fn read_env_file() -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    if !Path::new(ENV_PATH).exists() {
        return Ok(values);
    }
    for raw in fs::read_to_string(ENV_PATH)?.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || !line.contains('=') {
            continue;
        }
        let (key, value) = line.split_once('=').unwrap();
        values.insert(
            key.trim().to_string(),
            value.trim().trim_matches('"').to_string(),
        );
    }
    Ok(values)
}

fn write_env_file(values: &BTreeMap<String, String>) -> Result<()> {
    let mut out = String::new();
    for (key, value) in values {
        out.push_str(key);
        out.push('=');
        out.push_str(value);
        out.push('\n');
    }
    fs::write(ENV_PATH, out).context("Failed to write .env")
}

pub fn masked(value: &Option<String>) -> String {
    match value {
        Some(v) if !v.is_empty() => "<set>".to_string(),
        _ => "<missing>".to_string(),
    }
}
