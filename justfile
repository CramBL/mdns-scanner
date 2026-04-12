import 'scripts/version.just'

alias c := check
alias t := test
alias l := lint
alias fmt := format
alias b := build
alias bm := build-musl
alias r := run

@_default:
    just --list

ci: format check lint test

test *ARGS:
    cargo nextest run --all --no-fail-fast --no-default-features {{ ARGS }}
    cargo nextest run --all --no-fail-fast --all-features {{ ARGS }}

format:
    cargo fmt --all

lint $RUSTFLAGS="--deny warnings":
    cargo clippy --all --tests --no-default-features
    cargo clippy --all --tests --all-features

check *ARGS:
    cargo check --all --tests --no-default-features
    cargo check --all --tests --all-features
    typos .

run *ARGS:
    cargo run {{ ARGS }}

build *ARGS:
    cargo build {{ ARGS }}

build-musl:
    cargo build --release --target x86_64-unknown-linux-musl --no-default-features
    -ldd target/x86_64-unknown-linux-musl/release/mdns-scanner
    -ls -lh target/x86_64-unknown-linux-musl/release/mdns-scanner
    cargo build --release --target x86_64-unknown-linux-musl --all-features
    -ldd target/x86_64-unknown-linux-musl/release/mdns-scanner
    -ls -lh target/x86_64-unknown-linux-musl/release/mdns-scanner

# build and measure with a specific feature set
[private]
bin-size-inner TARGET LABEL CARGO_ARGS:
    cargo build --release --target {{ TARGET }} {{ CARGO_ARGS }}
    echo " $(stat -c%s target/{{ TARGET }}/release/mdns-scanner | numfmt --to=iec) ($(stat -c%s target/{{ TARGET }}/release/mdns-scanner) bytes): {{ CARGO_ARGS }}" \
    | tee -a bin_size.txt

# Combined recipe
bin-size TARGET="x86_64-unknown-linux-musl":
    cargo nextest run --all --target {{ TARGET }} --all-features
    just bin-size-inner {{ TARGET }} "All features" "--all-features"
    just bin-size-inner {{ TARGET }} "No default features" "--no-default-features"
    cat bin_size.txt
