# Giao thức Bảo mật ZK (Mạng thử nghiệm Sepolia)

Dự án này là một hệ thống giao dịch ẩn danh trên blockchain Sepolia - Ethereum Testnet, sử dụng Zero-Knowledge Virtual Machine (zkVM) của RISC Zero.

Dự án đã triển khai thành công mô hình mật mã cốt lõi của bằng chứng Zero-Knowledge (ZK), cho phép người dùng gửi và rút tiền dựa trên bằng chứng toán học thay vì khóa riêng tư.

## Cài đặt

Để thiết lập môi trường, cần cài đặt các công cụ sau:

1. **Rust**: Cài đặt Rust từ [rustup.rs](https://rustup.rs). Đảm bảo bạn có phiên bản ổn định mới nhất.
2. **Foundry**: Cài đặt công cụ Foundry (forge, cast) từ [getfoundry.sh](https://getfoundry.sh).
3. **Docker**: Cài đặt Docker từ [docker.com](https://www.docker.com). Điều này cần thiết để nén bằng chứng ZK với Groth16.

Sau khi cài đặt, clone repository và cấu hình môi trường:

```bash
git clone <repository-url>
cd my-zk-privacy-app
cp .env.example .env
```

Chỉnh sửa `.env` để bao gồm `PRIVATE_KEY` của người dùng (ví có ETH Sepolia cho phí gas).

## Thành phần

### Guest
Guest là mạch ZK được triển khai bằng Rust, chạy bên trong zkVM của RISC Zero. Nó xác minh bao gồm Merkle tree và tính toán nullifier mà không tiết lộ bí mật.

### Host
Host là ứng dụng Rust xây dựng đầu vào, tạo bằng chứng ZK, và tương tác với blockchain. Nó bao gồm các module cho thực thi, chứng minh, tương tác chuỗi, và hiển thị.

### Smart Contract
Smart contract viết bằng Solidity, xử lý gửi và rút tiền với xác minh ZK. Nó duy trì Merkle tree cho các cam kết và xác minh bằng chứng sử dụng trình xác minh của RISC Zero.

## Luồng hoạt động

Hệ thống hiện tại hoạt động như sau:

1. **Gửi tiền**: Người dùng tạo `secret` và `amount` ngẫu nhiên, hash chúng thành `commitment`, và gửi ETH đến smart contract, smart contract thêm commitment vào Merkle tree.
2. **Chứng minh ZK**: Để rút tiền, người dùng cung cấp `secret`, `amount`, và địa chỉ người nhận. zkVM xác minh bao gồm commitment trong Merkle tree và tạo bằng chứng ZK và nullifier.
3. **Rút tiền**: Người dùng gửi bằng chứng và nullifier đến smart contract, smart contract xác minh và chuyển tiền nếu hợp lệ.

## Cách sử dụng

## Các biến được dùng:
- amount: Số tiền giao dịch, tính bằng đơn vị Wei, 1 ETH tương ứng 1000000000000000000 Wei (10^18)
- secret: mã hex ngẫu nhiên 32 bytes
- recipient-address: địa chỉ ví người nhận
### Gửi tiền
Chạy lệnh sau để tính commitment từ secret ngẫu nhiên.

```bash
cd host
cargo run -- --deposit --amount "amount" --secret "secret"
```

Lệnh này xuất ra lệnh `cast send`. Copy và chạy để gửi tiền.

Chờ 15-30 giây để xác nhận trên Sepolia.

### Rút tiền
Để rút tiền ẩn danh, ta có thể thực hiện theo hai bước riêng biệt:

#### Bước 1: Tạo bằng chứng
```bash
cargo run -- --generate-proof --chain --amount <amount> --secret <secret> --recipient <recipient-address> --groth16 --output proof.json
```

Lệnh này quét blockchain, xây dựng lại Merkle tree, tạo bằng chứng Groth16 (yêu cầu Docker và RAM đáng kể), và lưu proof vào file `proof.json`.

Lưu ý: Nén Groth16 yêu cầu ít nhất 16GB RAM và mất 5-10 phút.

#### Bước 2: Gửi bằng chứng
```bash
cargo run -- --submit-proof --chain --proof proof.json
```

Lệnh này đọc proof từ file `proof.json`, verify locally, và gửi giao dịch rút lên Sepolia.

#### Lệnh gộp 2 bước trên
Nếu không quan tâm đến bằng chứng, có thể chạy lệnh sau để gộp 2 lệnh trên:

```bash
cargo run -- --chain --amount <amount> --secret <secret> --recipient <recipient-address> --groth16
```


## Phần này chạy cũng được không chạy cũng không có vấn đề gì.
## CLI chuyên nghiệp `zkprivacy`

Dự án có binary CLI mới `zkprivacy` để người dùng thao tác toàn bộ flow privacy bằng terminal, hạn chế copy/paste lệnh `cast send` thủ công.

### Cấu trúc CLI mới

Phần CLI được tách thành các module trong `host/src`:

```text
zkprivacy_cli.rs        # clap command/argument definitions
zkprivacy_commands.rs   # orchestration và UX từng bước
zkprivacy_config.rs     # đọc/ghi .env
zkprivacy_notes.rs      # local note store
zkprivacy_chain.rs      # deposit, balance, nullifier check
zkprivacy_utils.rs      # validate amount/address/secret
```

Logic ZK/crypto hiện tại vẫn được giữ lại và tái sử dụng từ `executor.rs`, `prover.rs`, `chain.rs`, `types.rs`. Smart contract không cần đổi ABI.

### Chạy CLI

Trong môi trường dev, chạy qua Cargo:

```bash
cargo run --bin zkprivacy -- <command>
```

Nếu muốn gọi trực tiếp `zkprivacy`, build/cài binary:

```bash
cargo install --path host --bin zkprivacy
zkprivacy --help
```

### Command chính

```bash
zkprivacy init
zkprivacy config show
zkprivacy config set --rpc-url <url> --private-key <key> --contract <address> --deploy-block <block>
zkprivacy deposit --amount 0.01eth
zkprivacy deposit --amount 10000000000000000wei --secret <secret-32-byte-hex>
zkprivacy notes list
zkprivacy notes show <note-id>
zkprivacy notes show <note-id> --show-secret
zkprivacy notes export <note-id> --output note.json
zkprivacy notes import note.json
zkprivacy prove --note <note-id> --recipient <address> --output proof.json --groth16
zkprivacy prove --amount <amount> --secret <secret> --recipient <address> --output proof.json
zkprivacy withdraw --proof proof.json
zkprivacy withdraw --note <note-id> --recipient <address> --groth16
zkprivacy status
zkprivacy balance
zkprivacy nullifier check <nullifier>
```

Các command quan trọng hỗ trợ global flags:

```bash
--dry-run
--verbose
--json
```

### Ví dụ end-to-end

```bash
cd host

cargo run --bin zkprivacy -- init

cargo run --bin zkprivacy -- config set \
  --rpc-url <sepolia-rpc-url> \
  --private-key <private-key> \
  --contract <privacy-contract-address> \
  --deploy-block <deploy-block>

cargo run --bin zkprivacy -- deposit --amount 0.01eth

cargo run --bin zkprivacy -- notes list

cargo run --bin zkprivacy -- prove \
  --note <note-id> \
  --recipient <recipient-address> \
  --output proof.json \
  --groth16

cargo run --bin zkprivacy -- withdraw --proof proof.json
```

Hoặc prove và withdraw gộp một bước:

```bash
cargo run --bin zkprivacy -- withdraw \
  --note <note-id> \
  --recipient <recipient-address> \
  --groth16
```

### Bảo mật note

Mỗi deposit lưu một note cục bộ trong `.zkprivacy-notes.json`, gồm `amount`, `secret`, `commitment`, `tx_hash`, `timestamp`, `network`. File này hiện lưu secret plaintext để ưu tiên end-to-end CLI trước.

Cảnh báo:

- Không commit `.zkprivacy-notes.json`.
- Không chia sẻ note/secret.
- Backup note an toàn; mất note là mất khả năng withdraw.
- TODO tiếp theo: mã hóa note store bằng passphrase hoặc OS keychain.



########################################################
########################################################
```
Đây là cái tao thêm mới nhé 
methods/zk-auth-guest
host/( các file có zkprivacy)
contracts/ZkAuthDemo.sol ; DeployZkAuthDemo.s.sol
script(theo đúng miêu tả)
```

###########################################################
###########################################################
##
## ZK-auth demo độc lập

Repo cũng có thêm demo `zk-auth` chạy song song với luồng deposit/withdraw hiện tại. Demo này mô phỏng lớp xác thực nghiệp vụ bằng ZK proof: Ethereum transaction vẫn cần ví hoặc relayer ký để trả gas, nhưng contract demo chỉ chấp nhận hành động ZK khi proof hợp lệ.

### Build image ID cho demo

```bash
cargo run --bin zk_auth_demo -- image-id
```

Khi deploy contract demo, truyền image ID này qua biến môi trường `ZK_AUTH_IMAGE_ID`.

### Tạo proof ZK-auth local

```bash
cargo run --bin zk_auth_demo -- generate \
  --payload "hello zk auth" \
  --secret <secret-32-byte-hex> \
  --recipient <recipient-address> \
  --chain-id 11155111 \
  --contract <zk-auth-contract-address> \
  --output zk-auth-demo/results/proof.json
```

Lệnh này tạo artifact local gồm `payload_hash`, `journal_hash`, `proof_hash`, kích thước journal/seal và thời gian tạo proof.

### Verify end-to-end và integrity cases

```bash
cargo run --bin zk_auth_demo -- verify-e2e --proof zk-auth-demo/results/proof.json
cargo run --bin zk_auth_demo -- integrity-cases --proof zk-auth-demo/results/proof.json
```

### Traditional baseline

```bash
cargo run --bin zk_auth_demo -- traditional \
  --payload "hello zk auth" \
  --recipient <recipient-address>
```

Thêm `--chain` để gửi lên contract `ZkAuthDemo` đã deploy, sử dụng `ZK_AUTH_CONTRACT_ADDRESS` hoặc `CONTRACT_ADDRESS` trong `.env`.

### Submit ZK proof lên chain

```bash
cargo run --bin zk_auth_demo -- submit-zk \
  --proof zk-auth-demo/results/proof.json \
  --chain
```

Để submit on-chain, proof nên được tạo với `--groth16` vì verifier router Ethereum cần seal EVM-compatible.

### Kịch bản đánh giá giống project PQC

Demo ZK-auth dùng kịch bản chính: **ZK-STARK Authentication Layer for Blockchain Record**.

```text
payload -> payloadHash -> secret/identityCommitment -> ZK proof -> smart contract verify -> store record
```

ECDSA vẫn tồn tại ở tầng gửi transaction Ethereum, nhưng không còn là cơ chế xác thực nghiệp vụ. Contract chỉ chấp nhận ghi record khi proof hợp lệ.

Năm kịch bản so sánh:

| Kịch bản | Mục tiêu | Script |
|---|---|---|
| Traditional ECDSA baseline | Mốc gas/latency thấp nhất khi chỉ ghi `payloadHash` | `script/traditional_demo.py` |
| ZK-STARK auth | Tạo proof và ghi record bằng `storeRecordWithProof` | `script/zk_demo.py` |
| ZK + off-chain proof storage | Lưu artifact/journal/proof ngoài chuỗi, on-chain lưu hash/CID | `script/zk_demo.py`, `script/verify_e2e.py` |
| Integrity test | Tamper payload/journal/proof, replay nullifier, sai chain/contract | `script/integrity_cases.py` |
| Availability benchmark | Chạy nhiều record để đo success rate/latency/throughput | `script/availability_benchmark.py` |

Bộ metric chính để đưa vào báo cáo:

```text
gas_used
proof_generation_seconds
proof_verify_seconds
seal_size_bytes
journal_size_bytes
raw_tx_size_bytes
calldata_size_bytes
send_and_confirm_seconds
total_latency_seconds
success_rate_percent
tamper_detection_rate
replay_rejection_rate
```

### Bộ script giống project PQC

Các script trong `script/` là wrapper mỏng gọi Rust binary `zk_auth_demo`, để giữ tên file giống project PQC nhưng không nhân đôi logic:

| Script | Rust command tương ứng |
|---|---|
| `script/traditional_demo.py` | `cargo run --bin zk_auth_demo -- traditional` |
| `script/zk_demo.py` | `cargo run --bin zk_auth_demo -- generate` |
| `script/verify_e2e.py` | `cargo run --bin zk_auth_demo -- verify-e2e` |
| `script/integrity_cases.py` | `cargo run --bin zk_auth_demo -- integrity-cases` |
| `script/availability_benchmark.py` | `cargo run --bin zk_auth_demo -- availability-benchmark` |
| `script/compare.py` | `cargo run --bin zk_auth_demo -- compare` |

Ví dụ:

```bash
python script/zk_demo.py \
  --payload "hello zk auth" \
  --secret <secret-32-byte-hex> \
  --recipient <recipient-address> \
  --chain-id 11155111 \
  --contract <zk-auth-contract-address> \
  --output zk-auth-demo/results/proof.json

python script/verify_e2e.py --proof zk-auth-demo/results/proof.json
python script/integrity_cases.py --proof zk-auth-demo/results/proof.json
```

### Benchmark tiêu chí đánh giá

Sau khi build/demo chạy được, dùng lệnh sau để xuất benchmark các tiêu chí đánh giá:

```bash
python script/availability_benchmark.py \
  --count 100 \
  --mode local \
  --output zk-auth-demo/results/benchmark.json
```

Các tiêu chí được ghi nhận/đề xuất gồm:

```text
payload_count
payload_hash_latency_ms
payload_hash_throughput
payload_hash_size
proof_generation_seconds
journal_size_bytes
seal_size_bytes
tx_build_seconds
send_and_confirm_seconds
gas_used
success_rate
```

Có thể so sánh nhiều artifact/report bằng:

```bash
cargo run --bin zk_auth_demo -- compare \
  --input zk-auth-demo/results/proof.json \
  --input zk-auth-demo/results/benchmark.json
```

## Vấn đề tồn đọng và hạn chế

1. **Liên kết số tiền**: Số tiền gửi và rút là công khai, cho phép theo dõi giao dịch qua khớp chính xác.
2. **Liên kết người trả gas**: Việc rút yêu cầu ví có ETH cho phí gas, có thể tiết lộ danh tính qua thanh toán gas.

Để đạt được tính ẩn danh đầy đủ, triển khai pool mệnh giá cố định và mạng relayer cho trừu tượng hóa gas.
