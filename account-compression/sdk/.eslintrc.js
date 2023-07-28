module.exports = {
    extends: ['turbo', '@solana/eslint-config-solana', '@solana/eslint-config-solana/jest'],
    overrides: [
        {
            files: ['tests/**.ts'],
            rules: {
                'no-empty': ['error', { allowEmptyCatch: true }],
            },
        },
    ],
    root: true,
};
