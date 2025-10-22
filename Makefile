PROFILE ?= dev

CURRENT_TAG := $(shell git describe --tags --exact-match HEAD 2>/dev/null)

ifeq ($(CURRENT_TAG),)
	LATEST_TAG := $(shell git describe --tags --abbrev=0 2>/dev/null || echo "v0.0.0")
    SHORT_SHA := $(shell git rev-parse --short HEAD)
	VERSION := $(LATEST_TAG)-$(SHORT_SHA)
else
	VERSION := $(CURRENT_TAG)
endif


ifeq ($(PROFILE), dev)
	export MODE_DIR := debug
	export CARGO_TARGET_DIR := ./target
endif

ifeq ($(PROFILE), prod)
	export MODE_DIR := release
	export RELEASE := --release

endif

# ALL

all: test format


# Misc

clean:
	@echo "Cleaning the project..."
	cargo clean

format:
	@echo "Running fmy..."
	cargo fmt --all -- --emit=files


# Test

test:
	@echo "Running tests with profile"
	JWT_SECRET=secret cargo test


# Env


# Dev run
run:
	APP_VERSION=$(VERSION) JWT_SECRET=secret cargo run

build_docker_matchbox_server:
	docker build -f ./Dockerfile ./ -t ghcr.io/bascanada/matchbox_server:latest


# Publish
push_docker_matchbox_server:
	docker push ghcr.io/bascanada/matchbox_server:latest

print_version:
	@echo "Current Tag: $(CURRENT_TAG)"
	@echo "Version: $(VERSION)"

