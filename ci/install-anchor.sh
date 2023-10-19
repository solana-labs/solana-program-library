#
# This file maintains the solana versions for use by CI.
#
# Obtain the environment variables without any automatic updating:
#   $ source ci/install-anchor.sh
#
# Obtain the environment variables and install update:
#   $ source ci/install-anchor.sh install

# Then to access the anchor version:
#   $ echo "$anchor_cli_version"
#

if [[ -n $ANCHOR_CLI_VERSION ]]; then
  anchor_cli_version="$ANCHOR_CLI_VERSION"
else
  anchor_cli_version=v0.29.0
fi

export anchor_cli_version="$anchor_cli_version"

if [[ -n $1 ]]; then
  case $1 in
  install)
    cargo install --git https://github.com/coral-xyz/anchor --tag $anchor_cli_version anchor-cli --locked
    anchor --version
    ;;
  *)
    echo "anchor-version.sh: Note: ignoring unknown argument: $1" >&2
    ;;
  esac
fi
