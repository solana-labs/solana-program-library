# Account Compression

This on-chain program provides an interface for composing smart-contracts to create and use
SPL ConcurrentMerkleTrees. The primary application of using SPL ConcurrentMerkleTrees is
to make edits to off-chain data with on-chain verification. 

Using this program requires an indexer to parse transaction information and write relevant information to an off-chain database.


## SDK 

The typescript SDK for this contract will be generated using Metaplex Foundation's [Solita](https://github.com/metaplex-foundation/solita/). 

## Testing

Testing contracts locally requires the SDK to be built. Then you can run: `anchor test`

Testing contracts against indexer + api: `anchor test --skip-build --skip-local-validator --skip-deploy` and limit the test script to only the continuous test.
