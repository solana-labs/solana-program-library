# Source this file.
#
# Detects any manifests whose versions have changed.
# Does so by comparing the current version in the Cargo.toml with the previous
# version in the last commit. Intended for squash-merged PRs.

detect_changed_manifests() {
  local changed_manifests=()
  local cargo_toml_files=$(find . -name "Cargo.toml")

  for file in $cargo_toml_files; do
    # Get the current version from the manifest.
    local current_version=$(grep -E '^version\s*=' "$file" | sed -E 's/version\s*=\s*"(.*)"/\1/')

    # Get the previous version from the last commit.
    local previous_version=$(git show HEAD~1:"$file" | grep -E '^version\s*=' | sed -E 's/version\s*=\s*"(.*)"/\1/' 2>/dev/null)

    # Compare the versions and add the path to the list if they are different.
    if [ "$current_version" != "$previous_version" ]; then
      changed_manifests+=("$file")
    fi
  done

  echo "${changed_manifests[@]}"
}