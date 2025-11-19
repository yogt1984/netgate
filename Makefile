.PHONY: run test build clean

# Default target: run the project
run:
	cargo run

# Run tests
test:
	cargo test -- --test-threads=1

# Build the project
build:
	cargo build

# Clean build artifacts
clean:
	cargo clean

