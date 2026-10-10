[private]
@list:
    just --list

check:
    cargo check --workspace --all-targets --all-features

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

fmt:
    cargo fmt --all

find-msrv:
    cargo msrv find --workspace --all-features

verify-msrv:
    cargo msrv verify --workspace --all-features

install-tools:
    cargo install cargo-msrv --version 0.20.0-beta.2 --locked

before-commit: fmt clippy verify-msrv
