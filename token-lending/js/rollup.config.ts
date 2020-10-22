import commonjs from '@rollup/plugin-commonjs'
import json from 'rollup-plugin-json'
import resolve from 'rollup-plugin-node-resolve'
import sourceMaps from 'rollup-plugin-sourcemaps'
import typescript from 'rollup-plugin-typescript2'

const pkg = require('./package.json')

export default {
  input: `client/index.ts`,
  output: [
    { file: pkg.main, format: 'cjs', sourcemap: true },
    { file: pkg.module, format: 'es', sourcemap: true }
  ],
  external: [
    'assert',
    'url',
    'http',
    'https',
    'crypto',
    'buffer',
    'path',
    'stream',
    'zlib',
    'events',
    'tls',
    'net',
    'os',
    'fs'
  ],
  watch: {
    include: 'src/**'
  },
  plugins: [
    json(),
    typescript({useTsconfigDeclarationDir: false}),
    commonjs(),
    resolve({preferBuiltins: true}),
    sourceMaps()
  ]
}
