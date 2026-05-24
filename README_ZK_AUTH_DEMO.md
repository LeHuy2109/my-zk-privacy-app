# ZK Auth Demo - Hướng dẫn từng bước

Tài liệu này chỉ tập trung vào phần `zkAuth demo` vừa được thêm vào repo. Mục tiêu của demo là:

- giữ nguyên ECDSA để gửi transaction và trả gas trên Ethereum
- đưa phần xác thực nghiệp vụ sang RISC Zero proof
- chỉ cho phép contract lưu record khi proof hợp lệ và nullifier chưa bị dùng lại

## 1. Những file thuộc tính năng này

Tất cả code mới thêm gần đây cho `zkAuth demo` nằm trong các file/cụm file sau:

```text
contracts/ZkAuthDemo.sol
contracts/DeployZkAuthDemo.s.sol
test/ZkAuthDemo.t.sol
host/src/lib.rs
host/src/zk_auth.rs
host/src/bin/zk_auth_traditional_demo.rs
host/src/bin/zk_auth_demo.rs
host/src/bin/zk_auth_verify_e2e.rs
host/src/bin/zk_auth_integrity_cases.rs
host/src/bin/zk_auth_availability_benchmark.rs
host/src/bin/zk_auth_compare.rs
host/src/bin/zk_auth_print_image_id.rs
methods/zk-auth-method/
shared/offchain_store/zk-auth/
results/zk-auth/
.env.example
host/Cargo.toml
methods/Cargo.toml
Cargo.lock
```

## 2. Kiến trúc ngắn gọn

Luồng `zkAuth demo` hiện tại như sau:

1. Host tạo `payload_hash` từ nội dung cần lưu.
2. Host tạo input cho guest:
   - `secret`
   - `payload_hash`
   - `recipient`
   - `chain_id`
   - `contract_address`
   - `nonce`
   - `action_type`
3. Guest `methods/zk-auth-method` sinh journal public gồm:
   - `payload_hash`
   - `identity_commitment`
   - `nullifier_hash`
   - `recipient`
   - `chain_id`
   - `contract_address`
   - `action_type`
   - `intent_hash`
   - `is_valid`
4. Host prove bằng RISC Zero, nén và đưa receipt về dạng Groth16 seal để contract verify on-chain.
5. Contract `ZkAuthDemo.sol` verify:
   - `verifier.verify(seal, imageId, sha256(journal))`
   - `payloadHash` trùng với payload dự kiến
   - `chainId == block.chainid`
   - `contractAddress == address(this)`
   - `recipient != address(0)`
   - `nullifierHash` chưa từng dùng
6. Nếu hợp lệ, contract lưu record và đánh dấu `usedNullifiers[nullifierHash] = true`.

## 3. Điều kiện cần trước khi chạy

Bạn cần có:

- Rust/Cargo
- Foundry (`forge`, `cast`)
- Docker đang chạy được
- Docker server architecture là `linux/amd64`
- Sepolia RPC URL
- private key có Sepolia ETH để trả gas

Lưu ý quan trọng:

- Binary `zk_auth_demo` đang hardcode `groth16: true`, nên demo on-chain bắt buộc cần Docker prover.
- Host sẽ kiểm tra image Docker `risczero/risc0-groth16-prover:v2025-04-03.1` trước khi prove.
- Nếu guest code thay đổi, `image id` sẽ đổi theo.

## 4. Chuẩn bị file `.env`

Copy `.env.example` thành `.env`, rồi đảm bảo ít nhất có các biến sau:

```env
SEPOLIA_RPC_URL=https://ethereum-sepolia-rpc.publicnode.com
PRIVATE_KEY=0xYOUR_PRIVATE_KEY
ZK_AUTH_CONTRACT_ADDRESS=0xYOUR_DEPLOYED_ZK_AUTH_CONTRACT
RISC0_VERIFIER_ADDRESS=0x925d8331ddc0a1F0d96E68CF073DFE1d92b69187
ZK_AUTH_IMAGE_ID=0xdf62eef25f0b276c34f4c87e91376befcbf02fa0756fdc942e9cd9c6e6b6df9e
TX_TIMEOUT_SECONDS=120
```

Ý nghĩa:

- `SEPOLIA_RPC_URL`: RPC dùng cho deploy và gửi transaction.
- `PRIVATE_KEY`: key của ví gửi transaction.
- `ZK_AUTH_CONTRACT_ADDRESS`: địa chỉ contract `ZkAuthDemo` sau khi deploy.
- `RISC0_VERIFIER_ADDRESS`: verifier router của RISC Zero. Nếu không set, code cũng mặc định dùng địa chỉ trên.
- `ZK_AUTH_IMAGE_ID`: image id của guest `zk-auth-method`.
- `TX_TIMEOUT_SECONDS`: có trong config, để sẵn cho các luồng mở rộng sau.

## 5. Build và kiểm tra image id

Build phần host/method:

```bash
cargo check -p host
forge build
```

Lấy image id hiện tại của guest:

```bash
cargo run -p host --bin zk_auth_print_image_id
```

Giá trị image id từ code hiện tại:

```text
0xdf62eef25f0b276c34f4c87e91376befcbf02fa0756fdc942e9cd9c6e6b6df9e
```

Nếu lệnh trên ra giá trị khác, ưu tiên giá trị mới đó và cập nhật lại `.env`.

## 6. Deploy contract `ZkAuthDemo`

### Bước 6.1. Export env trong shell hiện tại

```bash
export SEPOLIA_RPC_URL=$(grep '^SEPOLIA_RPC_URL=' .env | cut -d= -f2- | tr -d '\r')
export PRIVATE_KEY=$(grep '^PRIVATE_KEY=' .env | cut -d= -f2- | tr -d '\r')
export ZK_AUTH_IMAGE_ID=$(grep '^ZK_AUTH_IMAGE_ID=' .env | cut -d= -f2- | tr -d '\r')
export RISC0_VERIFIER_ADDRESS=$(grep '^RISC0_VERIFIER_ADDRESS=' .env | cut -d= -f2- | tr -d '\r')
```

Nếu `PRIVATE_KEY` trong `.env` không có tiền tố `0x`, thêm vào trước khi dùng.

### Bước 6.2. Deploy

```bash
forge script contracts/DeployZkAuthDemo.s.sol:DeployZkAuthDemoScript \
  --rpc-url "$SEPOLIA_RPC_URL" \
  --broadcast
```

Script deploy sẽ:

- đọc `PRIVATE_KEY`
- đọc `ZK_AUTH_IMAGE_ID`
- đọc `RISC0_VERIFIER_ADDRESS`, nếu không có thì dùng mặc định `0x925d8331ddc0a1F0d96E68CF073DFE1d92b69187`
- tạo contract `new ZkAuthDemo(verifierAddress, imageId)`

### Bước 6.3. Ghi lại địa chỉ contract vào `.env`

Sau khi deploy xong, lấy địa chỉ contract vừa tạo và cập nhật:

```env
ZK_AUTH_CONTRACT_ADDRESS=0x...
```

## 7. Chạy baseline không dùng proof

Lệnh:

```bash
cargo run -p host --bin zk_auth_traditional_demo
```

Lệnh này sẽ:

- tạo payload mặc định
- hash payload bằng `keccak256`
- gọi `storeRecordTraditional(payloadHash, "traditional")`
- ghi kết quả ra file JSON trong `results/zk-auth/`

Output JSON mẫu:

```text
results/zk-auth/traditional_<timestamp>.json
```

Khi cần, bạn có thể sửa code để truyền payload tùy chỉnh vào `run_traditional_demo`, nhưng binary hiện tại chưa expose CLI cho payload.

## 8. Chạy zkAuth demo thật

Lệnh cơ bản:

```bash
cargo run -p host --bin zk_auth_demo
```

Lệnh với tham số tùy chọn:

```bash
cargo run -p host --bin zk_auth_demo -- \
  --payload "demo payload" \
  --secret 1111111111111111111111111111111111111111111111111111111111111111 \
  --nonce 2222222222222222222222222222222222222222222222222222222222222222 \
  --recipient 0xYourRecipient \
  --action-type 1
```

Binary này sẽ làm lần lượt:

1. Đọc config từ `.env`.
2. Tính `payload_hash = keccak256(payload)`.
3. Nếu không truyền `recipient`, dùng địa chỉ của `PRIVATE_KEY`.
4. Đọc `chain_id` từ RPC hiện tại.
5. Tạo proof bằng guest `zk-auth-method`.
6. Nén receipt về Groth16 seal.
7. Verify proof local một lần nữa trước khi gửi.
8. Lưu artifact off-chain vào:

```text
shared/offchain_store/zk-auth/<timestamp>_<nullifier-prefix>/
```

Thư mục artifact sẽ có:

```text
journal.bin
seal.bin
receipt.json
metadata.json
```

9. Gửi transaction `storeRecordWithProof(...)` lên contract.
10. Ghi kết quả benchmark ra:

```text
results/zk-auth/zk_auth_<timestamp>.json
```

## 9. Verify end-to-end từ record đã lưu on-chain

Nếu muốn verify record mới nhất:

```bash
cargo run -p host --bin zk_auth_verify_e2e
```

Nếu muốn verify một record cụ thể:

```bash
cargo run -p host --bin zk_auth_verify_e2e -- --record-id 1
```

Binary này sẽ:

- đọc record từ contract
- mở `artifactRef` đã lưu trong record
- đọc `metadata.json`, `journal.bin`, `seal.bin`, `receipt.json`
- so sánh:
  - payload hash local với `record.payloadHash`
  - `sha256(journal)` với `record.journalHash`
  - `sha256(seal)` với `record.proofHash`
  - receipt local verify đúng `ZK_AUTH_METHOD_ID`
- ghi kết quả ra:

```text
results/zk-auth/verify_e2e_<timestamp>.json
```

## 10. Chạy bộ integrity cases

Lệnh:

```bash
cargo run -p host --bin zk_auth_integrity_cases
```

Bộ test này tự động tạo 1 proof hợp lệ để setup, sau đó thử các trường hợp sau:

- `tampered_payload_hash`
- `tampered_journal`
- `tampered_seal`
- `reused_nullifier`
- `wrong_chain_id`
- `wrong_contract_address`
- `wrong_recipient`

Kết quả được ghi vào:

```text
results/zk-auth/integrity_<timestamp>.json
```

## 11. Chạy availability benchmark

Lệnh mặc định:

```bash
cargo run -p host --bin zk_auth_availability_benchmark
```

Lệnh tùy chỉnh:

```bash
cargo run -p host --bin zk_auth_availability_benchmark -- \
  --count 10 \
  --max-retries 0
```

Binary này benchmark cả 2 mode:

- `traditional`
- `zk_auth`

Và ghi kết quả vào:

```text
results/zk-auth/availability_<timestamp>.json
```

Metric gồm:

- `success_rate_percent`
- `average_latency_seconds`
- `p50_latency_seconds`
- `p95_latency_seconds`
- `p99_latency_seconds`
- `throughput_tx_per_second`
- `average_gas_used`
- `retry_count`
- `error_breakdown`

## 12. So sánh kết quả mới nhất

Sau khi đã có ít nhất 4 file:

- `traditional_*.json`
- `zk_auth_*.json`
- `availability_*.json`
- `integrity_*.json`

chạy:

```bash
cargo run -p host --bin zk_auth_compare
```

Binary sẽ tổng hợp các chỉ số chính cho 3 nhóm:

- `traditional`
- `zk_auth`
- `zk_auth_offchain_artifact`

Trong đó có các metric như:

- `gas_used`
- `proof_generation_seconds`
- `proof_verify_seconds`
- `seal_size_bytes`
- `journal_size_bytes`
- `artifact_size_bytes`
- `calldata_size_bytes`
- `send_and_confirm_seconds`
- `total_latency_seconds`
- `success_rate_percent`
- `tamper_detection_rate`
- `replay_rejection_rate`

## 13. Unit test contract

Chạy test Solidity:

```bash
forge test --match-contract ZkAuthDemoTest
```

Test hiện tại cover:

- lưu record theo mode `traditional`
- lưu record bằng proof
- chặn replay nullifier
- reject payload hash sai
- reject chain id sai
- reject contract address sai

## 14. Thư mục output cần nhớ

Artifact off-chain:

```text
shared/offchain_store/zk-auth/
```

Kết quả JSON:

```text
results/zk-auth/
```

## 15. Các lưu ý để tránh lỗi

- `zk_auth_demo` sẽ fail nếu Docker không chạy được hoặc không phải `linux/amd64`.
- `ZK_AUTH_CONTRACT_ADDRESS` trong `.env` phải là địa chỉ contract `ZkAuthDemo`, không phải contract privacy cũ.
- `ZK_AUTH_IMAGE_ID` phải trùng với guest đang được build.
- `artifactRef` là đường dẫn tương đối trong repo, nên không được xóa thư mục `shared/offchain_store/zk-auth/` nếu còn muốn verify e2e.
- Nullifier bị đánh dấu đã dùng sau lần submit thành công, nên không thể replay cùng proof.
- Demo này chỉ thay thế xác thực nghiệp vụ, không thay thế ECDSA ở tầng protocol Ethereum.

## 16. Lệnh chạy nhanh từ đầu đến cuối

Nếu đã có `.env` đầy đủ và contract đã deploy, thứ tự chạy nhanh là:

```bash
cargo run -p host --bin zk_auth_traditional_demo
cargo run -p host --bin zk_auth_demo
cargo run -p host --bin zk_auth_verify_e2e
cargo run -p host --bin zk_auth_integrity_cases
cargo run -p host --bin zk_auth_availability_benchmark -- --count 10
cargo run -p host --bin zk_auth_compare
```
