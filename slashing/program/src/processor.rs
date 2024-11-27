//! Program state processor

use {
    crate::{
        duplicate_block_proof::DuplicateBlockProofData,
        error::SlashingError,
        instruction::{
            decode_instruction_data, decode_instruction_type, DuplicateBlockProofInstructionData,
            SlashingInstruction,
        },
        state::SlashingProofData,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::Slot,
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar::{clock::Clock, epoch_schedule::EpochSchedule, Sysvar},
    },
};

fn verify_proof_data<'a, T>(slot: Slot, pubkey: &Pubkey, proof_data: &'a [u8]) -> ProgramResult
where
    T: SlashingProofData<'a>,
{
    // Statue of limitations is 1 epoch
    let clock = Clock::get()?;
    let Some(elapsed) = clock.slot.checked_sub(slot) else {
        return Err(ProgramError::ArithmeticOverflow);
    };
    let epoch_schedule = EpochSchedule::get()?;
    if elapsed > epoch_schedule.slots_per_epoch {
        return Err(SlashingError::ExceedsStatueOfLimitations.into());
    }

    let proof_data: T =
        T::unpack(proof_data).map_err(|_| SlashingError::ShredDeserializationError)?;

    SlashingProofData::verify_proof(proof_data, slot, pubkey)?;

    // TODO: follow up PR will record this violation in context state account. just
    // log for now.
    msg!(
        "{} violation verified in slot {}. This incident will be recorded",
        T::PROOF_TYPE.violation_str(),
        slot
    );
    Ok(())
}

/// Instruction processor
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction_type = decode_instruction_type(input)?;
    let account_info_iter = &mut accounts.iter();
    let proof_data_info = next_account_info(account_info_iter);

    match instruction_type {
        SlashingInstruction::DuplicateBlockProof => {
            let data = decode_instruction_data::<DuplicateBlockProofInstructionData>(input)?;
            let proof_data = &proof_data_info?.data.borrow()[u64::from(data.offset) as usize..];
            verify_proof_data::<DuplicateBlockProofData>(
                data.slot.into(),
                &data.node_pubkey,
                proof_data,
            )?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::verify_proof_data,
        crate::{
            duplicate_block_proof::DuplicateBlockProofData, error::SlashingError,
            shred::tests::new_rand_data_shred,
        },
        rand::Rng,
        solana_ledger::shred::Shredder,
        solana_sdk::{
            clock::{Clock, Slot, DEFAULT_SLOTS_PER_EPOCH},
            epoch_schedule::EpochSchedule,
            program_error::ProgramError,
            signature::Keypair,
            signer::Signer,
        },
        std::sync::{Arc, RwLock},
    };

    const SLOT: Slot = 53084024;
    lazy_static::lazy_static! {
        static ref CLOCK_SLOT: Arc<RwLock<Slot>> = Arc::new(RwLock::new(SLOT));
    }

    fn generate_proof_data(leader: Arc<Keypair>) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let (slot, parent_slot, reference_tick, version) = (SLOT, SLOT - 1, 0, 0);
        let shredder = Shredder::new(slot, parent_slot, reference_tick, version).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let shred1 =
            new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true, true);
        let shred2 =
            new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true, true);
        let proof = DuplicateBlockProofData {
            shred1: shred1.payload().as_slice(),
            shred2: shred2.payload().as_slice(),
        };
        proof.pack()
    }

    #[test]
    fn test_statue_of_limitations() {
        *CLOCK_SLOT.write().unwrap() = SLOT + 5;
        verify_with_clock().unwrap();

        *CLOCK_SLOT.write().unwrap() = SLOT - 1;
        assert_eq!(
            verify_with_clock().unwrap_err(),
            ProgramError::ArithmeticOverflow
        );

        *CLOCK_SLOT.write().unwrap() = SLOT + DEFAULT_SLOTS_PER_EPOCH + 1;
        assert_eq!(
            verify_with_clock().unwrap_err(),
            SlashingError::ExceedsStatueOfLimitations.into()
        );
    }

    fn verify_with_clock() -> Result<(), ProgramError> {
        struct SyscallStubs {}
        impl solana_sdk::program_stubs::SyscallStubs for SyscallStubs {
            fn sol_get_clock_sysvar(&self, var_addr: *mut u8) -> u64 {
                unsafe {
                    let clock = Clock {
                        slot: *CLOCK_SLOT.read().unwrap(),
                        ..Clock::default()
                    };
                    *(var_addr as *mut _ as *mut Clock) = clock;
                }
                solana_program::entrypoint::SUCCESS
            }

            fn sol_get_epoch_schedule_sysvar(&self, var_addr: *mut u8) -> u64 {
                unsafe {
                    *(var_addr as *mut _ as *mut EpochSchedule) = EpochSchedule::default();
                }
                solana_program::entrypoint::SUCCESS
            }
        }

        solana_sdk::program_stubs::set_syscall_stubs(Box::new(SyscallStubs {}));
        let leader = Arc::new(Keypair::new());
        verify_proof_data::<DuplicateBlockProofData>(
            SLOT,
            &leader.pubkey(),
            &generate_proof_data(leader),
        )
    }
}
