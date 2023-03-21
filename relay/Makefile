include ../base.mk

CARGO ?= cargo
DOCKER ?= docker
AWS ?= aws

.PHONY: run
run: ## run
	$(CARGO) run

.PHONY: ecr-login
ecr-login: ## Login to ECR
	$(AWS) ecr get-login-password --region ap-northeast-1 | $(DOCKER) login --username AWS --password-stdin 827770520923.dkr.ecr.ap-northeast-1.amazonaws.com

.PHONY: build
build: ## Build the relay image
	cd .. && $(DOCKER) build -f relay/Dockerfile -t nostr-relay .

.PHONY: push
push: ## Push the relay image
	$(DOCKER) tag nostr-relay:latest 827770520923.dkr.ecr.ap-northeast-1.amazonaws.com/nostr-relay:latest
	$(DOCKER) push 827770520923.dkr.ecr.ap-northeast-1.amazonaws.com/nostr-relay:latest
