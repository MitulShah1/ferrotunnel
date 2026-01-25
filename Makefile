.PHONY: help fmt check lint test build clean all

# Default target
all: fmt check test build

# Show help
help:
	@echo "FerroTunnel - Development Commands"
	@echo ""
	@echo "Usage:"
	@echo "  make fmt          - Format code with rustfmt"
	@echo "  make check        - Run formatter and linter checks"
	@echo "  make lint         - Run clippy linter"
	@echo "  make test         - Run all tests"
	@echo "  make build        - Build the project"
	@echo "  make clean        - Clean build artifacts"
	@echo "  make all          - Run fmt, check, test, and build"
	@echo ""

# Format code
fmt:
	@echo "Running rustfmt..."
	cargo fmt --all

# Check formatting and linting
check:
	@echo "Checking formatting..."
	cargo fmt --all -- --check
	@echo "Running clippy..."
	cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run clippy linter
lint:
	@echo "Running clippy..."
	cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run tests
test:
	@echo "Running tests..."
	cargo test --workspace --all-features

# Build the project
build:
	@echo "Building project..."
	cargo build --workspace

# Build release
release:
	@echo "Building release..."
	cargo build --workspace --release

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean

# Install development tools
install-tools:
	@echo "Installing development tools..."
	rustup component add rustfmt clippy

# Dry run cargo publish for all crates (in dependency order)
publish-dry-run:
	@echo "Dry running cargo publish..."
	cargo publish -p ferrotunnel-common --dry-run --allow-dirty
	cargo publish -p ferrotunnel-protocol --dry-run --allow-dirty
	cargo publish -p ferrotunnel-core --dry-run --allow-dirty
	cargo publish -p ferrotunnel-plugin --dry-run --allow-dirty
