.DEFAULT_GOAL := help

##@ Manage

install:: ## Install helm-templexer
	cargo install --path .

update:: install

uninstall:: ## Uninstall helm-templexer
	cargo uninstall --bin helm-templexer

##@ Misc

clean:: ## Clean local build data
	cargo clean

help: ## Display this help
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[.a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)