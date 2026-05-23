// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import {ZkAuthDemo} from "../contracts/ZkAuthDemo.sol";
import {RiscZeroMockVerifier} from "risc0/test/RiscZeroMockVerifier.sol";

contract ZkAuthDemoTest is Test {
    RiscZeroMockVerifier internal verifier;
    ZkAuthDemo internal demo;

    bytes32 internal payloadHash = sha256("payload");
    bytes32 internal identityCommitment = sha256("identity");
    bytes32 internal nullifierHash = sha256("nullifier");
    bytes32 internal intentHash = sha256("intent");
    address internal recipient = address(0xBEEF);

    function setUp() public {
        verifier = new RiscZeroMockVerifier(bytes4("MOCK"));
        demo = new ZkAuthDemo(address(verifier));
    }

    function testTraditionalStoreSucceeds() public {
        uint256 id = demo.storeRecordTraditional(payloadHash, "traditional");

        assertEq(id, 0);
        assertEq(demo.recordCount(), 1);

        (bytes32 storedPayload,,,, address storedRecipient, string memory mode,, uint256 timestamp, bool verified) =
            demo.records(id);
        assertEq(storedPayload, payloadHash);
        assertEq(storedRecipient, address(this));
        assertEq(mode, "traditional");
        assertGt(timestamp, 0);
        assertFalse(verified);
    }

    function testZkStoreWithValidProofSucceeds() public {
        bytes memory journal = validJournal(nullifierHash);
        bytes memory seal = verifier.mockProve(demo.IMAGE_ID(), sha256(journal)).seal;

        uint256 id = demo.storeRecordWithProof(journal, seal, payloadHash, "local://artifact");

        assertEq(id, 0);
        assertEq(demo.recordCount(), 1);
        assertTrue(demo.usedNullifiers(nullifierHash));

        (
            bytes32 storedPayload,
            bytes32 journalHash,
            bytes32 proofHash,
            bytes32 storedNullifier,
            address storedRecipient,,,,
            bool verified
        ) = demo.records(id);
        assertEq(storedPayload, payloadHash);
        assertEq(journalHash, sha256(journal));
        assertEq(proofHash, sha256(seal));
        assertEq(storedNullifier, nullifierHash);
        assertEq(storedRecipient, recipient);
        assertTrue(verified);
    }

    function testReplayNullifierIsRejected() public {
        bytes memory journal = validJournal(nullifierHash);
        bytes memory seal = verifier.mockProve(demo.IMAGE_ID(), sha256(journal)).seal;

        demo.storeRecordWithProof(journal, seal, payloadHash, "first");

        vm.expectRevert("Nullifier already used");
        demo.storeRecordWithProof(journal, seal, payloadHash, "replay");
    }

    function testPayloadMismatchIsRejected() public {
        bytes memory journal = validJournal(nullifierHash);
        bytes memory seal = verifier.mockProve(demo.IMAGE_ID(), sha256(journal)).seal;
        bytes32 differentPayload = sha256("different");

        vm.expectRevert("Payload hash mismatch");
        demo.storeRecordWithProof(journal, seal, differentPayload, "bad");
    }

    function validJournal(bytes32 selectedNullifier) internal view returns (bytes memory) {
        return abi.encode(
            payloadHash,
            identityCommitment,
            selectedNullifier,
            recipient,
            uint64(block.chainid),
            address(demo),
            uint32(1),
            intentHash,
            true
        );
    }
}
