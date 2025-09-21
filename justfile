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

# Generate documentation
doc *args="--all-features":
    cargo doc --workspace --no-deps {{args}}

# Run rustdoc with docsrs configuration
rustdoc:
    cargo rustdoc --all-features -- --cfg docsrs

# Run all task
all: fmt build clippy doc test

# run postgres example
run-example example_name *args="":
    cargo run --example {{example_name}} --features {{example_name}} -- {{args}}
