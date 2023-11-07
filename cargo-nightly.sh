#!/usr/bin/env bash

here=$(dirname "$0")
source "${here}"/ci/rust-version.sh nightly
# shellcheck disable=SC2054 # rust_nightly is sourced from rust-version.sh
toolchain="$rust_nightly"
set -x
exec cargo "+${toolchain}" "${@}"