# Source this file.
#
# Sorts a provided list of manifest paths according to their position in the
# workspace dependency tree.

sort_with_dependency_tree() {
  # Input list of manifest paths
  local manifest_paths=("$@")

  # Generate the dependency graph using cargo metadata
  local metadata
  metadata=$(cargo metadata --no-deps --format-version 1)
  local sorted_manifests=()

  # Function to get dependencies for a given package
  get_dependencies() {
    local package_name=$1
    echo "$metadata" | jq -r --arg package_name "$package_name" '.packages[] | select(.name == $package_name) | .dependencies[].name'
  }

  # Topological sort function
  topological_sort() {
    local visited=()
    local temp_visited=()
    local stack=()

    visit() {
      local pkg=$1
      if [[ " ${temp_visited[*]} " =~ " ${pkg} " ]]; then
        echo "Cyclic dependency detected!" >&2
        exit 1
      fi

      if [[ ! " ${visited[*]} " =~ " ${pkg} " ]]; then
        temp_visited+=("$pkg")
        local deps
        deps=$(get_dependencies "$pkg")
        for dep in $deps; do
          visit "$dep"
        done
        visited+=("$pkg")
        temp_visited=(${temp_visited[@]/$pkg})
        stack+=("$pkg")
      fi
    }

    for file in "${manifest_paths[@]}"; do
      local package_name
      package_name=$(grep -E '^name\s*=' "$file" | sed -E 's/name\s*=\s*"(.*)"/\1/')
      visit "$package_name"
    done

    sorted_manifests=("${stack[@]}")
  }

  # Perform topological sort
  topological_sort

  # Map sorted package names back to their manifest paths
  local final_sorted_manifests=()
  for pkg in "${sorted_manifests[@]}"; do
    for file in "${manifest_paths[@]}"; do
      local package_name
      package_name=$(grep -E '^name\s*=' "$file" | sed -E 's/name\s*=\s*"(.*)"/\1/')
      if [ "$pkg" == "$package_name" ]; then
        final_sorted_manifests+=("$file")
      fi
    done
  done

  # Return the sorted list of manifest paths
  echo "${final_sorted_manifests[@]}"
}

# Export the function so it can be used in other scripts
export -f sort_with_dependency_tree
