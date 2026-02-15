# UltraBalancer

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/bas3line/ultrabalancer/actions/workflows/ci.yml/badge.svg)](https://github.com/bas3line/ultrabalancer/actions)

Production-grade, high-performance load balancer written in Rust. Capable of 500K+ requests per second.

## Installation

### Quick Install (Recommended)
```bash
curl -L https://package.ultrabalancer.com/install.sh | bash
```

### From Source
```bash
cargo install ultrabalancer
```

Or build from source:
```bash
cargo build --release
```

## Quick Start

```bash
ultrabalancer start round-robin 10.0.1.10:8080 10.0.1.11:8080
```

With config file:
```bash
ultrabalancer -c config.yaml
```

## Package Server

Self-hosted package distribution at [package.ultrabalancer.com](https://package.ultrabalancer.com)

Available packages:
- Linux x86_64
- Linux ARM64
- macOS x86_64
- macOS ARM64 (Apple Silicon)
- Windows x86_64

## Documentation

Full documentation available at [docs.ultrabalancer.com](https://docs.ultrabalancer.com)

## Performance

| Hardware | Requests/sec | Avg Latency | P99 Latency |
|----------|--------------|-------------|-------------|
| Apple M4 Pro | ~850,000 | 0.12ms | 0.45ms |
| Apple M2 | ~620,000 | 0.18ms | 0.68ms |
| Apple M1 | ~580,000 | 0.21ms | 0.75ms |
| Intel i5-12400 | ~420,000 | 0.28ms | 0.95ms |
| RTX 2050 Laptop | ~380,000 | 0.32ms | 1.10ms |

_Benchmark: 10,000 concurrent connections, 30s duration_

## Features

- Multiple algorithms: Round Robin, Least Connections, IP Hash, Random, Weighted
- Health checking with automatic failover
- Real-time metrics at `/metrics` and `/health`
- Admin API for dynamic backend management
- Async Tokio runtime for maximum performance

## Links

- Website: [ultrabalancer.com](https://ultrabalancer.com)
- Package Server: [package.ultrabalancer.com](https://package.ultrabalancer.com)
- Docs: [docs.ultrabalancer.com](https://docs.ultrabalancer.com)
- Contact: [hi@ultrabalancer.com](mailto:hi@ultrabalancer.com)
- GitHub: [github.com/bas3line/ultrabalancer](https://github.com/bas3line/ultrabalancer)

## License

MIT License - see [LICENSE](LICENSE) file.