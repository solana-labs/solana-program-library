#!/usr/bin/env bash
#
# This file maintains the solana versions for use by CI.
#
# Obtain the environment variables without any automatic updating:
#   $ source ci/solana-version.sh
#
# Obtain the environment variables and install update:
#   $ source ci/solana-version.sh install

# Then to access the solana version:
#   $ echo "$solana_version"
#

if [[ -n $SOLANA_VERSION ]]; then
  solana_version="$SOLANA_VERSION"
else
  solana_version=v1.5.5
fi

export solana_version="$solana_version"
export solana_docker_image=solanalabs/solana:"$solana_version"
export solana_path="$PWD/solana-install"

if [[ -n $1 ]]; then
  case $1 in
  install)
    curl -sSfL https://release.solana.com/$solana_version/install \
      | sh -s - $solana_version \
        --no-modify-path \
        --data-dir "$solana_path" \
        --config "$solana_path"/config.yml

    export PATH="$solana_path"/active_release/bin:"$PATH"
    solana --version
    ;;
  *)
    echo "$0: Note: ignoring unknown argument: $1" >&2
    ;;
  esac
fi
