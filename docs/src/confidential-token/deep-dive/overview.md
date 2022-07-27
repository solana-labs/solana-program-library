---
title: Protocol Overview
---

In this section, we provide the main intuition behind the underlying
cryptography that is used in the confidential token extension. We start from the
regular Token program and incrementally add cryptographic features.

TODO: add a warning regarding

## Tokens with Encryption and Proofs

The main state structures that are used in the Token program are `Mint` and
`Account`. The `Mint` data structure is used to store the global information for
a class of tokens.

```rust
/// Mint data.
struct Mint {
    mint_authority: COption<Pubkey>,
    supply: u64,
    ... // other fields omitted
}
```

The `Account` data structure is used to store the token balance of a user.

```rust
/// Account data.
struct Account {
    mint: Pubkey,
    owner: Pubkey,
    amount: u64,
    ... // other fields omitted
}
```

Users can initialize these two data structures with the `InitializeMint` and
`InitializeAccount` instructions. There are a number of additional instructions
that users can use to modify these states including the `Transfer` instruction.
For the sake of simplicity in this overview, we model a `Transfer` instruction
as follows.

```rust
/// Transfer instruction data
///
/// Accounts expected:
///   0. `[writable]` The source account.
///   1. `[writable]` The destination account.
///   2. `[signer]` The source account's owner.
struct Transfer {
  amount: u64,
}
```

### Encryption

Since an `Account` state is stored on chain, anyone can look up the balance that
is associated with any user. In the confidential extension, we use the most
basic way to hide these balances: keep them in encrypted form. For simplicity,
let us work with any public key encryption (PKE) scheme with the following
syntax.

```rust
trait PKE<Message> {
  type SecretKey;
  type PublicKey;
  type Ciphertext;

  keygen() -> (SecretKey, PublicKey);
  encrypt(PublicKey, Message) -> Ciphertext;
  decrypt(SecretKey, Ciphertext) -> Message;
}
```

Then, consider the following example of an `Account` state.

```rust
Account {
    mint: Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB,
    owner: 5vBrLAPeMjJr9UfssGbjUaBmWtrXTg2vZuMN6L4c8HE6,
    amount: 50,
    ...
}
```

To hide the balance, we can encrypt the balance under the account owner's public
key before storing it on chain.

```rust
Account {
    mint: Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB,
    owner: 5vBrLAPeMjJr9UfssGbjUaBmWtrXTg2vZuMN6L4c8HE6, // pubkey_owner
    amount: PKE::encrypt(pubkey_owner, 50), // amount encrypted
    ...
}
```

We can similarly use encryption to hide transfer amounts in a transaction.

Consider the following example of a transfer instruction.

To hide the transaction amount, we can encrypt it under the sender's public key
before submitting it to the chain.

```rust
Transfer {
  amount: PKE::encrypt(pubkey_owner, 10),
}
```

By simply encrypting account balances and transfer amounts, we can add
confidentiality to the Token program.

### Linear homomorphism

One problem with this simple approach is that the token program cannot deduct or
add transaction amounts to accounts as they are all in encrypted form. One way
to resolve this issue is to use a class of encryption schemes that are _linearly
homomorphic_ such as the ElGamal encryption scheme. An encryption scheme is
linearly homomorphic if for any two numbers `x_0`, `x_1` and their encryptions
`ct_0`, `ct_1` under the same public key, there exist ciphertext-specific add
and subtract operations such that

```rust
let (sk, pk) = PKE::keygen();

let ct_0 = PKE::encrypt(pk, x_0);
let ct_1 = PKE::encrypt(pk, x_1);

assert_eq!(x_0 + x_1, PKE::decrypt(sk, ct_0 + ct_1));
```

In other words, a linearly homomorphic encryption scheme allows numbers to be
added and subtracted in encrypted form. The sum and the difference of the
individual encryptions of `x_0`, `x_1` results in a ciphertext that is
equivalent to an encryption of the sum and the difference of the numbers `x_0`
and `x_1`.

By using a linearly homomorphic encryption scheme to encrypt balances and
transfer amounts, we can allow the token program to process balances and
transfer amounts in encrypted form. As linear homomorphism holds only when
ciphertexts are encrypted under the same public key, we require that a transfer
amount be encrypted under both the sender and receiver public keys.

```rust
Transfer {
  amount_sender: PKE::encrypt(pubkey_sender, 10),
  amount_receiver: PKE::encrypt(pubkey_receiver, 10),
}
```

Then, upon receiving a transfer instruction of this form, the token program can
subtract and add ciphertexts to the source and destination accounts accordingly.

```rust
Account {
    mint: Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB,
    owner: 5vBrLAPeMjJr9UfssGbjUaBmWtrXTg2vZuMN6L4c8HE6, // pubkey_sender
    amount: PKE::encrypt(pubkey_sender, 50) - PKE::encrypt(pubkey_sender, 10),
    ...
}
```

```rust
Account {
    mint: Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB,
    owner: 0x89205A3A3b2A69De6Dbf7f01ED13B2108B2c43e7, // pubkey_receiver
    amount: PKE::encrypt(pubkey_receiver, 50) + PKE::encrypt(pubkey_receiver, 10),
    ...
}
```

### Zero-knowledge proofs

Another problem with encrypting account balances and transfer amounts is that
the token program cannot check the validity of a transfer amount. For example, a
user with an account balance of 50 tokens should not be able to transfer 70
tokens to another account. For regular SPL tokens, the token program can easily
detect that there are not enough funds in a user's account. However, if account
balances and transfer amounts are encrypted, then these values are hidden to the
token program itself, preventing it from verifying the validity of a
transaction.

To fix this, we require that transfer instructions include zero-knowledge proofs
that validate theie correctness.

```
trait ZKP<PublicData, PrivateData> {
  type Proof;

  prove(PublicData, PrivateData) -> Proof;
  verify(PublicData, Proof) -> bool;
}
```

In a transfer instruction, we require the following two TODO

- _Range proof_: Range proofs are special types of zero-knowledge proof systems
  that allow users to generate a proof `proof` that a ciphertext `ct` encrypts a
  value `x` that falls in a specified range `lower_bound`, `upper_bound`:
  ```
  assert!(RangeProof::verify(proof));
  ```
- _Equality proof_
  ```
  assert!(EqualityProof::verify(proof));
  ```

We formally specify model and specify these algorithms in subsequent sections.

## Usability Features

### Encryption key

### Global auditor

### Pending and available balance

## Cryptographic Optimizations

### Dealing with discrete log

### Twisted ElGamal encryption
