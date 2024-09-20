---
title: Presentation
---

- Why a new token program?
- What are extensions?
- How can I get you excited with an FAQ?

---

### Why

- SPL Token works and is battle-tested
- ...but it needs more protocol-level functionality, without impacting existing tokens
- Let's deploy a new and separate token program 
- ...even though it's no longer 2022!

---

### Wait, are you sure about this?

- Adopting a separate token program is tricky
- ...but extremely valuable to the ecosystem
- Going from 1 to 2 is hard, but 2 to N is easy

---

### Are you aware that it's not 2022?

Yes.

---

### Ok... how does it work?

- Token-2022 is a superset of Token: structures and instructions have the same ABI
- Opt-in to *extensions* on mints and accounts
- New data is written after the 165th byte

---

### Cool, but can I even use this?

- Yes! Out on all networks *for testing*
- `solana` tools version >= 1.14.17
- `@solana/spl-token` version >= 0.3
- `spl-token-cli` version >= 2.2

---

### Who supports it?

The base is mostly there.

- RPC indexes Token-2022
- Anchor
- Wallets
- DeFi Protocols
- Token Metadata

---

### That's great! Is it safe?

- 4 audits
- 1 more after WIP features
- Currently upgradeable
- Officially recommended after 1.17 on mainnet (~January 2024)
- More ZK features in 1.18 (~May 2024)
- May be frozen ~6 months after that

---

### I'll bite: what are the extensions for accounts?

- Confidential transfers
- CPI guard
- Memo required on transfer
- Immutable ownership

---

### Not bad, what are the extensions for mints?

- Confidential transfers
- Transfer fees
- Closing mint
- Interest-bearing tokens
- Non-transferable tokens
- Default account state
- Permanent delegate
- Transfer-hook
- Metadata pointer + metadata
- Group pointer + group

---

### Wow that's a lot!

Yeah.

---

### I don't get what they're for.

Let's learn with a game!

- Describe a token design
- Think about how to do it with Token-2022
- I give the answer

Hint: the answers are in the CLI docs at https://spl.solana.com/token-2022/extensions

---

### Question 1

I heard about compressed NFTs, so how can I make a token that can be compressed,
decompressed, and recompressed with an off-chain merkle tree?

---

### Answer 1

Create a mint with the close mint authority extension, so you can close and
re-open the mint account when the supply is 0.

---

### Question 2

I want to send my token without anyone knowing how much I have or how much I transferred.

---

### Answer 2

Add the confidential transfer extension to your mint!

Although the first deposit is public, transfer amounts are encrypted and
validated through zero-knowledge proofs.

* Used to require larger transaction sizes, but instead we're splitting
up the proofs!

---

### Question 3

I run a stake pool / lending protocol, and I want the pool token amount to go up
over time to approximate the value of the token.

---

### Answer 3

Create a mint with the interest-bearing extension, and have the protocol update
the interest rate every epoch.

---

### Question 4

I'm creating a bank-like payments system, and I want to create legible monthly
statements for my clients.

And I don't want them to get rugged by sketchy protocols.

---

### Answer 4

Enforce that all client token accounts require memos on incoming transfers.
Clients can figure out the motive for all funds coming into their account.

Also add the CPI guard extension, to force dapp transfers to go through a delegate.

---

### Question 5

For my game, I only want players to hold my token, and I don't want them
to dump it on an exchange.

---

### Answer 5

Create the mint with the default account state extension, set to `frozen`. Players
must go through your program or service to unfreeze their account.

---

### Question 6

My DAO needs a privileged token for council members.

I don't want them to sell or move the tokens, and the DAO must be able to
revoke the token if they behave poorly.

---

### Answer 6

Create a mint with:
- permanent delegation to the DAO, so it can burn any token
- non-transferable, so members can't move them
- Bonus: non-transferable forces immutable ownership

---

### Question 7

There's definitely a lot of new features, but I just want to program my own
token.

---

### Answer 7

This isn't possible currently. We need to develop a suite of interfaces and move
everyone to using them.

In the meantime, you can configure your token-2022 mint to call into a program that
implements the "transfer hook" interface.

More info at https://github.com/solana-labs/solana-program-library/tree/master/token/transfer-hook-interface

---

### Question 8

You mentioned something about metadata. Does this mean there's going to be more
than one metadata program? That sounds like chaos.

---

### Answer 8

It could be! That's why the "metadata pointer" extension in token-2022 lets you
specify which account holds the metadata for your mint.

For safety, you *must* make sure that the mint and metadata point at each other.

---

### Question 9

Can't we just put the metadata in the mint?

---

### Answer 9

Yes! With the WIP "metadata" extension, you just put everything in the mint.

---

### Question 10

These features sound awesome, but I already have lots of token holders,
so how can I migrate them to Token-2022?

---

### Answer 10

Create a new mint with Token-2022, and have them use the `spl-token-upgrade`
program to convert.

- Stateless protocol with an escrow account
- Mint new tokens to the escrow
- Protocol burns old tokens and gives new tokens

Fun fact: you can use this between any two mints!

---

### Question 11

Yeah, hi, same as number 10, but I don't want to burn tokens.

---

### Answer 11

That's fine! The WIP `token-wrap` program allows you to wrap between any two mints.

Note: the default wrapping program does not add extensions, but can be forked
into a new program if you want to wrap your token with extensions.

---

### Question 12

I have an on-chain program (smart contract), how can I add support for Token-2022?

---

### Answer 12

That's awesome! If you only process one token in an instruction, it's easy.

If you use multiple token programs at once (e.g. trading), it's trickier since
you need both programs in your instruction.

Extensive docs and examples at https://spl.solana.com/token-2022/onchain

---

### Question 13

I work on a wallet, so how can I show and transfer Token-2022 tokens?

---

### Answer 13

Nice! It's pretty easy to add support.

Docs and examples at https://spl.solana.com/token-2022/wallet

---

### Question 14

Why did you add metadata?

---

### Answer 14

- On-chain programming should become more open
- People kept bothering us about it

---

### Question 15

What if I don't want to use your metadata?

---

### Answer 15

- No problem, bring your own!
- The "metadata pointer" extension lets you point to *any* account
- You can also implement the "SPL Token Metadata Interface" in your program

Security bonus: check that the mint and metadata point to each other!

---

### Question 16

Can I just use my own token program?

---

#### Answer 16

- That's the future! In the meantime, we have Transfer Hooks
- With a Transfer Hook, Token-2022 calls a program of your choice during all
transfers for your mint
- The program must implement `spl-transfer-hook-interface`
- Feel free to fork `spl-transfer-hook-example`

---

### I'm a bit overwhelmed

No problem, we're done, here are your links:

- Token-2022: https://spl.solana.com/token-2022
- Token-upgrade: https://spl.solana.com/token-upgrade
- Metadata interface: https://docs.rs/crate/spl-token-metadata-interface/latest
- Transfer hook interface: https://docs.rs/crate/spl-transfer-hook-interface/latest
- Confidential transfers: https://github.com/solana-labs/solana-program-library/blob/master/token/zk-token-protocol-paper/part1.pdf

Thanks for listening!
