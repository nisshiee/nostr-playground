include ../base.mk

CARGO ?= cargo
DOCKER ?= docker
AWS ?= aws

ECS_CLUSTER_NAME ?= nostr-relay
ECS_SERVICE_NAME ?= nostr-relay2

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

.PHONY: use-ecs-task-arns
use-ecs-task-arns:
	$(eval export ECS_TASK_ARNS ?= $(shell $(AWS) --profile $(AWS_PROFILE) --output text ecs list-tasks --cluster $(ECS_CLUSTER_NAME) --service-name $(ECS_SERVICE_NAME) --query 'taskArns[]'))

.PHONY: restart
restart: use-ecs-task-arns ## restart ECS task
	for t in $(ECS_TASK_ARNS); do $(AWS) ecs stop-task --cluster $(ECS_CLUSTER_NAME) --task $$t > /dev/null; done

.PHONY: deploy
deploy: ## build, push, and restart
	$(MAKE) ecr-login
	$(MAKE) build
	$(MAKE) push
	$(MAKE) restart
