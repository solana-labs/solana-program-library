import typescript from '@rollup/plugin-typescript';
import commonjs from '@rollup/plugin-commonjs';
import json from '@rollup/plugin-json';
import nodeResolve from '@rollup/plugin-node-resolve';
import {terser} from 'rollup-plugin-terser';

const extensions = ['.js', '.ts'];

function generateConfig(configType, format) {
  const browser = configType === 'browser';

  const config = {
    input: 'src/index.ts',
    plugins: [
      commonjs(),
      nodeResolve({
        browser,
        dedupe: ['bn.js', 'buffer'],
        extensions,
        preferBuiltins: !browser,
      }),
      typescript(),
    ],
    onwarn: function (warning, rollupWarn) {
      if (warning.code !== 'CIRCULAR_DEPENDENCY') {
        rollupWarn(warning);
      }
    },
    treeshake: {
      moduleSideEffects: false,
    },
  };

  if (configType !== 'browser') {
    // Prevent dependencies from being bundled
    config.external = [
      '@project-serum/borsh',
      '@solana/buffer-layout',
      '@solana/spl-token',
      '@solana/web3.js',
      'bn.js',
      'buffer'
    ];
  }

  switch (configType) {
    case 'browser':
      switch (format) {
        case 'esm': {
          config.output = [
            {
              file: 'dist.browser/index.browser.esm.js',
              format: 'es',
              sourcemap: true,
            },
          ];

          // Prevent dependencies from being bundled
          config.external = [
            '@project-serum/borsh',
            '@solana/buffer-layout',
            '@solana/spl-token',
            '@solana/web3.js',
            'bn.js',
            'buffer'
          ];

          break;
        }
        case 'iife': {
          config.external = ['http', 'https'];

          config.output = [
            {
              file: 'dist.browser/index.iife.js',
              format: 'iife',
              name: 'solanaStakePool',
              sourcemap: true,
            },
            {
              file: 'dist.browser/index.iife.min.js',
              format: 'iife',
              name: 'solanaStakePool',
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

      break;
    case 'node':
      config.output = [
        {
          file: 'dist.browser/index.cjs.js',
          format: 'cjs',
          sourcemap: true,
        },
        {
          file: 'dist.browser/index.esm.js',
          format: 'es',
          sourcemap: true,
        },
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
