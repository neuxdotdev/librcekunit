PROJECT_NAME := librcekunit
CARGO := cargo
RUST_LOG ?= trace
ENV_FILE ?= .env
.DEFAULT_GOAL := help
define print
	@printf "\033[36m▶ %s\033[0m\n" "$(1)"
endef
help:
	@echo ""
	@echo "Available targets:"
	@echo "  make build        Build project"
	@echo "  make run          Run binary (if any)"
	@echo "  make test         Run all tests"
	@echo "  make test-verbose Run tests with full logs"
	@echo "  make check        cargo check"
	@echo "  make fmt          Format code"
	@echo "  make lint         Run clippy"
	@echo "  make doc          Build docs"
	@echo "  make clean        Clean target directory"
	@echo "  make env-check    Validate env variables"
	@echo "  make login-test   Run login flow test"
	@echo ""
build:
	$(call print,"Building project")
	$(CARGO) build
run:
	$(call print,"Running project")
	RUST_LOG=$(RUST_LOG) $(CARGO) run
check:
	$(call print,"Running cargo check")
	$(CARGO) check
fmt:
	$(call print,"Formatting code")
	$(CARGO) fmt --all
lint:
	$(call print,"Running clippy")
	$(CARGO) clippy --all-targets --all-features -- -D warnings
clean:
	$(call print,"Cleaning project")
	$(CARGO) clean
test:
	$(call print,"Running tests")
	RUST_LOG=info $(CARGO) test
test-verbose:
	$(call print,"Running tests (verbose)")
	RUST_LOG=$(RUST_LOG) $(CARGO) test -- --nocapture
doc:
	$(call print,"Building documentation")
	$(CARGO) doc --no-deps --document-private-items --open
env-check:
	$(call print,"Checking environment variables")
	@test -f $(ENV_FILE) || (echo ".env file not found" && exit 1)
	@grep -q USER_EMAIL $(ENV_FILE) || (echo "USER_EMAIL missing" && exit 1)
	@grep -q USER_PASSWORD $(ENV_FILE) || (echo "USER_PASSWORD missing" && exit 1)
	@grep -q BASE_URL $(ENV_FILE) || (echo "BASE_URL missing" && exit 1)
	@grep -q LOGIN_ENDPOINT $(ENV_FILE) || (echo "LOGIN_ENDPOINT missing" && exit 1)
	@grep -q LOGOUT_ENDPOINT $(ENV_FILE) || (echo "LOGOUT_ENDPOINT missing" && exit 1)
	@echo "Env OK"
login-test:
	$(call print,"Running login flow integration test")
	RUST_LOG=$(RUST_LOG) $(CARGO) test login -- --nocapture
ci:
	$(call print,"Running CI pipeline")
	make fmt
	make lint
	make check
	make test
