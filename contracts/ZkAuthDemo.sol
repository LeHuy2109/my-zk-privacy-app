// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";

contract ZkAuthDemo {
    IRiscZeroVerifier public immutable verifier;
    bytes32 public immutable imageId;

    struct ZkAuthOutput {
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

    struct Record {
        bytes32 payloadHash;
        bytes32 identityCommitment;
        bytes32 nullifierHash;
        bytes32 journalHash;
        bytes32 proofHash;
        string proofCid;
        string algorithm;
        address recipient;
        uint256 actionType;
        uint256 timestamp;
        bool verified;
    }

    mapping(uint256 => Record) public records;
    mapping(bytes32 => bool) public usedNullifiers;
    uint256 public recordCount;

    event ZkRecordStored(
        uint256 indexed recordId,
        bytes32 indexed payloadHash,
        bytes32 indexed nullifierHash,
        address recipient,
        string algorithm
    );
    event TraditionalRecordStored(
        uint256 indexed recordId,
        bytes32 indexed payloadHash,
        address recipient,
        string algorithm
    );

    constructor(address _verifier, bytes32 _imageId) {
        verifier = IRiscZeroVerifier(_verifier);
        imageId = _imageId;
    }

    function storeRecordWithProof(
        bytes calldata journal,
        bytes calldata seal,
        bytes32 payloadHash,
        bytes32 nullifierHash,
        string calldata proofCid,
        string calldata algorithm
    ) external returns (uint256 recordId) {
        verifier.verify(seal, imageId, sha256(journal));

        ZkAuthOutput memory output = decodeJournal(journal);
        require(output.is_valid, "Proof output invalid");
        require(output.payload_hash == payloadHash, "Payload hash mismatch");
        require(output.nullifier_hash == nullifierHash, "Nullifier mismatch");
        require(output.chain_id == block.chainid, "Chain ID mismatch");
        require(output.contract_address == address(this), "Contract address mismatch");
        require(!usedNullifiers[nullifierHash], "Nullifier already used");

        usedNullifiers[nullifierHash] = true;
        recordId = recordCount;
        records[recordId] = Record({
            payloadHash: payloadHash,
            identityCommitment: output.identity_commitment,
            nullifierHash: nullifierHash,
            journalHash: sha256(journal),
            proofHash: sha256(seal),
            proofCid: proofCid,
            algorithm: algorithm,
            recipient: output.recipient,
            actionType: output.action_type,
            timestamp: block.timestamp,
            verified: true
        });
        recordCount += 1;

        emit ZkRecordStored(recordId, payloadHash, nullifierHash, output.recipient, algorithm);
    }

    function storeRecordTraditional(
        bytes32 payloadHash,
        string calldata cid,
        string calldata algorithm,
        address recipient,
        uint256 actionType
    ) external returns (uint256 recordId) {
        recordId = recordCount;
        records[recordId] = Record({
            payloadHash: payloadHash,
            identityCommitment: bytes32(0),
            nullifierHash: bytes32(0),
            journalHash: bytes32(0),
            proofHash: bytes32(0),
            proofCid: cid,
            algorithm: algorithm,
            recipient: recipient,
            actionType: actionType,
            timestamp: block.timestamp,
            verified: true
        });
        recordCount += 1;

        emit TraditionalRecordStored(recordId, payloadHash, recipient, algorithm);
    }

    function decodeJournal(bytes calldata journal) internal pure returns (ZkAuthOutput memory) {
        (
            bytes32 payload_hash,
            bytes32 identity_commitment,
            bytes32 nullifier_hash,
            address recipient,
            uint256 action_type,
            uint256 chain_id,
            address contract_address,
            uint256 nonce,
            bool is_valid
        ) = abi.decode(journal, (bytes32, bytes32, bytes32, address, uint256, uint256, address, uint256, bool));

        return ZkAuthOutput({
            payload_hash: payload_hash,
            identity_commitment: identity_commitment,
            nullifier_hash: nullifier_hash,
            recipient: recipient,
            action_type: action_type,
            chain_id: chain_id,
            contract_address: contract_address,
            nonce: nonce,
            is_valid: is_valid
        });
    }
}
