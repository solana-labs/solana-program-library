#!/bin/bash

# Source the sort-with-dependency-tree.sh script
source ci/sort-with-dependency-tree.sh

# List of manifest paths
manifest_paths=(
    "token/program-2022/Cargo.toml"
    "libraries/pod/Cargo.toml"
    "token/transfer-hook/interface/Cargo.toml"
)

# Invoke the sourced function with the manifest paths
sorted=$(sort_with_dependency_tree "${manifest_paths[@]}")

# Print the sorted manifest paths
echo "Sorted manifest paths:"
for manifest_path in $sorted; do
  echo "$manifest_path"
done