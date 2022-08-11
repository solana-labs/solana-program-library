# Farm SDK

Farm SDK is a lower-level Rust library with a common code that is used by all Solana Farms tools and contracts. You might only need this SDK if you plan to build your own on-chain program or custom client that uses some of the SDK's functionality. Otherwise you should be using existing [HTTP Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/http_client.md) or [Rust Client](https://github.com/solana-labs/solana-program-library/blob/master/farms/docs/rust_client.md).

To use the library, specify it in the `[dependencies]` section of your Cargo.toml, e.g.:

```
[dependencies]
solana-farm-sdk = "1.1.2"
```

The best way to learn what can be done with SDK is to look at the source code, which can be found [here](https://github.com/solana-labs/solana-program-library/tree/master/farms/farm-sdk/src).
