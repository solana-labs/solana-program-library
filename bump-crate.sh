#!/usr/bin/env bash
#
# Updates a crate's version (major, minor, or patch) and subsequently updates
# all dependencies on that crate in the workspace.
#
# Usage: bump-crate.sh (patch|minor|major) <manifest-path>

if [ $# -ne 2 ]; then
    echo "Usage: $0 (patch|minor|major) <manifest-path>"
    exit 1
fi

release_type="$1"
manifest_path="$2"

# Release type should be one of patch, minor, or major.
if [[ ! $release_type =~ ^(major|minor|patch)$ ]]; then
  echo "Invalid release type: $release_type. Must be one of patch, minor, or major." 1>&2
  exit 1
fi

# Path should be to a Cargo.toml file.
if [ ! -e "$manifest_path" ] || [[ "$manifest_path" != *"Cargo.toml" ]]; then
  echo "Invalid manifest path: $manifest_path. Must be a path to a Cargo.toml file." 1>&2
  exit 1
fi

# First read the package name and current version from the manifest.
package_name=$(grep '^name\s*=' "$manifest_path" | awk -F'\"' '{print $2}')
current_version=$(grep '^version\s*=' "$manifest_path" | awk -F'\"' '{print $2}')
echo "Package name: $package_name"
echo "Current version: $current_version"

# Increment the version based on the release type.
major=$(echo $current_version | cut -d. -f1)
minor=$(echo $current_version | cut -d. -f2)
patch=$(echo $current_version | cut -d. -f3)

case $release_type in
  major)
    major=$((major + 1))
    minor=0
    patch=0
    ;;
  minor)
    minor=$((minor + 1))
    patch=0
    ;;
  patch)
    patch=$((patch + 1))
    ;;
esac

new_version="$major.$minor.$patch"
echo "New version: $new_version"

# Increment version in each manifest of the specified package.
sed -i '' -e "s/^version.*$/version = \"$new_version\"/g" "$manifest_path"

# Now find all packages in the workspace that depend on the specified package
# and update their dependency to the new version.
declare tomls=()
while IFS='' read -r line; do tomls+=("$line"); done < <(find . -name Cargo.toml)
sed -E -i '' -e "s:(${package_name} = \")([=<>]*)${current_version}([^\"]*)\".*:\1\2${new_version}\3\":" "${tomls[@]}"
sed -E -i '' -e "s:(${package_name} = \{ version = \")([=<>]*)${current_version}([^\"]*)(\".*):\1\2${new_version}\3\4:" "${tomls[@]}"
