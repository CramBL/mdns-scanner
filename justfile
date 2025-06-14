alias t := test
alias l := lint
alias fmt := format
alias b := build
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
