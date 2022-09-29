# SPL Account Compression Indexer

This indexer is meant to exemplify how to parse transactions with Account Compression instructions.

### How it works

`Backfill` takes a Concurrent Merkle Tree account, and backfills all on-chain merkle tree updates into a Sqlite3 database.

As the tree backfills, you can construct Merkle tree proofs to modify your on-chain merkle tree.

### Limitations of this toolset

Transactions that only interact with `acount-compression` via level 2 CPI depth or greater are unsupported with our default parsing at the moment.

This is because transactions lack depth for instructions in the `innerInstructions` field of the RPC response. 

Note: it is still **possible** to index calls made to `account-compression` of CPI depth 2 or greater. 
It just requires custom logic based upon the pattern of how your program invokes `account-compression`.


Once this issue is addressed, we can support indexing general-depth `account-compression` calls.
