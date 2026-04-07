# BLE SSH Speed Test Report

- **Date**: 2026-04-06 15:35:50 CST
- **Device**: whisplay-chatbot [pisugar] (PeripheralId(a5a5ed1a-7579-c1e7-f7d4-47683daa7542))
- **SSH User**: pi
- **Total transferred**: TX 166.3 KB / RX 151.6 KB

## Results

| Test | Result |
|------|--------|
| Echo Latency (10 rounds) | min 989 ms / avg 1055 ms / median 1047 ms / max 1202 ms (10/10 ok) |
| Download 1 KB | 1.4 KB in 1.2s = **1.2 KB/s** (9.3 kbit/s) |
| Download 10 KB | 13.5 KB in 1.9s = **6.9 KB/s** (55.5 kbit/s) |
| Download 50 KB | 67.5 KB in 5.8s = **11.7 KB/s** (93.8 kbit/s) |
| Upload 1 KB | sent 1 KB, Pi received 1024 bytes in 1.3s = **0.8 KB/s** (6.3 kbit/s) |
| Upload 10 KB | sent 10 KB, Pi received 10240 bytes in 2.6s = **3.9 KB/s** (31.0 kbit/s) |
| Upload 100 KB | sent 100 KB, Pi received 102400 bytes in 16.2s = **6.2 KB/s** (49.4 kbit/s) |
| Command Execution | Kernel info: 1107 ms / List root: 991 ms / CPU info: 1166 ms / Disk usage: 1140 ms / Memory info: 1200 ms |

## Details

### Echo Latency (10 rounds)

min 989 ms / avg 1055 ms / median 1047 ms / max 1202 ms (10/10 ok)

### Download 1 KB

1.4 KB in 1.2s = **1.2 KB/s** (9.3 kbit/s)

### Download 10 KB

13.5 KB in 1.9s = **6.9 KB/s** (55.5 kbit/s)

### Download 50 KB

67.5 KB in 5.8s = **11.7 KB/s** (93.8 kbit/s)

### Upload 1 KB

sent 1 KB, Pi received 1024 bytes in 1.3s = **0.8 KB/s** (6.3 kbit/s)

### Upload 10 KB

sent 10 KB, Pi received 10240 bytes in 2.6s = **3.9 KB/s** (31.0 kbit/s)

### Upload 100 KB

sent 100 KB, Pi received 102400 bytes in 16.2s = **6.2 KB/s** (49.4 kbit/s)

### Command Execution

Kernel info: 1107 ms / List root: 991 ms / CPU info: 1166 ms / Disk usage: 1140 ms / Memory info: 1200 ms


## Analysis

- **Latency**: Each SSH command includes full session setup (TCP connect → BLE CONNECT/OK → SSH handshake → command → response). The ~1s latency is dominated by SSH key exchange over the low-bandwidth BLE link.
- **Throughput**: BLE 4.x GATT notifications have a practical ceiling of ~5-10 KB/s due to MTU size (typically 20-512 bytes per notification) and connection interval. Upload (client→Pi) uses `WriteWithResponse` which is slower due to acknowledgment overhead.
- **Download vs Upload**: Download (Pi→client via BLE notifications) tends to be faster than upload (client→Pi via GATT writes) because notifications are fire-and-forget at the BLE level, while writes require per-packet ACK.
- **Large transfers**: 100KB+ transfers may fail due to SSH session timeout or BLE connection instability over extended periods. BLE is best suited for interactive SSH sessions, not bulk file transfers.
