# Source this file.
#
# Detects any changed Cargo manifests.

detect_changed_manifests() {
  local changed_manifests=()
  local cargo_toml_files=$(find . -name "Cargo.toml")

  for file in $cargo_toml_files; do
    # Get the current version from the Cargo.toml.
    local current_version=$(grep -E '^version\s*=' "$file" | sed -E 's/version\s*=\s*"(.*)"/\1/')

    # Get the previous version from the last committed Cargo.toml.
    local previous_version=$(git show HEAD~1:"$file" | grep -E '^version\s*=' | sed -E 's/version\s*=\s*"(.*)"/\1/' 2>/dev/null)

    # Compare the versions and add the path to the list if they are different.
    if [ "$current_version" != "$previous_version" ]; then
      changed_manifests+=("$file")
    fi
  done

  echo "${changed_manifests[@]}"
}
