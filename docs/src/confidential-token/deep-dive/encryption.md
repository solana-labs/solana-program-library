---
title: Encryption
---

The confidential extension program makes use of a public key encryption scheme
and an authenticated symmetric encryption scheme. For public key encryption, the
program uses the [twisted ElGamal encryption](https://eprint.iacr.org/2019/319)
scheme. For symmetric encryption, it uses
[AES-GCM-SIV](https://datatracker.ietf.org/doc/html/rfc8452).

## Twisted ElGamal Encryption

The twisted ElGamal encryption scheme is a simple variant of the standard
ElGamal encryption scheme where a ciphertext is divided into two components:

- A Pedersen commitment of the encrypted message. This component is independent
  of the public key.
- A "decryption handle" that binds the encryption randomness with respect to a
  specific ElGamal public key. This component is independent of the actually
  encrypted message.

The structure of the twisted ElGamal ciphertexts simplifies their design of some
zero-knowledge proof systems. Furthermore, since the encrypted messages are
encoded as Pedersen commitments, many of the existing zero-knowledge proof
systems that are designed to work specifically for Pedersen commitments can be
directly used on the twisted ElGamal ciphertexts.

We provide the formal description of the twisted ElGamal encryption in the
[notes](./twisted_elgamal.pdf).

### Ciphertext Decryption

One aspect that makes the use of the ElGamal encryption cumbersome in protocols
is the inefficiency of decryption. The decryption of an ElGamal ciphertext grows
exponentially with the size of the encrypted number. With modern hardware, the
decryption of 32-bit messages can be in the order of seconds, but it quickly
becomes infeasible as the message size grows. A standard Token account stores
general `u64` balances, but an ElGamal ciphertext that encrypts large 64-bit
values are not decryptable. Therefore, extra care is put into the way the
balances and transfer amounts are encrypted and handled in the account state and
transfer data.

## Account State

If the decryption of the twisted ElGamal encryption scheme were fast, then a
confidential transfer account and a confidential instruction data could be
modeled as follows:

```rust
struct ConfidentialTransferAccount {
  /// `true` if this account has been approved for use. All confidential
  /// transfer operations for
  /// the account will fail until approval is granted.
  approved: PodBool,

  /// The public key associated with ElGamal encryption
  encryption_pubkey: ElGamalPubkey,

  /// The pending balance (encrypted by `encryption_pubkey`)
  pending_balance: ElGamalCiphertext,

  /// The available balance (encrypted by `encryption_pubkey`)
  available_balance: ElGamalCiphertext,
}
```

```rust
// Actual cryptographic components are organized in `VerifyTransfer`
// instruction data
struct ConfidentialTransferInstructionData {
  /// The transfer amount encrypted under the sender ElGamal public key
  encrypted_amount_sender: ElGamalCiphertext,
  /// The transfer amount encrypted under the receiver ElGamal public key
  encrypted_amount_receiver: ElGamalCiphertext,
}
```

Upon receiving a transfer instruction, the Token program aggregates
`encrypted_amount_receiver` into the account `pending_balance`.

The actual structures of these two components are more involved. Since the
`TransferInstructionData` requires zero-knowledge proof components, we defer the
discussion of its precise structure to the next subsection and focus on
`ConfidentialTransferAccount` here. We start from the ideal
`ConfidentialTransferAccount` structure above and incrementally modify it to
produce the final structure.

### Available Balance

If the available balance is encrypted solely as general `u64` values, then it
becomes infeasible for clients to decrypt and recover the exact balance in an
account. Therefore, in the Token program, the available balance is additionally
encrypted using an authenticated symmetric encryption scheme. The resulting
ciphertext is stored as the `decryptable_balance` of an account and the
corresponding symmetric key should either be stored on the client side as an
independent key or be derived on-the-fly from the owner signing key.

```rust
struct ConfidentialTransferAccount {
  /// `true` if this account has been approved for use. All confidential
  /// transfer operations for
  /// the account will fail until approval is granted.
  approved: PodBool,

  /// The public key associated with ElGamal encryption
  encryption_pubkey: ElGamalPubkey,

  /// The pending balance (encrypted by `encryption_pubkey`)
  pending_balance: ElGamalCiphertext,

  /// The available balance (encrypted by `encryption_pubkey`)
  available_balance: ElGamalCiphertext,

  /// The decryptable available balance
  decryptable_available_balance: AeCiphertext,
}
```

Since `decryptable_available_balance` is easily decryptable, clients should
generally use it to decrypt the available balance in an account. The
`available_balance` ElGamal ciphertext should generally only be used to generate
zero-knowledge proofs when creating a transfer instruction.

The `available_balance` and `decryptable_available_balance` should encrypt the
same available balance that is associated with the account. The available
balance of an account can change only after an `ApplyPendingBalance` instruction
and an outgoing `Transfer` instruction. Both of these instructions require a
`new_decryptable_available_balance` to be included as part of their instruction
data.

### Pending Balance

Like in the case of the available balance, one can consider adding a
`decryptable_pending_balance` to the pending balance. However, whereas the
available balance is always controlled by the owner of an account (via the
`ApplyPendingBalance` and `Transfer` instructions), the pending balance of an
account could constantly change with incoming transfers. Since the corresponding
decryption key of a decryptable balance ciphertext is only known to the owner of
an account, the sender of a `Transfer` instruction cannot update the decryptable
balance of the receiver's account.

Therefore, for the case of the pending balance, the Token program stores two
independent ElGamal ciphertexts, one encrypting the low bits of the 64-bit
pending balance and one encrypting the high bits.

```rust
struct ConfidentialTransferAccount {
  /// `true` if this account has been approved for use. All confidential
  /// transfer operations for
  /// the account will fail until approval is granted.
  approved: PodBool,

  /// The public key associated with ElGamal encryption
  encryption_pubkey: ElGamalPubkey,

  /// The low-bits of the pending balance (encrypted by `encryption_pubkey`)
  pending_balance_lo: ElGamalCiphertext,

  /// The high-bits of the pending balance (encrypted by `encryption_pubkey`)
  pending_balance_hi: ElGamalCiphertext,

  /// The available balance (encrypted by `encryption_pubkey`)
  available_balance: ElGamalCiphertext,

  /// The decryptable available balance
  decryptable_available_balance: AeCiphertext,
}
```

We correspondingly divide the ciphertext that encrypts the transfer amount in
the transfer instruction data as low and high bit encryptions.

```rust
// Actual cryptographic components are organized in `VerifyTransfer`
// instruction data
struct ConfidentialTransferInstructionData {
  /// The transfer amount encrypted under the sender ElGamal public key
  encrypted_amount_sender: ElGamalCiphertext,
  /// The low-bits of the transfer amount encrypted under the receiver
  /// ElGamal public key
  encrypted_amount_lo_receiver: ElGamalCiphertext,
  /// The high-bits of the transfer amount encrypted under the receiver
  /// ElGamal public key
  encrypted_amount_hi_receiver: ElGamalCiphertext,
}
```

Upon receiving a transfer instruction, the Token program aggregates
`encrypted_amount_lo_receiver` in the instruction data to `pending_balance_lo`
in the account and `encrypted_amount_hi_receiver` to `pending_balance_hi`.

One natural way to divide the 64-bit pending balance and transfer amount in the
structures above is to evenly split the number as low and high 32-bit numbers.
Then since the amounts that are encrypted in each ciphertexts are 32-bit
numbers, each of their decryption can be done efficiently.

The problem with this approach is that the 32-bit number that is encrypted as
`pending_balance_lo` could easily overflow and grow larger than a 32-bit number.
For example, two transfers of the amount `2^32-1` to an account force the
`pending_balance_lo` ciphertext in the account to `2^32`, a 33-bit number.
As the encrypted amount overflows, it becomes increasingly more difficult to
decrypt the ciphertext.

To cope with overflows, we add the following two components to the account
state.

- The account state keeps track of the number of incoming transfers that it
  received since the last `ApplyPendingBalance` instruction.
- The account state stores a `maximum_pending_balance_credit_counter` which
  limits the number of incoming transfers that it can receive before an
  `ApplyPendingBalance` instruction is applied to the account. This upper bound
  can be configured with the `ConfigureAccount` and should typically be set to
  `2^16`.

```rust
struct ConfidentialTransferAccount {
  ... // `approved`, `encryption_pubkey`, available balance fields omitted

  /// The low bits of the pending balance (encrypted by `encryption_pubkey`)
  pending_balance_lo: ElGamalCiphertext,

  /// The high bits of the pending balance (encrypted by `encryption_pubkey`)
  pending_balance_hi: ElGamalCiphertext,

  /// The maximum number of `Deposit` and `Transfer` instructions that can credit
  /// `pending_balance` before the `ApplyPendingBalance` instruction is executed
  pub maximum_pending_balance_credit_counter: u64,

  /// The number of incoming transfers since the `ApplyPendingBalance` instruction
  /// was executed
  pub pending_balance_credit_counter: u64,
}
```

For the case of the transfer instruction data, we make the following
modifications:

- The transfer amount is restricted to be a 48-bit number.
- The transfer amount is divided into 16 and 32-bit numbers and is encrypted as
  two ciphertexts `encrypted_amount_lo_receiver` and
  `encrypted_amount_hi_receiver`.

```rust
// Actual cryptographic components are organized in `VerifyTransfer`
// instruction data
struct ConfidentialTransferInstructionData {
  /// The transfer amount encrypted under the sender ElGamal public key
  encrypted_amount_sender: ElGamalCiphertext,
  /// The low *16-bits* of the transfer amount encrypted under the receiver
  /// ElGamal public key
  encrypted_amount_lo_receiver: ElGamalCiphertext,
  /// The high *32-bits* of the transfer amount encrypted under the receiver
  /// ElGamal public key
  encrypted_amount_hi_receiver: ElGamalCiphertext,
}
```

The fields `pending_balance_credit_counter` and
`maximum_pending_balance_credit_counter` are used to limit amounts that are
encrypted in the pending balance ciphertexts `pending_balance_lo` and
`pending_balance_hi`. The choice of the limit on the transfer amount is
done to balance the efficiency of ElGamal decryption with the usability of a
confidential transfer.

Consider the case where `maximum_pending_balance_credit_counter` is set to
`2^16`.

- The `encrypted_amount_lo_receiver` encrypts a number that is at most a 16-bit
  number. Therefore, even after `2^16` incoming transfers, the ciphertext
  `pending_balance_lo` in an account encrypts a balance that is at most a 32-bit
  number. This component of the pending balance can be decrypted efficiently.

- The `encrypted_amount_hi_receiver` encrypts a number that is at most a 32-bit
  number. Therefore, after `2^16` incoming transfers, the ciphertext
  `pending_balance_hi` encrypts a balance that is at most a 48-bit number.

  The decryption of a large 48-bit number is slow. However, for most
  applications, transfers of very high transaction amounts are relatively more
  rare. For an account to hold a pending balance of large 48-bit numbers, it
  must receive a large number of high transactions amounts. Clients that
  maintain accounts with high token balances can frequently submit the
  `ApplyPendingBalance` instruction to flush out the pending balance into the
  available balance to prevent `pending_balance_hi` from encrypting a number
  that is too large.
