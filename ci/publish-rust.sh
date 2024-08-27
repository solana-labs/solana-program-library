#!/usr/bin/env bash

set -e
base="$(dirname "${BASH_SOURCE[0]}")"
source "$base/read-cargo-variable.sh"
cd "$base/.."

if [[ -z $1 ]]; then
  echo 'A package manifest path — e.g. "token/program" — must be provided.'
  exit 1
fi
PACKAGE_PATH=$1
if [[ -z $2 ]]; then
  echo 'A version level — e.g. "patch" — must be provided.'
  exit 1
fi
LEVEL=$2
DRY_RUN=$3

# Go to the directory
cd "${PACKAGE_PATH}"

# Publish the new version.
if [[ -n ${DRY_RUN} ]]; then
  cargo release ${LEVEL}
else
  cargo release ${LEVEL} --no-push --no-tag --no-confirm --execute
fi

# Stop here if this is a dry run.
if [[ -n $DRY_RUN ]]; then
  exit 0
fi

# Get the new version.
new_version=$(readCargoVariable version "Cargo.toml")
package_name=$(readCargoVariable name "Cargo.toml")
tag_name="$(echo $package_name | sed 's/spl-//')"
new_git_tag="${tag_name}-v${new_version}"

# Expose the new version to CI if needed.
if [[ -n $CI ]]; then
  echo "new_git_tag=${new_git_tag}" >> $GITHUB_OUTPUT
fi

# Soft reset the last commit so we can create our own commit and tag.
git reset --soft HEAD~1

# Commit the new version.
git commit -am "Publish ${tag_name} v${new_version}"

# Tag the new version.
git tag -a ${new_git_tag} -m "${tag_name} v${new_version}"
