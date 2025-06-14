# Bump version, either of patch|minor|major
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
	just set-new-released-changelog "${NEW_VERSION}"

[private]
set-new-released-changelog VERSION:
	#!/usr/bin/env bash
	set -euo pipefail
	CURRENT_DATE=$(date +"%Y-%m-%d")
	NEW_SECTION="## [{{VERSION}}] - ${CURRENT_DATE}"
	sed -i 's/^## \[unreleased\]/'"${NEW_SECTION}"'/' CHANGELOG.md
	sed -i '0,/^## \[/s//## [unreleased]\n\n&/' CHANGELOG.md
