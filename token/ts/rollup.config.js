import commonjs from '@rollup/plugin-commonjs';
import json from '@rollup/plugin-json';
import nodeResolve from '@rollup/plugin-node-resolve';
import typescript from '@rollup/plugin-typescript';
import nodePolyfills from 'rollup-plugin-polyfill-node';

function config(target, format) {
    const config = {
        plugins: [typescript()],
        input: 'src/index.ts',
        output: {
            file: `lib/index${target === 'node' ? '' : '.' + target}.${format}.js`,
            format: format === 'esm' ? 'es' : format,
            sourcemap: true,
        },
        external: [],
        treeshake: {
            moduleSideEffects: false,
        },
    };

    if (target === 'browser' && format === 'cjs') {
        config.plugins.push(commonjs(), json(), nodeResolve({ browser: true, preferBuiltins: false }), nodePolyfills());
    }

    if (target === 'node' || format === 'esm') {
        config.external.push('@solana/buffer-layout', '@solana/buffer-layout-utils', '@solana/web3.js');
    }

    return config;
}

export default [config('node', 'esm'), config('node', 'cjs'), config('browser', 'esm'), config('browser', 'cjs')];
