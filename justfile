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

# Bump version
bump ARG:
	#!/usr/bin/env bash
	set -euo pipefail
	case {{ARG}} in
		major|minor|patch)
			VERSION_TYPE={{ARG}}
			;;
		*)
			echo "Error: Invalid version type '{{ARG}}'"
			exit 1
			;;
	esac

	CURRENT_VERSION=$(grep '^version = ' Cargo.toml | cut -d '"' -f 2)
	echo "Current version: $CURRENT_VERSION"

	if [[ ! $CURRENT_VERSION =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
		echo "Error: Version format is not valid semver (expected: major.minor.patch)"
		exit 1
	fi

	MAJOR=${BASH_REMATCH[1]}
	MINOR=${BASH_REMATCH[2]}
	PATCH=${BASH_REMATCH[3]}

	case $VERSION_TYPE in
		major)
			NEW_MAJOR=$((MAJOR + 1))
			NEW_MINOR=0
			NEW_PATCH=0
			;;
		minor)
			NEW_MAJOR=$MAJOR
			NEW_MINOR=$((MINOR + 1))
			NEW_PATCH=0
			;;
		patch)
			NEW_MAJOR=$MAJOR
			NEW_MINOR=$MINOR
			NEW_PATCH=$((PATCH + 1))
			;;
	esac

	NEW_VERSION="$NEW_MAJOR.$NEW_MINOR.$NEW_PATCH"
	sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" Cargo.toml
	echo "New version: $NEW_VERSION"
