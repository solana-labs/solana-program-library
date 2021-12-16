import babel from '@rollup/plugin-babel';
import commonjs from '@rollup/plugin-commonjs';
import copy from 'rollup-plugin-copy';
import flowRemoveTypes from 'flow-remove-types';
import json from '@rollup/plugin-json';
import nodeResolve from '@rollup/plugin-node-resolve';
import nodePolyfills from 'rollup-plugin-node-polyfills';
import {terser} from 'rollup-plugin-terser';

function generateConfig(configType, format) {
  const browser = configType === 'browser';
  const bundle = format === 'iife';

  const config = {
    input: 'client/token.js',
    plugins: [
      flow(),
      commonjs(),
      nodeResolve({
        browser,
        preferBuiltins: !browser,
        dedupe: ['bn.js', 'buffer'],
      }),
      babel({
        exclude: '**/node_modules/**',
        babelHelpers: bundle ? 'bundled' : 'runtime',
        plugins: bundle ? [] : ['@babel/plugin-transform-runtime'],
      }),
      copy({
        targets: [{src: 'module.d.ts', dest: 'lib', rename: 'index.d.ts'}],
      }),
    ],
    treeshake: {
      moduleSideEffects: false,
    },
  };

  switch (configType) {
    case 'browser':
      switch (format) {
        case 'esm': {
          config.output = [
            {
              file: 'lib/index.browser.esm.js',
              format: 'es',
              sourcemap: true,
            },
          ];

          // Prevent dependencies from being bundled
          config.external = [
            /@babel\/runtime/,
            'bn.js',
            // Bundled for `Buffer` consistency
            // 'bs58',
            // 'buffer',
            // '@solana/buffer-layout',
            '@solana/web3.js',
          ];

          break;
        }
        case 'iife': {
          config.output = [
            {
              file: 'lib/index.iife.js',
              format: 'iife',
              name: 'splToken',
              sourcemap: true,
            },
            {
              file: 'lib/index.iife.min.js',
              format: 'iife',
              name: 'splToken',
              sourcemap: true,
              plugins: [terser({mangle: false, compress: false})],
            },
          ];

          break;
        }
        default:
          throw new Error(`Unknown format: ${format}`);
      }

      // TODO: Find a workaround to avoid resolving the following JSON file:
      // `node_modules/secp256k1/node_modules/elliptic/package.json`
      config.plugins.push(json());
      config.plugins.push(nodePolyfills());

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
        /@babel\/runtime/,
        'assert',
        'bn.js',
        '@solana/buffer-layout',
        '@solana/web3.js',
      ];
      break;
    default:
      throw new Error(`Unknown configType: ${configType}`);
  }

  return config;
}

export default [
  generateConfig('node'),
  generateConfig('browser', 'esm'),
  generateConfig('browser', 'iife'),
];

// Using this instead of rollup-plugin-flow due to
// https://github.com/leebyron/rollup-plugin-flow/issues/5
function flow() {
  return {
    name: 'flow-remove-types',
    transform: code => ({
      code: flowRemoveTypes(code).toString(),
      map: null,
    }),
  };
}
