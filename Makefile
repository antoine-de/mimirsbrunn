BRAGI_VERSION = $(shell cat Cargo.toml | grep '^version' | cut -d '=' -f 2 | tr -d '[[:space:]]'\")

SHELL=/bin/bash
# Configuration
.PHONY: check help
.DEFAULT_GOAL := help

check: pre-build ## Runs several tests (alias for pre-build)
pre-build: fmt lint test

fmt: format ## Check formatting of the code (alias for 'format')
format: ## Check formatting of the code
	cargo fmt --all -- --check

clippy: lint ## Check quality of the code (alias for 'lint')
lint: ## Check quality of the code
	CLIPPY_EXTRA="--warn clippy::cargo --allow clippy::multiple_crate_versions --deny warnings"
	cargo clippy --all-targets -- $$CLIPPY_EXTRA
	cargo clippy --bins --all-features -- $$CLIPPY_EXTRA

test: ## Launch all tests
	cargo test --lib
	cargo test --bins
	cargo test --doc
	cargo test --test end_to_end
	cargo test --package mimir2
	cargo test --package common
	cargo test --package places

.PHONY: version
version: ## display version of bragi
	@echo $(BRAGI_VERSION)
