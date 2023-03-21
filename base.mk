help: ## print this message
	@echo "Command list:"
	@echo ""
	@printf "\033[36m%-30s\033[0m %-50s %s\n" "[Sub command]" "[Description]" "[Example]"
	@grep -E '^[/a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | perl -pe 's%^(?:[^:]+:)?([/a-zA-Z_-]+):.*?(##)%$$1 $$2%' | awk -F " *?## *?" '{printf "\033[36m%-30s\033[0m %-50s %s\n", $$1, $$2, $$3}'

export ENV ?= dev

export AWS_PROFILE ?= $(AWS_PROFILE_$(ENV))
export AWS_PROFILE_dev ?= dev

export RUST_LOG ?= info
