use anyhow::{Context, Result};
use rand::RngCore;

pub fn parse_amount(input: &str) -> Result<u64> {
    let s = input.trim().to_lowercase();
    if let Some(raw) = s.strip_suffix("wei") {
        return raw.parse::<u64>().context("Invalid wei amount");
    }
    if let Some(raw) = s.strip_suffix("eth") {
        return eth_to_wei(raw);
    }
    s.parse::<u64>()
        .context("Invalid amount. Use wei integer or suffix like 0.01eth / 100wei")
}

fn eth_to_wei(raw: &str) -> Result<u64> {
    let (whole, frac) = raw.split_once('.').unwrap_or((raw, ""));
    let whole_wei =
        whole.parse::<u128>().context("Invalid ETH amount")? * 1_000_000_000_000_000_000u128;
    let mut frac_string = frac.to_string();
    anyhow::ensure!(
        frac_string.len() <= 18,
        "ETH amount has more than 18 decimals"
    );
    while frac_string.len() < 18 {
        frac_string.push('0');
    }
    let frac_wei = if frac_string.is_empty() {
        0
    } else {
        frac_string
            .parse::<u128>()
            .context("Invalid ETH decimal amount")?
    };
    let total = whole_wei + frac_wei;
    anyhow::ensure!(
        total <= u64::MAX as u128,
        "Amount exceeds u64 limit used by current ZK circuit"
    );
    Ok(total as u64)
}

pub fn parse_hex32(input: &str) -> Result<[u8; 32]> {
    let bytes = parse_hex(input)?;
    anyhow::ensure!(
        bytes.len() == 32,
        "Expected 32-byte hex, got {} bytes",
        bytes.len()
    );
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

pub fn parse_address_bytes(input: &str) -> Result<[u8; 20]> {
    let bytes = parse_hex(input)?;
    anyhow::ensure!(bytes.len() == 20, "Expected Ethereum address with 20 bytes");
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes);
    Ok(out)
}

pub fn validate_address(input: &str) -> Result<()> {
    parse_address_bytes(input).map(|_| ())
}

pub fn random_secret() -> [u8; 32] {
    let mut out = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut out);
    out
}

pub fn hex(bytes: &[u8]) -> String {
    format!("0x{}", alloy::hex::encode(bytes))
}

fn parse_hex(input: &str) -> Result<Vec<u8>> {
    let cleaned = input.strip_prefix("0x").unwrap_or(input);
    alloy::hex::decode(cleaned).context("Invalid hex string")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wei_and_eth_amounts() {
        assert_eq!(parse_amount("100wei").unwrap(), 100);
        assert_eq!(parse_amount("100").unwrap(), 100);
        assert_eq!(parse_amount("0.01eth").unwrap(), 10_000_000_000_000_000);
    }

    #[test]
    fn rejects_invalid_amounts() {
        assert!(parse_amount("nope").is_err());
        assert!(parse_amount("0.0000000000000000001eth").is_err());
    }

    #[test]
    fn validates_address_and_secret_lengths() {
        assert!(validate_address("0x1111111111111111111111111111111111111111").is_ok());
        assert!(validate_address("0x123").is_err());
        assert!(
            parse_hex32("0x1111111111111111111111111111111111111111111111111111111111111111")
                .is_ok()
        );
        assert!(parse_hex32("0x123").is_err());
    }
}
