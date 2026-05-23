#![no_main]

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
    pub chain_id: u64,
    pub contract_address: [u8; 20],
    pub nonce: [u8; 32],
    pub action_type: u32,
}

sol! {
    struct Journal {
        bytes32 payload_hash;
        bytes32 identity_commitment;
        bytes32 nullifier_hash;
        address recipient;
        uint64 chain_id;
        address contract_address;
        uint32 action_type;
        bytes32 intent_hash;
        bool is_valid;
    }
}

fn sha256_chunks(chunks: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

fn is_nonzero(bytes: &[u8]) -> bool {
    bytes.iter().any(|&b| b != 0)
}

fn main() {
    let input: ZkAuthInput = env::read();
    let chain_id_bytes = input.chain_id.to_be_bytes();
    let action_type_bytes = input.action_type.to_be_bytes();

    let identity_commitment = sha256_chunks(&[b"IDENTITY", &input.secret]);
    let nullifier_hash = sha256_chunks(&[
        b"NULLIFIER",
        &input.secret,
        &input.nonce,
        &input.contract_address,
        &chain_id_bytes,
    ]);
    let intent_hash = sha256_chunks(&[
        b"ZK_AUTH_RECORD",
        &input.payload_hash,
        &input.recipient,
        &chain_id_bytes,
        &input.contract_address,
        &action_type_bytes,
    ]);

    let is_valid = is_nonzero(&input.secret)
        && is_nonzero(&input.payload_hash)
        && is_nonzero(&input.recipient)
        && input.chain_id > 0
        && is_nonzero(&input.contract_address);

    let journal = Journal {
        payload_hash: input.payload_hash.into(),
        identity_commitment: identity_commitment.into(),
        nullifier_hash: nullifier_hash.into(),
        recipient: input.recipient.into(),
        chain_id: input.chain_id,
        contract_address: input.contract_address.into(),
        action_type: input.action_type,
        intent_hash: intent_hash.into(),
        is_valid,
    };

    let encoded = Journal::abi_encode(&journal);
    env::commit_slice(&encoded);
}
