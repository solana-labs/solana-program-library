//! ZOL program
use crate::{
    error::ZolError,
    instruction::ZolInstruction,
    state::{EquivalenceProof, SolvencyProof, State, User},
};
use curve25519_dalek::{constants::RISTRETTO_BASEPOINT_POINT, scalar::Scalar};
use elgamal_ristretto::{ciphertext::Ciphertext, public::PublicKey};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
    pubkey::Pubkey,
};

/// Process an InitializeVault instruction. Initialize the account
/// state to the State::Vault type.
fn initialize_vault(vault_account: &AccountInfo) -> Result<(), ProgramError> {
    let state = State::deserialize(&vault_account.data.borrow())?;
    if let State::Uninitialized = state {
        State::Vault.serialize(&mut vault_account.data.borrow_mut())
    } else {
        Err(ProgramError::AccountAlreadyInitialized)
    }
}

/// Process an InitializeUser instruction. Initialize the account
/// state to the State::User type.
fn initialize_user(
    user_account: &AccountInfo,
    encryption_pubkey: PublicKey,
) -> Result<(), ProgramError> {
    let state = State::deserialize(&user_account.data.borrow())?;
    if let State::Uninitialized = state {
        State::User(User::new(encryption_pubkey)).serialize(&mut user_account.data.borrow_mut())
    } else {
        Err(ProgramError::AccountAlreadyInitialized)
    }
}

/// Process a Deposit instruction. Move SOL from the given SOL account to the
/// `vault_account` and add ZOL to the given user account.
fn deposit(
    sender_account: &AccountInfo,
    vault_account: &AccountInfo,
    recipient_account: &AccountInfo,
    amount: u64,
) -> Result<(), ProgramError> {
    if !sender_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !matches!(
        State::deserialize(&sender_account.data.borrow())?,
        State::Uninitialized
    ) {
        return Err(ZolError::DepositFromInvalidAccount.into());
    }

    if sender_account.lamports() < amount {
        return Err(ProgramError::InsufficientFunds);
    }

    **sender_account.lamports.borrow_mut() -= amount;
    **vault_account.lamports.borrow_mut() += amount;

    let mut recipient_state = State::deserialize(&recipient_account.data.borrow())?;

    let encoded_amount = Scalar::from(amount) * RISTRETTO_BASEPOINT_POINT;
    let encryption_pubkey = recipient_state.user_mut()?.encrypted_amount.pk;
    let encrypted_amount = encryption_pubkey.encrypt(&encoded_amount);

    recipient_state.user_mut()?.encrypted_amount += encrypted_amount;

    recipient_state.serialize(&mut recipient_account.data.borrow_mut())
}

/// Process a Withdraw instruction. Move SOL from the given user
/// account to the given SOL account.
fn withdraw(
    sender_account: &AccountInfo,
    vault_account: &AccountInfo,
    recipient_account: &AccountInfo,
    amount: u64,
    solvency_proof: SolvencyProof,
) -> Result<(), ProgramError> {
    // TODO: Does the solvency proof verification make this check redundant?
    if !sender_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut sender_state = State::deserialize(&sender_account.data.borrow())?;

    let encoded_amount = Scalar::from(amount) * RISTRETTO_BASEPOINT_POINT;
    let encryption_pubkey = sender_state.user_mut()?.encrypted_amount.pk;
    let encrypted_amount = encryption_pubkey.encrypt(&encoded_amount);

    sender_state.user_mut()?.encrypted_amount -= encrypted_amount;
    **vault_account.lamports.borrow_mut() -= amount;
    **recipient_account.lamports.borrow_mut() += amount;

    // Ensure the debit results in a positive balance
    solvency_proof.verify(&sender_state.user_mut()?.encrypted_amount)?;

    sender_state.serialize(&mut sender_account.data.borrow_mut())
}

/// Process a Transfer instruction. Move the given encrypted amount from the
/// sender user account to receiver user account.
fn transfer(
    sender_account: &AccountInfo,
    recipient_account: &AccountInfo,
    sender_amount: Ciphertext,
    recipient_amount: Ciphertext,
    solvency_proof: SolvencyProof,
    equivalence_proof: EquivalenceProof,
) -> Result<(), ProgramError> {
    // TODO: Does the solvency proof verification make this check redundant?
    if !sender_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    equivalence_proof.verify()?;

    let mut sender_state = State::deserialize(&sender_account.data.borrow())?;
    let mut recipient_state = State::deserialize(&recipient_account.data.borrow())?;

    sender_state.user_mut()?.encrypted_amount -= sender_amount;
    recipient_state.user_mut()?.encrypted_amount += recipient_amount;

    // Ensure the debit results in a positive balance
    solvency_proof.verify(&sender_state.user_mut()?.encrypted_amount)?;

    sender_state.serialize(&mut sender_account.data.borrow_mut())?;
    recipient_state.serialize(&mut recipient_account.data.borrow_mut())
}

/// Process the given transaction instruction
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> Result<(), ProgramError> {
    let account_iter = &mut accounts.iter();
    let instruction = ZolInstruction::deserialize(input)?;

    match instruction {
        ZolInstruction::InitializeVault => {
            let vault_account = next_account_info(account_iter)?;
            initialize_vault(&vault_account)
        }
        ZolInstruction::InitializeUser { encryption_pubkey } => {
            let user_account = next_account_info(account_iter)?;
            initialize_user(&user_account, encryption_pubkey)
        }
        ZolInstruction::Deposit { amount } => {
            let sender_account = next_account_info(account_iter)?;
            let vault_account = next_account_info(account_iter)?;
            let recipient_account = next_account_info(account_iter)?;
            deposit(&sender_account, &vault_account, &recipient_account, amount)
        }
        ZolInstruction::Withdraw {
            amount,
            solvency_proof,
        } => {
            let sender_account = next_account_info(account_iter)?;
            let vault_account = next_account_info(account_iter)?;
            let recipient_account = next_account_info(account_iter)?;
            withdraw(
                &sender_account,
                &vault_account,
                &recipient_account,
                amount,
                solvency_proof,
            )
        }
        ZolInstruction::Transfer {
            sender_amount,
            recipient_amount,
            solvency_proof,
            equivalence_proof,
        } => {
            let sender_account = next_account_info(account_iter)?;
            let recipient_account = next_account_info(account_iter)?;
            transfer(
                &sender_account,
                &recipient_account,
                sender_amount,
                recipient_amount,
                solvency_proof,
                equivalence_proof,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elgamal_ristretto::private::SecretKey;
    use rand_core::OsRng;
    use solana_sdk::account::Account;

    #[test]
    fn test_initialize_vault() {
        let program_id = Pubkey::new_unique();
        let account_key = Pubkey::new_unique();
        let mut account = Account::new(0, State::Vault.packed_len(), &program_id);
        let state = State::deserialize(&account.data).unwrap();
        assert!(matches!(state, State::Uninitialized));

        let account_info = (&account_key, false, &mut account).into();
        initialize_vault(&account_info).unwrap();
        let state = State::deserialize(&account_info.data.borrow()).unwrap();
        assert!(matches!(state, State::Vault));

        // Do it again. Confirm error.
        assert_eq!(
            initialize_vault(&account_info).unwrap_err(),
            ProgramError::AccountAlreadyInitialized
        );
    }

    #[test]
    fn test_initialize_user() {
        let program_id = Pubkey::new_unique();
        let account_key = Pubkey::new_unique();
        let mut csprng = OsRng;
        let encryption_sk = SecretKey::new(&mut csprng);
        let encryption_pubkey = PublicKey::from(&encryption_sk);

        let mut account = Account::new(
            0,
            State::User(User::new(encryption_pubkey.clone())).packed_len(),
            &program_id,
        );
        let state = State::deserialize(&account.data).unwrap();
        assert!(matches!(state, State::Uninitialized));

        let account_info = (&account_key, false, &mut account).into();
        initialize_user(&account_info, encryption_pubkey.clone()).unwrap();
        let state = State::deserialize(&account_info.data.borrow()).unwrap();
        assert!(matches!(state, State::User(_)));

        // Do it again. Confirm error.
        assert_eq!(
            initialize_vault(&account_info).unwrap_err(),
            ProgramError::AccountAlreadyInitialized
        );
    }

    #[test]
    fn test_transfer() {
        // Initialize a vault
        let program_id = Pubkey::new_unique();
        let vault_pubkey = Pubkey::new_unique();
        let mut vault_account = Account::new(0, State::Vault.packed_len(), &program_id);
        let vault_account_info = (&vault_pubkey, false, &mut vault_account).into();
        initialize_vault(&vault_account_info).unwrap();

        // Initialize a funding account
        let mint_pubkey = Pubkey::new_unique();
        let mut account = Account::new(42, 1, &program_id);
        let mint_account_info = (&mint_pubkey, true, &mut account).into();

        // Initialize Alice's ZOL user account
        let alice_pubkey = Pubkey::new_unique();
        let mut csprng = OsRng;
        let encryption_sk = SecretKey::new(&mut csprng);
        let alice_encryption_pubkey = PublicKey::from(&encryption_sk);
        let mut account = Account::new(
            0,
            State::User(User::new(alice_encryption_pubkey.clone())).packed_len(),
            &program_id,
        );
        let alice_account_info = (&alice_pubkey, true, &mut account).into();
        initialize_user(&alice_account_info, alice_encryption_pubkey.clone()).unwrap();

        // Deposit
        deposit(
            &mint_account_info,
            &vault_account_info,
            &alice_account_info,
            42,
        )
        .unwrap();
        assert_eq!(mint_account_info.lamports(), 0);
        assert_eq!(vault_account_info.lamports(), 42);

        // Initialize Bob's ZOL user account
        let bob_pubkey = Pubkey::new_unique();
        let mut csprng = OsRng;
        let encryption_sk = SecretKey::new(&mut csprng);
        let bob_encryption_pubkey = PublicKey::from(&encryption_sk);
        let mut account = Account::new(
            0,
            State::User(User::new(bob_encryption_pubkey.clone())).packed_len(),
            &program_id,
        );
        let bob_account_info = (&bob_pubkey, true, &mut account).into();
        initialize_user(&bob_account_info, bob_encryption_pubkey.clone()).unwrap();

        //
        // Transfer ZOL from Alice to Bob
        //
        use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
        use curve25519_dalek::scalar::Scalar;
        use merlin::Transcript;
        use rand::thread_rng;

        let pc_gens = PedersenGens::default();
        let bp_gens = BulletproofGens::new(64, 1);

        // The balance after a debit - we want to prove lies in the range [0, 2^32)
        let alice_end_balance = 0u64;

        let blinding = Scalar::random(&mut thread_rng());

        // The proof can be chained to an existing transcript.
        // Here we create a transcript with a doctest domain separator.
        let mut prover_transcript = Transcript::new(b"example");

        // Create a 32-bit rangeproof.
        let (proof, _committed_value) = RangeProof::prove_single(
            &bp_gens,
            &pc_gens,
            &mut prover_transcript,
            alice_end_balance,
            &blinding,
            32,
        )
        .unwrap();

        let alice_solvency_proof = SolvencyProof::new(proof.clone());
        let encoded_amount = Scalar::from(42u64) * RISTRETTO_BASEPOINT_POINT;
        let sender_amount = alice_encryption_pubkey.encrypt(&encoded_amount);
        let recipient_amount = bob_encryption_pubkey.encrypt(&encoded_amount);
        let equivalence_proof = EquivalenceProof::new();
        transfer(
            &alice_account_info,
            &bob_account_info,
            sender_amount,
            recipient_amount,
            alice_solvency_proof,
            equivalence_proof,
        )
        .unwrap();

        // Withdraw from Bob. Verify Mint has its SOL back.
        let bob_solvency_proof = SolvencyProof::new(proof);
        withdraw(
            &bob_account_info,
            &vault_account_info,
            &mint_account_info,
            42,
            bob_solvency_proof,
        )
        .unwrap();
        assert_eq!(mint_account_info.lamports(), 42);
        assert_eq!(vault_account_info.lamports(), 0);
    }

    #[test]
    fn test_bulletproofs() {
        use curve25519_dalek::scalar::Scalar;
        use merlin::Transcript;
        use rand::thread_rng;

        use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};

        // Generators for Pedersen commitments.  These can be selected
        // independently of the Bulletproofs generators.
        let pc_gens = PedersenGens::default();

        // Generators for Bulletproofs, valid for proofs up to bitsize 64
        // and aggregation size up to 1.
        let bp_gens = BulletproofGens::new(64, 1);

        // A secret value we want to prove lies in the range [0, 2^32)
        let secret_value = 1037578891u64;

        // The API takes a blinding factor for the commitment.
        let blinding = Scalar::random(&mut thread_rng());

        // The proof can be chained to an existing transcript.
        // Here we create a transcript with a doctest domain separator.
        let mut prover_transcript = Transcript::new(b"doctest example");

        // Create a 32-bit rangeproof.
        let (proof, committed_value) = RangeProof::prove_single(
            &bp_gens,
            &pc_gens,
            &mut prover_transcript,
            secret_value,
            &blinding,
            32,
        )
        .expect("A real program could handle errors");

        // TODO: Test relationship between Penderson commitment and ElGamal encryption
        //let sk = SecretKey::from(blinding);
        //let pk = PublicKey::from(&sk);
        //let encoded_amount = Scalar::from(secret_value) * RISTRETTO_BASEPOINT_POINT;
        //let ciphertext = pk.encrypt(&encoded_amount);
        //assert_eq!(ciphertext.get_points().1.compress(), committed_value);

        // Verification requires a transcript with identical initial state:
        let mut verifier_transcript = Transcript::new(b"doctest example");
        assert!(proof
            .verify_single(
                &bp_gens,
                &pc_gens,
                &mut verifier_transcript,
                &committed_value,
                32
            )
            .is_ok());
    }
}
