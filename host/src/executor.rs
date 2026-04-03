use anyhow::{Context, Result};
use rand::RngCore;
use risc0_zkvm::ExecutorEnv;
use sha2::{Digest, Sha256};

use crate::types::TransactionInput;

/// Độ sâu cây Merkle – phải khớp với guest và smart contract
/// TODO(on-chain): đổi thành depth của smart contract (thường là 20).
/// Phải khớp với TREE_DEPTH trong guest/main.rs và hằng số trong Solidity contract.
pub const TREE_DEPTH: usize = 4; // TODO(on-chain): đổi thành 20 để khớp contract thật

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
fn compute_leaf(secret: &[u8; 32], amount: u64) -> [u8; 32] {
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

/// Build Merkle path cho một note – HIỆN TẠI LÀ DEMO OFFLINE.
///
/// TODO(on-chain): Khi tích hợp smart contract thật, hàm này cần:
///   1. Kết nối Sepolia qua RPC (alloy Provider đã có sẵn trong chain.rs)
///   2. Query tất cả `Deposit(bytes32 commitment)` events từ contract
///      để rebuild lại cây Merkle giống hệt on-chain state
///   3. Gọi `contract.currentRoot()` để lấy Merkle root chính xác
///   4. Tìm vị trí (index) của leaf = compute_leaf(secret, amount)
///      trong danh sách commitments từ events
///   5. Tính merkle_path và merkle_indices theo index đó
///
/// Gợi ý: Dùng crate `incremental-merkle-tree` hoặc tự implement
/// IncrementalMerkleTree giống contract Solidity để đảm bảo hash nhất quán.
pub fn build_merkle_for_note(
    secret: &[u8; 32],
    amount: u64,
) -> (Vec<[u8; 32]>, Vec<bool>, [u8; 32]) {
    let zeros = zero_hashes();
    let leaf = compute_leaf(secret, amount);

    // DEMO ONLY: note ở index 0, tất cả siblings là zero hashes
    // TODO(on-chain): thay bằng path thật tính từ deposit events
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
///
/// TODO(on-chain): Sau khi deposit lên contract, user cần cung cấp ĐÚNG secret
/// đã dùng khi tạo commitment (commitment = SHA256(secret ∥ amount_le_bytes)).
/// Host phải verify: compute_leaf(secret, amount) có trong cây Merkle on-chain.
pub fn build_custom_input(recipient_hex: &str, amount: u64) -> Result<TransactionInput> {
    let secret = random_bytes32();
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
