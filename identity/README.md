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

## API

The Identity program responds to the following instructions:


## JS client

The JS bindings for the Identity program can be found in the [js](./js) folder.

## Usage in other programs

To see an example of using the identity program in other programs,
see the [token-swap](../token-swap) program.

In general, adding identity checking to other solana programs follows two steps:

1. Add the Identity program as a dependency 

    ```
    [dependencies]
    spl-identity = { path = "../../identity/program", features = [ "no-entrypoint" ] }
    ```

2. Register an IdV or set of IdVs

    When creating an instance of the program (e.g. a Swap, or a Mint), add as a parameter,
    the public key or keys of the identity validators, that is, external services that are
    trusted to attest to a user's identity on-chain. 
   
    e.g. in the TokenSwap program:
    
    ```
    let obj = SwapInfo {
               is_initialized: true,
               nonce,
               token_program_id: *token_program_info.key,
               token_a: *token_a_info.key,
               token_b: *token_b_info.key,
               pool_mint: *pool_mint_info.key,
               token_a_mint: token_a.mint,
               token_b_mint: token_b.mint,
               pool_fee_account: *fee_account_info.key,
               idv: *idv_info.key,  <-- the IdV key
               swap_curve,
           };
           SwapInfo::pack(obj, &mut swap_info.data.borrow_mut())?;
           Ok(())
    ``` 
3. Accept an Identity Account on an instruction that should be identity-gated.

    e.g. in the Token-Swap process_swap function:
    
    ```
   pub fn process_swap(
           program_id: &Pubkey,
           amount_in: u64,
           minimum_amount_out: u64,
           accounts: &[AccountInfo],
       ) -> ProgramResult {
           ...
           let identity_account_info = next_account_info(account_info_iter)?;
    ```

4. Call the identity program verify function when executing an instruction

    ```
    let identity_account = Self::unpack_identity_account(&identity_account_info.data.borrow())?;
    
    // verify that the user is allowed to use the program
    let identity_verification_result = spl_identity::processor::Processor::verify(account, expected_owner, idv).map_err(|_| SwapError::UnauthorizedIdentity)
    ```
