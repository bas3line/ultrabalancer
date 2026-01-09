# UltraBalancer

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

Production-grade, high-performance load balancer written in Rust. Built for speed, reliability, and ease of use.

## Features

- **Multiple Load Balancing Algorithms**
  - Round Robin
  - Least Connections
  - IP Hash (consistent hashing)
  - Random
  - Weighted Round Robin

- **Health Checking**
  - Automatic backend health monitoring
  - Configurable intervals and failure thresholds
  - Zero-downtime failover

- **Metrics & Monitoring**
  - Real-time request metrics
  - Response time tracking
  - Built-in `/metrics` and `/health` endpoints

- **Production Ready**
  - Async Tokio runtime
  - Lock-free performance
  - Efficient connection handling

## Quick Start

### Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
```

### Usage Examples

**One-line setup:**
```bash
ultrabalancer start round-robin 10.0.1.10:8080 10.0.1.11:8080 10.0.1.12:8080
```

**With custom port:**
```bash
ultrabalancer start least-connections server1:8080 server2:8080 -p 80
```

**With backends flag:**
```bash
ultrabalancer -b 192.168.1.10:8080 -b 192.168.1.11:8080 -a round-robin
```

**Using config file:**
```bash
ultrabalancer -c config.yaml
```

**Disable health checks:**
```bash
ultrabalancer start round-robin server1:8080 server2:8080 --no-health-check
```

### Configuration File

Generate example config:
```bash
ultrabalancer example > config.yaml
```

Example `config.yaml`:
```yaml
listen_address: "0.0.0.0"
listen_port: 8080
algorithm: "round-robin"

backends:
  - host: "192.168.1.10"
    port: 8080
    weight: 100
  - host: "192.168.1.11"
    port: 8080
    weight: 100

health_check:
  enabled: true
  interval_ms: 5000
  max_failures: 3

timeout:
  connect_ms: 5000
  request_ms: 30000
```

## CLI Reference

### Commands

- `start <algorithm> <backends...>` - Start with inline configuration
- `validate -f <config>` - Validate configuration file
- `example` - Generate example configuration
- `info` - Display system information

### Flags

- `-p, --port <PORT>` - Listen port (default: 8080)
- `--host <HOST>` - Listen address (default: 0.0.0.0)
- `-a, --algorithm <ALGO>` - Load balancing algorithm
- `-b, --backend <HOST:PORT>` - Add backend server (repeatable)
- `-c, --config <FILE>` - Configuration file path
- `--no-health-check` - Disable health checks
- `--health-check-interval <MS>` - Health check interval (default: 5000)
- `--health-check-fails <N>` - Max failures before DOWN (default: 3)
- `-w, --weight <N>` - Default backend weight (default: 100)

### Algorithms

| Algorithm | Description | Use Case |
|-----------|-------------|----------|
| `round-robin` | Equal distribution across backends | General purpose |
| `least-connections` | Route to backend with fewest active connections | Long-lived connections |
| `ip-hash` | Consistent hashing based on client IP | Session persistence |
| `random` | Random backend selection | Stateless workloads |
| `weighted` | Weight-based round robin | Heterogeneous backends |

## Monitoring

### Health Endpoint

```bash
curl http://localhost:8080/health
```

Response:
```json
{
  "status": "ok",
  "healthy_backends": "3/3",
  "uptime": "3600s"
}
```

### Metrics Endpoint

```bash
curl http://localhost:8080/metrics
```

Response:
```yaml
total_requests: 125432
successful_requests: 125100
failed_requests: 332
avg_response_time_ms: 12.5
min_response_time_ms: 1.2
max_response_time_ms: 450.3
uptime_seconds: 86400
```

## Architecture

```
┌─────────────────────────────────────────┐
│            Client Requests              │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│          Proxy Server (Tokio)           │
│  • Connection Handling                  │
│  • Request Routing                      │
│  • Metrics Collection                   │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│       Load Balancer (Algorithms)        │
│  • Round Robin   • Least Connections    │
│  • IP Hash       • Random               │
│  • Weighted RR                          │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│          Backend Pool (Arc)             │
│  • Health Tracking                      │
│  • Connection Counting                  │
│  • Automatic Failover                   │
└──────────────┬──────────────────────────┘
               │
        ┌──────┴──────┬────────┐
        ▼             ▼        ▼
   Backend 1    Backend 2  Backend 3
```

## Performance

Built with performance in mind:
- Async/await with Tokio runtime
- Lock-free operations using atomic types
- Arc-based shared state for zero-copy semantics
- Efficient connection pooling

## Development

### Project Structure

```
ultrabalancer-rs/
├── src/
│   ├── main.rs          # CLI and entry point
│   ├── backend/         # Backend management
│   ├── balancer/        # Load balancing algorithms
│   ├── config/          # Configuration handling
│   ├── health/          # Health checking
│   ├── metrics/         # Metrics collection
│   └── proxy/           # HTTP proxy server
├── examples/            # Usage examples
└── Cargo.toml
```

### Build

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

## Contributing

Contributions welcome! Please feel free to submit issues or pull requests.

## License

MIT License - see LICENSE file for details

## Credits

**Author:** Kira <kiraa@tuta.io>

Inspired by HAProxy, NGINX, and the need for a modern Rust-based load balancer.
