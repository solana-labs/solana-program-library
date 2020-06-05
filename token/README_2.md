
### Building

This project cannot be built directly via cargo and instead requires the build scripts located in Solana's BPF-SDK.

To build via NPM, from the repo's root directory:

`$ npm run build:program`

You can also refer to the `build:program` script in `package.json` as an example of how to call the build scripts directly

### Testing

Unit tests contained within this project can be built via:

`$ ./do.sh test`

For additional system tests refer to [program-test](`../program-test`).  System tests are separated into a different project due to conflicts with dependency features

### Clippy

Clippy is also supported via:

`$ ./do.sh clippy`
