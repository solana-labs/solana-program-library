// eslint-disable-next-line
module.exports = {
  // eslint-disable-line import/no-commonjs
  env: {
    browser: true,
    es6: true,
    node: true,
  },
  extends: [
    'eslint:recommended',
    'plugin:import/errors',
    'plugin:import/warnings',
  ],
  parser: 'babel-eslint',
  parserOptions: {
    sourceType: 'module',
    ecmaVersion: 8,
  },
  rules: {
    'no-trailing-spaces': ['error'],
    'import/first': ['error'],
    'import/no-commonjs': ['error'],
    'import/order': [
      'error',
      {
        groups: [
          ['internal', 'external', 'builtin'],
          ['index', 'sibling', 'parent'],
        ],
        'newlines-between': 'always',
      },
    ],
    indent: [
      'error',
      2,
      {
        MemberExpression: 1,
        SwitchCase: 1,
      },
    ],
    'linebreak-style': ['error', 'unix'],
    'no-console': [0],
    quotes: [
      'error',
      'single',
      {avoidEscape: true, allowTemplateLiterals: true},
    ],
    'require-await': ['error'],
    semi: ['error', 'always'],
  },
  settings: {
    react: {
      version: 'detect',
    },
  },
  overrides: [
    {
      files: ['*.js'],
      plugins: ['flowtype'],
      extends: ['plugin:flowtype/recommended'],
      rules: {
        'flowtype/no-types-missing-file-annotation': 'off',
        'flowtype/generic-spacing': [0],
      },
    },
    {
      files: ['*.d.ts'],
      parser: '@typescript-eslint/parser',
      plugins: ['@typescript-eslint'],
      rules: {
        'no-unused-vars': 'off',
        '@typescript-eslint/no-unused-vars': ['error'],
      },
    },
  ],
};
