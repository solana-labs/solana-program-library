module.exports = {
  docs: [
    "introduction",
    "token",
    {
      type: 'category',
      label: 'Token-2022',
      collapsed: true,
      items: [
        "token-2022",
        "token-2022/extensions",
        "token-2022/wallet-migration",
        "token-2022/onchain-migration",
      ],
    },
    "token-swap",
    "token-lending",
    "associated-token-account",
    "memo",
    "name-service",
    "shared-memory",
    {
      type: 'category',
      label: 'Stake Pool',
      collapsed: true,
      items: [
        "stake-pool",
        "stake-pool/quickstart",
        "stake-pool/overview",
        "stake-pool/cli",
      ],
    },
    "feature-proposal",
  ],
};
