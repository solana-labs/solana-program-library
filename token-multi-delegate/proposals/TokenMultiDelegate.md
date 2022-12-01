# Token multi delegate

The token program allows only a single delegate per token account. While this allows interacting with a single program conveniently it easily interfers when multiple programs are involved.

On ethereum the approval + transferFrom for spending tokens is per contract, as a result people can trivially allow multiple contracts to spend their balance.

## Terms

- Multi delegate: The state or set of states that allow the multi delegate mechanism
- Multi delegate authority: the PDA signing as the token-program token account delegate, it can be the multi delegate itself if it is a PDA

## Design consideration

- The user needs to be able to revoke any delegate at any time, independently from other delegates
- Since the multi delegate relies on the token program delegate, there will be some UX issues related to a tradeoff between approving the multi delegate authority for the sum of delegated amounts or for the maximum delegated amount (OR delegates vs AND delegates)
- When interacting once with for instance Orca, it will wipe the delegation to the multi delegate authority. How to avoid a clash with ephemeral keypair approvals?
- To allow easy discovery we might want to allow a single canonical multi delegate account per token account per user (because token accounts ownership can be transfered), a canonical PDA to derive `seeds = [wallet_address.as_ref(), token_account_address.as_ref()]`

## Technical

1. One multi delegate data structure

```rust
struct Delegate {
    authority: Pubkey,
    amount: u64,
}

struct MultiDelegate {
    owner: Pubkey,
    delegates: Vec<Delegate>, // We can leverage realloc but doesn't scale particularly well, could be a zero copy hashmap?
}
```

User process: User initializes a multi delegate, token-program approves the multi delegate authority and adds delegates to the multi delegate

## Use cases

- Create ask/bid orders for fungible tokens in multiple platforms
- Put non fungible assets (NFT) for sale on multiple marketplaces, also bid on multiple marketplaces without ever locking funds in escrow
- Allowing spending/streaming in multiple programs of an agreed upon amount without ever escrowing amounts

## Links
1. Ethereum ERC20 approve https://docs.openzeppelin.com/contracts/2.x/api/token/erc20#IERC20-approve-address-uint256-
2. Sokoban: Compact, efficient data structures https://github.com/Ellipsis-Labs/sokoban
3. Cardinal labs token manager which specializes in NFTs https://github.com/cardinal-labs/cardinal-token-manager