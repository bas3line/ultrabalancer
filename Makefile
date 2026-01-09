.PHONY: build release clean test install run help

BINARY_NAME=ultrabalancer
INSTALL_PATH=/usr/local/bin

help:
	@echo "UltraBalancer - Makefile Commands"
	@echo ""
	@echo "  make build      - Build debug binary"
	@echo "  make release    - Build optimized release binary"
	@echo "  make test       - Run tests"
	@echo "  make install    - Install binary to $(INSTALL_PATH)"
	@echo "  make clean      - Clean build artifacts"
	@echo "  make run        - Run with example config"
	@echo ""

build:
	@echo "Building debug binary..."
	cargo build

release:
	@echo "Building release binary..."
	cargo build --release
	@echo "✓ Binary: target/release/$(BINARY_NAME)"

test:
	@echo "Running tests..."
	cargo test

install: release
	@echo "Installing $(BINARY_NAME) to $(INSTALL_PATH)..."
	@cp target/release/$(BINARY_NAME) $(INSTALL_PATH)/
	@echo "✓ Installed: $(INSTALL_PATH)/$(BINARY_NAME)"

clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	@echo "✓ Clean complete"

run:
	@echo "Running with example configuration..."
	cargo run -- -c examples/config.yaml

fmt:
	@echo "Formatting code..."
	cargo fmt

lint:
	@echo "Running clippy..."
	cargo clippy -- -D warnings
