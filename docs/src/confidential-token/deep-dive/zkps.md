---
title: Zero-Knowledge Proofs
---

Zero-knowledge proofs are tools that allow users to prove certain properties of
encrypted data. Most of the zero-knowledge proofs that are used in the
confidential extension are relatively small systems that are specifically
designed for the simple use-case of the confidential extension. Due to their
simplicity, none of the zero-knowledge systems that are used in the program
require any trusted setup or sophisticated circuit design.

The zero-knowledge proofs that are used in the confidential extension can be
divided into two categories: _sigma protocols_ and _bulletproofs_. Sigma
protocols are simple systems that are tailor designed for the confidential
extension use-cases. Bulletproofs is an existing range proof system that was
developed in the specified [paper](https://eprint.iacr.org/2017/1066).

## Transfer Instruction Data

The confidential extension `Transfer` instruction data requires a number of
cryptographic components. Here, we provide intuition for each of these
components by building the transfer data in a series of steps.

```rust
struct TransferData {
  ...
}
```

### ElGamal Public Keys

A transfer instruction has three associated ElGamal public keys: sender,
receiver, and auditor. A transfer instruction data must include these three
encryption public keys.

```rust
struct TransferPubkeys {
  source_pubkey: ElGamalPubkey,
  destination_pubkey: ElGamal Pubkey,
  auditor_pubkey: ElGamalPubkey,
}

struct TransferData {
  transfer_pubkeys: TransferPubkeys,
}
```

If there is no associated auditor associated with the mint, then the auditor
pubkey is simply 32 zero bytes.

### Low and High-bit Encryption

Transfer instruction data must include the transfer amount that is encrypted
under the three ElGamal public keys associated with the instruction. To cope
with ElGamal decryption as discussed in the previous section, the transfer
amount is restricted to 48-bit numbers and is encrypted as two separate
numbers: `amount_lo` that represents the low 16-bits and `amount_hi` that
represents the high 32-bits.

Each `amount_lo` and `amount_hi` is encrypted under the three ElGamal public
keys associated with a transfer. Instead of including three independent
ciphertexts as part of the transfer data, we use the randomness-reuse property
of ElGamal encryption to minimize the size of ciphertexts.

```rust
/// Ciphertext structure of the transfer amount encrypted under three ElGamal
/// public keys
struct TransferAmountEncryption {
  commitment: PedersenCommitment,
  source_handle: DecryptHandle,
  destination_handle: DecryptionHandle,
  auditor_handle: DecryptHandle,
}

struct TransferData {
  ciphertext_lo: TransferAmountEncryption,
  ciphertext_hi: TransferAmountEncryption,
  transfer_pubkeys: TransferPubkeys,
}
```

In addition to these ciphertexts, transfer data must include proofs that these
ciphertexts are generated properly. There are two ways that a user can
potentially cheat the program. First a user may provide ciphertexts that are
malformed. For example, even if a user may encrypt the transfer amount under a
wrong public key, there is no way for the program to check the validity of a
ciphertext. Therefore, we require that transfer data require a _ciphertext
validity_ proof that certifies that the ciphertexts are properly generated.

Ciphertext validity proof only guarantees that a twisted ElGamal ciphertext is
properly generated. However, it does not certify any property regarding the
encrypted amount in a ciphertext. For example, a malicious user can encrypt
negative values, but there is no way for the program to detect this by simply
inspecting the ciphertext. Therefore, in addition to a ciphertext validity
proof, a transfer instruction must include a _range proof_ that certifies that
the encrypted amounts `amount_lo` and `amount_hi` are positive 16 and 32-bit
values respectively.

```rust
struct TransferProof {
  validity_proof: ValidityProof,
  range_proof: RangeProof,
}

struct TransferData {
  ciphertext_lo: TransferAmountEncryption,
  ciphertext_hi: TransferAmountEncryption,
  transfer_pubkeys: TransferPubkeys,
  proof: TransferProof,
}
```

### Verifying Net-Balance

Finally, in addition to proving that the transfer amount is properly encrypted,
a user must include a proof that the source account has enough balance to
make the transfer. The canonical way to do this is for the user to generate a
range proof that certifies that the ciphertext
`source_available_balance - (ciphertext_lo + 2^16 * ciphertext_hi)`, which holds
the available balance of the source account subtracted by the transfer amount,
encrypts a positive 64-bit value. Since Bulletproofs supports proof
aggregation, this additional range proof can be aggregated into the original
range proof on the transfer amount.

```rust
struct TransferProof {
  validity_proof: ValidityProof,
  range_proof: RangeProof, // certifies ciphertext amount and net-balance
}

struct TransferData {
  ciphertext_lo: TransferAmountEncryption,
  ciphertext_hi: TransferAmountEncryption,
  transfer_pubkeys: TransferPubkeys,
  proof: TransferProof,
}
```

One technical problem with the above is that although the sender of a transfer
knows an ElGamal decryption key for the ciphertext `source_available_balance`,
it does not necessarily know a Pedersen opening for the ciphertext, which is
needed to generate the range proofs on the ciphertext
`source_available_balance - (ciphertext_lo + 2^16 * ciphertext_hi)`. Therefore,
in a transfer instruction, we require that the sender decrypt the ciphertext
`source_available_balance - (ciphertext_lo + 2^16 * ciphertext_hi)` on the
client side and include a new Pedersen commitment on the new source balance
`new_source_commitment` along with an _equality proof_ that certifies that the
ciphertext `source_available_balance - (ciphertext_lo + 2^16 * ciphertext_hi)`
and `new_source_commitment` encrypt the same message.

```rust
struct TransferProof {
  new_source_commitment: PedersenCommitment,
  equality_proof: CtxtCommEqualityProof,
  validity_proof: ValidityProof,
  range_proof: RangeProof,
}

struct TransferData {
  ciphertext_lo: TransferAmountEncryption,
  ciphertext_hi: TransferAmountEncryption,
  transfer_pubkeys: TransferPubkeys,
  proof: TransferProof,
}
```

## Transfer With Fee Instruction Data

The confidential extension can be enabled for mints that are extended for fees.
If a mint is extended for fees, then any confidential transfer of the
corresponding tokens must use the confidential extension `TransferWithFee`
instruction. In addition to the data that are required for the `Transfer`
instruction, the `TransferWithFee` instruction requires additional cryptographic
components associated with fees.

### Background on Transfer Fees

If a mint is extended for fees, then transfers of tokens that pertains to the
mint requires a transfer fee that is calculated as a percentage of the transfer
amount. Specifically, a transaction fee is determined by two paramters:

- `bp`: The base point representing the fee rate. It is a positive integer that
  represents a percentage rate that is two points to the right of the decimal
  place.

  For example, `bp = 1` represents the fee rate of 0.01%, `bp = 100` represents
  the fee rate of 1%, and `bp = 10000` represents the fee rate of 100%.

- `max_fee`: the max fee rate. A transfer fee is calculated using the fee rate
  that is determined by `bp`, but it is capped by `max_fee`. 

  For example, consider a transfer amount of 200 tokens.
  - For fee parameter `bp = 100` and `max_fee = 3`, the fee is simply 1% of the
    transfer amount, which is 2.
  - For fee parameter `bp = 200` and `max_fee = 3`, the fee is 3 since 2% of 200
    is 4, which is greater than the max fee of 3.

The transfer fee is always rounded up to the nearest positive integer. For
example, if a transfer amount is `100` and the fee parameter is `bp = 110` and
`max_fee = 3`, then the fee is `2`, which is rounded up from 1.1% of the
transfer amount.

The fee parameters can be specified in mints that are extended for fees. In
addition to the fee parameters, mints that are extended for fees contain the
`withdraw_withheld_authority` field, which specifies the public key of an
authority that can collect fees that are withheld from transfer amounts.

A Token account that is extended for fees has an associated field
`withheld_amount`. Any transfer fee that is deducted from a transfer amount is
aggregated into the `withheld_amount` field of the destination account of the
transfer. The `withheld_amount` can be collected by the withdraw-withheld
authority into a specific account using the
`TransferFeeInstructions::WithdrawWithheldTokensFromAccounts` or into the mint
account using the `TransferFeeInstructions::HarvestWithheldTokensToMint`. The
withheld fees that accumulate in a mint can be collected into an account using
the `TransferFeeInstructions::WithdrawWithheldTokensFromMint`.

### Fee Encryption

The actual amount of a transfer fee cannot be included in the confidential
extension `TransferWithFee` instruction in the clear since the transfer amount
can be inferred from the fee. Therefore, in the confidential extension, the
transfer fee is encrypted under the destination and withheld authority ElGamal
public key. 

```rust
struct FeeEncryption {
    commitment: PedersenCommitment,
    destination_handle: DecryptHandle,
    withdraw_withheld_authority_handle: DecryptHandle,
}

struct TransferWithFeeData {
  ... // `TransferData` components
  fee_ciphertext: FeeEncryption,
}
```

Upon receiving a `TransferWithFee` instruction, the Token program deducts the
encrypted fee under the destination ElGamal public key from the encrypted
transfer amount under the same public key. Then it aggregates the ciphertext
that encrypts the fee under the withdraw withheld authority's ElGamal public key
into the `withheld_fee` component of the destination account.

### Verifying the Fee Ciphertext

The remaining pieces of the `TransferWithFee` instruction data are fields that
are required to verify the validity of the encrypted fee. Since the fee is
encrypted, the Token program cannot check that the fee was computed correctly by
simply inspecting the ciphertext. A `TransferWithFee` must include three
additional proofs to certify that the fee ciphertext is valid.

- _ciphertext validity proof_: This proof component certifies that the actual
  fee ciphertext is properly generated under the correct destination and
  withdraw withheld authority ElGamal public key.
- _fee sigma proof_: In combination with range proof component, the fee sigma
  proof certifies that the fee that is encrypted in `fee_ciphertext` is properly
  calculated according to the fee parameter.
- _range proof_: In combination with the fee sigma proof components, the range
  proof component certifies that the encrypted fee in `fee_ciphertext` is
  properly calculated according to the fee parameter.

We refer to the proof specifications below for the additional details.

## Sigma Protocols

### Validity Proof

A validity proof certifies that a twisted ElGamal ciphertext is a well-formed
ciphertext. The precise description of the system is specified in the following
notes.

[[Notes]](./validity_proof.pdf)

Validity proofs is required for the `Withdraw`, `Transfer`, and
`TransferWithFee` instructions. These instructions require the client to include
twisted ElGamal ciphertexts as part of the instruction data. Validity proofs
that are attached with these instructions certify that these ElGamal ciphertexts
are well-formed.

### Zero-balance Proof

A zero-balance proof certifies that a twisted ElGamal ciphertext encrypts the
number zero. The precise description of the system is specified in the following
notes.

[[Notes]](./zero_proof.pdf).

Zero-balance proofs are required for the `EmptyAccount` instruction, which
prepares a token account for closing. An account may only be closed if the
balance in an account is zero. Since the balance is encrypted in the
confidential extension, the Token program cannot directly check that the
encrypted balance in an account is zero by inspecting the account state.
Instead, the program verifies the zero-balance proof that is attached in the
`EmptyAccount` instruction to check that the balance is indeed zero.

### Equality Proof

The confidential extension makes use of two kinds of equality proof. The first
variant _ciphertext-commitment_ equality proof certifies that a twisted ElGamal
ciphertext and a Pedersen commitment encode the same message. The second variant
_ciphertext-ciphertext_ equality proof certifies that two twisted ElGamal
ciphertexts encrypt the same message. The precise description of the system is
specified in the following notes.

[[Notes]](./equality_proof.pdf).

Ciphertext-commitment equality proofs are required for the `Transfer` and
`TransferWithFee` instructions. Ciphertext-ciphertext equaltiy proofs are
required for the `WithdrawWithheldTokensFromMint` and
`WithdrawWithheldTokensFromAccounts` instructions.

### Fee Sigma Proof

The fee sigma proof certifies that a committed transfer fee is computed
correctly. The precise description of the system is specified in the following
notes.

[Notes]

The fee sigma proof is required for the `TransferWithFee` instruction.

## Range Proofs

The confidential extension uses Bulletproofs for range proofs. We refer to the
[academic paper](https://eprint.iacr.org/2017/1066) and the
[dalek](https://doc-internal.dalek.rs/bulletproofs/notes/index.html)
implementation for the details.
