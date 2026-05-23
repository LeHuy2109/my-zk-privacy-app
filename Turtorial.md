# Tutorial chạy relayer local tạm thời

File này hướng dẫn test flow privacy với relayer chạy local. Mô hình test dùng 2 terminal:

```text
Terminal 1: chạy relayer local, giữ RELAYER_PRIVATE_KEY
Terminal 2: chạy user/client, tạo deposit, proof và withdraw qua relayer
```

Relayer local sẽ chạy tại:

```text
http://127.0.0.1:8787/withdraw
```

## 1. Kiểm tra compile

Chạy từ root repo:

```bash
cargo check --manifest-path host/Cargo.toml --bin zkprivacy
cargo check --manifest-path host/Cargo.toml --bin zkprivacy_relayer
forge build --via-ir
```

## 2. Kiểm tra ví relayer

Contract `PrivacyVerifierFixedGasPayer` đang hard-code relayer:

```text
0x268b1445F3BC85b73e1812a617712F6F11Eb6F5B
```

Kiểm tra private key relayer:

```bash
cast wallet address --private-key <RELAYER_PRIVATE_KEY>
```

Output phải đúng là:

```text
0x268b1445F3BC85b73e1812a617712F6F11Eb6F5B
```

Nếu khác địa chỉ này thì `withdraw` qua relayer sẽ bị contract reject.

## 3. Deploy contract relayer version

Deploy contract mới, không dùng contract cũ:

```bash
forge script contracts/DeployFixedGasPayer.s.sol:DeployFixedGasPayerScript \
  --rpc-url <SEPOLIA_RPC_URL> \
  --broadcast \
  --via-ir
```

Sau khi deploy, ghi lại:

```text
CONTRACT_ADDRESS=<dia-chi-contract-moi>
DEPLOY_BLOCK=<block-deploy-contract>
```

Contract cần dùng là `PrivacyVerifierFixedGasPayer`.

## 4. Cấu hình `.env`

Tạo hoặc sửa file `.env` ở root repo:

```env
SEPOLIA_RPC_URL=<sepolia-rpc-url>
PRIVATE_KEY=<private-key-cua-vi-deposit-test>
CONTRACT_ADDRESS=<dia-chi-contract-moi>
DEPLOY_BLOCK=<block-deploy-contract>
RELAYER_URL=http://127.0.0.1:8787/withdraw
```

Trong test local, có thể thêm dòng này vào `.env` để Terminal 1 dùng:

```env
RELAYER_PRIVATE_KEY=<private-key-cua-vi-0x268b...6F5B>
```

Lưu ý:

- `PRIVATE_KEY` dùng cho client deposit.
- `RELAYER_PRIVATE_KEY` dùng cho relayer submit withdraw.
- Khi chạy thật, không đưa `RELAYER_PRIVATE_KEY` cho user.

## 5. Terminal 1: chạy relayer local

Mở terminal thứ nhất, chạy:

```bash
cargo run --manifest-path host/Cargo.toml --bin zkprivacy_relayer -- --bind 127.0.0.1:8787
```

Nếu chạy đúng sẽ thấy:

```text
zkprivacy relayer listening on http://127.0.0.1:8787/withdraw
```

Giữ terminal này chạy, không tắt.

Nếu không muốn để `RELAYER_PRIVATE_KEY` trong `.env`, có thể export trực tiếp:

```bash
export SEPOLIA_RPC_URL=<sepolia-rpc-url>
export CONTRACT_ADDRESS=<dia-chi-contract-moi>
export RELAYER_PRIVATE_KEY=<private-key-cua-vi-0x268b...6F5B>

cargo run --manifest-path host/Cargo.toml --bin zkprivacy_relayer -- --bind 127.0.0.1:8787
```

## 6. Terminal 2: cấu hình client

Mở terminal thứ hai, chạy:

```bash
cargo run --manifest-path host/Cargo.toml --bin zkprivacy -- config set \
  --rpc-url <sepolia-rpc-url> \
  --private-key <private-key-cua-vi-deposit-test> \
  --contract <dia-chi-contract-moi> \
  --deploy-block <block-deploy-contract> \
  --relayer-url http://127.0.0.1:8787/withdraw
```

Kiểm tra config:

```bash
cargo run --manifest-path host/Cargo.toml --bin zkprivacy -- config show
```

## 7. Deposit

Deposit vào pool:

```bash
cargo run --manifest-path host/Cargo.toml --bin zkprivacy -- deposit --amount 0.01eth
```

Sau khi deposit thành công, xem note:

```bash
cargo run --manifest-path host/Cargo.toml --bin zkprivacy -- notes list
```

Ghi lại `note-id`.

Ví dụ:

```text
note-1
```

## 8. Tạo proof withdraw

Tạo proof Groth16 để submit on-chain:

```bash
cargo run --manifest-path host/Cargo.toml --bin zkprivacy -- prove \
  --note <note-id> \
  --recipient <recipient-address> \
  --output proof.json \
  --groth16
```

Ví dụ:

```bash
cargo run --manifest-path host/Cargo.toml --bin zkprivacy -- prove \
  --note note-1 \
  --recipient 0xf8329687322ADC276eDEA5cC25a6959Da1f5Dd7a \
  --output proof.json \
  --groth16
```

Lưu ý: `--groth16` cần Docker và có thể mất vài phút.

## 9. Withdraw qua relayer local

Gửi proof tới relayer local:

```bash
cargo run --manifest-path host/Cargo.toml --bin zkprivacy -- withdraw \
  --proof proof.json \
  --relayer
```

Kết quả mong đợi:

```text
Withdraw tx: 0x...
Gas used   : ...
Explorer   : https://sepolia.etherscan.io/tx/0x...
```

Trên Terminal 1, relayer sẽ nhận request và submit transaction.

Trên Etherscan:

- `from` của transaction withdraw là ví relayer `0x268b1445F3BC85b73e1812a617712F6F11Eb6F5B`.
- `recipient` nhận ETH từ contract.
- User/client không cần dùng private key relayer để submit withdraw.

## 10. Flow gộp prove và withdraw

Sau khi đã deposit và có note, có thể chạy gộp:

```bash
cargo run --manifest-path host/Cargo.toml --bin zkprivacy -- withdraw \
  --note <note-id> \
  --recipient <recipient-address> \
  --groth16 \
  --relayer
```

Ví dụ:

```bash
cargo run --manifest-path host/Cargo.toml --bin zkprivacy -- withdraw \
  --note note-1 \
  --recipient 0xf8329687322ADC276eDEA5cC25a6959Da1f5Dd7a \
  --groth16 \
  --relayer
```

## 11. Test lỗi double withdraw

Sau khi withdraw thành công, chạy lại cùng proof:

```bash
cargo run --manifest-path host/Cargo.toml --bin zkprivacy -- withdraw \
  --proof proof.json \
  --relayer
```

Kết quả mong đợi là fail:

```text
Nullifier already used
```

Đây là đúng, vì contract không cho rút hai lần cùng một note.

## 12. Checklist debug

Nếu `withdraw` fail, kiểm tra theo thứ tự:

```bash
cast wallet address --private-key <RELAYER_PRIVATE_KEY>
```

Phải ra:

```text
0x268b1445F3BC85b73e1812a617712F6F11Eb6F5B
```

Kiểm tra contract address:

```bash
cargo run --manifest-path host/Cargo.toml --bin zkprivacy -- config show
```

Kiểm tra relayer còn chạy:

```bash
curl http://127.0.0.1:8787/withdraw
```

Request `GET` có thể bị báo unsupported route, nhưng nếu có response từ relayer nghĩa là server còn sống.

Kiểm tra ví relayer có ETH Sepolia:

```bash
cast balance 0x268b1445F3BC85b73e1812a617712F6F11Eb6F5B \
  --rpc-url <SEPOLIA_RPC_URL>
```

Nếu balance bằng 0, relayer không có gas để submit withdraw.

