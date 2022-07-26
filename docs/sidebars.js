module.exports = {
  docs: [
    "introduction",
    "token",
    "token-swap",
    "token-lending",
    "associated-token-account",
    "memo",
    "name-service",
    "shared-memory",
    {
      type: "category",
      label: "Stake Pool",
      collapsed: true,
      items: [
        "stake-pool",
        "stake-pool/quickstart",
        "stake-pool/overview",
        "stake-pool/cli",
      ],
    },
    "feature-proposal",
    {
      type: "category",
      label: "Confidential Token Extension",
      collapsed: true,
      items: [
        "confidential-token",
        "confidential-token/quickstart",
        {
          type: "category",
          label: "Protocol Deep Dive",
          collapsed: true,
          items: [
            "confidential-token/deep-dive/overview",
            "confidential-token/deep-dive/encryption",
            "confidential-token/deep-dive/zkps",
          ],
        },
      ],
    },
  ],
};
