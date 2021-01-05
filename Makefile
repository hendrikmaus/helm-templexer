.DEFAULT_GOAL := help

VERSION         ?= $(shell cat Cargo.toml | grep -Po 'version = "\K([0-9].[0-9].[0-9])')
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

docker-build: ## Build Docker image
	DOCKER_BUILDKIT=1 docker build \
		--tag $(DOCKER_REGISTRY)/$(DOCKER_IMAGE):$(DOCKER_TAG) \
		$(CURDIR)
.PHONY: docker-build

docker-push: ## Push Docker image
	docker push \
		$(DOCKER_REGISTRY)/$(DOCKER_IMAGE):$(DOCKER_TAG)
	docker tag \
		$(DOCKER_REGISTRY)/$(DOCKER_IMAGE):$(DOCKER_TAG) \
		$(DOCKER_REGISTRY)/$(DOCKER_IMAGE):latest
	docker push \
		$(DOCKER_REGISTRY)/$(DOCKER_IMAGE):latest
.PHONY: docker-push

##@ Release

release: ## Publish to crates.io, dockerhub
	$(MAKE) -j \
		cargo-publish \
		docker-publish \
		github-publish
.PHONY: release

cargo-publish: ## Run cargo publish
	cargo publish
.PHONY: cargo-publish

docker-publish: ## Run docker build and push
	$(MAKE) docker-build docker-push
.PHONY: docker-publish

github-publish: ## Run gh release create for a github release
	gh release create $(VERSION) \
		--title $(VERSION)
.PHONY: github-publish

##@ Misc

clean: ## Clean local build data
	cargo clean
.PHONY: clean

help: ## Display this help
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[.a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)
.PHONY: help
