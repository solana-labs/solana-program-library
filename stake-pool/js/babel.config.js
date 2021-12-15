// it's needed for jest - https://jestjs.io/docs/getting-started#using-typescript
module.exports = {
  presets: [
    ['@babel/preset-env', {targets: {node: 'current'}}],
    '@babel/preset-typescript',
  ],
};
