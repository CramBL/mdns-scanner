import 'scripts/version.just'

alias t := test
alias l := lint
alias fmt := format
alias b := build
alias bm := build-musl
alias r := run

@_default:
	just --list

ci: format lint test

test *ARGS:
	cargo nextest run --all --no-fail-fast --no-default-features {{ARGS}}
	cargo nextest run --all --no-fail-fast --all-features {{ARGS}}

format:
	cargo fmt --all

lint:
	cargo clippy --all --tests --no-default-features
	cargo clippy --all --tests --all-features

run *ARGS:
	cargo run {{ARGS}}

build *ARGS:
	cargo build {{ARGS}}

build-musl:
	cargo build --release --target x86_64-unknown-linux-musl --no-default-features
	-ldd target/x86_64-unknown-linux-musl/release/mdns-scanner
	-ls -lh target/x86_64-unknown-linux-musl/release/mdns-scanner
	cargo build --release --target x86_64-unknown-linux-musl --all-features
	-ldd target/x86_64-unknown-linux-musl/release/mdns-scanner
	-ls -lh target/x86_64-unknown-linux-musl/release/mdns-scanner
