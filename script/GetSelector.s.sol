// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "risc0/groth16/RiscZeroGroth16Verifier.sol";

contract GetSelector is Script {
    function run() external {
        // Need to pass the correct control_root and bn254_control_id
        // Wait! We can't deploy it without knowing them!
        // But the Sepolia Router VERIFIER was ALREADY DEPLOYED!
        // Can we query the Sepolia Router for verifier mapping? Not easily without the key.
        // Wait, what if we just fetch the selector from the receipt itself?
    }
}
