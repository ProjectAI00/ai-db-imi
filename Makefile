BIN := $(HOME)/.local/bin/imi

# Build from source and install locally — your "localhost" for testing
install:
	cargo build --release
	cp target/release/imi $(BIN)
	@echo "Installed: $$($(BIN) --version)"

# Just build, don't install
build:
	cargo build --release

# Run integration tests
test:
	bash tests/integration.sh

.PHONY: install build test
