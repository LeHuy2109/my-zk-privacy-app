use anyhow::{Context, Result};
use rand::RngCore;
use risc0_zkvm::ExecutorEnv;
use sha2::{Digest, Sha256};

use crate::types::{ChainConfig, TransactionInput};
use crate::chain;

/// Độ sâu cây Merkle – phải khớp với guest và smart contract
/// TODO(on-chain): đổi thành depth của smart contract (thường là 20).
/// Phải khớp với TREE_DEPTH trong guest/main.rs và hằng số trong Solidity contract.
pub const TREE_DEPTH: usize = 20; // TODO(on-chain): đổi thành 20 để khớp contract thật

// ─── Hash helpers (giống hệt guest) ──────────────────────────

fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    let r = h.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&r);
    out
}

fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(left);
    h.update(right);
    let r = h.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&r);
    out
}

// ─── Merkle tree (offline demo) ──────────────────────────────

/// Leaf = SHA256(secret ∥ amount_le_bytes)  — giống hệt guest
pub fn compute_leaf(secret: &[u8; 32], amount: u64) -> [u8; 32] {
    let mut data = [0u8; 40];
    data[..32].copy_from_slice(secret);
    data[32..].copy_from_slice(&amount.to_le_bytes());
    sha256_bytes(&data)
}

/// ZERO hashes: hash của tầng rỗng tại mỗi level
///   zero[0] = [0u8;32]  (leaf trống)
///   zero[i] = SHA256(zero[i-1] ∥ zero[i-1])
fn zero_hashes() -> Vec<[u8; 32]> {
    let mut z: Vec<[u8; 32]> = vec![[0u8; 32]];
    for i in 0..TREE_DEPTH {
        let p = z[i];
        z.push(hash_pair(&p, &p));
    }
    z // z[0..=TREE_DEPTH]
}

/// Tính Merkle path cho một leaf tại vị trí cụ thể trong tree
/// commitments: danh sách tất cả commitments từ on-chain
/// leaf_index: vị trí của leaf cần tính path
fn compute_merkle_path_for_index(
    leaf_index: usize,
    commitments: &[[u8; 32]],
) -> (Vec<[u8; 32]>, Vec<bool>, [u8; 32]) {
    let zeros = zero_hashes();

    // Pad commitments to tree size
    let mut tree_level: Vec<[u8; 32]> = commitments.to_vec();

    // Pad with zero hashes to reach tree depth
    while tree_level.len() < (1 << TREE_DEPTH) {
        tree_level.push(zeros[0]);
    }

    let mut path: Vec<[u8; 32]> = Vec::new();
    let mut indices: Vec<bool> = Vec::new();

    let mut current_index = leaf_index;

    // Walk up the tree, collecting siblings
    for level in 0..TREE_DEPTH {
        let _level_size = 1 << (TREE_DEPTH - level);
        let is_right = current_index % 2 == 1;
        let sibling_index = if is_right {
            current_index - 1
        } else {
            current_index + 1
        };

        // Handle case where sibling doesn't exist
        let sibling = if sibling_index < tree_level.len() {
            tree_level[sibling_index]
        } else {
            zeros[level]
        };

        path.push(sibling);
        indices.push(is_right);

        // Move to parent level
        let mut next_level = Vec::new();
        for i in (0..tree_level.len()).step_by(2) {
            let left = tree_level[i];
            let right = if i + 1 < tree_level.len() {
                tree_level[i + 1]
            } else {
                zeros[level]
            };
            next_level.push(hash_pair(&left, &right));
        }

        tree_level = next_level;
        current_index = current_index / 2;
    }

    // Tính root từ tree cuối cùng
    let root = if !tree_level.is_empty() {
        tree_level[0]
    } else {
        zeros[TREE_DEPTH]
    };

    (path, indices, root)
}

/// Build Merkle path từ danh sách commitments on-chain
/// 
/// secret, amount: dùng tính leaf commit
/// commitments: danh sách commitments từ on-chain (từ query events)
pub fn build_merkle_for_note_on_chain(
    secret: &[u8; 32],
    amount: u64,
    commitments: &[[u8; 32]],
) -> Result<(Vec<[u8; 32]>, Vec<bool>, [u8; 32])> {
    let leaf = compute_leaf(secret, amount);

    // Tìm index của leaf trong commitments
    let leaf_index = commitments
        .iter()
        .position(|&c| c == leaf)
        .context(
            "Secret + Amount không tồn tại trên contract. \
             Bạn cần deposit với amount này trước.",
        )?;

    Ok(compute_merkle_path_for_index(leaf_index, commitments))
}

/// Build Merkle path cho một note – DEMO OFFLINE.
///
/// DEPRECATED: Dùng build_merkle_for_note_on_chain() thay thế khi có on-chain commitments
/// HIỆN TẠI dùng demo version này nếu không có kết nối on-chain
pub fn build_merkle_for_note(
    secret: &[u8; 32],
    amount: u64,
) -> (Vec<[u8; 32]>, Vec<bool>, [u8; 32]) {
    let zeros = zero_hashes();
    let leaf = compute_leaf(secret, amount);

    // DEMO ONLY: note ở index 0, tất cả siblings là zero hashes
    let path: Vec<[u8; 32]> = zeros[..TREE_DEPTH].to_vec();
    let indices: Vec<bool> = vec![false; TREE_DEPTH];

    // Tính root
    let mut node = leaf;
    for (sibling, &is_right) in path.iter().zip(indices.iter()) {
        node = if is_right {
            hash_pair(sibling, &node)
        } else {
            hash_pair(&node, sibling)
        };
    }

    (path, indices, node)
}

// ─── Build TransactionInput ───────────────────────────────────

/// Demo input: secret random, amount tuỳ chỉnh, recipient mặc định.
///
/// TODO(on-chain): Trong thực tế, secret phải được user TỰ TẠO và LƯU AN TOÀN
/// trước khi gọi contract.deposit(commitment). Secret không được lưu bất kỳ đâu
/// trên server hay truyền qua network – chỉ user giữ.
pub fn build_demo_input(amount: u64) -> TransactionInput {
    let secret = random_bytes32();
    let recipient = [
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00,
        0x11, 0x22, 0xAA, 0xBB,
    ];
    let (merkle_path, merkle_indices, merkle_root) =
        build_merkle_for_note(&secret, amount);

    TransactionInput {
        secret,
        amount,
        merkle_path,
        merkle_indices,
        merkle_root,
        recipient,
    }
}

/// Custom input: chỉ định recipient hex + amount.
/// DEMO offline – không kết nối on-chain
pub fn build_custom_input(recipient_hex: &str, amount: u64, secret_hex: Option<&str>) -> Result<TransactionInput> {
    let secret = match secret_hex {
        Some(hex_str) => {
            let cleaned = hex_str.strip_prefix("0x").unwrap_or(hex_str);
            let bytes = hex::decode(cleaned).context("Secret hex decode thất bại")?;
            if bytes.len() != 32 {
                anyhow::bail!("Secret phải đúng 32 bytes, nhận được {}", bytes.len());
            }
            let mut s = [0u8; 32];
            s.copy_from_slice(&bytes);
            s
        }
        None => random_bytes32(),
    };

    let recipient = parse_address(recipient_hex)
        .context("Địa chỉ recipient không hợp lệ (phải là hex 20-byte)")?;
    let (merkle_path, merkle_indices, merkle_root) =
        build_merkle_for_note(&secret, amount);

    Ok(TransactionInput {
        secret,
        amount,
        merkle_path,
        merkle_indices,
        merkle_root,
        recipient,
    })
}

/// Build TransactionInput từ on-chain data
/// 
/// Bắt buộc:
/// - secret: hex string (32 bytes) – secret mà user đã dùng để deposit
/// - amount: số tiền
/// - recipient_hex: người nhận
/// - config: chain config để query commitments
/// 
/// Flow:
/// 1. Query tất cả Deposit events từ contract
/// 2. Rebuild Merkle tree từ commitments
/// 3. Tìm leaf (SHA256(secret + amount)) trong commitments
/// 4. Tính merkle_path + merkle_indices
pub async fn build_custom_input_on_chain(
    secret_hex: &str,
    amount: u64,
    recipient_hex: &str,
    config: &ChainConfig,
) -> Result<TransactionInput> {
    // Parse secret
    let secret_cleaned = secret_hex.strip_prefix("0x").unwrap_or(secret_hex);
    let secret_bytes = hex::decode(secret_cleaned).context("Secret hex decode thất bại")?;
    if secret_bytes.len() != 32 {
        anyhow::bail!("Secret phải đúng 32 bytes, nhận được {}", secret_bytes.len());
    }
    let mut secret = [0u8; 32];
    secret.copy_from_slice(&secret_bytes);

    // Parse recipient
    let recipient = parse_address(recipient_hex)
        .context("Địa chỉ recipient không hợp lệ (phải là hex 20-byte)")?;

    // Query commitments từ on-chain
    let commitments = chain::query_deposit_events(config).await?;
    
    if commitments.is_empty() {
        anyhow::bail!(
            "Không có deposits trên contract. Hãy deposit trước ({:?}",
            config.contract_address
        );
    }

    // Build merkle path từ commitments on-chain
    let (merkle_path, merkle_indices, merkle_root) =
        build_merkle_for_note_on_chain(&secret, amount, &commitments)?;

    Ok(TransactionInput {
        secret,
        amount,
        merkle_path,
        merkle_indices,
        merkle_root,
        recipient,
    })
}

// ─── ExecutorEnv ──────────────────────────────────────────────

pub fn create_executor_env(input: &TransactionInput) -> Result<ExecutorEnv<'static>> {
    let env = ExecutorEnv::builder()
        .write(input)
        .context("Ghi TransactionInput vào env thất bại")?
        .build()
        .context("Tạo ExecutorEnv thất bại")?;
    Ok(env)
}

// ─── Helpers ──────────────────────────────────────────────────

fn random_bytes32() -> [u8; 32] {
    let mut out = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut out);
    out
}

fn parse_address(hex_str: &str) -> Result<[u8; 20]> {
    let cleaned = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let bytes = hex::decode(cleaned).context("Hex decode thất bại")?;
    if bytes.len() != 20 {
        anyhow::bail!("Địa chỉ phải đúng 20 bytes, nhận được {}", bytes.len());
    }
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&bytes);
    Ok(addr)
}
