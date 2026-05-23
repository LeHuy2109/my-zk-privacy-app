use anyhow::Result;
use serde_json::Value;

use host::zk_auth;

fn main() -> Result<()> {
    let traditional = read_latest("traditional_")?;
    let zk = read_latest("zk_auth_")?;
    let availability = read_latest("availability_")?;
    let integrity = read_latest("integrity_")?;

    println!(
        "{:<32} {:>14} {:>14} {:>14} {:>14} {:>14} {:>14}",
        "metric", "traditional", "zk-auth", "zk+artifact", "availability", "tamper", "replay"
    );
    println!("{}", "-".repeat(116));

    row(
        "gas_used",
        get(&traditional, "gas_used"),
        get(&zk, "gas_used"),
        get(&zk, "gas_used"),
        "-",
        "-",
        "-",
    );
    row(
        "proof_generation_seconds",
        "-",
        get(&zk, "proof_generation_seconds"),
        get(&zk, "proof_generation_seconds"),
        "-",
        "-",
        "-",
    );
    row(
        "proof_verify_seconds",
        "-",
        get(&zk, "proof_verify_seconds"),
        get(&zk, "proof_verify_seconds"),
        "-",
        "-",
        "-",
    );
    row(
        "seal_size_bytes",
        "-",
        get(&zk, "seal_size_bytes"),
        get(&zk, "seal_size_bytes"),
        "-",
        "-",
        "-",
    );
    row(
        "journal_size_bytes",
        "-",
        get(&zk, "journal_size_bytes"),
        get(&zk, "journal_size_bytes"),
        "-",
        "-",
        "-",
    );
    row(
        "raw_tx_size_bytes",
        get(&traditional, "raw_tx_size_bytes"),
        get(&zk, "raw_tx_size_bytes"),
        get(&zk, "raw_tx_size_bytes"),
        "-",
        "-",
        "-",
    );
    row(
        "calldata_size_bytes",
        get(&traditional, "calldata_size_bytes"),
        get(&zk, "calldata_size_bytes"),
        get(&zk, "calldata_size_bytes"),
        "-",
        "-",
        "-",
    );
    row(
        "send_and_confirm_seconds",
        get(&traditional, "send_and_confirm_seconds"),
        get(&zk, "send_and_confirm_seconds"),
        get(&zk, "send_and_confirm_seconds"),
        "-",
        "-",
        "-",
    );
    row(
        "total_latency_seconds",
        get(&traditional, "total_latency_seconds"),
        get(&zk, "total_latency_seconds"),
        get(&zk, "total_latency_seconds"),
        "-",
        "-",
        "-",
    );

    let (traditional_rate, zk_rate) = availability_rates(&availability);
    row(
        "success_rate_percent",
        traditional_rate,
        zk_rate.clone(),
        zk_rate.clone(),
        "-",
        "-",
        "-",
    );
    row(
        "tamper_detection_rate",
        "-",
        "-",
        "-",
        "-",
        detection_rate(&integrity, "tampered"),
        "-",
    );
    row(
        "replay_rejection_rate",
        "-",
        "-",
        "-",
        "-",
        "-",
        detection_rate(&integrity, "reused_nullifier"),
    );

    if let Some(path) = zk_auth::latest_file_with_prefix("traditional_")? {
        println!("\ntraditional: {}", path.display());
    }
    if let Some(path) = zk_auth::latest_file_with_prefix("zk_auth_")? {
        println!("zk-auth:     {}", path.display());
    }
    if let Some(path) = zk_auth::latest_file_with_prefix("integrity_")? {
        println!("integrity:   {}", path.display());
    }
    if let Some(path) = zk_auth::latest_file_with_prefix("availability_")? {
        println!("availability: {}", path.display());
    }

    Ok(())
}

fn read_latest(prefix: &str) -> Result<Option<Value>> {
    match zk_auth::latest_file_with_prefix(prefix)? {
        Some(path) => Ok(Some(serde_json::from_str(&std::fs::read_to_string(path)?)?)),
        None => Ok(None),
    }
}

fn get(value: &Option<Value>, key: &str) -> String {
    value
        .as_ref()
        .and_then(|v| v.get(key))
        .map(format_value)
        .unwrap_or_else(|| "-".to_string())
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "-".to_string(),
        Value::Number(number) => {
            if let Some(f) = number.as_f64() {
                if f.fract() == 0.0 {
                    format!("{f:.0}")
                } else {
                    format!("{f:.4}")
                }
            } else {
                number.to_string()
            }
        }
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        _ => "-".to_string(),
    }
}

fn availability_rates(value: &Option<Value>) -> (String, String) {
    let Some(results) = value
        .as_ref()
        .and_then(|v| v.get("results"))
        .and_then(Value::as_array)
    else {
        return ("-".to_string(), "-".to_string());
    };

    let mut traditional = "-".to_string();
    let mut zk = "-".to_string();
    for result in results {
        let mode = result
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let rate = get(&Some(result.clone()), "success_rate_percent");
        if mode.contains("traditional") {
            traditional = rate;
        } else if mode.contains("zk-auth") {
            zk = rate;
        }
    }
    (traditional, zk)
}

fn detection_rate(value: &Option<Value>, needle: &str) -> String {
    let Some(cases) = value
        .as_ref()
        .and_then(|v| v.get("cases"))
        .and_then(Value::as_array)
    else {
        return "-".to_string();
    };

    let selected = cases
        .iter()
        .filter(|case| {
            case.get("case_name")
                .and_then(Value::as_str)
                .map(|name| name.contains(needle))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return "-".to_string();
    }
    let passed = selected
        .iter()
        .filter(|case| case.get("passed").and_then(Value::as_bool).unwrap_or(false))
        .count();
    format!("{:.2}", passed as f64 * 100.0 / selected.len() as f64)
}

fn row(
    metric: &str,
    traditional: impl ToString,
    zk: impl ToString,
    artifact: impl ToString,
    availability: impl ToString,
    tamper: impl ToString,
    replay: impl ToString,
) {
    println!(
        "{:<32} {:>14} {:>14} {:>14} {:>14} {:>14} {:>14}",
        metric,
        traditional.to_string(),
        zk.to_string(),
        artifact.to_string(),
        availability.to_string(),
        tamper.to_string(),
        replay.to_string()
    );
}
