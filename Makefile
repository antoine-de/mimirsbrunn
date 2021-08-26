BRAGI_VERSION = $(shell cat Cargo.toml | grep '^version' | cut -d '=' -f 2 | tr -d '[[:space:]]'\")

SHELL=/bin/bash
# Configuration
.PHONY: check docker-build-bragi-release docker-build-bragi-master dockerhub-login push-bragi-image-master push-bragi-image-release wipe-useless-images help
.DEFAULT_GOAL := help
ELASTICSEARCH_TEST_URL=http://localhost:9201

check: pre-build ## Runs several tests (alias for pre-build)
pre-build: fmt lint test

docker-build-bragi-release:
	@echo "Building Bragi image $(BRAGI_VERSION) for debian $(DEBIAN_VERSION) / rust $(RUST_VERSION)"; \
	ARG_DEB="--build-arg DEBIAN_VERSION=$(DEBIAN_VERSION)"; \
	ARG_RST="--build-arg RUST_VERSION=$(RUST_VERSION)"; \
	TAGS="--tag navitia/bragi:$(BRAGI_VERSION) --tag navitia/bragi:latest --tag navitia/bragi:release"; \
	docker build $$ARG_DEB $$ARG_RST $$TAGS -f docker/bragi/Dockerfile .; \

docker-build-bragi-master:
	@echo "Building Bragi image $(BRAGI_VERSION) for debian $(DEBIAN_VERSION) / rust $(RUST_VERSION)"; \
	ARG_DEB="--build-arg DEBIAN_VERSION=$(DEBIAN_VERSION)"; \
	ARG_RST="--build-arg RUST_VERSION=$(RUST_VERSION)"; \
	TAGS="--tag navitia/bragi:master"; \
	docker build $$ARG_DEB $$ARG_RST $$TAGS -f docker/bragi/Dockerfile .; \

dockerhub-login: ## Login Docker hub, DOCKER_USER, DOCKER_PASSWORD, must be provided
	$(info Login Dockerhub)
	echo ${DOCKER_PASSWORD} | docker login --username ${DOCKER_USER} --password-stdin

push-bragi-image-master: ## Push bragi-image to dockerhub
	$(info Push bragi-image-master to Dockerhub)
	docker push navitia/bragi:master

push-bragi-image-release: ## Push bragi-image to dockerhub
	$(info Push bragi-image-release to Dockerhub)
	docker push navitia/bragi:$(BRAGI_VERSION)
	docker push navitia/bragi:release
	docker push navitia/bragi:latest

wipe-useless-images: ## Remove all useless images
	$(info Remove useless images)
	docker images -q --filter "dangling=true" --no-trunc | xargs --no-run-if-empty docker rmi -f
	docker images "navitia/bragi*" -q | xargs --no-run-if-empty docker rmi -f

fmt: format ## Check formatting of the code (alias for 'format')
format: ## Check formatting of the code
	cargo fmt --all -- --check

clippy: lint ## Check quality of the code (alias for 'lint')
lint: ## Check quality of the code
	cargo clippy --all-features --all-targets -- --warn clippy::cargo --allow clippy::multiple_crate_versions --deny warnings

test: ## Launch all tests
	ELASTICSEARCH_TEST_URL="${ELASTICSEARCH_TEST_URL}" cargo test --all-targets
