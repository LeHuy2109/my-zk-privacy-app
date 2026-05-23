# Tutorial: ZK-STARK Authentication Layer for Blockchain Record

File này hướng dẫn chạy demo mới từng bước, theo đúng repo hiện tại. Mục tiêu là giúp bạn có thể:

- build guest, host, contract
- deploy contract demo `ZkAuthDemo`
- chạy baseline traditional
- chạy ZK-auth demo
- chạy integrity cases
- chạy availability benchmark
- so sánh kết quả JSON

Tất cả lệnh bên dưới giả định bạn đang đứng ở root của repo:

```bash
cd /home/elixirfy/Code/MMHCS/my-zk-privacy-app
```

## 1. Chuẩn bị môi trường

### 1.1. Công cụ cần có

Bạn cần cài sẵn:

- Rust
- Foundry: `forge`, `cast`
- Docker

Kiểm tra nhanh:

```bash
rustc --version
cargo --version
forge --version
cast --version
docker --version
```

### 1.2. Tạo file `.env`

Nếu chưa có:

```bash
cp .env.example .env
```

Trong `.env`, ít nhất cần các biến sau:

```bash
SEPOLIA_RPC_URL=https://ethereum-sepolia-rpc.publicnode.com
PRIVATE_KEY=0xYOUR_PRIVATE_KEY
ZK_AUTH_CONTRACT_ADDRESS=0xYOUR_DEPLOYED_ZK_AUTH_DEMO
```

Ghi chú:

- `PRIVATE_KEY` là ví Sepolia có ETH test để trả gas.
- `ZK_AUTH_CONTRACT_ADDRESS` là địa chỉ của contract `ZkAuthDemo`.
- Không dùng `CONTRACT_ADDRESS` cũ của `PrivacyVerifier` cho demo này.

## 2. Nạp biến global một lần

Đoạn này đọc trực tiếp từ `.env` và export thành biến shell để các lệnh sau dùng luôn.

```bash
export SEPOLIA_RPC_URL=$(grep '^SEPOLIA_RPC_URL=' .env | cut -d= -f2- | tr -d '\r')
export PRIVATE_KEY=$(grep '^PRIVATE_KEY=' .env | cut -d= -f2- | tr -d '\r')
export ZK_AUTH_CONTRACT_ADDRESS=$(grep '^ZK_AUTH_CONTRACT_ADDRESS=' .env | cut -d= -f2- | tr -d '\r')

if [[ "$PRIVATE_KEY" != 0x* ]]; then
  export PRIVATE_KEY="0x$PRIVATE_KEY"
fi
```

Kiểm tra lại các biến:

```bash
printf 'SEPOLIA_RPC_URL=%s\n' "$SEPOLIA_RPC_URL"
printf 'PRIVATE_KEY=%s\n' "${PRIVATE_KEY:0:10}..."
printf 'ZK_AUTH_CONTRACT_ADDRESS=%s\n' "$ZK_AUTH_CONTRACT_ADDRESS"
cast wallet address --private-key "$PRIVATE_KEY"
```

Nếu `ZK_AUTH_CONTRACT_ADDRESS` đang trống, bạn chưa deploy contract demo và cần làm bước 4 trước.

## 3. Build và test local

### 3.1. Build host + guest

```bash
cargo check -p host --bins
```

Lệnh này sẽ:

- build các binary demo mới trong `host/src/bin/`
- build guest RISC Zero mới trong `methods/zk-auth-guest/`
- generate `ZK_AUTH_METHOD_ID` để host và contract dùng đúng guest image

### 3.2. Chạy test contract

```bash
forge test
```

Test tối thiểu đã có:

- traditional store thành công
- zk store với proof hợp lệ thành công
- replay nullifier bị reject
- payload mismatch bị reject

## 4. Deploy contract demo ZK-auth

### 4.1. Deploy lên Sepolia

```bash
forge script contracts/DeployZkAuthDemo.s.sol:DeployZkAuthDemoScript \
  --rpc-url "$SEPOLIA_RPC_URL" \
  --private-key "$PRIVATE_KEY" \
  --broadcast
```

Contract này deploy:

- `contracts/ZkAuthDemo.sol`
- verifier đang dùng là RISC Zero verifier router trên Sepolia

### 4.2. Lấy địa chỉ contract vừa deploy

Sau khi deploy xong, Foundry sẽ ghi log vào thư mục `broadcast/`. Bạn mở file `run-latest.json` tương ứng để lấy địa chỉ contract vừa tạo.

Ví dụ bạn có thể tìm nhanh bằng:

```bash
rg '"contractAddress"|"contractName"' broadcast/DeployZkAuthDemo.s.sol -n
```

Nếu muốn xem JSON mới nhất:

```bash
find broadcast/DeployZkAuthDemo.s.sol -name run-latest.json -print
```

### 4.3. Cập nhật `.env`

Ghi lại địa chỉ vừa deploy:

```bash
ZK_AUTH_CONTRACT_ADDRESS=0x...
```

Sau đó nạp lại biến global:

```bash
export ZK_AUTH_CONTRACT_ADDRESS=$(grep '^ZK_AUTH_CONTRACT_ADDRESS=' .env | cut -d= -f2- | tr -d '\r')
printf 'ZK_AUTH_CONTRACT_ADDRESS=%s\n' "$ZK_AUTH_CONTRACT_ADDRESS"
```

## 5. Chạy baseline traditional

### 5.1. Gửi record theo kiểu truyền thống

```bash
cargo run -p host --bin zk_auth_traditional_demo
```

Script này sẽ:

- tạo payload dạng text kèm timestamp
- tính `payload_hash`
- gọi `storeRecordTraditional(payloadHash, mode)`
- gửi transaction bằng `PRIVATE_KEY`
- ghi JSON kết quả vào `results/zk-auth/`

### 5.2. File kết quả

Tìm file mới nhất:

```bash
find results/zk-auth -maxdepth 1 -name 'traditional_*.json' | sort | tail -n 1
```

Xem nhanh nội dung:

```bash
LATEST_TRAD=$(find results/zk-auth -maxdepth 1 -name 'traditional_*.json' | sort | tail -n 1)
sed -n '1,220p' "$LATEST_TRAD"
```

Các trường quan trọng:

- `payload`
- `payload_hash`
- `tx_hash`
- `gas_used`
- `send_and_confirm_seconds`
- `total_latency_seconds`

## 6. Chạy ZK-auth demo

### 6.1. Chạy bản chuẩn on-chain

```bash
cargo run -p host --bin zk_auth_demo -- --groth16
```

Giải thích:

- `--groth16` là bắt buộc nếu bạn muốn transaction thực sự được contract Sepolia chấp nhận
- host sẽ tạo proof từ guest `zk-auth-guest`
- host lưu artifact local vào `results/zk-auth/artifacts/`
- host gọi `storeRecordWithProof(journal, seal, payloadHash, artifactRef)`

### 6.2. Chạy với `secret`, `nonce`, `recipient` tự chỉ định

Nếu muốn cố định đầu vào để benchmark lặp lại:

```bash
cargo run -p host --bin zk_auth_demo -- \
  --groth16 \
  --recipient 0xf8329687322ADC276eDEA5cC25a6959Da1f5Dd7a \
  --secret 0bfb62f21f20f4edfaa33fbfdfd6bdcd0da813046fc78fda3bc0823550ae7a12 \
  --nonce 1111111111111111111111111111111111111111111111111111111111111111
```

Ghi chú:

- `secret` phải là hex 32 byte
- `nonce` phải là hex 32 byte
- `recipient` là address nhận record trong journal

### 6.3. File kết quả

Tìm file mới nhất:

```bash
find results/zk-auth -maxdepth 1 -name 'zk_auth_*.json' | sort | tail -n 1
```

Xem nhanh:

```bash
LATEST_ZK=$(find results/zk-auth -maxdepth 1 -name 'zk_auth_*.json' | sort | tail -n 1)
sed -n '1,260p' "$LATEST_ZK"
```

Các trường quan trọng:

- `identity_commitment`
- `nullifier_hash`
- `intent_hash`
- `journal_hash`
- `proof_hash`
- `artifact_ref`
- `proof_generation_seconds`
- `proof_verify_seconds`
- `seal_size_bytes`
- `journal_size_bytes`
- `calldata_size_bytes`
- `gas_used`

## 7. Xem artifact off-chain

Artifact local được ghi vào:

```bash
find results/zk-auth/artifacts -type f | sort | tail -n 3
```

Xem artifact mới nhất:

```bash
LATEST_ARTIFACT=$(find results/zk-auth/artifacts -type f | sort | tail -n 1)
sed -n '1,260p' "$LATEST_ARTIFACT"
```

Artifact hiện chứa:

- `input`
- `receipt`
- `journal_hex`
- `journal_hash`

Hiện tại đây là local off-chain store, chưa phải IPFS thật.

## 8. Chạy integrity cases

Script này dùng để chứng minh contract reject các trường hợp sai lệch hoặc replay.

### 8.1. Chạy integrity cases

```bash
cargo run -p host --bin zk_auth_integrity_cases -- --groth16
```

Các case âm tính hiện có:

- sửa `payloadHash`
- sửa một byte trong `journal`
- sửa một byte trong `seal`
- dùng sai `chainId`
- dùng sai `contractAddress`
- dùng sai `recipient` bằng zero address
- dùng lại `nullifier`

### 8.2. Xem kết quả integrity

```bash
LATEST_INTEGRITY=$(find results/zk-auth -maxdepth 1 -name 'integrity_*.json' | sort | tail -n 1)
sed -n '1,320p' "$LATEST_INTEGRITY"
```

Mỗi case sẽ có:

- `case_name`
- `expected_result`
- `actual_result`
- `passed`
- `revert_reason`
- `latency_seconds`

## 9. Chạy availability benchmark

### 9.1. Chạy benchmark mặc định 10 vòng

```bash
cargo run -p host --bin zk_auth_availability_benchmark -- --n 10 --groth16
```

### 9.2. Chạy chỉ benchmark traditional

```bash
cargo run -p host --bin zk_auth_availability_benchmark -- --n 10 --mode traditional
```

### 9.3. Chạy chỉ benchmark ZK-auth

```bash
cargo run -p host --bin zk_auth_availability_benchmark -- --n 10 --mode zk --groth16
```

### 9.4. Dùng env để đặt số vòng lặp

```bash
export ZK_AUTH_BENCH_N=5
cargo run -p host --bin zk_auth_availability_benchmark -- --groth16
```

### 9.5. Xem file benchmark

```bash
LATEST_AVAIL=$(find results/zk-auth -maxdepth 1 -name 'availability_*.json' | sort | tail -n 1)
sed -n '1,320p' "$LATEST_AVAIL"
```

Các trường quan trọng:

- `success_count`
- `failure_count`
- `success_rate_percent`
- `average_latency_seconds`
- `p50_latency_seconds`
- `p95_latency_seconds`
- `p99_latency_seconds`
- `throughput_tx_per_second`
- `average_gas_used`
- `error_breakdown`

## 10. So sánh kết quả

### 10.1. In bảng compare

```bash
cargo run -p host --bin zk_auth_compare
```

Script sẽ đọc các file mới nhất trong `results/zk-auth/` và in bảng so sánh:

- traditional baseline
- zk-auth
- zk-auth + artifact
- success rate
- tamper detection rate
- replay rejection rate

### 10.2. Khi compare chưa có dữ liệu

Nếu bảng hiện `-` ở nhiều cột thì thường là do bạn chưa chạy đủ các script sau:

```bash
cargo run -p host --bin zk_auth_traditional_demo
cargo run -p host --bin zk_auth_demo -- --groth16
cargo run -p host --bin zk_auth_integrity_cases -- --groth16
cargo run -p host --bin zk_auth_availability_benchmark -- --n 10 --groth16
```

## 11. Quy trình khuyến nghị từ đầu đến cuối

Nếu bạn muốn chạy theo đúng flow ngắn gọn nhất:

### Bước 1: build và test

```bash
cargo check -p host --bins
forge test
```

### Bước 2: deploy contract demo

```bash
forge script contracts/DeployZkAuthDemo.s.sol:DeployZkAuthDemoScript \
  --rpc-url "$SEPOLIA_RPC_URL" \
  --private-key "$PRIVATE_KEY" \
  --broadcast
```

### Bước 3: cập nhật lại `ZK_AUTH_CONTRACT_ADDRESS`

```bash
export ZK_AUTH_CONTRACT_ADDRESS=$(grep '^ZK_AUTH_CONTRACT_ADDRESS=' .env | cut -d= -f2- | tr -d '\r')
printf 'ZK_AUTH_CONTRACT_ADDRESS=%s\n' "$ZK_AUTH_CONTRACT_ADDRESS"
```

### Bước 4: chạy baseline

```bash
cargo run -p host --bin zk_auth_traditional_demo
```

### Bước 5: chạy ZK-auth

```bash
cargo run -p host --bin zk_auth_demo -- --groth16
```

### Bước 6: chạy integrity

```bash
cargo run -p host --bin zk_auth_integrity_cases -- --groth16
```

### Bước 7: chạy benchmark

```bash
cargo run -p host --bin zk_auth_availability_benchmark -- --n 10 --groth16
```

### Bước 8: so sánh kết quả

```bash
cargo run -p host --bin zk_auth_compare
```

## 12. Xử lý lỗi thường gặp

### Lỗi thiếu gas hoặc transaction không lên chain

Kiểm tra ví:

```bash
cast wallet address --private-key "$PRIVATE_KEY"
cast balance "$(cast wallet address --private-key "$PRIVATE_KEY")" --rpc-url "$SEPOLIA_RPC_URL"
```

### Lỗi contract reject proof

Các nguyên nhân phổ biến:

- quên `--groth16`
- `ZK_AUTH_CONTRACT_ADDRESS` đang trỏ sai contract
- proof tạo cho `chain_id` hoặc `contract_address` khác
- nullifier bị reuse

### Lỗi benchmark ZK chạy quá lâu

Nguyên nhân:

- mỗi lần chạy có proof generation
- khi dùng `--groth16` còn có thêm bước compress sang proof cho EVM

Cách giảm tải:

```bash
cargo run -p host --bin zk_auth_availability_benchmark -- --n 3 --mode zk --groth16
```

### Lỗi không đọc được `.env` trong shell hiện tại

Chạy lại block nạp biến global ở mục 2.

## 13. File đầu ra chính

Các file quan trọng của demo:

- `contracts/ZkAuthDemo.sol`
- `contracts/DeployZkAuthDemo.s.sol`
- `methods/zk-auth-guest/src/main.rs`
- `host/src/zk_auth.rs`
- `host/src/bin/zk_auth_traditional_demo.rs`
- `host/src/bin/zk_auth_demo.rs`
- `host/src/bin/zk_auth_integrity_cases.rs`
- `host/src/bin/zk_auth_availability_benchmark.rs`
- `host/src/bin/zk_auth_compare.rs`
- `README_ZK_AUTH_DEMO.md`

Các thư mục output:

- `results/zk-auth/`
- `results/zk-auth/artifacts/`
