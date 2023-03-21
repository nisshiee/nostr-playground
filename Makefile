include ./base.mk

AWS ?= aws

.PHONY: setup
setup: ## setup
	$(AWS) configure --profile $(AWS_PROFILE)
	$(MAKE) -C relay ecr-login
