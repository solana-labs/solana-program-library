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
set -x

if [[ -n $SOLANA_VERSION ]]; then
  solana_version="$SOLANA_VERSION"
else
  solana_version=v1.14.10
fi

export solana_version="$solana_version"
export PATH="$HOME"/.local/share/solana/install/active_release/bin:"$PATH"

if [[ -n $1 ]]; then
  case $1 in
  install)
    curl -vsSfL https://release.solana.com/$solana_version/install > lol.sh
    printf "\n\nHANA SCRIPT BEGIN\n"
    cat lol.sh
    printf "\nHANA SCRIPT END\n\n"
    sed -i 's/set -e/set -ex/' lol.sh
    #sed -i 's/\( *\)\(ignore "\$solana.*\)/\1cat "\$solana_install_init"\n\1\2/' lol.sh
    chmod 755 lol.sh
    sh -c "$(./lol.sh)"
    solana --version
    ;;
  *)
    echo "solana-version.sh: Note: ignoring unknown argument: $1" >&2
    ;;
  esac
fi
