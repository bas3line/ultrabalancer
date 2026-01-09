# UltraBalancer Project Structure

## Source Code Organization

```
src/
├── main.rs                              # Entry point, CLI parsing
├── backend/                             # Backend server management
│   ├── mod.rs                          # Module exports
│   ├── server.rs                       # Server representation
│   ├── pool.rs                         # Server pool management
│   └── health_tracker/
│       ├── mod.rs                      # Health tracker exports
│       ├── checker.rs                  # Health checking logic
│       └── circuit_breaker.rs          # Circuit breaker pattern
├── balancer/                            # Load balancing algorithms
│   ├── mod.rs                          # Module exports
│   ├── selector.rs                     # Unified selector interface
│   └── algorithms/
│       ├── mod.rs                      # Algorithm exports
│       ├── round_robin.rs              # Round robin
│       ├── least_connections.rs        # Least connections
│       ├── ip_hash.rs                  # IP hash
│       ├── weighted.rs                 # Weighted round robin
│       └── random.rs                   # Random selection
├── proxy/                               # HTTP proxying
│   ├── mod.rs                          # Proxy server
│   └── handler.rs                      # Request handling
├── metrics/                             # Metrics collection
│   ├── mod.rs                          # Module exports
│   ├── collector.rs                    # Metric collection
│   └── exporter.rs                     # Metric formatting
├── config/                              # Configuration
│   ├── mod.rs                          # Config structures
│   ├── parser.rs                       # YAML/TOML parsing
│   └── validator.rs                    # Configuration validation
├── error/                               # Error handling
│   └── mod.rs                          # Error types
├── health/                              # Health check endpoints
│   └── mod.rs                          # Health check server
└── utils/                               # Utilities
    ├── mod.rs                          # Module exports
    ├── rate_limiter.rs                 # Rate limiting
    └── connection_pool.rs              # Connection pooling
```

## Key Files

### Entry Point
- `src/main.rs` - CLI parsing, configuration loading, server startup

### Core Modules
- `src/backend/pool.rs` - Thread-safe backend server pool
- `src/balancer/selector.rs` - Algorithm selection and execution
- `src/proxy/mod.rs` - Main proxy server (Tokio-based)
- `src/proxy/handler.rs` - HTTP request/response handling

### Configuration
- `config.example.yaml` - Example configuration
- `Cargo.toml` - Dependencies and build configuration

## Build Artifacts

```
target/
├── debug/                  # Debug build
│   └── ultrabalancer      # Debug binary
└── release/                # Release build
    └── ultrabalancer      # Optimized binary (3.5MB)
```

## File Count
- Total Rust files: 28
- Total lines of code: ~2500+
- Binary size (release): 3.5MB

## Build Commands

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Check code
cargo check

# Format code
cargo fmt

# Lint
cargo clippy
```

## Running

```bash
# From config file
./target/release/ultrabalancer -c config.yaml

# From CLI arguments
./target/release/ultrabalancer -p 8080 \
  -b 192.168.1.10:8080 \
  -b 192.168.1.11:8080 \
  -a round-robin
```
