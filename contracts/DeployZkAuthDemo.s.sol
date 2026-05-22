// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "./ZkAuthDemo.sol";

contract DeployZkAuthDemoScript is Script {
    function run() external {
        string memory privateKey = vm.envString("PRIVATE_KEY");
        bytes memory privateKeyBytes = bytes(privateKey);
        if (
            privateKeyBytes.length < 2
                || privateKeyBytes[0] != 0x30
                || (privateKeyBytes[1] != 0x78 && privateKeyBytes[1] != 0x58)
        ) {
            privateKey = string.concat("0x", privateKey);
        }

        uint256 deployerPrivateKey = vm.parseUint(privateKey);
        bytes32 imageId = vm.envBytes32("ZK_AUTH_IMAGE_ID");

        vm.startBroadcast(deployerPrivateKey);

        address RISC0_VERIFIER_ROUTER = 0x925d8331ddc0a1F0d96E68CF073DFE1d92b69187;
        new ZkAuthDemo(RISC0_VERIFIER_ROUTER, imageId);

        vm.stopBroadcast();
    }
}
