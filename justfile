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
	cargo nextest run --all {{ARGS}}

format:
	cargo fmt --all

lint:
	cargo clippy --all

run *ARGS:
	cargo run -- {{ARGS}}

build *ARGS:
	cargo build {{ARGS}}

build-musl:
	cargo build --release --target x86_64-unknown-linux-musl
	-ldd target/x86_64-unknown-linux-musl/release/mdns-scanner
	-ls -lh target/x86_64-unknown-linux-musl/release/mdns-scanner
