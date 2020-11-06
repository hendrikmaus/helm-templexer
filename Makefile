.DEFAULT_GOAL := help

VERSION         ?= $(shell cat Cargo.toml | grep -Po 'version = "\K([0-9].[0-9].[0-9][\-a-z]+)')
DOCKER_REGISTRY ?= hendrikmaus
DOCKER_IMAGE    ?= helm-templexer
DOCKER_TAG      ?= $(VERSION)

##@ Manage

install: ## Install helm-templexer
	cargo install --path .
.PHONY: install

update: install
.PHONY: update

uninstall: ## Uninstall helm-templexer
	cargo uninstall --bin helm-templexer
.PHONY: uninstall

##@ Containerize

docker-build:
	docker build \
		--tag $(DOCKER_REGISTRY)/$(DOCKER_IMAGE):$(DOCKER_TAG) \
		$(CURDIR)
.PHONY: docker-build

docker-push:
	docker push \
		$(DOCKER_REGISTRY)/$(DOCKER_IMAGE):$(DOCKER_TAG)
.PHONY: docker-push

##@ Misc

clean: ## Clean local build data
	cargo clean
.PHONY: clean

help: ## Display this help
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[.a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)
.PHONY: help
