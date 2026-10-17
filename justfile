# Set environment variable of rust backtrace to full
export RUST_BACKTRACE:= "full"

# List all available recipes
list:
  just --list --justfile {{justfile()}}

# Run cargo build
build *args="--all-features":
    cargo build --workspace {{args}}

# Check whether rust code is properly formatted or not (nightly only)
fmt:
    #!/usr/bin/env bash
    set -x
    if [[ "$(rustc --version)" == *nightly* ]]; then
        cargo fmt -- --check
    fi

# Run clippy to catch common mistakes and improve code (nightly only)
clippy *args="--all-features":
    #!/usr/bin/env bash
    set -x
    if [[ "$(rustc --version)" == *nightly* ]]; then
        cargo clippy --workspace {{args}} -- -D warnings
    fi

# Run tests
test *args="--all-features":
    cargo test --workspace {{args}}

# Run the backend tests against real databases. Needs POSTGRES_DATABASE_URL
# and MYSQL_DATABASE_URL
test-backend:
    cargo test --workspace --all-features --test backend -- --include-ignored

# Generate documentation
doc *args="--all-features":
    cargo doc --workspace --no-deps {{args}}

# Run rustdoc with docsrs configuration
rustdoc:
    cargo rustdoc --all-features -- --cfg docsrs

# Run all task
all: fmt build clippy doc test

# Run an example cli with the given arguments (example name doubles as the feature)
run-example example_name *args="":
    cargo run --example {{example_name}} --features {{example_name}} -- {{args}}

# Run the cli flow of an example against its database (used by CI)
run-example-flow example_name:
    just --justfile {{justfile()}} run-example {{example_name}} --help
    just --justfile {{justfile()}} run-example {{example_name}} apply --plan
    just --justfile {{justfile()}} run-example {{example_name}} apply --plan --format json
    just --justfile {{justfile()}} run-example {{example_name}} apply
    just --justfile {{justfile()}} run-example {{example_name}} apply --check
    just --justfile {{justfile()}} run-example {{example_name}} list
    just --justfile {{justfile()}} run-example {{example_name}} list --format json
    just --justfile {{justfile()}} run-example {{example_name}} redo --force
    just --justfile {{justfile()}} run-example {{example_name}} prune --force
    just --justfile {{justfile()}} run-example {{example_name}} revert --plan --format json
    just --justfile {{justfile()}} run-example {{example_name}} revert --all --force
    just --justfile {{justfile()}} run-example {{example_name}} list
    just --justfile {{justfile()}} run-example {{example_name}} drop
