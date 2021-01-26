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
export PATH="$solana_path"/active_release/bin:"$PATH"

installed_solana_version() {
  declare version_yml="$solana_path"/active_release/version.yml
  sed -e $'s/channel: \\(.*\\)/\\1/\nt\nd' "$version_yml" 2>/dev/null
}

if [[ -n $1 ]]; then
  case $1 in
  install)
    if [[ "$(installed_solana_version)" = "$solana_version" ]]; then
      echo "$0: Skipping install. Requested version ($solana_version) already available" >&2
    else
      curl -sSfL https://release.solana.com/$solana_version/install \
        | sh -s - $solana_version \
          --no-modify-path \
          --data-dir "$solana_path" \
          --config "$solana_path"/config.yml

      solana --version
    fi
    ;;
  *)
    echo "$0: Note: ignoring unknown argument: $1" >&2
    ;;
  esac
fi
