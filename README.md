# ZK Privacy Transaction – RISC Zero zkVM

Chương trình chạy trên **RISC Zero zkVM** để thực hiện **giao dịch riêng tư kiểu Tornado Cash**:
- 🔒 Secret của người gửi không bao giờ lộ ra blockchain
- 🌳 Chứng minh note hợp lệ tồn tại trong **Merkle tree** on-chain
- 🚫 **Nullifier** chống double-spend
- ✅ Blockchain xác minh proof mà không biết nội dung giao dịch

---

## Kiến trúc ZK

```
DEPOSIT (công khai)
  user tạo: secret (lưu bí mật)
            commitment = SHA256(secret ∥ amount_le_bytes)
  → gọi contract.deposit(commitment) {value: amount ETH}
  → contract thêm commitment vào Merkle tree, cập nhật root

WITHDRAW (ẩn danh – dùng ZK proof)
  Host:  lấy merkle_root + merkle_path từ on-chain [*]
         gửi (secret, amount, path) vào Guest (private)

  Guest: tính leaf = SHA256(secret ∥ amount)
         verify: walk_up(leaf, path) == merkle_root  ← XÁC MINH INCLUSION
         tính:   nullifier = SHA256(secret ∥ "nullify")
         commit public: { merkle_root, nullifier_hash, recipient, amount }

  Contract: verify RISC0 proof
            kiểm tra merkle_root khớp on-chain
            kiểm tra nullifier chưa dùng → ghi lại
            chuyển ETH cho recipient
```

> `[*]` Phiên bản hiện tại dùng **Merkle tree offline** (demo). Xem [Roadmap on-chain](#roadmap-tích-hợp-smart-contract).

---

## Input / Output

### Private Input (chỉ Guest biết, không bao giờ lộ ra)

| Trường | Kiểu | Mô tả |
|---|---|---|
| `secret` | `[u8; 32]` | Bí mật 32-byte của note (user tự giữ) |
| `amount` | `u64` | Số tiền của note |
| `merkle_path` | `Vec<[u8;32]>` | Sibling hashes từ leaf lên root |
| `merkle_indices` | `Vec<bool>` | Hướng tại mỗi tầng (false=trái, true=phải) |
| `merkle_root` | `[u8; 32]` | Root hiện tại của smart contract |
| `recipient` | `[u8; 20]` | Địa chỉ người nhận tiền |

### Public Output – Journal (ai cũng đọc được)

| Trường | Kiểu | Ý nghĩa |
|---|---|---|
| `merkle_root` | `[u8; 32]` | Phải khớp on-chain root |
| `nullifier_hash` | `[u8; 32]` | `SHA256(secret ∥ "nullify")` – chống double-spend |
| `recipient` | `[u8; 20]` | Người nhận tiền |
| `amount` | `u64` | Số tiền withdraw |
| `is_valid` | `bool` | Proof hợp lệ không |

---

## Yêu cầu môi trường

| Công cụ | Mô tả |
|---|---|
| Rust + Cargo | Ngôn ngữ lập trình chính |
| rzup | RISC0 toolchain manager (cài RISC-V cross-compiler) |
| WSL (trên Windows) | Môi trường Linux để chạy rzup |

> ⚠️ **Windows:** RISC0 toolchain (`rzup`) **không hỗ trợ Git Bash / PowerShell thuần**. Cần dùng WSL.

---

## Cài đặt môi trường (WSL)

### Bước 1 – Cài WSL

```powershell
wsl --install
```

Khởi động lại máy. Sau đó mở **Ubuntu** từ Start Menu.

### Bước 2 – Cài GCC và Rust trong WSL

```bash
sudo apt update && sudo apt install -y build-essential
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

> ⚠️ Bỏ qua `build-essential` sẽ gây lỗi `linker 'cc' not found`.

### Bước 3 – Cài RISC0 Toolchain

```bash
curl -L https://risczero.com/install | bash
source "$HOME/.bashrc"
rzup install   # lần đầu mất ~5-10 phút
```

### Bước 4 – Truy cập dự án

```bash
# Nếu dự án nằm trên Windows
cd /mnt/c/Users/Admin/PTIT/MMHCS/RISC0/my-zk-privacy-app

# Hoặc clone mới
git clone <repo-url> && cd my-zk-privacy-app
```

---

## Quick Start

### Chạy nhanh (Dev Mode – không cần prove thật)

```bash
RISC0_DEV_MODE=1 cargo run
```

### Chạy với tham số tuỳ chỉnh

```bash
# Chỉ định số tiền
RISC0_DEV_MODE=1 cargo run -- --amount 300

# Chỉ định người nhận
RISC0_DEV_MODE=1 cargo run -- --amount 300 --recipient 0xabcdefabcdefabcdefabcdefabcdefabcdefabcd

# Xuất JSON (dùng để tích hợp với script hoặc backend)
RISC0_DEV_MODE=1 cargo run -- --json

# Gửi proof lên Sepolia (cần cấu hình .env)
RISC0_DEV_MODE=1 cargo run -- --chain
```

### CLI Arguments

| Tham số | Mặc định | Mô tả |
|---|---|---|
| `--amount <N>` | `500` | Số tiền withdraw |
| `--recipient <HEX>` | demo address | Địa chỉ người nhận (20-byte hex, có/không có `0x`) |
| `--chain` | `false` | Gửi proof lên Sepolia Testnet |
| `--groth16` | `false` | Nén STARK → Groth16 SNARK (cần RAM ~16GB+) |
| `--json` | `false` | Xuất kết quả dạng JSON thay vì terminal UI |

### Chạy Full Proof (chậm, proof thật)

```bash
cargo run
cargo run -- --amount 300 --recipient 0x1234567890abcdef1234567890abcdef12345678
```

### Dev Mode với log chi tiết

```bash
RUST_LOG="[executor]=info" RISC0_DEV_MODE=1 cargo run
```

### Chạy trên Bonsai (remote proving)

```bash
BONSAI_API_KEY="YOUR_API_KEY" BONSAI_API_URL="BONSAI_URL" cargo run
```

---

## Cấu hình Sepolia (`.env`)

Copy `.env.example` → `.env` và điền:

```bash
SEPOLIA_RPC_URL=https://rpc.sepolia.org
PRIVATE_KEY=0x_YOUR_PRIVATE_KEY_HERE
CONTRACT_ADDRESS=0x_YOUR_CONTRACT_ADDRESS_HERE
```

---

## Cấu trúc thư mục

```text
my-zk-privacy-app/
├── Cargo.toml
├── .env.example
├── host/
│   └── src/
│       ├── main.rs        ← CLI entry point, điều phối pipeline
│       ├── executor.rs    ← Build TransactionInput + Merkle tree (offline demo)
│       ├── prover.rs      ← Chạy RISC0 prover, verify receipt
│       ├── chain.rs       ← Gửi proof lên Sepolia
│       ├── display.rs     ← In kết quả ra terminal / JSON
│       └── types.rs       ← Struct dùng chung
└── methods/
    └── guest/src/
        └── main.rs        ← ZK circuit: Merkle verify + Nullifier
```

---

## Roadmap tích hợp Smart Contract

Phiên bản hiện tại dùng **Merkle tree offline** (demo). Để tích hợp với smart contract thật trên Sepolia, cần thực hiện các bước sau:

### Những gì KHÔNG cần sửa ✅

- `methods/guest/src/main.rs` — ZK circuit hoàn chỉnh
- `host/src/types.rs` — Struct Input/Output đã khớp
- `host/src/prover.rs` — Pipeline prove/verify
- `host/src/display.rs` — Chỉ hiển thị

### Những gì CẦN sửa khi có contract thật ⚠️

**1. `TREE_DEPTH` trong `guest/main.rs` và `executor.rs`**
```
Đổi từ 4 → 20 (khớp với hằng số trong Solidity contract).
⚠️ Thay đổi này làm IMAGE_ID thay đổi hoàn toàn.
   Contract phải được deploy với IMAGE_ID mới tương ứng.
```

**2. `executor.rs` – `build_merkle_for_note()`**
```
Thay Merkle tree offline bằng:
  1. Query tất cả Deposit(bytes32 commitment) events từ contract
  2. Gọi contract.currentRoot() để lấy root chính xác
  3. Rebuild cây Merkle từ events (giống hệt contract)
  4. Tính merkle_path + merkle_indices theo index của leaf
```

**3. `chain.rs` – `submit_proof()`**
```
Thay calldata thủ công bằng ABI-encoded call:
  - Dùng alloy::sol! macro với ABI contract thật
  - Function: withdraw(bytes journal, bytes seal)
  - Thêm: deposit(bytes32 commitment) payable
  - Thêm: query_deposits() để đọc events
```

### Smart Contract cần viết (repo Solidity riêng)

```solidity
contract PrivacyPool {
    // Incremental Merkle tree (depth = TREE_DEPTH)
    bytes32 public currentRoot;
    mapping(bytes32 => bool) public nullifierUsed;

    function deposit(bytes32 commitment) external payable { ... }
    function withdraw(
        bytes calldata seal,       // RISC0 ZK proof
        bytes32 nullifierHash,
        address recipient,
        uint256 amount
    ) external { ... }
}
```

Contract cần tích hợp với **RISC0 Verifier** đã deploy sẵn trên Sepolia:
- RISC0 Verifier Sepolia: xem tại [dev.risczero.com/api/blockchain-integration/contracts/verifier](https://dev.risczero.com/api/blockchain-integration/contracts/verifier)

---

## Bảo mật ZK

| Dữ liệu | Ai biết? | Lộ ra không? |
|---|---|---|
| `secret` | Chỉ user | ❌ Không |
| `amount` | Chỉ user | ❌ Không |
| `merkle_path` | Chỉ host + guest | ❌ Không |
| `merkle_root` | **Công khai** | ✅ Journal + On-chain |
| `nullifier_hash` | **Công khai** | ✅ Journal + On-chain |
| `recipient` | **Công khai** | ✅ Journal + On-chain |
| ZK Proof (seal) | **Công khai** | ✅ On-chain |

**Kết quả**: Blockchain xác minh được *"note hợp lệ đã được chi tiêu đúng một lần"* mà không biết secret, ai gửi, hay lịch sử giao dịch.

---

## Tài liệu tham khảo

- [RISC Zero Developer Docs][dev-docs]
- [RISC Zero zkVM API][risc0-zkvm]
- [Blockchain Integration Guide][blockchain-integration]
- [Dev Mode][dev-mode]
- [Bonsai Access][bonsai access]
- [Examples][examples]

[bonsai access]: https://bonsai.xyz/apply
[cargo-risczero]: https://docs.rs/cargo-risczero
[crates]: https://github.com/risc0/risc0/blob/main/README.md#rust-binaries
[dev-docs]: https://dev.risczero.com
[dev-mode]: https://dev.risczero.com/api/generating-proofs/dev-mode
[discord]: https://discord.gg/risczero
[docs.rs]: https://docs.rs/releases/search?query=risc0
[examples]: https://github.com/risc0/risc0/tree/main/examples
[risc0-build]: https://docs.rs/risc0-build
[risc0-repo]: https://www.github.com/risc0/risc0
[risc0-zkvm]: https://docs.rs/risc0-zkvm
[blockchain-integration]: https://dev.risczero.com/api/blockchain-integration
[rust-toolchain]: rust-toolchain.toml
[rustup]: https://rustup.rs
[twitter]: https://twitter.com/risczero
[zkhack-iii]: https://www.youtube.com/watch?v=Yg_BGqj_6lg&list=PLcPzhUaCxlCgig7ofeARMPwQ8vbuD6hC5&index=5
[zkvm-overview]: https://dev.risczero.com/zkvm
