# ZK-auth Compare Results

## 10.1. In bảng compare

```bash
cargo run -p host --bin zk_auth_compare
```

Script đọc các file mới nhất trong `results/zk-auth/` và in bảng so sánh:

- traditional baseline
- zk-auth
- zk-auth + artifact
- success rate
- tamper detection rate
- replay rejection rate

```text
metric                              traditional        zk-auth    zk+artifact   availability         tamper         replay
--------------------------------------------------------------------------------------------------------------------
gas_used                                 151378         554413         554413              -              -              -
proof_generation_seconds                      -        55.0774        55.0774              -              -              -
proof_verify_seconds                          -         0.0261         0.0261              -              -              -
seal_size_bytes                               -            260            260              -              -              -
journal_size_bytes                            -            288            288              -              -              -
raw_tx_size_bytes                             -              -              -              -              -              -
calldata_size_bytes                         132            868            868              -              -              -
send_and_confirm_seconds                 8.4845         8.6859         8.6859              -              -              -
total_latency_seconds                    8.4900         8.7478         8.7478              -              -              -
success_rate_percent                        100            100            100              -              -              -
tamper_detection_rate                         -              -              -              -         100.00              -
replay_rejection_rate                         -              -              -              -              -         100.00
```

Nguồn dữ liệu:

- traditional: `results/zk-auth/traditional_1779630981.json`
- zk-auth: `results/zk-auth/zk_auth_1779632361.json`
- integrity: `results/zk-auth/integrity_1779632690.json`
- availability: `results/zk-auth/availability_1779633143.json`
