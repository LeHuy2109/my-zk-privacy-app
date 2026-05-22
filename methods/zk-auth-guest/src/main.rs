#![no_main]

use alloy_primitives::U256;
use alloy_sol_types::{sol, SolType};
use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

risc0_zkvm::guest::entry!(main);

#[derive(Serialize, Deserialize)]
pub struct ZkAuthInput {
    pub secret: [u8; 32],
    pub payload_hash: [u8; 32],
    pub recipient: [u8; 20],
    pub action_type: u64,
    pub chain_id: u64,
    pub contract_address: [u8; 20],
    pub nonce: u64,
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    let r = h.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&r);
    out
}

fn identity_commitment(secret: &[u8; 32]) -> [u8; 32] {
    let mut data = Vec::with_capacity(secret.len() + b"identity".len());
    data.extend_from_slice(secret);
    data.extend_from_slice(b"identity");
    sha256(&data)
}

fn nullifier_hash(secret: &[u8; 32], payload_hash: &[u8; 32], nonce: u64) -> [u8; 32] {
    let mut data = Vec::with_capacity(secret.len() + payload_hash.len() + 8 + b"zk-auth-nullifier".len());
    data.extend_from_slice(secret);
    data.extend_from_slice(payload_hash);
    data.extend_from_slice(&nonce.to_le_bytes());
    data.extend_from_slice(b"zk-auth-nullifier");
    sha256(&data)
}

sol! {
    struct Journal {
        bytes32 payload_hash;
        bytes32 identity_commitment;
        bytes32 nullifier_hash;
        address recipient;
        uint256 action_type;
        uint256 chain_id;
        address contract_address;
        uint256 nonce;
        bool is_valid;
    }
}

fn main() {
    let input: ZkAuthInput = env::read();

    let identity_commitment = identity_commitment(&input.secret);
    let nullifier_hash = nullifier_hash(&input.secret, &input.payload_hash, input.nonce);
    let is_valid = input.payload_hash != [0u8; 32]
        && input.recipient != [0u8; 20]
        && input.chain_id > 0
        && input.contract_address != [0u8; 20];

    let journal = Journal {
        payload_hash: input.payload_hash.into(),
        identity_commitment: identity_commitment.into(),
        nullifier_hash: nullifier_hash.into(),
        recipient: input.recipient.into(),
        action_type: U256::from(input.action_type),
        chain_id: U256::from(input.chain_id),
        contract_address: input.contract_address.into(),
        nonce: U256::from(input.nonce),
        is_valid,
    };

    env::commit_slice(&Journal::abi_encode(&journal));
}
