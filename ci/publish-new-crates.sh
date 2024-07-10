#!/usr/bin/env bash

set -e

source ci/detect-changed-manifests.sh

changed_manifests=$(detect_changed_manifests)

if [ -n "$changed_manifests" ]; then
  for manifest_path in $changed_manifests; do
    # Read the package name and version from its Cargo.toml.
    package_name=$(grep '^name\s*=' "$manifest_path" | awk -F'\"' '{print $2}')
    new_version=$(grep '^version\s*=' "$manifest_path" | awk -F'\"' '{print $2}')

    # Create a new git tag.
    tag_name="${package_name}-v${new_version}"
    git tag "$tag_name"
    echo "Created tag: $tag_name"
  done
else
  echo "No versions changed."
fi
