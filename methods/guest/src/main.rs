// ============================================================
// RISC0 Guest Program: ZK Privacy Transaction v2
// Merkle Inclusion Proof + Nullifier
//
// Private input (chỉ guest biết, KHÔNG bao giờ lộ ra journal):
//   - secret:           32-byte bí mật của note
//   - amount:           số tiền của note
//   - merkle_path:      sibling hashes từ leaf → root
//   - merkle_indices:   hướng tại mỗi tầng (false=trái, true=phải)
//   - merkle_root:      root hiện tại lấy từ smart contract
//   - recipient:        địa chỉ người nhận
//
// Public output (commit ra journal – smart contract đọc):
//   - merkle_root:      phải khớp on-chain root
//   - nullifier_hash:   SHA256(secret ∥ "nullify") – chống double-spend
//   - recipient:        người nhận tiền
//   - amount:           số tiền
//   - is_valid:         proof hợp lệ không
// ============================================================

#![no_main]

use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

risc0_zkvm::guest::entry!(main);

/// Độ sâu cây Merkle – phải khớp với smart contract khi deploy
///
/// TODO(on-chain): Đổi thành depth mà contract dùng (thường là 20, tức 2^20 ≈ 1M slots).
/// Thay đổi giá trị này sẽ làm IMAGE_ID của guest thay đổi hoàn toàn.
/// Contract phải được deploy với IMAGE_ID mới tương ứng.
const TREE_DEPTH: usize = 20; // TODO(on-chain): đổi thành 20 để khớp contract thật

// ─── Private Input ────────────────────────────────────────────
#[derive(Serialize, Deserialize)]
pub struct TransactionInput {
    /// Bí mật 32-byte của note – chỉ owner giữ, không bao giờ lộ
    pub secret: [u8; 32],
    /// Số tiền của note
    pub amount: u64,
    /// Sibling hashes tại mỗi tầng (từ leaf lên root)
    pub merkle_path: Vec<[u8; 32]>,
    /// Hướng tại mỗi tầng:
    ///   false = current node là con TRÁI  → hash(current, sibling)
    ///   true  = current node là con PHẢI  → hash(sibling, current)
    pub merkle_indices: Vec<bool>,
    /// Merkle root hiện tại từ smart contract (on-chain)
    pub merkle_root: [u8; 32],
    /// Địa chỉ người nhận
    pub recipient: [u8; 20],
}

// ─── Public Output (commit ra journal) ───────────────────────
#[derive(Serialize, Deserialize)]
pub struct TransactionOutput {
    /// Root phải khớp on-chain – contract kiểm tra điều này
    pub merkle_root: [u8; 32],
    /// Nullifier = SHA256(secret ∥ "nullify") – contract lưu lại chống reuse
    pub nullifier_hash: [u8; 32],
    /// Địa chỉ người nhận tiền
    pub recipient: [u8; 20],
    /// Số tiền withdraw
    pub amount: u64,
    /// true nếu Merkle path hợp lệ và amount > 0
    pub is_valid: bool,
}

// ─── Hash primitives ──────────────────────────────────────────

fn sha256(data: &[u8]) -> [u8; 32] {
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

// ─── Merkle primitives ────────────────────────────────────────

/// Leaf = SHA256(secret ∥ amount_le_bytes)
/// Công thức này phải giống hệt trong smart contract Solidity
fn compute_leaf(secret: &[u8; 32], amount: u64) -> [u8; 32] {
    let mut data = [0u8; 40];
    data[..32].copy_from_slice(secret);
    data[32..].copy_from_slice(&amount.to_le_bytes());
    sha256(&data)
}

/// Nullifier = SHA256(secret ∥ "nullify")
/// Commit công khai để contract ghi nhận note đã được chi tiêu
fn compute_nullifier(secret: &[u8; 32]) -> [u8; 32] {
    let mut data = [0u8; 39]; // 32 + 7
    data[..32].copy_from_slice(secret);
    data[32..].copy_from_slice(b"nullify");
    sha256(&data)
}

/// Tính root từ leaf + Merkle path
/// Đây là trái tim của ZK: chứng minh inclusion mà không lộ leaf hay path
fn compute_root_from_path(
    leaf: &[u8; 32],
    path: &[[u8; 32]],
    indices: &[bool],
) -> [u8; 32] {
    let mut node = *leaf;
    for (sibling, &is_right) in path.iter().zip(indices.iter()) {
        node = if is_right {
            hash_pair(sibling, &node) // node ở phải → sibling ở trái
        } else {
            hash_pair(&node, sibling) // node ở trái → sibling ở phải
        };
    }
    node
}

use alloy_primitives::U256;
use alloy_sol_types::{sol, SolType};

sol! {
    struct Journal {
        bytes32 merkle_root;
        bytes32 nullifier_hash;
        address recipient;
        uint256 amount;
        bool is_valid;
    }
}

// ─── Entry point ──────────────────────────────────────────────

fn main() {
    // 1. Đọc private input – không bao giờ xuất hiện trong journal
    let tx: TransactionInput = env::read();

    // 2. Validate: path phải đúng độ sâu TREE_DEPTH
    let valid_depth =
        tx.merkle_path.len() == TREE_DEPTH && tx.merkle_indices.len() == TREE_DEPTH;

    // 3. Tính leaf commitment từ secret + amount (private)
    let leaf = compute_leaf(&tx.secret, tx.amount);

    // 4. Tính lại root từ leaf + path
    //    Nếu note thật sự tồn tại trong cây → computed_root == merkle_root
    let computed_root =
        compute_root_from_path(&leaf, &tx.merkle_path, &tx.merkle_indices);

    let root_matches = computed_root == tx.merkle_root;

    // 5. Tính nullifier – contract lưu lại sau khi withdraw để chống double-spend
    let nullifier_hash = compute_nullifier(&tx.secret);

    // 6. Giao dịch hợp lệ khi:
    //    ✓ Path đúng độ sâu
    //    ✓ Root khớp (note tồn tại trong cây on-chain)
    //    ✓ Amount > 0
    let is_valid = valid_depth && root_matches && tx.amount > 0;

    // 7. Commit public output ra journal theo chuẩn ABI encode
    //    Hợp đồng tĩnh trên Solidity sẽ đọc trực tiếp bằng abi.decode.
    let journal = Journal {
        merkle_root: tx.merkle_root.into(),
        nullifier_hash: nullifier_hash.into(),
        recipient: tx.recipient.into(),
        amount: U256::from(tx.amount),
        is_valid,
    };

    let encoded = Journal::abi_encode(&journal);
    env::commit_slice(&encoded);
}
