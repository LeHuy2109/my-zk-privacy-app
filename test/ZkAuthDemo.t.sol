// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import {ZkAuthDemo} from "../contracts/ZkAuthDemo.sol";
import {RiscZeroMockVerifier} from "../lib/risc0-ethereum/contracts/src/test/RiscZeroMockVerifier.sol";
import {Receipt as RiscZeroReceipt} from "../lib/risc0-ethereum/contracts/src/IRiscZeroVerifier.sol";

contract ZkAuthDemoTest is Test {
    bytes32 internal constant IMAGE_ID = keccak256("zk-auth-demo-test-image");
    bytes4 internal constant SELECTOR = bytes4(0xFFFFFFFF);

    RiscZeroMockVerifier internal verifier;
    ZkAuthDemo internal demo;

    function setUp() public {
        verifier = new RiscZeroMockVerifier(SELECTOR);
        demo = new ZkAuthDemo(address(verifier), IMAGE_ID);
    }

    function testStoreRecordTraditionalSucceeds() public {
        bytes32 payloadHash = keccak256("traditional");

        demo.storeRecordTraditional(payloadHash, "traditional");

        ZkAuthDemo.Record memory record = demo.getRecord(1);
        assertEq(demo.recordCount(), 1);
        assertEq(record.payloadHash, payloadHash);
        assertEq(record.recipient, address(this));
        assertEq(record.mode, "traditional");
        assertTrue(record.verified);
    }

    function testStoreRecordWithProofSucceeds() public {
        bytes32 payloadHash = keccak256("zk-auth");
        bytes32 identityCommitment = keccak256("identity");
        bytes32 nullifierHash = keccak256("nullifier");
        bytes memory journal = abi.encode(
            payloadHash,
            identityCommitment,
            nullifierHash,
            address(0xBEEF),
            uint64(block.chainid),
            address(demo),
            uint32(1),
            keccak256("intent"),
            true
        );

        bytes memory seal = mockSeal(journal);
        demo.storeRecordWithProof(journal, seal, payloadHash, "shared/offchain_store/zk-auth/demo/metadata.json");

        ZkAuthDemo.Record memory record = demo.getRecord(1);
        assertEq(demo.recordCount(), 1);
        assertEq(record.payloadHash, payloadHash);
        assertEq(record.journalHash, sha256(journal));
        assertEq(record.proofHash, sha256(seal));
        assertEq(record.nullifierHash, nullifierHash);
        assertEq(record.identityCommitment, identityCommitment);
        assertEq(record.recipient, address(0xBEEF));
        assertEq(record.mode, "zk_auth");
        assertTrue(record.verified);
        assertTrue(demo.usedNullifiers(nullifierHash));
    }

    function testReplayNullifierRejected() public {
        bytes32 payloadHash = keccak256("zk-auth");
        bytes memory journal = abi.encode(
            payloadHash,
            keccak256("identity"),
            keccak256("nullifier"),
            address(0xCAFE),
            uint64(block.chainid),
            address(demo),
            uint32(1),
            keccak256("intent"),
            true
        );
        bytes memory seal = mockSeal(journal);

        demo.storeRecordWithProof(journal, seal, payloadHash, "artifact.json");

        vm.expectRevert("Nullifier already used");
        demo.storeRecordWithProof(journal, seal, payloadHash, "artifact.json");
    }

    function testPayloadMismatchRejected() public {
        bytes32 payloadHash = keccak256("zk-auth");
        bytes memory journal = abi.encode(
            payloadHash,
            keccak256("identity"),
            keccak256("nullifier"),
            address(0xCAFE),
            uint64(block.chainid),
            address(demo),
            uint32(1),
            keccak256("intent"),
            true
        );
        bytes memory seal = mockSeal(journal);

        vm.expectRevert("Payload hash mismatch");
        demo.storeRecordWithProof(journal, seal, keccak256("different"), "artifact.json");
    }

    function testChainMismatchRejected() public {
        bytes32 payloadHash = keccak256("zk-auth");
        bytes memory journal = abi.encode(
            payloadHash,
            keccak256("identity"),
            keccak256("nullifier"),
            address(0xCAFE),
            uint64(block.chainid + 1),
            address(demo),
            uint32(1),
            keccak256("intent"),
            true
        );
        bytes memory seal = mockSeal(journal);

        vm.expectRevert("Chain ID mismatch");
        demo.storeRecordWithProof(journal, seal, payloadHash, "artifact.json");
    }

    function testContractMismatchRejected() public {
        bytes32 payloadHash = keccak256("zk-auth");
        bytes memory journal = abi.encode(
            payloadHash,
            keccak256("identity"),
            keccak256("nullifier"),
            address(0xCAFE),
            uint64(block.chainid),
            address(0x1234),
            uint32(1),
            keccak256("intent"),
            true
        );
        bytes memory seal = mockSeal(journal);

        vm.expectRevert("Contract mismatch");
        demo.storeRecordWithProof(journal, seal, payloadHash, "artifact.json");
    }

    function mockSeal(bytes memory journal) internal view returns (bytes memory) {
        RiscZeroReceipt memory receipt = verifier.mockProve(IMAGE_ID, sha256(journal));
        return receipt.seal;
    }
}
