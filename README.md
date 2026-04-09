# ZK Privacy Protocol (Sepolia Testnet)

Dự án này là một phiên bản Proof-of-Concept (Bản thử nghiệm khái niệm) cho hệ thống **Giao dịch ẩn danh (Anonymous Transactions)** trên blockchain Ethereum (Sepolia Testnet), sử dụng **Máy ảo Zero-Knowledge (zkVM)** của **RISC Zero**.

Hiện tại, dự án đã triển khai thành công mô hình lõi của Mật mã ZK (Zero-Knowledge Cryptography) cho phép người dùng Nạp tiền và Rút tiền dựa trên Bằng chứng Toán học thay vì Private Key. Tuy nhiên, để đạt được **Tính riêng tư hoàn hảo (True Anonymity)** như Tornado Cash, dự án vẫn cần hoàn thiện thêm ở các giai đoạn sau.

---

## 1. Luồng hoạt động HIỆN TẠI (The Current Flow)

Mô hình hiện tại đang vận hành như sau:

*   **Bước 1: Deposit (Nạp tiền):**
    *   Người gửi tự tạo một chuỗi văn bản ngẫu nhiên làm `secret` và quyết định số tiền `amount`.
    *   Ở dưới máy cá nhân (Local), người gửi băm hai giá trị này lại thành `commitment`: `Hash(secret, amount)`.
    *   Người gửi thực hiện giao dịch nạp ETH lên Smart Contract kèm theo `commitment`. Contract đưa `commitment` này vào một cấu trúc dữ liệu tên là **Cây Merkle (Incremental Merkle Tree)**.
*   **Bước 2: ZK Proving (Tạo bằng chứng):**
    *   Khi muốn rút tiền, người gửi cung cấp `secret`, `amount`, và `địa chỉ người nhận`.
    *   zkVM của RISC Zero sẽ chạy đoạn code Rust bí mật. Nó kiểm tra xem `secret` và `amount` này có thực sự tạo ra một `commitment` nằm trong Cây Merkle của Smart Contract hay không.
    *   Nếu đúng, nó tạo ra một **ZK Proof (Groth16)** và một chuỗi `nullifier` (để đánh dấu tờ tiền này đã xài). Xuyên suốt quá trình này, `secret` không hề bị rò rỉ ra ngoài!
*   **Bước 3: Withdraw (Rút tiền):**
    *   Người dùng (hoặc người nhận) gọi hàm `withdraw()` trên Smart Contract, nộp ZK Proof và `nullifier` lên.
    *   Smart Contract kiểm tra tính hợp lệ của Proof. Nếu đúng, nó chuyển tiền cho `địa chỉ người nhận` và đánh dấu `nullifier` là đã dùng.

**❌ Lỗ hổng bảo mật hiện tại khiến giao dịch CHƯA ẨN DANH:**
1.  **Lộ số tiền (Amount Linking):** Mọi số tiền nạp và rút đều công khai (ví dụ: Nạp chính xác `1.458 ETH` và Rút `1.458 ETH`). Kẻ do thám (Chainalysis) chỉ cần nhìn lướt qua Etherscan là biết ngay giao dịch Rút đó từ đâu mà ra, qua đó ánh xạ Địa chỉ gửi -> Địa chỉ nhận.
2.  **Lộ vết phí Gas (Gas Payer Linking):** Lệnh Rút tiền cần có ví trả tiền phí Gas (ETH Sepolia). Nếu người nhận phải lấy ví có sẵn ETH ra trả phí, hoặc người gửi phải chuyển 1 ít ETH vào ví cho người nhận làm phí Gas => Danh tính lại bị lộ qua việc chuyển tiền phí.

---

## 2. Những việc cần làm để Rút tiền MÙ HOÀN TOÀN (The Roadmap)

Để ẩn giấu hoàn toàn vết tích gửi/nhận, dự án này phải được nâng cấp thêm hai thành phần cốt lõi:

### Vấn đề 1: Đồng bộ số tiền nạp rút (Fixed Denomination Pools)
Thay vì cho phép người dùng nạp số tiền bất kỳ, Smart Contract phải chia làm các **Danh mục cố định (Cùng mệnh giá)**.
*   *Ví dụ:* Chỉ có Pool `0.1 ETH`, Pool `1 ETH`, Pool `10 ETH`.
*   Tất cả mọi người nạp tiền vào Pool `1 ETH` đều phải nạp đúng `1 ETH`.
*   Khi có 100 người nạp, và 1 người rút `1 ETH` ra, kẻ do thám sẽ phải đoán 1 trên 100 (Anonymity Set size = 100), không thể biết được `1 ETH` được rút ra là của ai trong số 100 người kia.

### Vấn đề 2: Tích hợp Mạng Relayer (Trạm trung chuyển Gas)
Thay vì Người Rút Tiền phải tự cầm ví có ETH để gọi Smart Contract, chúng ta sử dụng **Relayer**.
*   Người gửi ném cục ZK Proof (trong đó mã hóa sẵn việc: "Trích 1% số tiền này cho người trả gas") lên một mạng lưới Server ẩn danh (Relayer).
*   Relayer cầm cục Proof đó, dùng **Ví của Relayer (có sẵn ETH dồi dào)** nộp lên Smart Contract để thực hiện giao dịch Rút.
*   Smart Contract tự động chuyển `99% tiền` bằng sạch, không dấu vết về cho Người Nhận (dù ví người nhận trước đó có 0 ETH), và `1% tiền` về cho Relayer làm phần thưởng. Không có bất cứ liên kết On-chain nào giữa Người Gửi và Người Nhận!

---

## 3. Luồng hoạt động CHÍNH XÁC CUỐI CÙNG (Final Architecture)

Khi áp dụng các bản vá trên, quy trình vận hành hoàn hảo của một Hệ thống Ẩn danh sẽ như sau:

1. **User Nạp tiền (Alice):** Alice chọn Pool `1 ETH`. Tạo `Secret` & `Nullifier_Secret`. Gọi hàm `deposit()` gửi đúng 1 ETH vào Smart Contract Pool (Sử dụng ví gốc).
2. **ZK Proof Offline:** Vài tuần sau, Alice (đã che IP) tạo ZK Proof chứa lệnh: "Gửi 1 ETH này cho Bob (0xBob...), phí cho Relayer là 0.005 ETH".
3. **Mạng Relayer nhận Proof:** Relayer lấy ZK Proof này, thay vì Alice, Relayer sẽ dùng chính tổ hợp Ví trung gian của Relayer (ví dụ: ví Charlie) gọi hàm `withdraw()` và tự trả 0.001 ETH Tiền Gas để xác thực với mạng Ethereum.
4. **Smart Contract phân bổ:** Smart Contract xác thực ZK Proof đúng. Nó chuyển `0.995 ETH` cho Bob, và `0.005 ETH` cho Charlie. 

Kết quả: **Alice mất 1.000 ETH. Bob nhận 0.995 ETH mà không hề tương tác với mạng lưới. Mối quan hệ giữa Alice và Bob vĩnh viễn bị ẩn giấu trong Cây Merkle.**

---

## 4. Hướng dẫn chạy thực tế (Step-by-Step Guide)

Để thử nghiệm ứng dụng này trên mạng Sepolia, bạn cần làm theo các bước sau:

### Chuẩn bị
1. Cài đặt **Rust**, **Foundry** (forge/cast) và **Docker** (để nén ZK Proof).
2. Copy file cấu hình:
   ```bash
   cp .env.example .env
   ```
3. Điền `PRIVATE_KEY` của bạn vào file `.env` (ví này cần có sẵn một ít Sepolia ETH để trả phí gas).

### Bước 1: Thực hiện lệnh Nạp tiền (Deposit)
Chạy lệnh sau để hệ thống tính toán ra mã băm (Commitment) từ Secret của bạn.
*Lưu ý: `secret` phải là một chuỗi Hex dài 64 ký tự (32 bytes).*

```bash
cd host
cargo run -- --deposit --amount "số tiền" --secret "secret"
```

**Kết quả màn hình sẽ in ra một lệnh `cast send` tương tự như sau.** Bạn hãy copy lệnh đó và chạy trực tiếp trên Terminal để gửi tiền vào Smart Contract:

```bash
# Ví dụ lệnh nạp 0.01 ETH vào hợp đồng
cast send <hợp_đồng> "deposit(bytes32)" <commit> --value "số tiền" --rpc-url <rpc_url> --private-key <YOUR_PRIVATE_KEY>
```
*Ghi chú: Đợi khoảng 15-30 giây để giao dịch Deposit được xác nhận trên Sepolia Etherscan.*

### Bước 2: Thực hiện lệnh Rút tiền ẩn danh (Withdraw)
Khi muốn rút tiền, bạn chạy lệnh sau. Hệ thống sẽ tự động quét blockchain, thu thập cây Merkle, tạo **ZK Proof (Groth16)** nội bộ và tự động gửi lệnh rút tiền lên Smart Contract.

```bash
# Đổi địa chỉ --recipient thành bất kỳ ví nào bạn muốn nhận tiền
cargo run -- --chain --amount "số tiền" --secret "secret" --recipient "Địa_chỉ_ví_người_nhận" --groth16
```

**⚠️ Lưu ý phần cứng:** Cờ `--groth16` bắt buộc Docker phải nén bằng chứng ZK dài hàng triệu dòng thành 1 chuỗi bytes cực ngắn. Quá trình này đòi hỏi máy tính có ít nhất **16GB RAM** (hoặc cấu hình RAM ảo/Swap 24GB) và sẽ mất khoảng **5-10 phút** để chạy xong. Truy cập thư mục dự án và gõ lệnh đợi giao dịch hoàn tất trên Etherscan.

---

## 5. Lời kết
Mô hình ZK Privacy App hiện tại đã xử lý thành công được chặng đường khó khăn và ngốn nhiều Toán nhất: **Đưa logic Risc0 Groth16 zkVM lên mạng blockchain thật (Sepolia).** Nếu tiếp tục mở rộng dựa án theo Roadmap nói trên, đây hoàn toàn có thể trở thành một giải pháp Tornado Cash phi tập trung thế hệ mới.
