//! Duplicate block proof data and verification
use {
    crate::{
        error::SlashingError,
        shred::{Shred, ShredType},
        state::{ProofType, SlashingProofData},
    },
    bytemuck::try_from_bytes,
    solana_program::{clock::Slot, msg, pubkey::Pubkey},
    spl_pod::primitives::PodU32,
};

/// Proof of a duplicate block violation
pub struct DuplicateBlockProofData<'a> {
    /// Shred signed by a leader
    pub shred1: &'a [u8],
    /// Conflicting shred signed by the same leader
    pub shred2: &'a [u8],
}

impl<'a> DuplicateBlockProofData<'a> {
    const LENGTH_SIZE: usize = std::mem::size_of::<PodU32>();

    /// Packs proof data to write in account for
    /// `SlashingInstruction::DuplicateBlockProof`
    pub fn pack(self) -> Vec<u8> {
        let mut buf = vec![];
        buf.extend_from_slice(&(self.shred1.len() as u32).to_le_bytes());
        buf.extend_from_slice(self.shred1);
        buf.extend_from_slice(&(self.shred2.len() as u32).to_le_bytes());
        buf.extend_from_slice(self.shred2);
        buf
    }

    /// Given the maximum size of a shred as `shred_size` this returns
    /// the maximum size of the account needed to store a
    /// `DuplicateBlockProofData`
    pub const fn size_of(shred_size: usize) -> usize {
        2usize
            .wrapping_mul(shred_size)
            .saturating_add(2 * Self::LENGTH_SIZE)
    }
}

impl<'a> SlashingProofData<'a> for DuplicateBlockProofData<'a> {
    const PROOF_TYPE: ProofType = ProofType::DuplicateBlockProof;

    fn verify_proof(self, slot: Slot, _node_pubkey: &Pubkey) -> Result<(), SlashingError> {
        // TODO: verify through instruction inspection that the shreds were sigverified
        // earlier in this transaction.
        // Ed25519 Singature verification is performed on the merkle root:
        // node_pubkey.verify_strict(merkle_root, signature).
        // We will verify that the pubkey merkle root and signature match the shred and
        // that the verification was successful.
        let shred1 = Shred::new_from_payload(self.shred1)?;
        let shred2 = Shred::new_from_payload(self.shred2)?;
        check_shreds(slot, &shred1, &shred2)
    }

    fn unpack(data: &'a [u8]) -> Result<Self, SlashingError>
    where
        Self: Sized,
    {
        if data.len() < Self::LENGTH_SIZE {
            return Err(SlashingError::ProofBufferTooSmall);
        }
        let (length1, data) = data.split_at(Self::LENGTH_SIZE);
        let shred1_length = try_from_bytes::<PodU32>(length1)
            .map_err(|_| SlashingError::ProofBufferDeserializationError)?;
        let shred1_length = u32::from(*shred1_length) as usize;

        if data.len() < shred1_length {
            return Err(SlashingError::ProofBufferTooSmall);
        }
        let (shred1, data) = data.split_at(shred1_length);

        if data.len() < Self::LENGTH_SIZE {
            return Err(SlashingError::ProofBufferTooSmall);
        }
        let (length2, shred2) = data.split_at(Self::LENGTH_SIZE);
        let shred2_length = try_from_bytes::<PodU32>(length2)
            .map_err(|_| SlashingError::ProofBufferDeserializationError)?;
        let shred2_length = u32::from(*shred2_length) as usize;

        if shred2.len() < shred2_length {
            return Err(SlashingError::ProofBufferTooSmall);
        }

        Ok(Self { shred1, shred2 })
    }
}

/// Check that `shred1` and `shred2` indicate a valid duplicate proof
///     - Must be for the same slot `slot`
///     - Must be for the same shred version
///     - Must have a merkle root conflict, otherwise `shred1` and `shred2` must
///       have the same `shred_type`
///     - If `shred1` and `shred2` share the same index they must be not have
///       equal payloads excluding the retransmitter signature
///     - If `shred1` and `shred2` do not share the same index and are data
///       shreds verify that they indicate an index conflict. One of them must
///       be the LAST_SHRED_IN_SLOT, however the other shred must have a higher
///       index.
///     - If `shred1` and `shred2` do not share the same index and are coding
///       shreds verify that they have conflicting erasure metas
fn check_shreds(slot: Slot, shred1: &Shred, shred2: &Shred) -> Result<(), SlashingError> {
    if shred1.slot()? != slot {
        msg!(
            "Invalid proof for different slots {} vs {}",
            shred1.slot()?,
            slot,
        );
        return Err(SlashingError::SlotMismatch);
    }

    if shred2.slot()? != slot {
        msg!(
            "Invalid proof for different slots {} vs {}",
            shred1.slot()?,
            slot,
        );
        return Err(SlashingError::SlotMismatch);
    }

    if shred1.version()? != shred2.version()? {
        msg!(
            "Invalid proof for different shred versions {} vs {}",
            shred1.version()?,
            shred2.version()?,
        );
        return Err(SlashingError::InvalidShredVersion);
    }

    // Merkle root conflict check
    if shred1.fec_set_index()? == shred2.fec_set_index()?
        && shred1.merkle_root()? != shred2.merkle_root()?
    {
        // Legacy shreds are discarded by validators and already filtered out
        // above during proof deserialization, so any valid proof should have
        // merkle roots.
        msg!(
            "Valid merkle root conflict for fec set {}, {:?} vs {:?}",
            shred1.fec_set_index()?,
            shred1.merkle_root()?,
            shred2.merkle_root()?
        );
        return Ok(());
    }

    // Overlapping fec set check
    if shred1.shred_type() == ShredType::Code && shred1.fec_set_index()? < shred2.fec_set_index()? {
        let next_fec_set_index = shred1.next_fec_set_index()?;
        if next_fec_set_index > shred2.fec_set_index()? {
            msg!(
                "Valid overlapping fec set conflict. fec set {}'s next set is {} \
                however we observed a shred with fec set index {}",
                shred1.fec_set_index()?,
                next_fec_set_index,
                shred2.fec_set_index()?
            );
            return Ok(());
        }
    }

    if shred2.shred_type() == ShredType::Code && shred1.fec_set_index()? > shred2.fec_set_index()? {
        let next_fec_set_index = shred2.next_fec_set_index()?;
        if next_fec_set_index > shred1.fec_set_index()? {
            msg!(
                "Valid overlapping fec set conflict. fec set {}'s next set is {} \
                however we observed a shred with fec set index {}",
                shred2.fec_set_index()?,
                next_fec_set_index,
                shred1.fec_set_index()?
            );
            return Ok(());
        }
    }

    if shred1.shred_type() != shred2.shred_type() {
        msg!(
            "Invalid proof for different shred types {:?} vs {:?}",
            shred1.shred_type(),
            shred2.shred_type()
        );
        return Err(SlashingError::ShredTypeMismatch);
    }

    if shred1.index()? == shred2.index()? {
        if shred1.is_shred_duplicate(shred2) {
            msg!("Valid payload mismatch for shred index {}", shred1.index()?);
            return Ok(());
        }
        msg!(
            "Invalid proof, payload matches for index {}",
            shred1.index()?
        );
        return Err(SlashingError::InvalidPayloadProof);
    }

    if shred1.shred_type() == ShredType::Data {
        if shred1.last_in_slot()? && shred2.index()? > shred1.index()? {
            msg!(
                "Valid last in slot conflict last index {} but shred with index {} is present",
                shred1.index()?,
                shred2.index()?
            );
            return Ok(());
        }
        if shred2.last_in_slot()? && shred1.index()? > shred2.index()? {
            msg!(
                "Valid last in slot conflict last index {} but shred with index {} is present",
                shred2.index()?,
                shred1.index()?
            );
            return Ok(());
        }
        msg!(
            "Invalid proof, no last in shred conflict for data shreds {} and {}",
            shred1.index()?,
            shred2.index()?
        );
        return Err(SlashingError::InvalidLastIndexConflict);
    }

    if shred1.fec_set_index() == shred2.fec_set_index()
        && !shred1.check_erasure_consistency(shred2)?
    {
        msg!(
            "Valid erasure meta conflict in fec set {}, config {:?} vs {:?}",
            shred1.fec_set_index()?,
            shred1.erasure_meta()?,
            shred2.erasure_meta()?,
        );
        return Ok(());
    }
    msg!(
        "Invalid proof, no erasure meta conflict for coding shreds set {} idx {} and set {} idx {}",
        shred1.fec_set_index()?,
        shred1.index()?,
        shred2.fec_set_index()?,
        shred2.index()?,
    );
    Err(SlashingError::InvalidErasureMetaConflict)
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::shred::{
            tests::{new_rand_coding_shreds, new_rand_data_shred, new_rand_shreds},
            SIZE_OF_SIGNATURE,
        },
        rand::Rng,
        solana_ledger::shred::{Shred as SolanaShred, Shredder},
        solana_sdk::signature::{Keypair, Signature, Signer},
        std::sync::Arc,
    };

    const SLOT: Slot = 53084024;
    const PARENT_SLOT: Slot = SLOT - 1;
    const REFERENCE_TICK: u8 = 0;
    const VERSION: u16 = 0;

    fn generate_proof_data<'a>(
        shred1: &'a SolanaShred,
        shred2: &'a SolanaShred,
    ) -> DuplicateBlockProofData<'a> {
        DuplicateBlockProofData {
            shred1: shred1.payload().as_slice(),
            shred2: shred2.payload().as_slice(),
        }
    }

    #[test]
    fn test_legacy_shreds_invalid() {
        let mut rng = rand::thread_rng();
        let leader = Arc::new(Keypair::new());
        let shredder = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let legacy_data_shred =
            new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, false, false);
        let legacy_coding_shred =
            new_rand_coding_shreds(&mut rng, next_shred_index, 5, &shredder, &leader, false)[0]
                .clone();
        let data_shred =
            new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true, false);
        let coding_shred =
            new_rand_coding_shreds(&mut rng, next_shred_index, 5, &shredder, &leader, true)[0]
                .clone();

        let test_cases = [
            (legacy_data_shred.clone(), legacy_data_shred.clone()),
            (legacy_coding_shred.clone(), legacy_coding_shred.clone()),
            (legacy_data_shred.clone(), legacy_coding_shred.clone()),
            // Mix of legacy and merkle
            (legacy_data_shred.clone(), data_shred.clone()),
            (legacy_coding_shred.clone(), coding_shred.clone()),
            (legacy_data_shred.clone(), coding_shred.clone()),
            (data_shred.clone(), legacy_coding_shred.clone()),
        ];
        for (shred1, shred2) in test_cases.iter().flat_map(|(a, b)| [(a, b), (b, a)]) {
            let proof_data = generate_proof_data(shred1, shred2);
            assert_eq!(
                proof_data.verify_proof(SLOT, &leader.pubkey()).unwrap_err(),
                SlashingError::LegacyShreds,
            );
        }
    }

    #[test]
    fn test_slot_invalid() {
        let mut rng = rand::thread_rng();
        let leader = Arc::new(Keypair::new());
        let shredder_slot = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let shredder_bad_slot =
            Shredder::new(SLOT + 1, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let data_shred = new_rand_data_shred(
            &mut rng,
            next_shred_index,
            &shredder_slot,
            &leader,
            true,
            false,
        );
        let data_shred_bad_slot = new_rand_data_shred(
            &mut rng,
            next_shred_index,
            &shredder_bad_slot,
            &leader,
            true,
            false,
        );
        let coding_shred =
            new_rand_coding_shreds(&mut rng, next_shred_index, 5, &shredder_slot, &leader, true)[0]
                .clone();

        let coding_shred_bad_slot = new_rand_coding_shreds(
            &mut rng,
            next_shred_index,
            5,
            &shredder_bad_slot,
            &leader,
            true,
        )[0]
        .clone();

        let test_cases = vec![
            (data_shred_bad_slot.clone(), data_shred_bad_slot.clone()),
            (coding_shred_bad_slot.clone(), coding_shred_bad_slot.clone()),
            (data_shred_bad_slot.clone(), coding_shred_bad_slot.clone()),
            (data_shred.clone(), data_shred_bad_slot.clone()),
            (coding_shred.clone(), coding_shred_bad_slot.clone()),
            (data_shred.clone(), coding_shred_bad_slot.clone()),
            (data_shred_bad_slot.clone(), coding_shred.clone()),
        ];

        for (shred1, shred2) in test_cases.iter().flat_map(|(a, b)| [(a, b), (b, a)]) {
            let proof_data = generate_proof_data(shred1, shred2);
            assert_eq!(
                proof_data.verify_proof(SLOT, &leader.pubkey()).unwrap_err(),
                SlashingError::SlotMismatch
            );
        }
    }

    #[test]
    fn test_payload_proof_valid() {
        let mut rng = rand::thread_rng();
        let leader = Arc::new(Keypair::new());
        let shredder = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let shred1 =
            new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true, true);
        let shred2 =
            new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true, true);
        let proof_data = generate_proof_data(&shred1, &shred2);
        proof_data.verify_proof(SLOT, &leader.pubkey()).unwrap();
    }

    #[test]
    fn test_payload_proof_invalid() {
        let mut rng = rand::thread_rng();
        let leader = Arc::new(Keypair::new());
        let shredder = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let data_shred =
            new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true, true);
        let coding_shreds =
            new_rand_coding_shreds(&mut rng, next_shred_index, 10, &shredder, &leader, true);
        let test_cases = vec![
            // Same data_shred
            (data_shred.clone(), data_shred),
            // Same coding_shred
            (coding_shreds[0].clone(), coding_shreds[0].clone()),
        ];

        for (shred1, shred2) in test_cases.into_iter() {
            let proof_data = generate_proof_data(&shred1, &shred2);
            assert_eq!(
                proof_data.verify_proof(SLOT, &leader.pubkey()).unwrap_err(),
                SlashingError::InvalidPayloadProof
            );
        }
    }

    #[test]
    fn test_merkle_root_proof_valid() {
        let mut rng = rand::thread_rng();
        let leader = Arc::new(Keypair::new());
        let shredder = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let (data_shreds, coding_shreds) = new_rand_shreds(
            &mut rng,
            next_shred_index,
            next_shred_index,
            10,
            true, /* merkle_variant */
            &shredder,
            &leader,
            false,
        );

        let (diff_data_shreds, diff_coding_shreds) = new_rand_shreds(
            &mut rng,
            next_shred_index,
            next_shred_index,
            10,
            true, /* merkle_variant */
            &shredder,
            &leader,
            false,
        );

        let test_cases = vec![
            (data_shreds[0].clone(), diff_data_shreds[1].clone()),
            (coding_shreds[0].clone(), diff_coding_shreds[1].clone()),
            (data_shreds[0].clone(), diff_coding_shreds[0].clone()),
            (coding_shreds[0].clone(), diff_data_shreds[0].clone()),
        ];

        for (shred1, shred2) in test_cases.iter().flat_map(|(a, b)| [(a, b), (b, a)]) {
            let proof_data = generate_proof_data(shred1, shred2);
            proof_data.verify_proof(SLOT, &leader.pubkey()).unwrap();
        }
    }

    #[test]
    fn test_merkle_root_proof_invalid() {
        let mut rng = rand::thread_rng();
        let leader = Arc::new(Keypair::new());
        let shredder = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let (data_shreds, coding_shreds) = new_rand_shreds(
            &mut rng,
            next_shred_index,
            next_shred_index,
            10,
            true,
            &shredder,
            &leader,
            true,
        );

        let (next_data_shreds, next_coding_shreds) = new_rand_shreds(
            &mut rng,
            next_shred_index + 33,
            next_shred_index + 33,
            10,
            true,
            &shredder,
            &leader,
            true,
        );

        let test_cases = vec![
            // Same fec set same merkle root
            (coding_shreds[0].clone(), data_shreds[0].clone()),
            // Different FEC set different merkle root
            (coding_shreds[0].clone(), next_data_shreds[0].clone()),
            (next_coding_shreds[0].clone(), data_shreds[0].clone()),
        ];

        for (shred1, shred2) in test_cases.iter().flat_map(|(a, b)| [(a, b), (b, a)]) {
            let proof_data = generate_proof_data(shred1, shred2);
            assert_eq!(
                proof_data.verify_proof(SLOT, &leader.pubkey()).unwrap_err(),
                SlashingError::ShredTypeMismatch
            );
        }
    }

    #[test]
    fn test_last_index_conflict_valid() {
        let mut rng = rand::thread_rng();
        let leader = Arc::new(Keypair::new());
        let shredder = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let test_cases = vec![
            (
                new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true, true),
                new_rand_data_shred(
                    &mut rng,
                    // With Merkle shreds, last erasure batch is padded with
                    // empty data shreds.
                    next_shred_index + 30,
                    &shredder,
                    &leader,
                    true,
                    false,
                ),
            ),
            (
                new_rand_data_shred(
                    &mut rng,
                    next_shred_index + 100,
                    &shredder,
                    &leader,
                    true,
                    true,
                ),
                new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true, true),
            ),
        ];

        for (shred1, shred2) in test_cases.iter().flat_map(|(a, b)| [(a, b), (b, a)]) {
            let proof_data = generate_proof_data(shred1, shred2);
            proof_data.verify_proof(SLOT, &leader.pubkey()).unwrap();
        }
    }

    #[test]
    fn test_last_index_conflict_invalid() {
        let mut rng = rand::thread_rng();
        let leader = Arc::new(Keypair::new());
        let shredder = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let test_cases = vec![
            (
                new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true, false),
                new_rand_data_shred(
                    &mut rng,
                    next_shred_index + 1,
                    &shredder,
                    &leader,
                    true,
                    true,
                ),
            ),
            (
                new_rand_data_shred(
                    &mut rng,
                    next_shred_index + 1,
                    &shredder,
                    &leader,
                    true,
                    true,
                ),
                new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true, false),
            ),
            (
                new_rand_data_shred(
                    &mut rng,
                    next_shred_index + 100,
                    &shredder,
                    &leader,
                    true,
                    false,
                ),
                new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true, false),
            ),
            (
                new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true, false),
                new_rand_data_shred(
                    &mut rng,
                    next_shred_index + 100,
                    &shredder,
                    &leader,
                    true,
                    false,
                ),
            ),
        ];

        for (shred1, shred2) in test_cases.iter().flat_map(|(a, b)| [(a, b), (b, a)]) {
            let proof_data = generate_proof_data(shred1, shred2);
            assert_eq!(
                proof_data.verify_proof(SLOT, &leader.pubkey()).unwrap_err(),
                SlashingError::InvalidLastIndexConflict
            );
        }
    }

    #[test]
    fn test_erasure_meta_conflict_valid() {
        let mut rng = rand::thread_rng();
        let leader = Arc::new(Keypair::new());
        let shredder = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let coding_shreds =
            new_rand_coding_shreds(&mut rng, next_shred_index, 10, &shredder, &leader, true);
        let coding_shreds_bigger =
            new_rand_coding_shreds(&mut rng, next_shred_index, 13, &shredder, &leader, true);
        let coding_shreds_smaller =
            new_rand_coding_shreds(&mut rng, next_shred_index, 7, &shredder, &leader, true);

        // Same fec-set, different index, different erasure meta
        let test_cases = vec![
            (coding_shreds[0].clone(), coding_shreds_bigger[1].clone()),
            (coding_shreds[0].clone(), coding_shreds_smaller[1].clone()),
        ];
        for (shred1, shred2) in test_cases.iter().flat_map(|(a, b)| [(a, b), (b, a)]) {
            let proof_data = generate_proof_data(shred1, shred2);
            proof_data.verify_proof(SLOT, &leader.pubkey()).unwrap();
        }
    }

    #[test]
    fn test_erasure_meta_conflict_invalid() {
        let mut rng = rand::thread_rng();
        let leader = Arc::new(Keypair::new());
        let shredder = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let coding_shreds =
            new_rand_coding_shreds(&mut rng, next_shred_index, 10, &shredder, &leader, true);
        let coding_shreds_different_fec = new_rand_coding_shreds(
            &mut rng,
            next_shred_index + 100,
            10,
            &shredder,
            &leader,
            true,
        );
        let coding_shreds_different_fec_and_size = new_rand_coding_shreds(
            &mut rng,
            next_shred_index + 100,
            13,
            &shredder,
            &leader,
            true,
        );

        let test_cases = vec![
            // Different index, different fec set, same erasure meta
            (
                coding_shreds[0].clone(),
                coding_shreds_different_fec[1].clone(),
            ),
            // Different index, different fec set, different erasure meta
            (
                coding_shreds[0].clone(),
                coding_shreds_different_fec_and_size[1].clone(),
            ),
            // Different index, same fec set, same erasure meta
            (coding_shreds[0].clone(), coding_shreds[1].clone()),
            (
                coding_shreds_different_fec[0].clone(),
                coding_shreds_different_fec[1].clone(),
            ),
            (
                coding_shreds_different_fec_and_size[0].clone(),
                coding_shreds_different_fec_and_size[1].clone(),
            ),
        ];

        for (shred1, shred2) in test_cases.iter().flat_map(|(a, b)| [(a, b), (b, a)]) {
            let proof_data = generate_proof_data(shred1, shred2);
            assert_eq!(
                proof_data.verify_proof(SLOT, &leader.pubkey()).unwrap_err(),
                SlashingError::InvalidErasureMetaConflict
            );
        }
    }

    #[test]
    fn test_shred_version_invalid() {
        let mut rng = rand::thread_rng();
        let leader = Arc::new(Keypair::new());
        let shredder = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let (data_shreds, coding_shreds) = new_rand_shreds(
            &mut rng,
            next_shred_index,
            next_shred_index,
            10,
            true,
            &shredder,
            &leader,
            true,
        );

        // Wrong shred VERSION
        let shredder = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION + 1).unwrap();
        let (wrong_data_shreds, wrong_coding_shreds) = new_rand_shreds(
            &mut rng,
            next_shred_index,
            next_shred_index,
            10,
            true,
            &shredder,
            &leader,
            true,
        );
        let test_cases = vec![
            // One correct shred VERSION, one wrong
            (coding_shreds[0].clone(), wrong_coding_shreds[0].clone()),
            (coding_shreds[0].clone(), wrong_data_shreds[0].clone()),
            (data_shreds[0].clone(), wrong_coding_shreds[0].clone()),
            (data_shreds[0].clone(), wrong_data_shreds[0].clone()),
        ];

        for (shred1, shred2) in test_cases.iter().flat_map(|(a, b)| [(a, b), (b, a)]) {
            let proof_data = generate_proof_data(shred1, shred2);
            assert_eq!(
                proof_data.verify_proof(SLOT, &leader.pubkey()).unwrap_err(),
                SlashingError::InvalidShredVersion
            );
        }
    }

    #[test]
    fn test_retransmitter_signature_payload_proof_invalid() {
        // TODO: change visbility of shred::layout::set_retransmitter_signature.
        // Hardcode offsets for now;
        const DATA_SHRED_OFFSET: usize = 1139;
        const CODING_SHRED_OFFSET: usize = 1164;

        let mut rng = rand::thread_rng();
        let leader = Arc::new(Keypair::new());
        let shredder = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let data_shred =
            new_rand_data_shred(&mut rng, next_shred_index, &shredder, &leader, true, true);
        let coding_shred =
            new_rand_coding_shreds(&mut rng, next_shred_index, 10, &shredder, &leader, true)[0]
                .clone();

        let mut data_shred_different_retransmitter_payload = data_shred.clone().into_payload();
        let buffer = data_shred_different_retransmitter_payload
            .get_mut(DATA_SHRED_OFFSET..DATA_SHRED_OFFSET + SIZE_OF_SIGNATURE)
            .unwrap();
        buffer.copy_from_slice(Signature::new_unique().as_ref());
        let data_shred_different_retransmitter =
            SolanaShred::new_from_serialized_shred(data_shred_different_retransmitter_payload)
                .unwrap();

        let mut coding_shred_different_retransmitter_payload = coding_shred.clone().into_payload();
        let buffer = coding_shred_different_retransmitter_payload
            .get_mut(CODING_SHRED_OFFSET..CODING_SHRED_OFFSET + SIZE_OF_SIGNATURE)
            .unwrap();
        buffer.copy_from_slice(Signature::new_unique().as_ref());
        let coding_shred_different_retransmitter =
            SolanaShred::new_from_serialized_shred(coding_shred_different_retransmitter_payload)
                .unwrap();

        let test_cases = vec![
            // Same data shred from different retransmitter
            (data_shred, data_shred_different_retransmitter),
            // Same coding shred from different retransmitter
            (coding_shred, coding_shred_different_retransmitter),
        ];
        for (shred1, shred2) in test_cases.iter().flat_map(|(a, b)| [(a, b), (b, a)]) {
            let proof_data = generate_proof_data(shred1, shred2);
            assert_eq!(
                proof_data.verify_proof(SLOT, &leader.pubkey()).unwrap_err(),
                SlashingError::InvalidPayloadProof
            );
        }
    }

    #[test]
    fn test_overlapping_erasure_meta_proof_valid() {
        let mut rng = rand::thread_rng();
        let leader = Arc::new(Keypair::new());
        let shredder = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let coding_shreds =
            new_rand_coding_shreds(&mut rng, next_shred_index, 10, &shredder, &leader, true);
        let (data_shred_next, coding_shred_next) = new_rand_shreds(
            &mut rng,
            next_shred_index + 1,
            next_shred_index + 33,
            10,
            true,
            &shredder,
            &leader,
            true,
        );

        // Fec set is overlapping
        let test_cases = vec![
            (coding_shreds[0].clone(), coding_shred_next[0].clone()),
            (coding_shreds[0].clone(), data_shred_next[0].clone()),
            (
                coding_shreds[2].clone(),
                coding_shred_next.last().unwrap().clone(),
            ),
            (
                coding_shreds[2].clone(),
                data_shred_next.last().unwrap().clone(),
            ),
        ];
        for (shred1, shred2) in test_cases.iter().flat_map(|(a, b)| [(a, b), (b, a)]) {
            let proof_data = generate_proof_data(shred1, shred2);
            proof_data.verify_proof(SLOT, &leader.pubkey()).unwrap();
        }
    }

    #[test]
    fn test_overlapping_erasure_meta_proof_invalid() {
        let mut rng = rand::thread_rng();
        let leader = Arc::new(Keypair::new());
        let shredder = Shredder::new(SLOT, PARENT_SLOT, REFERENCE_TICK, VERSION).unwrap();
        let next_shred_index = rng.gen_range(0..32_000);
        let (data_shred, coding_shred) = new_rand_shreds(
            &mut rng,
            next_shred_index,
            next_shred_index,
            10,
            true,
            &shredder,
            &leader,
            true,
        );
        let next_shred_index = next_shred_index + data_shred.len() as u32;
        let next_code_index = next_shred_index + coding_shred.len() as u32;
        let (data_shred_next, coding_shred_next) = new_rand_shreds(
            &mut rng,
            next_shred_index,
            next_code_index,
            10,
            true,
            &shredder,
            &leader,
            true,
        );
        let test_cases = vec![
            (
                coding_shred[0].clone(),
                data_shred_next[0].clone(),
                SlashingError::ShredTypeMismatch,
            ),
            (
                coding_shred[0].clone(),
                coding_shred_next[0].clone(),
                SlashingError::InvalidErasureMetaConflict,
            ),
            (
                coding_shred[0].clone(),
                data_shred_next.last().unwrap().clone(),
                SlashingError::ShredTypeMismatch,
            ),
            (
                coding_shred[0].clone(),
                coding_shred_next.last().unwrap().clone(),
                SlashingError::InvalidErasureMetaConflict,
            ),
        ];

        for (shred1, shred2, expected) in test_cases
            .iter()
            .flat_map(|(a, b, c)| [(a, b, c), (b, a, c)])
        {
            let proof_data = generate_proof_data(shred1, shred2);
            assert_eq!(
                proof_data.verify_proof(SLOT, &leader.pubkey()).unwrap_err(),
                *expected,
            );
        }
    }
}
