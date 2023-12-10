---
title: Protocol Overview
---

In this section, we provide an overview of the underlying cryptographic protocol
for the confidential Token extension. An understanding of the details that are
discussed in the following subsections is not needed to actually use the
confidential extension. We refer to the previous section for a quick start
guide.

We note that this overview exists mainly to provide the design intuition behind
the underlying cryptography that is used in the confidential extension. Some parts of
the description of the protocol in the overview could differ from the actual
implementation. We refer to the subsequent subsections, the [source
code](https://github.com/solana-labs/solana-program-library), and the
documentation within for the precise details of the underlying cryptography.

## Tokens with Encryption and Proofs

The main state data structures that are used in the Token program are `Mint` and
`Account`. The `Mint` data structure is used to store the global information for
a class of tokens.

```rust
/// Mint data.
struct Mint {
    mint_authority: Option<Pubkey>,
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
that users can use to modify these states. For this overview, we focus on the
`Transfer` instruction. For the sake of simplicity in this section, let us model
a `Transfer` instruction with the following structure.

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
basic way to hide these balances: encrypt them using a public key encryption
scheme (PKE). For simplicity, let us model a public key encryption scheme with
the following syntax.

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

Now, consider the following example of an `Account` state.

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
Consider the following example of a transfer instruction. To hide the
transaction amount, we can encrypt it under the sender's public key before
submitting it to the chain.

```rust
Transfer {
  amount: PKE::encrypt(pubkey_owner, 10),
}
```

By simply encrypting account balances and transfer amounts, we can add
confidentiality to the Token program.

### Linear homomorphism

One problem with this simple approach is that the Token program cannot deduct or
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
transfer amounts, we can allow the Token program to process balances and
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
that validate their correctness. Put simply, zero-knowledge proofs consist of
two pair of algorithms `prove` and `verify` that work over public and private
data. The `prove` algorithm generates a "proof" that certifies that some
property of the public and private data is true. The `verify` algorithm checks
that the proof is valid.

```rust
trait ZKP<PublicData, PrivateData> {
  type Proof;

  prove(PublicData, PrivateData) -> Proof;
  verify(PublicData, Proof) -> bool;
}
```

A special property of a zero-knowledge proof system is that a proof does not
reveal any information about the actual private data.

In a transfer instruction, we require the following special classes of
zero-knowledge proofs.

- _Range proof_: Range proofs are special types of zero-knowledge proof systems
  that allow users to generate a proof `proof` that a ciphertext `ct` encrypts a
  value `x` that falls in a specified range `lower_bound`, `upper_bound`:

  - For any `x` such that `lower_bound <= x < upper_bound`:

  ```rust
  let ct = PKE::encrypt(pk, x);
  let public_data = (pk, ct);
  let private_data = (sk, x);

  let proof = RangeProof::prove(public_data, private_data);
  assert_eq!(RangeProof::verify(public_data, proof), true);
  ```

  - Let `x` be any value that falls out of the bounds. Then for any `proof: Proof`:

  ```rust
  let ct = PKE::encrypt(pk, x);
  let public_data = (pk, ct);

  assert_eq!(RangeProof::verify(public_data, proof), false);
  ```

  The zero-knowledge property guarantees that the generated proof does not
  reveal the actual value of the input `x`, but only the fact that
  `lower_bound <= x < upper_bound`.

  In the confidential extension, we require that a transfer instruction includes
  a range proof that certifies the following:

  - The proof should certify that there are enough funds in the source account.
    Specifically, let `ct_source` be the encrypted balance of a source account
    and `ct_transfer` be the encrypted transfer amount. Then we require that
    `ct_source - ct_transfer` encrypts a value `x` such that `0 <= x < u64::MAX`.

  - The proof should certify that the transfer amount itself is a positive
    64 bit number. Let `ct_transfer` be the encrypted amount of a transfer. Then
    the proof should certify that `ct_transfer` encrypts a value `x` such that
    `0 <= x < u64::MAX`.

- _Equality proof_: Recall that a transfer instruction contains two ciphertexts
  of the transfer value `x`: a ciphertext under the sender public key
  `ct_sender = PKE::encrypt(pk_sender, x)` and one under the receiver public key
  `ct_receiver = PKE::encrypt(pk_receiver, x)`. A malicious user can encrypt two
  different values for `ct_sender` and `ct_receiver`.

  Equality proofs are special types of zero-knowledge proof systems that allow
  users to prove that two ciphertexts `ct_0`, `ct_1` encrypt a same value `x`.
  In the confidential extension program, we require that a transfer instruction
  contains an equality proof that certifies that the two ciphertexts encrypt the
  same value.

  The zero-knowledge property guarantees that `proof_eq` does not reveal the
  actual values of `x_0`, `x_1` but only the fact that `x_0 == x_1`.

We formally model and specify these algorithms in the subsequent sections.

## Usability Features

### Encryption key

In the previous section, we used the public key of the account owner to encrypt
the balance of an account. In the actual implementation of the confidential
extension program, we use a separate account-specific encryption key to encrypt
the account balances.

```rust
Account {
    mint: Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB,
    owner: 5vBrLAPeMjJr9UfssGbjUaBmWtrXTg2vZuMN6L4c8HE6,
    encryption_key: mpbpvs1LksLmdMhCEzyu5UEWEb3dsRPbB5, // pke_pubkey
    amount: PKE::encrypt(pke_pubkey, 50),
    ...
}
```

The account-specific `encryption_key` can be set by the owner of the account
using the `ConfidentialTransferInstruction::ConfigureAccount` instruction. A
corresponding secret key can be stored privately on a client-side wallet or
can also be deterministically derived from an owner signing key.

In general, a direct re-use of a signing key for encryption is discouraged for
potential vulnerabilities. The confidential extension is designed to be as
general as possible. Separate dedicated keys for signing transactions and
decrypting transaction amounts allow for a more flexible interface.

In a potential application, the decryption key for specific accounts can be
shared among multiple users (e.g. regulators) that should have access to an
account balance. Although these users can decrypt account balances, only the
owner of the account who has access to the owner signing key can sign a
transaction that initiates a transfer of tokens. The owner of an account can
update the account with a new encryption key using the `ConfigureAccount`.

### Global auditor

As separate decryption keys are associated with each user accounts, users can
provide read access to balances of _specific_ accounts to potential auditors.
The confidential extension also allows a _global_ auditor feature that can be
optionally enabled for mints. Specifically, in the confidential extension, the
mint data structure maintains an additional global auditor encryption key. This
auditor encryption key can be specified when the mint is first initialized and
updated via the `ConfidentialTransferInstruction::ConfigureMint` instruction. If
the transfer auditor encryption key in the mint is not `None`, then any transfer
instruction must additionally contain an encryption of the transfer amount under
the auditor's encryption key.

```rust
Transfer {
  amount_sender: PKE::encrypt(pke_pubkey_sender, 10),
  amount_receiver: PKE::encrypt(pke_pubkey_receiver, 10),
  amount_auditor: PKE::encrypt(pke_pubkey_auditor, 10),
  range_proof: RangeProof,
  equality_proof: EqualityProof,
  ...
}
```

This allows any entity with a corresponding auditor secret key to be able to
decrypt any transfer amounts for a particular mint.

Similarly to how a dishonest sender can encrypt inconsistent transfer amounts
under the source and destination keys, it can encrypt inconsistent transfer
amount under the auditor encryption key. If the auditor encryption key is not
`None` in the mint, then the token program requires that a transfer amount in a
transfer instruction contain additional zero-knowledge proof that certifies that
the encryption is done consistently.

### Pending and available balance

One way an attacker can disrupt the use of a confidential extension account is
by using _front-running_. Zero-knowledge proofs are verified with respect to the
encrypted balance of an account. Suppose that a user Alice generates a proof
with respect to her current encrypted account balance. If another user Bob
transfers some tokens to Alice, and Bob's transaction is processed first, then
Alice's transaction will be rejected by the Token program as the proof will not
verify with respect to the newly updated account state.

Under normal conditions, upon a rejection by the program, Alice can simply look
up the newly updated ciphertext and submit a new transaction. However, if a
malicious attacker continuously floods the network with a transfer to Alice's
account, then the account may theoretically become unusable. To prevent this
type of attack, we modify the account data structure such that the encrypted
balance of an account is divided into two separate components: the _pending_
balance and _available_ balance.

```rust
let ct_pending = PKE::encrypt(pke_pubkey, 10);
let ct_available = PKE::encryption(pke_pubkey, 50);

Account {
    mint: Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB,
    owner: 5vBrLAPeMjJr9UfssGbjUaBmWtrXTg2vZuMN6L4c8HE6,
    encryption_key: mpbpvs1LksLmdMhCEzyu5UEWEb3dsRPbB5,
    pending_balance: ct_pending,
    account_balance: ct_available,
    ...
}
```

Any outgoing funds from an account are subtracted from its available balance. Any
incoming funds to an account is added to its pending balance.

As an example, consider a transfer instruction that moves 10 tokens from a
sender's account to a receiver's account.

```rust
let ct_transfer_sender = PKE::encrypt(pke_pubkey_sender, 10);
let ct_transfer_receiver = PKE::encrypt(pke_pubkey_receiver, 10);
let ct_transfer_auditor = PKE::encrypt(pke_pubkey_auditor, 10);

Transfer {
  amount_sender: ct_transfer_sender,
  amount_receiver: ct_transfer_receiver,
  amount_auditor: ct_transfer_auditor,
  range_proof: RangeProof,
  equality_proof: EqualityProof,
  ...
}
```

Upon receiving this transaction and after verifying, the Token program subtracts
the encrypted amount from the sender's available balance and adds the encrypted
amount to the receiver's pending balance.

```rust
Account {
    mint: Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB,
    owner: 5vBrLAPeMjJr9UfssGbjUaBmWtrXTg2vZuMN6L4c8HE6,
    encryption_key: mpbpvs1LksLmdMhCEzyu5UEWEb3dsRPbB5,
    pending_balance: ct_sender_pending,
    available_balance: ct_sender_available - ct_transfer_sender,
    ...
}
```

This modification removes the sender's ability to change the receiver's
available balance of a source account. As range proofs are generated and
verified with respect to the available balance, this prevents a user's
transaction from being invalidated due to a transaction that is generated by
another user.

An account's pending balance can be merged into its available balance via the
`ApplyPendingBalance` instruction, which only the owner of the account can
authorize. Upon receiving this instruction and after verifying that the owner of
the account signed the transaction, the token program adds the pending balance
into the available balance.

```rust
Account {
    mint: Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB,
    owner: 5vBrLAPeMjJr9UfssGbjUaBmWtrXTg2vZuMN6L4c8HE6,
    encryption_key: mpbpvs1LksLmdMhCEzyu5UEWEb3dsRPbB5,
    pending_balance: ct_pending_receiver - ct_transfer_receiver,
    available_balance: ct_available_receiver,
    ...
}
```

## Cryptographic Optimizations

### Dealing with discrete log

A well-known limitation of using linearly-homomorphic ElGamal encryption is the
inefficiency of decryption. Even with a proper secret key, in order to recover
the originally encrypted value, one must solve a computational problem called
the _discrete logarithm_, which requires an exponential time to solve. In the
confidential extension program, we address this issue in the following two ways:

- Transfer amounts are restricted to 48-bit numbers.
- Transfer amounts and account pending balances are encrypted as two
  independent ciphertexts.
- Account available balances are additionally encrypted using a symmetric
  encryption scheme.

We refer to the subsequent sections and the documentation in the source code for
additional details.

### Twisted ElGamal encryption

A key challenge in designing any private payment system is minimizing the size
of a transaction. In the confidential extension, we make a number of
optimizations that reduces the transaction size. Among these optimizations, a
significant amount of savings stem from the use of the _twisted_ ElGamal
encryption (formulated in [CMTA19](https://eprint.iacr.org/2019/319)). The
twisted ElGamal encryption is a simple variant of the standard ElGamal
encryption scheme where a ciphertext is divided into two components:

- A _Pedersen commitment_ of the encrypted message, which is independent of any
  ElGamal public key.
- A _decryption handle_ that encodes the encryption randomness with respect to a
  specific ElGamal public key, and is independent of the encrypted message.

We provide the formal details of the twisted ElGamal encryption in the
subsequent sections.
