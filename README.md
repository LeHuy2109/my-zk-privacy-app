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

## Vấn đề tồn đọng và hạn chế

1. **Liên kết số tiền**: Số tiền gửi và rút là công khai, cho phép theo dõi giao dịch qua khớp chính xác.
2. **Liên kết người trả gas**: Việc rút yêu cầu ví có ETH cho phí gas, có thể tiết lộ danh tính qua thanh toán gas.

Để đạt được tính ẩn danh đầy đủ, triển khai pool mệnh giá cố định và mạng relayer cho trừu tượng hóa gas.
