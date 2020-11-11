# SPL Identity

SPL Identity is a Solana program, which adds the concept of Self-Sovereign Identity
to the Solana blockchain. It allows other Solana programs to add
"identity gate" functionality: A user can interact with the program
only if they are in possession of a valid identity,
certified by a trusted identity validator.

A user's personal identity information is not stored on-chain, rather, the identity
validator stores a hash of the data against a new account type called an "Identity Account"

## Identity Account

An Identity Account contains the following information:

    owner: PublicKey                        // The Solana account that owns this identity.  
                                            // Only this account can use the identity.
    num_attestations: u8                    // The number of attestations stored in this identity account.
                                            // At present, this can be only 0 or 1.
    attestation.idv: PublicKey              // The public key of the IdV that has stored an attestation against the identity
    attestation.attestation_data: [u8, 32]  // A 32-byte array representing the attestation hash

## JS client

The JS bindings for the Identity program can be found in the [js](./js) folder.

## Usage in other programs

To see an example of using the identity program in other programs,
see the [token-swap](../token-swap) program.


