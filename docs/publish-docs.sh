#!/usr/bin/env bash

set -e

if [[ -d .vercel ]]; then
  rm -r .vercel
fi

CONFIG_FILE=vercel.json

if [[ -n "$CI" ]]; then
  PROJECT_NAME=spl-docs-solana-com
else
  PROJECT_NAME=spl-docs
fi

cat > "$CONFIG_FILE" <<EOF
{
  "name": "$PROJECT_NAME",
  "scope": "solana-labs"
}
EOF

[[ -n $VERCEL_TOKEN ]] || {
  echo "VERCEL_TOKEN is undefined.  Needed for Vercel authentication."
  exit 1
}
vercel deploy . --local-config="$CONFIG_FILE" --confirm --token "$VERCEL_TOKEN" --prod
