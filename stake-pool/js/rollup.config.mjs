import typescript from '@rollup/plugin-typescript';
import commonjs from '@rollup/plugin-commonjs';
import json from '@rollup/plugin-json';
import nodeResolve from '@rollup/plugin-node-resolve';
import terser from '@rollup/plugin-terser';

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

  if (browser) {
    if (format === 'iife') {
      config.external = ['http', 'https'];

      config.output = [
        {
          file: 'dist/index.iife.js',
          format: 'iife',
          name: 'solanaStakePool',
          sourcemap: true,
        },
        {
          file: 'dist/index.iife.min.js',
          format: 'iife',
          name: 'solanaStakePool',
          sourcemap: true,
          plugins: [terser({ mangle: false, compress: false })],
        },
      ];
    } else {
      config.output = [
        {
          file: 'dist/index.browser.cjs.js',
          format: 'cjs',
          sourcemap: true,
        },
        {
          file: 'dist/index.browser.esm.js',
          format: 'es',
          sourcemap: true,
        },
      ];

      // Prevent dependencies from being bundled
      config.external = [
        '@coral-xyz/borsh',
        '@solana/buffer-layout',
        '@solana/spl-token',
        '@solana/web3.js',
        'bn.js',
        'buffer',
      ];
    }

    // TODO: Find a workaround to avoid resolving the following JSON file:
    // `node_modules/secp256k1/node_modules/elliptic/package.json`
    config.plugins.push(json());
  } else {
    config.output = [
      {
        file: 'dist/index.cjs.js',
        format: 'cjs',
        sourcemap: true,
      },
      {
        file: 'dist/index.esm.js',
        format: 'es',
        sourcemap: true,
      },
    ];

    // Prevent dependencies from being bundled
    config.external = [
      '@coral-xyz/borsh',
      '@solana/buffer-layout',
      '@solana/spl-token',
      '@solana/web3.js',
      'bn.js',
      'buffer',
    ];
  }

  return config;
}

export default [
  generateConfig('node'),
  generateConfig('browser'),
  generateConfig('browser', 'iife'),
];
