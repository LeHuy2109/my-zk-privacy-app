# ZK-STARK Authentication Layer for Blockchain Record

Demo này thêm một lớp xác thực nghiệp vụ bằng RISC Zero proof cho hành động ghi record lên blockchain. Transaction Ethereum vẫn do EOA ký để trả gas; điểm khác là contract `ZkAuthDemo` không dùng `msg.sender` làm điều kiện xác thực nghiệp vụ cho flow ZK-auth. Contract chỉ ghi record khi proof hợp lệ, journal đúng payload, đúng chain, đúng contract, và nullifier chưa dùng.

Demo độc lập với flow privacy/deposit/withdraw hiện tại.

## Thành phần mới

- `contracts/ZkAuthDemo.sol`: contract record demo với traditional baseline và ZK-auth write.
- `contracts/DeployZkAuthDemo.s.sol`: deploy script riêng cho demo.
- `methods/zk-auth-guest/`: RISC Zero guest riêng.
- `host/src/bin/zk_auth_*.rs`: các CLI demo, integrity, benchmark, compare.
- `results/zk-auth/`: nơi CLI ghi JSON kết quả và artifact local.

## ECDSA baseline vs ZK-auth

Traditional baseline:

- EOA ký transaction bằng `PRIVATE_KEY`.
- Contract ghi record qua `storeRecordTraditional`.
- Đây là baseline gas/latency của một record write thông thường.

ZK-auth application verification:

- EOA vẫn ký transaction để trả gas ở protocol Ethereum.
- Guest chứng minh private `secret` hợp lệ với payload, recipient, chain id, contract address, nonce và action type.
- Contract verify RISC Zero proof, decode journal, kiểm tra payload/chain/contract/nullifier rồi mới ghi record.
- Nullifier chống replay ở tầng nghiệp vụ.

## Guest journal

Guest nhận:

- `secret: [u8; 32]`
- `payload_hash: [u8; 32]`
- `recipient: [u8; 20]`
- `chain_id: u64`
- `contract_address: [u8; 20]`
- `nonce: [u8; 32]`
- `action_type: u32`

Guest commit ABI journal gồm `payload_hash`, `identity_commitment`, `nullifier_hash`, `recipient`, `chain_id`, `contract_address`, `action_type`, `intent_hash`, `is_valid`.

## Cấu hình

Trong `.env`:

```bash
SEPOLIA_RPC_URL=...
PRIVATE_KEY=...
ZK_AUTH_CONTRACT_ADDRESS=...
```

`ZK_AUTH_CONTRACT_ADDRESS` là contract `ZkAuthDemo`, không phải `PrivacyVerifier`.

## Build và test

```bash
cargo check -p host --bins
forge test
```

## Deploy

```bash
forge script contracts/DeployZkAuthDemo.s.sol:DeployZkAuthDemoScript \
  --rpc-url "$SEPOLIA_RPC_URL" \
  --broadcast
```

Sau deploy, ghi địa chỉ contract vào `.env`:

```bash
ZK_AUTH_CONTRACT_ADDRESS=0x...
```

## Chạy demo

Traditional baseline:

```bash
cargo run -p host --bin zk_auth_traditional_demo
```

ZK-auth:

```bash
cargo run -p host --bin zk_auth_demo -- --groth16
```

`--groth16` cần cho verifier router Sepolia. Không có flag này, CLI vẫn tạo receipt local nhưng seal không phải Groth16 nên contract thật sẽ từ chối.

Integrity cases:

```bash
cargo run -p host --bin zk_auth_integrity_cases -- --groth16
```

Availability benchmark, mặc định `N=10`:

```bash
cargo run -p host --bin zk_auth_availability_benchmark -- --n 10 --groth16
```

So sánh JSON mới nhất:

```bash
cargo run -p host --bin zk_auth_compare
```

## JSON kết quả

Kết quả nằm trong:

```text
results/zk-auth/
results/zk-auth/artifacts/
```

Các trường chính:

- `gas_used`, `tx_build_seconds`, `send_and_confirm_seconds`, `total_latency_seconds`
- `proof_generation_seconds`, `proof_verify_seconds`
- `seal_size_bytes`, `journal_size_bytes`, `calldata_size_bytes`
- `success_rate_percent`, `average_latency_seconds`, `throughput_tx_per_second`
- `tamper_detection_rate`, `replay_rejection_rate` qua `zk_auth_compare`

## Hạn chế và TODO

- Artifact off-chain hiện là file local, chưa dùng IPFS thật.
- `raw_tx_size_bytes` và `ecdsa_sign_seconds` để `null` vì Alloy ký/gửi qua provider pipeline nội bộ.
- Benchmark ZK-auth có thể rất chậm khi bật Groth16 vì mỗi vòng nén proof.
- Chưa đưa PQC Dilithium/Falcon/SPHINCS+ vào guest; đây là bước mở rộng sau.
