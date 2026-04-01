# ZK Privacy Transaction – RISC Zero zkVM

Chương trình Guest chạy trên RISC Zero zkVM để **ẩn thông tin giao dịch blockchain**:
- 🔒 Địa chỉ người gửi (`sender_address`)
- 🔒 Địa chỉ người nhận (`receiver_address`)
- 🔒 Số tiền giao dịch (`amount`)

Guest chứng minh giao dịch hợp lệ (`amount > 0` và `balance >= amount`) và commit
các **commitment hash** ra journal công khai — blockchain xác minh proof mà không
biết dữ liệu thật sự.

---

## Yêu cầu môi trường

| Công cụ | Mô tả |
|---|---|
| Rust + Cargo | Ngôn ngữ lập trình chính |
| rzup | RISC0 toolchain manager (cài RISC-V cross-compiler) |
| WSL (trên Windows) | Môi trường Linux để chạy rzup |

> ⚠️ **Windows:** RISC0 toolchain (`rzup`) **không hỗ trợ Git Bash / PowerShell thuần**. Cần dùng WSL (Windows Subsystem for Linux).

---

## Cài đặt môi trường trên Windows (WSL)

### Bước 1 – Cài WSL

Mở **PowerShell với quyền Administrator** và chạy:

```powershell
wsl --install
```

Khởi động lại máy sau khi cài xong. Windows sẽ tự cài **Ubuntu** làm distro mặc định.

> Sau khi khởi động lại, mở app **Ubuntu** từ Start Menu để vào môi trường WSL.

---

### Bước 2 – Cài GCC và Rust trong WSL

Trong terminal **Ubuntu WSL**:

```bash
# Cài GCC / C linker (bắt buộc, Rust cần cc để link)
sudo apt update
sudo apt install -y build-essential

# Cài Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

Kiểm tra:
```bash
gcc --version
rustc --version
cargo --version
```

> ⚠️ **Bỏ qua bước `build-essential` sẽ gây lỗi `linker 'cc' not found`** khi `cargo build`.

---

### Bước 3 – Cài RISC0 Toolchain

```bash
# Cài rzup
curl -L https://risczero.com/install | bash
source "$HOME/.bashrc"

# Cài RISC-V cross-compiler (lần đầu mất ~5-10 phút)
rzup install
```

Kiểm tra:
```bash
rzup --version
```

---

### Bước 4 – Clone / truy cập dự án

Nếu dự án nằm trên ổ C: của Windows, WSL có thể truy cập qua `/mnt/c/`:

```bash
cd /mnt/c/Users/Admin/PTIT/MMHCS/RISC0/my-zk-privacy-app
```

Hoặc clone mới trong WSL:
```bash
git clone <repo-url>
cd my-zk-privacy-app
```

---

## Quick Start

First, make sure [rustup] is installed. The
[`rust-toolchain.toml`][rust-toolchain] file will be used by `cargo` to
automatically install the correct version.

To build all methods and execute the method within the zkVM, run the following
command:

```bash
cargo run
```

### Tuỳ chỉnh Giao dịch và Output (Custom CLI Arguments)

Ứng dụng hỗ trợ cấu hình động qua giao diện dòng lệnh (CLI) để mô phỏng thực tế các kịch bản gửi/nhận khác nhau. Dưới đây là danh sách tham số bạn có thể truyền vào:

- `--amount <SỐ>`: Định mức số lượng token cần chuyển (mặc định: `500`).
- `--balance <SỐ>`: Số dư hiện tại thực tế của người gửi để xác thực (mặc định: `1000`).
- `--sender <HEX_ADDRESS>`: Địa chỉ ví người gửi. **Bắt buộc** là chuỗi Hexadecimal dài chuẩn 20-byte (Ví dụ định dạng ví EVM có hoặc không có `0x`).
- `--receiver <HEX_ADDRESS>`: Địa chỉ ví người nhận. **Bắt buộc** là chuỗi Hexadecimal dài chuẩn 20-byte.
- `--chain`: Thêm cờ này để kích hoạt luồng Submit Proof lên Sepolia Testnet (yêu cầu cấu hình sẵn file `.env`).
- `--groth16`: Nén STARK thành Groth16 SNARK local (không cần Bonsai). Yêu cầu RAM ~16GB+ và tải Proving Key lần đầu. Mặc định: tắt.
- `--json`: Xuất toàn bộ kết quả Proof Process dưới định dạng JSON thô thay vì hiển thị giao diện bảng biểu Terminal UI (rất hữu ích khi tích hợp script hoặc backend).

**Ví dụ lệnh chạy hoàn chỉnh:**
```bash
cargo run -- --amount 250 --balance 800 --sender 0x1234567890abcdef1234567890abcdef12345678 --receiver 0xabcdefabcdefabcdefabcdefabcdefabcdefabcd
```
*(Ghi chú: Nếu chạy `cargo run` trơn không kèm cờ, hệ thống tự động sinh dữ liệu ảo (Demo) để phục vụ test nhanh).*

### Executing the Project Locally in Development Mode

During development, faster iteration upon code changes can be achieved by leveraging [dev-mode], we strongly suggest activating it during your early development phase. Furthermore, you might want to get insights into the execution statistics of your project, and this can be achieved by specifying the environment variable `RUST_LOG="[executor]=info"` before running your project.

Put together, the command to run your project in development mode while getting execution statistics is:

```bash
RUST_LOG="[executor]=info" RISC0_DEV_MODE=1 cargo run
```

### Running Proofs Remotely on Bonsai

_Note: The Bonsai proving service is still in early Alpha; an API key is
required for access. [Click here to request access][bonsai access]._

If you have access to the URL and API key to Bonsai you can run your proofs
remotely. To prove in Bonsai mode, invoke `cargo run` with two additional
environment variables:

```bash
BONSAI_API_KEY="YOUR_API_KEY" BONSAI_API_URL="BONSAI_URL" cargo run
```

## How to Create a Project Based on This Template

Search this template for the string `TODO`, and make the necessary changes to
implement the required feature described by the `TODO` comment. Some of these
changes will be complex, and so we have a number of instructional resources to
assist you in learning how to write your own code for the RISC Zero zkVM:

- The [RISC Zero Developer Docs][dev-docs] is a great place to get started.
- Example projects are available in the [examples folder][examples] of
  [`risc0`][risc0-repo] repository.
- Reference documentation is available at [https://docs.rs][docs.rs], including
  [`risc0-zkvm`][risc0-zkvm], [`cargo-risczero`][cargo-risczero],
  [`risc0-build`][risc0-build], and [others][crates].

## Directory Structure

It is possible to organize the files for these components in various ways.
However, in this starter template we use a standard directory structure for zkVM
applications, which we think is a good starting point for your applications.

```text
project_name
├── Cargo.toml
├── host
│   ├── Cargo.toml
│   └── src
│       └── main.rs                    <-- [Host code goes here]
└── methods
    ├── Cargo.toml
    ├── build.rs
    ├── guest
    │   ├── Cargo.toml
    │   └── src
    │       └── method_name.rs         <-- [Guest code goes here]
    └── src
        └── lib.rs
```

## Video Tutorial

For a walk-through of how to build with this template, check out this [excerpt
from our workshop at ZK HACK III][zkhack-iii].

## Questions, Feedback, and Collaborations

We'd love to hear from you on [Discord][discord] or [Twitter][twitter].

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
[rust-toolchain]: rust-toolchain.toml
[rustup]: https://rustup.rs
[twitter]: https://twitter.com/risczero
[zkhack-iii]: https://www.youtube.com/watch?v=Yg_BGqj_6lg&list=PLcPzhUaCxlCgig7ofeARMPwQ8vbuD6hC5&index=5
[zkvm-overview]: https://dev.risczero.com/zkvm
