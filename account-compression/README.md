# Account Compression

This on-chain program provides an interface for composing smart-contracts to create and use
SPL ConcurrentMerkleTrees. The primary application of using SPL ConcurrentMerkleTrees is
to make edits to off-chain data with on-chain verification. 

Using this program requires an indexer to parse transaction information and write relevant information to an off-chain database.


## SDK 

The typescript SDK for this contract will be generated using Metaplex Foundation's [Solita](https://github.com/metaplex-foundation/solita/). 

## Testing

Testing contracts locally requires the SDK to be built. 
See the SDK folder for instructions.

With a built local SDK, the test suite can be ran with:

1. `yarn link @solana/spl-account-compression`
2. `yarn`
3. `anchor test`
