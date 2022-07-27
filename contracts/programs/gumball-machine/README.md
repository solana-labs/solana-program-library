# Gumball Machine

## Docs (Maybe)

## Design Decisions

`extension_len` is a property in `GumballMachineHeader` which indicates the length of each `config_line`, which is intended to be a (unique) identifier of each NFT in the Gumball Machine. Each `config_line` corresponds to an index. This index is randomly selected when a user calls `dispense_nft_sol` or `dispense_nft_token`. To enable the creator of the gumball machine to replay the order in which the NFTs were minted from their machine, and to ensure that duplicate `config_lines` are not minted twice, we store both the `config_lines` and a literal array of indices on the `gumball_machine` account. 

The array of indices holds values 0...`max_items` and a randomly selected index from this range is multiplied by `extension_len` to act as a byte index into `config_data` to pull out the `config_line` for the NFT being dispensed.

As per the above, we must initialize (write) 0..`max_items` into the gumball_machine account. When `max_items` is large (as we expect for this project), that can be slow, and exceed compute budget. For that reason, we have seperated the initialization of the indices from the `initialize_gumball_machine` instruction, and created an instruction to initialize the indices in chunks. This instruction may need to be executed multiple times before the indices are fully initialized. See the `./cli/gumball-machine` at the root for convience here.

Lastly, since account space is limited to 10MB. For very large use cases, we wanted to provide the option to omit `config_lines` (as it often takes up the most space in the gumball machine account). In order for your gumball machine to not store any `config_lines` simply set `extension_len` to 0. This will enable users to mint NFTs as soon as your indices are initialized. Note that in this case, the value of the index itself will be used as the "`config_line`". We recommend that you construct an off-chain mapping of these indices to decentralized storage identifiers, or other identifiers used to give your NFTs unique meaning.

