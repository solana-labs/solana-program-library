//! State transition types

use {
    crate::{
        account::RequiredAccount,
        error::AccountResolutionError,
        pod::{PodAccountMeta, PodSlice, PodSliceMut, TryFromAccountType},
        seeds::SeedConfig,
    },
    solana_program::{
        account_info::AccountInfo,
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_type_length_value::{
        discriminator::TlvDiscriminator,
        state::{TlvState, TlvStateBorrowed, TlvStateMut},
    },
};

/// Stateless helper for storing additional accounts required for an instruction.
///
/// This struct works with any `TlvDiscriminator`, and stores the extra accounts
/// needed for that specific instruction, using the given `Discriminator` as the
/// type-length-value `Discriminator`, and then storing all of the given
/// `AccountMeta`s as a zero-copy slice.
///
/// Sample usage:
///
/// ```
/// use {
///     solana_program::{
///         account_info::AccountInfo, instruction::{AccountMeta, Instruction},
///         pubkey::Pubkey
///     },
///     spl_type_length_value::discriminator::{Discriminator, TlvDiscriminator},
///     spl_tlv_account_resolution::state::ExtraAccountMetas,
/// };
///
/// struct MyInstruction;
/// impl TlvDiscriminator for MyInstruction {
///     // Give it a unique discriminator, can also be generated using a hash function
///     const TLV_DISCRIMINATOR: Discriminator = Discriminator::new([1; Discriminator::LENGTH]);
/// }
///
/// // actually put it in the additional required account keys and signer / writable
/// let extra_metas = [
///     AccountMeta::new(Pubkey::new_unique(), false),
///     AccountMeta::new(Pubkey::new_unique(), true),
///     AccountMeta::new_readonly(Pubkey::new_unique(), true),
///     AccountMeta::new_readonly(Pubkey::new_unique(), false),
/// ];
///
/// // assume that this buffer is actually account data, already allocated to `account_size`
/// let account_size = ExtraAccountMetas::size_of(extra_metas.len()).unwrap();
/// let mut buffer = vec![0; account_size];
///
/// // Initialize the structure for your instruction
/// ExtraAccountMetas::init_with_account_metas::<MyInstruction>(&mut buffer, &extra_metas).unwrap();
///
/// // Off-chain, you can add the additional accounts directly from the account data
/// let program_id = Pubkey::new_unique();
/// let mut instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
/// ExtraAccountMetas::add_to_instruction::<MyInstruction>(
///     &program_id,
///     &mut instruction,
///     &buffer,
///     None
/// ).unwrap();
///
/// // On-chain, you can add the additional accounts *and* account infos
/// let mut cpi_instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
/// // Assume the other required account infos are already included
/// let mut cpi_account_infos = vec![];
/// // These are the account infos provided to the instruction that are
/// // *not* part of any other known interface
/// let remaining_account_infos: &[AccountInfo<'_>] = &[];
/// ExtraAccountMetas::add_to_cpi_instruction::<MyInstruction>(
///     &program_id,
///     &mut cpi_instruction,
///     &mut cpi_account_infos,
///     &buffer,
///     &remaining_account_infos,
///     None,
/// );
/// ```
/// If you want to store information about required additional accounts that have a
/// Program-Derived Address (PDA), thus their address may not be known at the time of
/// packing the account data, you can do so by modifying the above example to look like this:
/// ```rust
/// use {
///     solana_program::{account_info::AccountInfo, instruction::{AccountMeta, Instruction}, pubkey::Pubkey},
///     spl_type_length_value::discriminator::{Discriminator, TlvDiscriminator},
///     spl_tlv_account_resolution::{
///         account::RequiredAccount,
///         seeds::{Seed, SeedArgType, SeedConfig},
///         state::ExtraAccountMetas,
///     },
/// };
///
/// struct MyInstruction;
/// impl TlvDiscriminator for MyInstruction {
///     const TLV_DISCRIMINATOR: Discriminator = Discriminator::new([1; Discriminator::LENGTH]);
/// }
///
/// // Notice the use of `into()` for the type `AccountMeta`, and we're building this
/// // array of `RequiredAccount`
/// let required_accounts = [
///     AccountMeta::new(Pubkey::new_unique(), false).into(),
///     AccountMeta::new(Pubkey::new_unique(), true).into(),
///     AccountMeta::new_readonly(Pubkey::new_unique(), true).into(),
///     AccountMeta::new_readonly(Pubkey::new_unique(), false).into(),
///     RequiredAccount::Pda {
///         seeds: vec![
///             Seed::Lit,
///             Seed::Arg(SeedArgType::U8),
///             Seed::Arg(SeedArgType::String),
///         ],
///         is_signer: false,
///         is_writable: true,
///     },
///     RequiredAccount::Pda {
///         seeds: vec![
///             Seed::Lit,
///             Seed::Arg(SeedArgType::Pubkey),
///             Seed::Arg(SeedArgType::Pubkey),
///             Seed::Arg(SeedArgType::Pubkey),
///         ],
///         is_signer: false,
///         is_writable: true,
///     },
/// ];
///
/// let account_size = ExtraAccountMetas::size_of(required_accounts.len()).unwrap();
/// let mut buffer = vec![0; account_size];
///
/// // Initialize with "required accounts" instead of `AccountMeta` or `AccountInfo`
/// ExtraAccountMetas::init_with_required_accounts::<MyInstruction>(
///     &mut buffer,
///     &required_accounts,
/// )
/// .unwrap();
///
/// // On the execution side of things, we have to provide the seeds we used to
/// // build the Program-Derived Addresses that correspond to each stored
/// // required PDA account in the TLV structure
/// let provided_seeds = vec![
///     SeedConfig::new(("SomeSeed", 1u8, String::from("SomeSeed"))),
///     SeedConfig::new((
///         "SomeSeed",
///         Pubkey::new_unique(),
///         Pubkey::new_unique(),
///         Pubkey::new_unique(),
///     )),
/// ];
///
/// // Then you provide the seeds as you add accounts to the instruction (off-chain)
/// let program_id = Pubkey::new_unique();
/// let mut instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
/// ExtraAccountMetas::add_to_instruction::<MyInstruction>(
///     &program_id,
///     &mut instruction,
///     &buffer,
///     Some(provided_seeds), // Seeds
/// ).unwrap();
///
/// // Same with on-chain
/// let provided_seeds_cpi = vec![
///     SeedConfig::new(("SomeSeed", 1u8, String::from("SomeSeed"))),
///     SeedConfig::new((
///         "SomeSeed",
///         Pubkey::new_unique(),
///         Pubkey::new_unique(),
///         Pubkey::new_unique(),
///     )),
/// ];
/// let mut cpi_instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
/// let mut cpi_account_infos = vec![];
/// let remaining_account_infos: &[AccountInfo<'_>] = &[];
/// ExtraAccountMetas::add_to_cpi_instruction::<MyInstruction>(
///     &program_id,
///     &mut cpi_instruction,
///     &mut cpi_account_infos,
///     &buffer,
///     &remaining_account_infos,
///     Some(provided_seeds_cpi), // Seeds,
/// );
/// ```
pub struct ExtraAccountMetas;
impl ExtraAccountMetas {
    /// Initialize pod slice data for the given instruction and any type
    /// convertible to account metas
    pub fn init<'a, T: TlvDiscriminator, M>(
        data: &mut [u8],
        convertible_account_types: &'a [M],
    ) -> Result<(), ProgramError>
    where
        PodAccountMeta: TryFromAccountType<&'a M>,
    {
        let mut state = TlvStateMut::unpack(data).unwrap();
        let tlv_size = PodSlice::<PodAccountMeta>::size_of(convertible_account_types.len())?;
        let bytes = state.alloc::<T>(tlv_size)?;
        let mut extra_account_metas = PodSliceMut::init(bytes)?;
        for account_metas in convertible_account_types {
            extra_account_metas.push(PodAccountMeta::try_from_account(account_metas)?)?;
        }
        Ok(())
    }

    /// Initialize a TLV entry for the given discriminator, populating the data
    /// with the given account infos
    pub fn init_with_account_infos<T: TlvDiscriminator>(
        data: &mut [u8],
        account_infos: &[AccountInfo<'_>],
    ) -> Result<(), ProgramError> {
        Self::init::<T, AccountInfo>(data, account_infos)
    }

    /// Get the underlying `PodSlice<PodAccountMeta>` from an unpacked TLV
    ///
    /// Due to lifetime annoyances, this function can't just take in the bytes,
    /// since then we would be returning a reference to a locally created
    /// `TlvStateBorrowed`. I hope there's a better way to do this!
    pub fn unpack_with_tlv_state<'a, T: TlvDiscriminator>(
        tlv_state: &'a TlvStateBorrowed,
    ) -> Result<PodSlice<'a, PodAccountMeta>, ProgramError> {
        let bytes = tlv_state.get_bytes::<T>()?;
        PodSlice::<PodAccountMeta>::unpack(bytes)
    }

    /// Initialize a TLV entry for the given discriminator, populating the data
    /// with the given account metas
    pub fn init_with_account_metas<T: TlvDiscriminator>(
        data: &mut [u8],
        account_metas: &[AccountMeta],
    ) -> Result<(), ProgramError> {
        Self::init::<T, AccountMeta>(data, account_metas)
    }

    /// Initialize a TLV entry for the given discriminator, populating the data
    /// with the given required accounts - which can be standard `AccountMeta`s or
    /// PDAs
    pub fn init_with_required_accounts<T: TlvDiscriminator>(
        data: &mut [u8],
        required_accounts: &[RequiredAccount],
    ) -> Result<(), ProgramError> {
        Self::init::<T, RequiredAccount>(data, required_accounts)
    }

    /// Get the byte size required to hold `num_items` items
    pub fn size_of(num_items: usize) -> Result<usize, ProgramError> {
        Ok(TlvStateBorrowed::get_base_len()
            .saturating_add(PodSlice::<PodAccountMeta>::size_of(num_items)?))
    }

    fn de_escalate_account_meta(
        account_meta: &mut AccountMeta,
        account_metas: &[AccountMeta],
        initial_length: usize,
    ) {
        // This is a little tricky to read, but the idea is to see if
        // this account is marked as writable or signer anywhere in
        // the instruction at the start. If so, DON'T escalate it to
        // be a writer or signer in the CPI
        let maybe_highest_privileges = account_metas
            .iter()
            .take(initial_length)
            .filter(|&x| x.pubkey == account_meta.pubkey)
            .map(|x| (x.is_signer, x.is_writable))
            .reduce(|acc, x| (acc.0 || x.0, acc.1 || x.1));
        // If `Some`, then the account was found somewhere in the instruction
        if let Some((is_signer, is_writable)) = maybe_highest_privileges {
            if !is_signer && is_signer != account_meta.is_signer {
                // Existing account is *NOT* a signer already, but the CPI
                // wants it to be, so de-escalate to not be a signer
                account_meta.is_signer = false;
            }
            if !is_writable && is_writable != account_meta.is_writable {
                // Existing account is *NOT* writable already, but the CPI
                // wants it to be, so de-escalate to not be writable
                account_meta.is_writable = false;
            }
        }
    }

    /// Add the additional account metas to an existing instruction
    pub fn add_to_vec<T: TlvDiscriminator>(
        program_id: &Pubkey,
        existing_account_metas: &mut Vec<AccountMeta>,
        data: &[u8],
        required_seeds: Option<Vec<SeedConfig>>,
    ) -> Result<(), ProgramError> {
        let state = TlvStateBorrowed::unpack(data)?;
        let bytes = state.get_bytes::<T>()?;
        let initial_instruction_length = existing_account_metas.len();

        // Seeds should be passed in the same order as required accounts
        let mut seeds_index = 0;
        let required_accounts_slice = PodSlice::<PodAccountMeta>::unpack(bytes)?;
        for required_account in required_accounts_slice.data().iter() {
            let account_type = RequiredAccount::try_from(required_account)?;
            let mut as_meta = match account_type {
                RequiredAccount::Account { .. } => AccountMeta::try_from(&account_type)?,
                RequiredAccount::Pda {
                    seeds,
                    is_signer,
                    is_writable,
                } => {
                    let res = match &required_seeds {
                        Some(seed_config_vec) => {
                            let pubkey = match seed_config_vec.get(seeds_index) {
                                Some(seed_config) => seed_config.evaluate(program_id, seeds)?,
                                None => {
                                    return Err(
                                        AccountResolutionError::NotEnoughSeedsProvided.into()
                                    )
                                }
                            };
                            AccountMeta {
                                pubkey,
                                is_signer,
                                is_writable,
                            }
                        }
                        None => return Err(AccountResolutionError::SeedsRequired.into()),
                    };
                    seeds_index += 1;
                    res
                }
            };
            Self::de_escalate_account_meta(
                &mut as_meta,
                existing_account_metas,
                initial_instruction_length,
            );
            existing_account_metas.push(as_meta);
        }
        Ok(())
    }

    /// Add the additional account metas to an existing instruction
    pub fn add_to_instruction<T: TlvDiscriminator>(
        program_id: &Pubkey,
        instruction: &mut Instruction,
        data: &[u8],
        required_seeds: Option<Vec<SeedConfig>>,
    ) -> Result<(), ProgramError> {
        Self::add_to_vec::<T>(program_id, &mut instruction.accounts, data, required_seeds)
    }

    /// Add the additional account metas and account infos for a CPI, while
    /// de-escalating repeated accounts.
    ///
    /// If an added account already exists in the instruction with lower
    /// privileges, match it to the existing account. This prevents a lower
    /// program from gaining unexpected privileges.
    pub fn add_to_cpi_instruction<'a, T: TlvDiscriminator>(
        program_id: &Pubkey,
        cpi_instruction: &mut Instruction,
        cpi_account_infos: &mut Vec<AccountInfo<'a>>,
        data: &[u8],
        account_infos: &[AccountInfo<'a>],
        required_seeds: Option<Vec<SeedConfig>>,
    ) -> Result<(), ProgramError> {
        let state = TlvStateBorrowed::unpack(data)?;
        let bytes = state.get_bytes::<T>()?;

        let initial_cpi_instruction_length = cpi_instruction.accounts.len();

        // Seeds should be passed in the same order as required accounts
        let mut seeds_index = 0;
        let required_accounts_slice = PodSlice::<PodAccountMeta>::unpack(bytes)?;
        for required_account in required_accounts_slice.data().iter() {
            let account_type = RequiredAccount::try_from(required_account)?;
            let mut as_meta = match account_type {
                RequiredAccount::Account { .. } => AccountMeta::try_from(&account_type)?,
                RequiredAccount::Pda {
                    seeds,
                    is_signer,
                    is_writable,
                } => {
                    let res = match &required_seeds {
                        Some(seed_config_vec) => {
                            let pubkey = match seed_config_vec.get(seeds_index) {
                                Some(seed_config) => seed_config.evaluate(program_id, seeds)?,
                                None => {
                                    return Err(
                                        AccountResolutionError::NotEnoughSeedsProvided.into()
                                    )
                                }
                            };
                            AccountMeta {
                                pubkey,
                                is_signer,
                                is_writable,
                            }
                        }
                        None => return Err(AccountResolutionError::SeedsRequired.into()),
                    };
                    seeds_index += 1;
                    res
                }
            };
            let account_info = account_infos
                .iter()
                .find(|&x| *x.key == as_meta.pubkey)
                .ok_or(AccountResolutionError::IncorrectAccount)?
                .clone();
            Self::de_escalate_account_meta(
                &mut as_meta,
                &cpi_instruction.accounts,
                initial_cpi_instruction_length,
            );
            cpi_account_infos.push(account_info);
            cpi_instruction.accounts.push(as_meta);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            account::{AccountMetaPda, RequiredAccount},
            seeds::{Seed, SeedArgType},
        },
        solana_program::{
            clock::Epoch, entrypoint::ProgramResult, instruction::AccountMeta, pubkey::Pubkey,
        },
        spl_type_length_value::discriminator::Discriminator,
    };

    pub struct TestInstruction;
    impl TlvDiscriminator for TestInstruction {
        const TLV_DISCRIMINATOR: Discriminator = Discriminator::new([1; Discriminator::LENGTH]);
    }

    pub struct TestOtherInstruction;
    impl TlvDiscriminator for TestOtherInstruction {
        const TLV_DISCRIMINATOR: Discriminator = Discriminator::new([2; Discriminator::LENGTH]);
    }

    #[test]
    fn seeds() -> ProgramResult {
        // Testing a handful of seed combinations for validation
        // against `find_program_address`
        let program_id = Pubkey::new_unique();
        let seed_lit = "SomeSeed";
        let seed_u8 = 2u8;
        let seed_u16 = 4u16;
        let seed_u32 = 6u32;
        let seed_u64 = 8u64;
        let seed_u128 = 10u128;
        let seed_string = String::from("SomeSeed");
        let seed_key_1 = Pubkey::new_unique();
        let seed_key_2 = Pubkey::new_unique();
        {
            let required_seeds = vec![
                Seed::Lit,
                Seed::Arg(SeedArgType::U8),
                Seed::Arg(SeedArgType::U16),
            ];
            assert_eq!(
                Pubkey::find_program_address(
                    &[
                        seed_lit.as_bytes(),
                        seed_u8.to_le_bytes().as_ref(),
                        seed_u16.to_le_bytes().as_ref(),
                    ],
                    &program_id
                )
                .0,
                SeedConfig::new((seed_lit, seed_u8, seed_u16))
                    .evaluate(&program_id, required_seeds)?
            );
        }
        {
            let required_seeds = vec![
                Seed::Arg(SeedArgType::U32),
                Seed::Arg(SeedArgType::U64),
                Seed::Arg(SeedArgType::U128),
            ];
            assert_eq!(
                Pubkey::find_program_address(
                    &[
                        seed_u32.to_le_bytes().as_ref(),
                        seed_u64.to_le_bytes().as_ref(),
                        seed_u128.to_le_bytes().as_ref(),
                    ],
                    &program_id
                )
                .0,
                SeedConfig::new((seed_u32, seed_u64, seed_u128))
                    .evaluate(&program_id, required_seeds)?
            );
        }
        {
            let required_seeds = vec![
                Seed::Lit,
                Seed::Arg(SeedArgType::String),
                Seed::Arg(SeedArgType::Pubkey),
            ];
            assert_eq!(
                Pubkey::find_program_address(
                    &[
                        seed_lit.as_bytes(),
                        seed_string.as_bytes(),
                        seed_key_1.as_ref(),
                    ],
                    &program_id
                )
                .0,
                SeedConfig::new((seed_lit, seed_string.clone(), seed_key_1))
                    .evaluate(&program_id, required_seeds)?
            );
        }
        {
            let required_seeds = vec![
                Seed::Lit,
                Seed::Arg(SeedArgType::U8),
                Seed::Arg(SeedArgType::String),
                Seed::Arg(SeedArgType::Pubkey),
            ];
            assert_eq!(
                Pubkey::find_program_address(
                    &[
                        seed_lit.as_bytes(),
                        seed_u8.to_le_bytes().as_ref(),
                        seed_string.as_bytes(),
                        seed_key_2.as_ref(),
                    ],
                    &program_id
                )
                .0,
                SeedConfig::new((seed_lit, seed_u8, seed_string, seed_key_2))
                    .evaluate(&program_id, required_seeds)?
            );
        }
        {
            let required_seeds = vec![
                Seed::Arg(SeedArgType::U64),
                Seed::Arg(SeedArgType::Pubkey),
                Seed::Arg(SeedArgType::Pubkey),
            ];
            assert_eq!(
                Pubkey::find_program_address(
                    &[
                        seed_u64.to_le_bytes().as_ref(),
                        seed_key_1.as_ref(),
                        seed_key_2.as_ref(),
                    ],
                    &program_id
                )
                .0,
                SeedConfig::new((seed_u64, seed_key_1, seed_key_2))
                    .evaluate(&program_id, required_seeds)?
            );
        }
        Ok(())
    }

    #[test]
    fn init_with_metas() {
        let program_id = Pubkey::new_unique();
        // You can see we can just use `AccountMeta` if we have no PDAs
        let metas = [
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new(Pubkey::new_unique(), true),
            AccountMeta::new_readonly(Pubkey::new_unique(), true),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
        ];
        let account_size = ExtraAccountMetas::size_of(metas.len()).unwrap();
        let mut buffer = vec![0; account_size];

        ExtraAccountMetas::init_with_account_metas::<TestInstruction>(&mut buffer, &metas).unwrap();

        let mut instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
        ExtraAccountMetas::add_to_instruction::<TestInstruction>(
            &program_id,
            &mut instruction,
            &buffer,
            None,
        )
        .unwrap();
        assert_eq!(
            instruction
                .accounts
                .iter()
                .map(PodAccountMeta::from)
                .collect::<Vec<_>>(),
            metas.iter().map(PodAccountMeta::from).collect::<Vec<_>>()
        );
    }

    #[test]
    fn init_with_metas_and_seeds() -> ProgramResult {
        let program_id = Pubkey::new_unique();
        // The enum `RequiredAccount` allows us to build one array/slice for
        // either `AccountMeta` or a PDA with seed configs
        let required_accounts = [
            AccountMeta::new(Pubkey::new_unique(), false).into(),
            AccountMeta::new(Pubkey::new_unique(), true).into(),
            RequiredAccount::Pda {
                seeds: vec![
                    Seed::Lit,
                    Seed::Arg(SeedArgType::U8),
                    Seed::Arg(SeedArgType::String),
                ],
                is_signer: false,
                is_writable: true,
            },
            // You can also use `AccountMetaPda`!
            AccountMetaPda::new(
                &vec![
                    Seed::Arg(SeedArgType::U8),
                    Seed::Arg(SeedArgType::U8),
                    Seed::Arg(SeedArgType::U32),
                    Seed::Arg(SeedArgType::Pubkey),
                ],
                false,
                true,
            )
            .unwrap()
            .try_into()
            .unwrap(),
            AccountMeta::new_readonly(Pubkey::new_unique(), true).into(),
            AccountMeta::new_readonly(Pubkey::new_unique(), false).into(),
            RequiredAccount::Pda {
                seeds: vec![
                    Seed::Lit,
                    Seed::Arg(SeedArgType::Pubkey),
                    Seed::Arg(SeedArgType::Pubkey),
                    Seed::Arg(SeedArgType::Pubkey),
                ],
                is_signer: false,
                is_writable: true,
            },
        ];
        let account_size = ExtraAccountMetas::size_of(required_accounts.len()).unwrap();
        let mut buffer = vec![0; account_size];

        ExtraAccountMetas::init_with_required_accounts::<TestInstruction>(
            &mut buffer,
            &required_accounts,
        )
        .unwrap();

        // We have three PDAs in our required accounts, so we're going to need three seed inputs
        // (Flexing how we can have varying sized tuples with varying types)
        let provided_seeds = vec![
            SeedConfig::new(("SomeSeed", 1u8, String::from("SomeSeed"))),
            SeedConfig::new((1u8, 2u8, 2u32, Pubkey::new_unique())),
            SeedConfig::new((
                "SomeSeed",
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                Pubkey::new_unique(),
            )),
        ];

        let mut instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
        ExtraAccountMetas::add_to_instruction::<TestInstruction>(
            &program_id,
            &mut instruction,
            &buffer,
            Some(provided_seeds.clone()), // Cloning only so we can use later in assertions
        )
        .unwrap();

        assert_eq!(instruction.accounts.len(), required_accounts.len());
        assert_eq!(
            required_accounts.get(0).unwrap(),
            instruction.accounts.get(0).unwrap()
        );
        assert_eq!(
            required_accounts.get(1).unwrap(),
            instruction.accounts.get(1).unwrap()
        );
        {
            let seeds = provided_seeds.get(0).unwrap();
            let req_seeds = match required_accounts.get(2).unwrap() {
                RequiredAccount::Pda { seeds, .. } => seeds.clone(),
                _ => return Err(AccountResolutionError::RequiredAccountNotPda.into()),
            };
            let meta = instruction.accounts.get(2).unwrap();
            assert_eq!(seeds.seed_types, req_seeds);
            assert_eq!(seeds.evaluate(&program_id, req_seeds)?, meta.pubkey,);
        }
        {
            let seeds = provided_seeds.get(1).unwrap();
            let req_seeds = match required_accounts.get(3).unwrap() {
                RequiredAccount::Pda { seeds, .. } => seeds.clone(),
                _ => return Err(AccountResolutionError::RequiredAccountNotPda.into()),
            };
            let meta = instruction.accounts.get(3).unwrap();
            assert_eq!(seeds.seed_types, req_seeds);
            assert_eq!(seeds.evaluate(&program_id, req_seeds)?, meta.pubkey,);
        }
        assert_eq!(
            required_accounts.get(4).unwrap(),
            instruction.accounts.get(4).unwrap()
        );
        assert_eq!(
            required_accounts.get(5).unwrap(),
            instruction.accounts.get(5).unwrap()
        );
        {
            let seeds = provided_seeds.get(2).unwrap();
            let req_seeds = match required_accounts.get(6).unwrap() {
                RequiredAccount::Pda { seeds, .. } => seeds.clone(),
                _ => return Err(AccountResolutionError::RequiredAccountNotPda.into()),
            };
            let meta = instruction.accounts.get(6).unwrap();
            assert_eq!(seeds.seed_types, req_seeds);
            assert_eq!(seeds.evaluate(&program_id, req_seeds)?, meta.pubkey,);
        }
        Ok(())
    }

    #[test]
    fn init_multiple() {
        let program_id = Pubkey::new_unique();
        let metas = [
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new(Pubkey::new_unique(), true),
            AccountMeta::new_readonly(Pubkey::new_unique(), true),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
        ];
        let other_metas = [AccountMeta::new(Pubkey::new_unique(), false)];
        let account_size = ExtraAccountMetas::size_of(metas.len()).unwrap()
            + ExtraAccountMetas::size_of(other_metas.len()).unwrap();
        let mut buffer = vec![0; account_size];

        ExtraAccountMetas::init_with_account_metas::<TestInstruction>(&mut buffer, &metas).unwrap();
        ExtraAccountMetas::init_with_account_metas::<TestOtherInstruction>(
            &mut buffer,
            &other_metas,
        )
        .unwrap();

        let mut instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
        ExtraAccountMetas::add_to_instruction::<TestInstruction>(
            &program_id,
            &mut instruction,
            &buffer,
            None,
        )
        .unwrap();
        assert_eq!(
            instruction
                .accounts
                .iter()
                .map(PodAccountMeta::from)
                .collect::<Vec<_>>(),
            metas.iter().map(PodAccountMeta::from).collect::<Vec<_>>()
        );
        let mut instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
        ExtraAccountMetas::add_to_instruction::<TestOtherInstruction>(
            &program_id,
            &mut instruction,
            &buffer,
            None,
        )
        .unwrap();
        assert_eq!(
            instruction
                .accounts
                .iter()
                .map(PodAccountMeta::from)
                .collect::<Vec<_>>(),
            other_metas
                .iter()
                .map(PodAccountMeta::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn init_multiple_with_seeds() -> ProgramResult {
        let program_id = Pubkey::new_unique();
        let required_accounts = [
            AccountMeta::new(Pubkey::new_unique(), false).into(),
            AccountMeta::new(Pubkey::new_unique(), true).into(),
            AccountMeta::new_readonly(Pubkey::new_unique(), true).into(),
            AccountMeta::new_readonly(Pubkey::new_unique(), false).into(),
            RequiredAccount::Pda {
                seeds: vec![
                    Seed::Lit,
                    Seed::Arg(SeedArgType::U32),
                    Seed::Arg(SeedArgType::String),
                ],
                is_signer: false,
                is_writable: true,
            },
            RequiredAccount::Pda {
                seeds: vec![Seed::Lit, Seed::Arg(SeedArgType::Pubkey)],
                is_signer: false,
                is_writable: true,
            },
        ];
        let other_metas = [AccountMeta::new(Pubkey::new_unique(), false)];
        let account_size = ExtraAccountMetas::size_of(required_accounts.len()).unwrap()
            + ExtraAccountMetas::size_of(other_metas.len()).unwrap();
        let mut buffer = vec![0; account_size];

        ExtraAccountMetas::init_with_required_accounts::<TestInstruction>(
            &mut buffer,
            &required_accounts,
        )
        .unwrap();
        ExtraAccountMetas::init_with_account_metas::<TestOtherInstruction>(
            &mut buffer,
            &other_metas,
        )
        .unwrap();

        let provided_seeds = vec![
            SeedConfig::new(("SomeSeed", 200u32, String::from("SomeSeed"))),
            SeedConfig::new(("SomeSeed", Pubkey::new_unique())),
        ];

        let mut instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
        ExtraAccountMetas::add_to_instruction::<TestInstruction>(
            &program_id,
            &mut instruction,
            &buffer,
            Some(provided_seeds.clone()), // Cloning only so we can use later in assertions
        )
        .unwrap();
        assert_eq!(instruction.accounts.len(), required_accounts.len());
        assert_eq!(
            required_accounts.get(0).unwrap(),
            instruction.accounts.get(0).unwrap()
        );
        assert_eq!(
            required_accounts.get(1).unwrap(),
            instruction.accounts.get(1).unwrap()
        );
        assert_eq!(
            required_accounts.get(2).unwrap(),
            instruction.accounts.get(2).unwrap()
        );
        assert_eq!(
            required_accounts.get(3).unwrap(),
            instruction.accounts.get(3).unwrap()
        );
        {
            let seeds = provided_seeds.get(0).unwrap();
            let req_seeds = match required_accounts.get(4).unwrap() {
                RequiredAccount::Pda { seeds, .. } => seeds.clone(),
                _ => return Err(AccountResolutionError::RequiredAccountNotPda.into()),
            };
            let meta = instruction.accounts.get(4).unwrap();
            assert_eq!(seeds.seed_types, req_seeds);
            assert_eq!(seeds.evaluate(&program_id, req_seeds)?, meta.pubkey,);
        }
        {
            let seeds = provided_seeds.get(1).unwrap();
            let req_seeds = match required_accounts.get(5).unwrap() {
                RequiredAccount::Pda { seeds, .. } => seeds.clone(),
                _ => return Err(AccountResolutionError::RequiredAccountNotPda.into()),
            };
            let meta = instruction.accounts.get(5).unwrap();
            assert_eq!(seeds.seed_types, req_seeds);
            assert_eq!(seeds.evaluate(&program_id, req_seeds)?, meta.pubkey,);
        }
        let mut instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
        ExtraAccountMetas::add_to_instruction::<TestOtherInstruction>(
            &program_id,
            &mut instruction,
            &buffer,
            None,
        )
        .unwrap();
        assert_eq!(
            instruction
                .accounts
                .iter()
                .map(PodAccountMeta::from)
                .collect::<Vec<_>>(),
            other_metas
                .iter()
                .map(PodAccountMeta::from)
                .collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn init_mixed() {
        // annoying to setup, but need to test this!
        let program_id = Pubkey::new_unique();

        let pubkey1 = Pubkey::new_unique();
        let mut lamports1 = 0;
        let mut data1 = [];
        let pubkey2 = Pubkey::new_unique();
        let mut lamports2 = 0;
        let mut data2 = [];
        let pubkey3 = Pubkey::new_unique();
        let mut lamports3 = 0;
        let mut data3 = [];
        let owner = Pubkey::new_unique();
        let account_infos = [
            AccountInfo::new(
                &pubkey1,
                false,
                true,
                &mut lamports1,
                &mut data1,
                &owner,
                false,
                Epoch::default(),
            ),
            AccountInfo::new(
                &pubkey2,
                true,
                false,
                &mut lamports2,
                &mut data2,
                &owner,
                false,
                Epoch::default(),
            ),
            AccountInfo::new(
                &pubkey3,
                false,
                false,
                &mut lamports3,
                &mut data3,
                &owner,
                false,
                Epoch::default(),
            ),
        ];
        let metas = [
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new(Pubkey::new_unique(), true),
            AccountMeta::new_readonly(Pubkey::new_unique(), true),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
        ];
        let account_size = ExtraAccountMetas::size_of(account_infos.len()).unwrap()
            + ExtraAccountMetas::size_of(metas.len()).unwrap();
        let mut buffer = vec![0; account_size];

        ExtraAccountMetas::init_with_account_infos::<TestInstruction>(&mut buffer, &account_infos)
            .unwrap();
        ExtraAccountMetas::init_with_account_metas::<TestOtherInstruction>(&mut buffer, &metas)
            .unwrap();

        let mut instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
        ExtraAccountMetas::add_to_instruction::<TestInstruction>(
            &program_id,
            &mut instruction,
            &buffer,
            None,
        )
        .unwrap();
        assert_eq!(
            instruction
                .accounts
                .iter()
                .map(PodAccountMeta::from)
                .collect::<Vec<_>>(),
            account_infos
                .iter()
                .map(PodAccountMeta::from)
                .collect::<Vec<_>>()
        );

        let mut instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
        ExtraAccountMetas::add_to_instruction::<TestOtherInstruction>(
            &program_id,
            &mut instruction,
            &buffer,
            None,
        )
        .unwrap();
        assert_eq!(
            instruction
                .accounts
                .iter()
                .map(PodAccountMeta::from)
                .collect::<Vec<_>>(),
            metas.iter().map(PodAccountMeta::from).collect::<Vec<_>>()
        );
    }

    #[test]
    fn cpi_instruction() {
        // annoying to setup, but need to test this!
        let pubkey1 = Pubkey::new_unique();
        let mut lamports1 = 0;
        let mut data1 = [];
        let pubkey2 = Pubkey::new_unique();
        let mut lamports2 = 0;
        let mut data2 = [];
        let pubkey3 = Pubkey::new_unique();
        let mut lamports3 = 0;
        let mut data3 = [];
        let owner = Pubkey::new_unique();
        let account_infos = [
            AccountInfo::new(
                &pubkey1,
                false,
                true,
                &mut lamports1,
                &mut data1,
                &owner,
                false,
                Epoch::default(),
            ),
            AccountInfo::new(
                &pubkey2,
                true,
                false,
                &mut lamports2,
                &mut data2,
                &owner,
                false,
                Epoch::default(),
            ),
            AccountInfo::new(
                &pubkey3,
                false,
                false,
                &mut lamports3,
                &mut data3,
                &owner,
                false,
                Epoch::default(),
            ),
        ];
        let account_size = ExtraAccountMetas::size_of(account_infos.len()).unwrap();
        let mut buffer = vec![0; account_size];

        ExtraAccountMetas::init_with_account_infos::<TestInstruction>(&mut buffer, &account_infos)
            .unwrap();

        // make an instruction to check later
        let program_id = Pubkey::new_unique();
        let mut instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
        ExtraAccountMetas::add_to_instruction::<TestInstruction>(
            &program_id,
            &mut instruction,
            &buffer,
            None,
        )
        .unwrap();

        // mess around with the account infos to make it harder
        let mut messed_account_infos = account_infos.to_vec();
        let pubkey4 = Pubkey::new_unique();
        let mut lamports4 = 0;
        let mut data4 = [];
        messed_account_infos.push(AccountInfo::new(
            &pubkey4,
            false,
            true,
            &mut lamports4,
            &mut data4,
            &owner,
            false,
            Epoch::default(),
        ));
        let pubkey5 = Pubkey::new_unique();
        let mut lamports5 = 0;
        let mut data5 = [];
        messed_account_infos.push(AccountInfo::new(
            &pubkey5,
            false,
            true,
            &mut lamports5,
            &mut data5,
            &owner,
            false,
            Epoch::default(),
        ));
        messed_account_infos.swap(0, 4);
        messed_account_infos.swap(1, 2);

        let mut cpi_instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
        let mut cpi_account_infos = vec![];
        ExtraAccountMetas::add_to_cpi_instruction::<TestInstruction>(
            &program_id,
            &mut cpi_instruction,
            &mut cpi_account_infos,
            &buffer,
            &messed_account_infos,
            None,
        )
        .unwrap();

        assert_eq!(cpi_instruction, instruction);
        assert_eq!(cpi_account_infos.len(), account_infos.len());
        for (a, b) in std::iter::zip(cpi_account_infos, account_infos) {
            assert_eq!(a.key, b.key);
            assert_eq!(a.is_signer, b.is_signer);
            assert_eq!(a.is_writable, b.is_writable);
        }
    }

    #[test]
    fn cpi_instruction_with_seeds() -> ProgramResult {
        let program_id = Pubkey::new_unique();
        let seeds1 = SeedConfig::new((37u8, 206u16, 3004u32));
        let seeds2 = SeedConfig::new(("SomeSeed", String::from("SomeSeed"), Pubkey::new_unique()));
        let pubkey1 = seeds1.pda(&program_id).0;
        let mut lamports1 = 0;
        let mut data1 = [];
        let pubkey2 = seeds2.pda(&program_id).0;
        let mut lamports2 = 0;
        let mut data2 = [];
        let pubkey3 = Pubkey::new_unique();
        let mut lamports3 = 0;
        let mut data3 = [];
        let owner = Pubkey::new_unique();
        let account_infos = [
            AccountInfo::new(
                &pubkey1,
                false,
                true,
                &mut lamports1,
                &mut data1,
                &owner,
                false,
                Epoch::default(),
            ),
            AccountInfo::new(
                &pubkey2,
                false,
                false,
                &mut lamports2,
                &mut data2,
                &owner,
                false,
                Epoch::default(),
            ),
            AccountInfo::new(
                &pubkey3,
                false,
                false,
                &mut lamports3,
                &mut data3,
                &owner,
                false,
                Epoch::default(),
            ),
        ];
        let required_accounts = vec![
            RequiredAccount::Pda {
                seeds: vec![
                    Seed::Arg(SeedArgType::U8),
                    Seed::Arg(SeedArgType::U16),
                    Seed::Arg(SeedArgType::U32),
                ],
                is_signer: false,
                is_writable: true,
            },
            RequiredAccount::Pda {
                seeds: vec![
                    Seed::Lit,
                    Seed::Arg(SeedArgType::String),
                    Seed::Arg(SeedArgType::Pubkey),
                ],
                is_signer: false,
                is_writable: false,
            },
            account_infos.get(2).unwrap().into(),
        ];
        let account_size = ExtraAccountMetas::size_of(required_accounts.len()).unwrap();
        let mut buffer = vec![0; account_size];

        ExtraAccountMetas::init_with_required_accounts::<TestInstruction>(
            &mut buffer,
            &required_accounts,
        )?;

        // make an instruction to check later
        let mut instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
        ExtraAccountMetas::add_to_instruction::<TestInstruction>(
            &program_id,
            &mut instruction,
            &buffer,
            Some(vec![seeds1.clone(), seeds2.clone()]),
        )?;

        // mess around with the account infos to make it harder
        let mut messed_account_infos = account_infos.to_vec();
        let pubkey4 = Pubkey::new_unique();
        let mut lamports4 = 0;
        let mut data4 = [];
        messed_account_infos.push(AccountInfo::new(
            &pubkey4,
            false,
            true,
            &mut lamports4,
            &mut data4,
            &owner,
            false,
            Epoch::default(),
        ));
        let pubkey5 = Pubkey::new_unique();
        let mut lamports5 = 0;
        let mut data5 = [];
        messed_account_infos.push(AccountInfo::new(
            &pubkey5,
            false,
            true,
            &mut lamports5,
            &mut data5,
            &owner,
            false,
            Epoch::default(),
        ));
        messed_account_infos.swap(0, 4);
        messed_account_infos.swap(1, 2);

        let mut cpi_instruction = Instruction::new_with_bytes(program_id, &[], vec![]);
        let mut cpi_account_infos = vec![];
        ExtraAccountMetas::add_to_cpi_instruction::<TestInstruction>(
            &program_id,
            &mut cpi_instruction,
            &mut cpi_account_infos,
            &buffer,
            &messed_account_infos,
            Some(vec![seeds1, seeds2]),
        )?;

        assert_eq!(cpi_instruction, instruction);
        assert_eq!(cpi_account_infos.len(), required_accounts.len());
        for (a, b) in std::iter::zip(cpi_account_infos, required_accounts) {
            match b {
                RequiredAccount::Account {
                    pubkey,
                    is_signer,
                    is_writable,
                } => {
                    assert_eq!(*a.key, pubkey);
                    assert_eq!(a.is_signer, is_signer);
                    assert_eq!(a.is_writable, is_writable);
                }
                RequiredAccount::Pda {
                    seeds: _,
                    is_signer,
                    is_writable,
                } => {
                    assert_eq!(a.is_signer, is_signer);
                    assert_eq!(a.is_writable, is_writable);
                }
            }
        }
        Ok(())
    }
}
