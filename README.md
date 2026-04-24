# Conduit

Conduit is a high-performance, cross-platform network utility built with **Rust**, **Iced**, and **Tokio**. It provides a modern GUI for complex network forwarding tasks.

## Features

- **System Network Share (NAT)**: Easily share internet from multiple WAN interfaces to a specific LAN interface (e.g., for development boards).
- **Multi-task Port Forwarding**: Concurrent TCP and UDP port forwarding (Sokit-like) with support for multiple active rules.
- **Asynchronous Engine**: Powered by `tokio` for low-latency, high-throughput data proxying.
- **Modern UI**: A clean, responsive interface built with the `iced` framework.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- `pkexec` (for system-level NAT configuration on Linux)

### Installation & Running

```bash
# Clone the repository
git clone git@github.com:xjimlinx/Conduit.git
cd Conduit

# Run the application
cargo run --release
```

## Usage

1. **Network Share**: Select one or more WAN interfaces, pick your LAN target, set the gateway IP, and click "Start Share".
2. **Port Forwarding**: Go to the "Port Forwarders" tab, click "Add New", configure your protocol (TCP/UDP) and ports, then click "Start".

## License

MIT License
