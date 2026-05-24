// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {ZkAuthImageID} from "./generated/ZkAuthImageID.sol";

contract ZkAuthDemo {
    IRiscZeroVerifier public immutable verifier;

    bytes32 public constant IMAGE_ID = ZkAuthImageID.IMAGE_ID;

    struct Record {
        bytes32 payloadHash;
        bytes32 journalHash;
        bytes32 proofHash;
        bytes32 nullifierHash;
        address recipient;
        string mode;
        string artifactRef;
        uint256 timestamp;
        bool verified;
    }

    struct ZkAuthJournal {
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

    uint256 public recordCount;
    mapping(uint256 => Record) public records;
    mapping(bytes32 => bool) public usedNullifiers;

    event RecordStored(
        uint256 indexed recordId,
        bytes32 indexed payloadHash,
        bytes32 indexed nullifierHash,
        address recipient,
        string mode,
        string artifactRef,
        bool verified
    );

    constructor(address _verifier) {
        verifier = IRiscZeroVerifier(_verifier);
    }

    function storeRecordTraditional(bytes32 payloadHash, string calldata mode) external returns (uint256 recordId) {
        require(payloadHash != bytes32(0), "Payload hash required");

        recordId = recordCount;
        records[recordId] = Record({
            payloadHash: payloadHash,
            journalHash: bytes32(0),
            proofHash: bytes32(0),
            nullifierHash: bytes32(0),
            recipient: msg.sender,
            mode: mode,
            artifactRef: "",
            timestamp: block.timestamp,
            verified: false
        });
        recordCount += 1;

        emit RecordStored(recordId, payloadHash, bytes32(0), msg.sender, mode, "", false);
    }

    function storeRecordWithProof(
        bytes calldata journal,
        bytes calldata seal,
        bytes32 expectedPayloadHash,
        string calldata artifactRef
    ) external returns (uint256 recordId) {
        verifier.verify(seal, IMAGE_ID, sha256(journal));

        ZkAuthJournal memory output = decodeJournal(journal);
        require(output.isValid, "Proof output invalid");
        require(output.payloadHash == expectedPayloadHash, "Payload hash mismatch");
        require(output.chainId == block.chainid, "Chain ID mismatch");
        require(output.contractAddress == address(this), "Contract address mismatch");
        require(!usedNullifiers[output.nullifierHash], "Nullifier already used");

        usedNullifiers[output.nullifierHash] = true;

        recordId = recordCount;
        records[recordId] = Record({
            payloadHash: output.payloadHash,
            journalHash: sha256(journal),
            proofHash: sha256(seal),
            nullifierHash: output.nullifierHash,
            recipient: output.recipient,
            mode: "zk-auth",
            artifactRef: artifactRef,
            timestamp: block.timestamp,
            verified: true
        });
        recordCount += 1;

        emit RecordStored(
            recordId, output.payloadHash, output.nullifierHash, output.recipient, "zk-auth", artifactRef, true
        );
    }

    function decodeJournal(bytes calldata journal) public pure returns (ZkAuthJournal memory) {
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

        return ZkAuthJournal({
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
