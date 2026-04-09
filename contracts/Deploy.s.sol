// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "./PrivacyVerifier.sol";

contract DeployScript is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        // Official RISC Zero Verifier Router on Sepolia
        address RISC0_VERIFIER_ROUTER = 0x925d8331ddc0a1F0d96E68CF073DFE1d92b69187;
        
        PrivacyVerifier privacy = new PrivacyVerifier(RISC0_VERIFIER_ROUTER);

        vm.stopBroadcast();
    }
}
