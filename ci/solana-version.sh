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
  # This file is now out of sync with the versions in Cargo.toml.
  # https://github.com/solana-labs/solana-program-library/pull/6182
  # This will require some manual cleanup the next time the version is updated.
  solana_version=v1.18.2
fi

export solana_version="$solana_version"
export PATH="$HOME"/.local/share/solana/install/active_release/bin:"$PATH"

if [[ -n $1 ]]; then
  case $1 in
  install)
    sh -c "$(curl -sSfL https://release.solana.com/$solana_version/install)"
    solana --version
    ;;
  *)
    echo "solana-version.sh: Note: ignoring unknown argument: $1" >&2
    ;;
  esac
fi
