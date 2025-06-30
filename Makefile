.PHONY: build release test clean install example

# Default target
all: build

# Build in debug mode
build:
	cargo build

# Build optimized release version
release:
	cargo build --release

# Run tests
test:
	cargo test

# Run tests with output
test-verbose:
	cargo test -- --nocapture

# Clean build artifacts
clean:
	cargo clean

# Install to system (requires cargo install)
install:
	cargo install --path .

# Run the basic usage example
example:
	cargo run --example basic_usage

# Run CLI with help
help:
	cargo run -- --help

# Format code
fmt:
	cargo fmt

# Run clippy lints
lint:
	cargo clippy -- -D warnings

# Run all checks (format, lint, test)
check: fmt lint test

# Generate documentation
docs:
	cargo doc --open

# Create a release build and copy to target
dist: release
	@echo "Release binary available at: target/release/vpk"
	@ls -lh target/release/vpk

# Development convenience targets
pack: build
	cargo run -- pack

unpack: build
	cargo run -- unpack

list: build
	cargo run -- list

verify: build
	cargo run -- verify

extract: build
	cargo run -- extract
