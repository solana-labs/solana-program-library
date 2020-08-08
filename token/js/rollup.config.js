import babel from '@rollup/plugin-babel';
import commonjs from '@rollup/plugin-commonjs';
import copy from 'rollup-plugin-copy';

function generateConfig(configType) {
  const config = {
    input: 'client/token.js',
    plugins: [
      babel({
        configFile: './babel.rollup.config.json',
        exclude: 'node_modules/**',
        babelHelpers: 'runtime',
      }),
      commonjs(),
      copy({
        targets: [{src: 'module.d.ts', dest: 'lib', rename: 'index.d.ts'}],
      }),
    ],
  };

  switch (configType) {
    case 'browser':
      // TODO: Add support
      break;
    case 'node':
      config.output = [
        {
          file: 'lib/index.cjs.js',
          format: 'cjs',
          sourcemap: true,
        },
        {
          file: 'lib/index.esm.js',
          format: 'es',
          sourcemap: true,
        },
      ];

      // Quash 'Unresolved dependencies' complaints for modules listed in the
      // package.json "dependencies" section.  Unfortunately this list is manually
      // maintained.
      config.external = [
        'assert',
        '@babel/runtime/core-js/get-iterator',
        '@babel/runtime/core-js/json/stringify',
        '@babel/runtime/core-js/object/assign',
        '@babel/runtime/core-js/object/get-prototype-of',
        '@babel/runtime/core-js/object/keys',
        '@babel/runtime/core-js/promise',
        '@babel/runtime/helpers/asyncToGenerator',
        '@babel/runtime/helpers/classCallCheck',
        '@babel/runtime/helpers/createClass',
        '@babel/runtime/helpers/defineProperty',
        '@babel/runtime/helpers/get',
        '@babel/runtime/helpers/getPrototypeOf',
        '@babel/runtime/helpers/inherits',
        '@babel/runtime/helpers/possibleConstructorReturn',
        '@babel/runtime/helpers/slicedToArray',
        '@babel/runtime/helpers/toConsumableArray',
        '@babel/runtime/helpers/typeof',
        '@babel/runtime/regenerator',
        'bn.js',
        'buffer-layout',
        '@solana/web3.js',
      ];
      break;
    default:
      throw new Error(`Unknown configType: ${configType}`);
  }

  return config;
}

export default [generateConfig('node')];
