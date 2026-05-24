// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";

contract ZkAuthDemo {
    struct Record {
        bytes32 payloadHash;
        bytes32 journalHash;
        bytes32 proofHash;
        bytes32 nullifierHash;
        bytes32 identityCommitment;
        address recipient;
        string mode;
        string artifactRef;
        uint256 timestamp;
        bool verified;
    }

    struct JournalOutput {
        bytes32 payloadHash;
        bytes32 identityCommitment;
        bytes32 nullifierHash;
        address recipient;
        uint64 chainId;
        address contractAddress;
        uint32 actionType;
        bytes32 intentHash;
        bool isValid;
    }

    IRiscZeroVerifier public immutable verifier;
    bytes32 public immutable imageId;

    mapping(uint256 => Record) public records;
    mapping(bytes32 => bool) public usedNullifiers;
    uint256 public recordCount;

    event RecordStored(
        uint256 indexed recordId,
        bytes32 indexed payloadHash,
        bytes32 indexed nullifierHash,
        address recipient,
        string mode,
        string artifactRef
    );

    constructor(address verifierAddress, bytes32 imageId_) {
        verifier = IRiscZeroVerifier(verifierAddress);
        imageId = imageId_;
    }

    function storeRecordTraditional(bytes32 payloadHash, string calldata mode) external {
        require(payloadHash != bytes32(0), "Payload hash required");

        uint256 recordId = ++recordCount;
        records[recordId] = Record({
            payloadHash: payloadHash,
            journalHash: bytes32(0),
            proofHash: bytes32(0),
            nullifierHash: bytes32(0),
            identityCommitment: bytes32(0),
            recipient: msg.sender,
            mode: mode,
            artifactRef: "",
            timestamp: block.timestamp,
            verified: true
        });

        emit RecordStored(recordId, payloadHash, bytes32(0), msg.sender, mode, "");
    }

    function storeRecordWithProof(
        bytes calldata journal,
        bytes calldata seal,
        bytes32 expectedPayloadHash,
        string calldata artifactRef
    ) external {
        require(expectedPayloadHash != bytes32(0), "Payload hash required");
        require(bytes(artifactRef).length > 0, "Artifact ref required");

        bytes32 journalHash = sha256(journal);
        verifier.verify(seal, imageId, journalHash);

        JournalOutput memory output = decodeJournal(journal);
        require(output.isValid, "Proof output invalid");
        require(output.payloadHash == expectedPayloadHash, "Payload hash mismatch");
        require(output.chainId == uint64(block.chainid), "Chain ID mismatch");
        require(output.contractAddress == address(this), "Contract mismatch");
        require(output.recipient != address(0), "Recipient required");
        require(!usedNullifiers[output.nullifierHash], "Nullifier already used");

        usedNullifiers[output.nullifierHash] = true;

        uint256 recordId = ++recordCount;
        records[recordId] = Record({
            payloadHash: output.payloadHash,
            journalHash: journalHash,
            proofHash: sha256(seal),
            nullifierHash: output.nullifierHash,
            identityCommitment: output.identityCommitment,
            recipient: output.recipient,
            mode: "zk_auth",
            artifactRef: artifactRef,
            timestamp: block.timestamp,
            verified: true
        });

        emit RecordStored(recordId, output.payloadHash, output.nullifierHash, output.recipient, "zk_auth", artifactRef);
    }

    function getRecord(uint256 recordId) external view returns (Record memory) {
        return records[recordId];
    }

    function decodeJournal(bytes calldata journal) internal pure returns (JournalOutput memory) {
        (
            bytes32 payloadHash,
            bytes32 identityCommitment,
            bytes32 nullifierHash,
            address recipient,
            uint64 chainId,
            address contractAddress,
            uint32 actionType,
            bytes32 intentHash,
            bool isValid
        ) = abi.decode(journal, (bytes32, bytes32, bytes32, address, uint64, address, uint32, bytes32, bool));

        return JournalOutput({
            payloadHash: payloadHash,
            identityCommitment: identityCommitment,
            nullifierHash: nullifierHash,
            recipient: recipient,
            chainId: chainId,
            contractAddress: contractAddress,
            actionType: actionType,
            intentHash: intentHash,
            isValid: isValid
        });
    }
}
