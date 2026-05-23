use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::{chain, prover, types::ProofResult};

const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayerSubmitResponse {
    pub ok: bool,
    pub tx_hash: Option<String>,
    pub block_number: Option<u64>,
    pub gas_used: Option<u128>,
    pub explorer_url: Option<String>,
    pub error: Option<String>,
}

pub async fn submit_proof_to_relayer(
    relayer_url: &str,
    proof_json: &str,
) -> Result<RelayerSubmitResponse> {
    let url = ParsedHttpUrl::parse(relayer_url)?;
    let mut stream = TcpStream::connect(format!("{}:{}", url.host, url.port))
        .await
        .with_context(|| format!("Failed to connect to relayer at {relayer_url}"))?;

    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        url.path,
        url.host,
        proof_json.len(),
        proof_json
    );
    stream
        .write_all(request.as_bytes())
        .await
        .context("Failed to send proof to relayer")?;

    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .await
        .context("Failed to read relayer response")?;

    let (_, body) = split_http_response(&raw)?;
    let response: RelayerSubmitResponse =
        serde_json::from_slice(body).context("Invalid relayer JSON response")?;
    if !response.ok {
        bail!(
            "Relayer rejected proof: {}",
            response.error.as_deref().unwrap_or("unknown error")
        );
    }
    Ok(response)
}

pub async fn serve(bind: &str) -> Result<()> {
    let listener = TcpListener::bind(bind)
        .await
        .with_context(|| format!("Failed to bind relayer server at {bind}"))?;
    println!("zkprivacy relayer listening on http://{bind}/withdraw");

    loop {
        let (stream, peer) = listener.accept().await.context("Accept failed")?;
        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream).await {
                eprintln!("Relayer request from {peer} failed: {err:#}");
            }
        });
    }
}

async fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let request = read_http_request(&mut stream).await?;
    let response = match request {
        HttpRequest { method, path, body } if method == "POST" && path == "/withdraw" => {
            submit_withdraw(&body).await
        }
        HttpRequest { method, path, .. } => Err(anyhow!("Unsupported route: {method} {path}")),
    };

    match response {
        Ok(response) => write_json(&mut stream, 200, &response).await?,
        Err(err) => {
            let response = RelayerSubmitResponse {
                ok: false,
                tx_hash: None,
                block_number: None,
                gas_used: None,
                explorer_url: None,
                error: Some(format!("{err:#}")),
            };
            write_json(&mut stream, 400, &response).await?;
        }
    }
    Ok(())
}

async fn submit_withdraw(body: &[u8]) -> Result<RelayerSubmitResponse> {
    let result: ProofResult = serde_json::from_slice(body).context("Invalid proof JSON")?;
    prover::verify_receipt(&result.receipt).context("Local receipt verification failed")?;

    let config = relayer_chain_config()?;
    let used = crate::zkprivacy_chain::is_nullifier_used(result.output.nullifier_hash, &config)
        .await
        .context("Failed to check nullifier")?;
    if used {
        bail!("Nullifier already used");
    }

    let tx = chain::submit_proof(&result.receipt, &result.output, &config)
        .await
        .context("Failed to submit withdraw transaction")?;

    Ok(RelayerSubmitResponse {
        ok: true,
        tx_hash: Some(format!("0x{}", alloy::hex::encode(tx.tx_hash))),
        block_number: tx.block_number,
        gas_used: tx.gas_used,
        explorer_url: Some(tx.explorer_url),
        error: None,
    })
}

fn relayer_chain_config() -> Result<crate::types::ChainConfig> {
    let rpc_url = env::var("SEPOLIA_RPC_URL").context("Missing SEPOLIA_RPC_URL")?;
    let private_key = env::var("RELAYER_PRIVATE_KEY")
        .or_else(|_| env::var("PRIVATE_KEY"))
        .context("Missing RELAYER_PRIVATE_KEY or PRIVATE_KEY")?;
    let contract_address = env::var("CONTRACT_ADDRESS").context("Missing CONTRACT_ADDRESS")?;
    let deploy_block = env::var("DEPLOY_BLOCK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    Ok(crate::types::ChainConfig {
        rpc_url,
        private_key,
        contract_address,
        deploy_block,
    })
}

struct HttpRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

async fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest> {
    let mut buffer = Vec::new();
    let header_end = loop {
        let mut chunk = [0u8; 4096];
        let read = stream
            .read(&mut chunk)
            .await
            .context("Failed to read request")?;
        if read == 0 {
            bail!("Connection closed before headers finished");
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > MAX_HEADER_BYTES {
            bail!("HTTP headers too large");
        }
        if let Some(pos) = find_header_end(&buffer) {
            break pos;
        }
    };

    let headers = std::str::from_utf8(&buffer[..header_end]).context("Headers are not UTF-8")?;
    let mut lines = headers.lines();
    let request_line = lines.next().context("Missing request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().context("Missing method")?.to_string();
    let path = parts.next().context("Missing path")?.to_string();

    let content_length = lines
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .context("Missing Content-Length")?;
    if content_length > MAX_BODY_BYTES {
        bail!("Request body too large");
    }

    let body_start = header_end + 4;
    let mut body = buffer[body_start..].to_vec();
    while body.len() < content_length {
        let remaining = content_length - body.len();
        let mut chunk = vec![0u8; remaining.min(8192)];
        let read = stream
            .read(&mut chunk)
            .await
            .context("Failed to read body")?;
        if read == 0 {
            bail!("Connection closed before body finished");
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);

    Ok(HttpRequest { method, path, body })
}

async fn write_json(
    stream: &mut TcpStream,
    status: u16,
    response: &RelayerSubmitResponse,
) -> Result<()> {
    let body = serde_json::to_vec(response).context("Failed to encode response")?;
    let reason = if status == 200 { "OK" } else { "Bad Request" };
    let headers = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes()).await?;
    stream.write_all(&body).await?;
    Ok(())
}

fn split_http_response(raw: &[u8]) -> Result<(&[u8], &[u8])> {
    let header_end = find_header_end(raw).context("Invalid HTTP response")?;
    Ok((&raw[..header_end], &raw[header_end + 4..]))
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

struct ParsedHttpUrl {
    host: String,
    port: u16,
    path: String,
}

impl ParsedHttpUrl {
    fn parse(raw: &str) -> Result<Self> {
        let rest = raw
            .strip_prefix("http://")
            .context("Only http:// relayer URLs are supported by the lightweight client")?;
        let (authority, path) = match rest.split_once('/') {
            Some((authority, path)) => (authority, format!("/{path}")),
            None => (rest, "/withdraw".to_string()),
        };
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => (host.to_string(), port.parse().context("Invalid port")?),
            None => (authority.to_string(), 80),
        };
        if host.is_empty() {
            bail!("Relayer URL host is empty");
        }
        Ok(Self { host, port, path })
    }
}
