// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";

/**
 * @title PrivacyVerifier - ZK Privacy Transaction Contract
 * @dev Implements deposit/withdraw with Merkle inclusion proof + nullifier
 *      Uses RISC Zero for ZK verification
 */
contract PrivacyVerifier {
    // ─── Constants ──────────────────────────────────────────────────────
    
    /// RISC Zero Verifier instance
    IRiscZeroVerifier public immutable verifier;

    /// Depth of Merkle tree (matches guest TREE_DEPTH)
    uint256 constant TREE_DEPTH = 20;

    /// IMAGE_ID from RISC0 guest (update after building guest)
    bytes32 constant IMAGE_ID = 0x758d2389d7d3d0e7915b0b2e81da7ac102c78d36a7c8611f59b493e58b0f1098;
    
    // ─── State Variables ─────────────────────────────────────────────────

    /// Current Merkle root
    bytes32 public currentRoot;

    /// Used nullifiers to prevent double-spend
    mapping(bytes32 => bool) public usedNullifiers;

    /// Incremental Merkle Tree storage
    uint32 public nextIndex = 0;
    bytes32[TREE_DEPTH] public zeros;
    bytes32[TREE_DEPTH] public filledSubtrees;

    // ─── Events ──────────────────────────────────────────────────────────

    event Deposit(bytes32 indexed commitment, uint256 indexed leafIndex);
    event Withdraw(bytes32 indexed nullifier, address indexed recipient, uint256 amount);

    // ─── Structs ─────────────────────────────────────────────────────────

    /// Public output from ZK proof (matches TransactionOutput in Rust)
    struct TransactionOutput {
        bytes32 merkle_root;
        bytes32 nullifier_hash;
        address recipient;
        uint256 amount;
        bool is_valid;
    }

    // ─── Constructor ─────────────────────────────────────────────────────

    constructor(address _verifier) {
        verifier = IRiscZeroVerifier(_verifier);
        // Initialize zeros for empty subtrees
        bytes32 currentZero = 0x0000000000000000000000000000000000000000000000000000000000000000;
        for (uint256 i = 0; i < TREE_DEPTH; i++) {
            zeros[i] = currentZero;
            filledSubtrees[i] = currentZero;
            currentZero = sha256(abi.encodePacked(currentZero, currentZero));
        }
        currentRoot = currentZero;
    }

    // ─── Deposit Function ─────────────────────────────────────────────────

    /**
     * @dev Deposit ETH and create commitment
     * @param commitment SHA256(secret || amount_le_bytes)
     */
    function deposit(bytes32 commitment) external payable {
        require(msg.value > 0, "Deposit amount must be > 0");
        require(nextIndex < uint32(2)**TREE_DEPTH, "Merkle tree is full");

        // Insert commitment into Incremental Merkle Tree
        uint32 currentIndex = nextIndex;
        bytes32 currentLevelHash = commitment;

        for (uint8 i = 0; i < TREE_DEPTH; i++) {
            if (currentIndex % 2 == 0) {
                filledSubtrees[i] = currentLevelHash;
                currentLevelHash = sha256(abi.encodePacked(currentLevelHash, zeros[i]));
            } else {
                currentLevelHash = sha256(abi.encodePacked(filledSubtrees[i], currentLevelHash));
            }
            currentIndex /= 2;
        }

        currentRoot = currentLevelHash;
        
        emit Deposit(commitment, nextIndex);
        nextIndex += 1;
    }

    // ─── Withdraw Function ────────────────────────────────────────────────

    /**
     * @dev Withdraw using ZK proof
     * @param journal Encoded TransactionOutput from guest
     * @param seal RISC0 proof seal
     * @param nullifier Nullifier hash to prevent double-spend
     * @param recipient Address to receive funds
     */
    function withdraw(
        bytes calldata journal,
        bytes calldata seal,
        bytes32 nullifier,
        address recipient
    ) external {
        // 1. Verify RISC0 proof
        verifier.verify(seal, IMAGE_ID, sha256(journal));

        // 2. Decode journal to TransactionOutput
        TransactionOutput memory output = decodeJournal(journal);

        // 3. Validate proof output
        require(output.is_valid, "Proof output invalid");
        require(output.merkle_root == currentRoot, "Merkle root mismatch");
        require(output.nullifier_hash == nullifier, "Nullifier hash mismatch");
        require(output.amount > 0, "Amount must be > 0");
        require(output.recipient == recipient, "Recipient mismatch");

        // 4. Check nullifier not used
        require(!usedNullifiers[nullifier], "Nullifier already used");

        // 5. Check sufficient balance (contract has enough ETH)
        require(address(this).balance >= output.amount, "Insufficient contract balance");

        // 6. Mark nullifier as used
        usedNullifiers[nullifier] = true;

        // 7. Transfer funds
        payable(recipient).transfer(output.amount);

        emit Withdraw(nullifier, recipient, output.amount);
    }

    // ─── Journal Decoding ────────────────────────────────────────────────

    /**
     * @dev Decode journal bytes to TransactionOutput struct
     * Journal format: ABI-encoded TransactionOutput
     */
    function decodeJournal(bytes calldata journal) internal pure returns (TransactionOutput memory) {
        (
            bytes32 merkle_root,
            bytes32 nullifier_hash,
            address recipient,
            uint256 amount,
            bool is_valid
        ) = abi.decode(journal, (bytes32, bytes32, address, uint256, bool));

        return TransactionOutput({
            merkle_root: merkle_root,
            nullifier_hash: nullifier_hash,
            recipient: recipient,
            amount: amount,
            is_valid: is_valid
        });
    }

    // ─── View Functions ──────────────────────────────────────────────────

    /**
     * @dev Get number of deposits
     */
    function getDepositCount() external view returns (uint256) {
        return nextIndex;
    }

    /**
     * @dev Check if nullifier is used
     */
    function isNullifierUsed(bytes32 nullifier) external view returns (bool) {
        return usedNullifiers[nullifier];
    }

    // ─── Receive Function ────────────────────────────────────────────────

    receive() external payable {}
}
