# Metadata

This is a very simple program designed to allow unique metadata tagging to a given mint, with a unique owner
that can change that metadata going forward. The app is composed of 3 actions, one which will create the empty
uninitialized accounts, one which will initialize them, and one which will update some of the fields on them.

## Permissioning and Architecture

Only the authority on a mint can create the unique metadata accounts. The two metadata accounts created are the Metadata account, which holds the Name, Symbol, and URI and the Owner account, which holds a key to the Owner of
the metadata. The authority on a mint is responsible for calling create_metadata_accounts to create uninitialized
allocated accounts on the chain and then initializing them in a follow up call to init_metadata_accounts.

To ensure the uniqueness of a mint's metadata, the address of a Metadata account is a PDA composed of seeds:

```rust
["metadata".as_bytes(), program_id.as_ref(), mint_key.as_ref()]
```

While the Owner address is a PDA composed of seeds:

```rust
["metadata".as_bytes(), program_id.as_ref(), name_as_bytes, symbol_as_bytes]
```

This ensures easy lookups by those interested - they can simply look up the metadata account by mint id, then
look up owner with the name and symbol if they need it.

The owner can only call the update_metadata_accounts command, which right now can only update the URI.

Due to the nature of the addresses on these accounts, name and symbol are immutable.

## create_metadata_accounts

(Mint authority must be signer)

This action creates the Owner and Metadata accounts. It can't both create and initialize the accounts because in
order to set the data on the accounts it needs to have the passed in account_infos be writable and have data
arrays at the time of scope start, but this is not so when the scope actually starts in this action.

Because both of these accounts have PDAs as addresses, the user cannot make these accounts
via separate system calls in the transaction beforehand, they must rely on this app to do it for them.

Since this action is the one making those calls, prior to making those calls, the account_infos that
do get passed in have zero-length arrays and are not writable. Trying to deserialize them AFTER making
those raw account create calls will still be attempting to deserialize zero-length arrays.

So we must do the initialization of these accounts in a separate follow up command.

## init_metadata_accounts

(Mint authority must be signer)

This call is called second by the mint authority, after create_metadata_accounts, and sets the data on Owner and
Metadata.

## update_metadata_accounts

(Owner must be signer)

This call can be called at any time by the owner to update the URI, and later other fields.

## Further extensions

This app is designed to be extended with further account buckets. If say, we wanted to add metadata for youtube
metadata, we could create a new struct called Youtube and seed it with the seed

```rust
["metadata".as_bytes(), program_id.as_ref(), mint_key.as_ref(), "youtube".as_bytes()]
```

And then only those interested in that metadata need search for it, and it's uniqueness is ensured. It can also
have it's own update action that follows a similar pattern to the original update action.
