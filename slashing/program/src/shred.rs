//! Shred representation
use {
    crate::error::SlashingError,
    bitflags::bitflags,
    bytemuck::Pod,
    generic_array::{typenum::U64, GenericArray},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    serde_derive::Deserialize,
    solana_program::{
        clock::Slot,
        hash::{hashv, Hash},
    },
    spl_pod::primitives::{PodU16, PodU32, PodU64},
};

pub(crate) const SIZE_OF_SIGNATURE: usize = 64;
const SIZE_OF_SHRED_VARIANT: usize = 1;
const SIZE_OF_SLOT: usize = 8;
const SIZE_OF_INDEX: usize = 4;
const SIZE_OF_VERSION: usize = 2;
const SIZE_OF_FEC_SET_INDEX: usize = 4;
const SIZE_OF_PARENT_OFFSET: usize = 2;
const SIZE_OF_NUM_DATA_SHREDS: usize = 2;
const SIZE_OF_NUM_CODING_SHREDS: usize = 2;
const SIZE_OF_POSITION: usize = 2;

const SIZE_OF_MERKLE_ROOT: usize = 32;
const SIZE_OF_MERKLE_PROOF_ENTRY: usize = 20;

const OFFSET_OF_SHRED_VARIANT: usize = SIZE_OF_SIGNATURE;
const OFFSET_OF_SLOT: usize = SIZE_OF_SIGNATURE + SIZE_OF_SHRED_VARIANT;
const OFFSET_OF_INDEX: usize = OFFSET_OF_SLOT + SIZE_OF_SLOT;
const OFFSET_OF_VERSION: usize = OFFSET_OF_INDEX + SIZE_OF_INDEX;
const OFFSET_OF_FEC_SET_INDEX: usize = OFFSET_OF_VERSION + SIZE_OF_VERSION;

const OFFSET_OF_DATA_PARENT_OFFSET: usize = OFFSET_OF_FEC_SET_INDEX + SIZE_OF_FEC_SET_INDEX;
const OFFSET_OF_DATA_SHRED_FLAGS: usize = OFFSET_OF_DATA_PARENT_OFFSET + SIZE_OF_PARENT_OFFSET;

const OFFSET_OF_CODING_NUM_DATA_SHREDS: usize = OFFSET_OF_FEC_SET_INDEX + SIZE_OF_FEC_SET_INDEX;
const OFFSET_OF_CODING_NUM_CODING_SHREDS: usize =
    OFFSET_OF_CODING_NUM_DATA_SHREDS + SIZE_OF_NUM_DATA_SHREDS;
const OFFSET_OF_CODING_POSITION: usize =
    OFFSET_OF_CODING_NUM_CODING_SHREDS + SIZE_OF_NUM_CODING_SHREDS;

type MerkleProofEntry = [u8; 20];
const MERKLE_HASH_PREFIX_LEAF: &[u8] = b"\x00SOLANA_MERKLE_SHREDS_LEAF";
const MERKLE_HASH_PREFIX_NODE: &[u8] = b"\x01SOLANA_MERKLE_SHREDS_NODE";

#[repr(transparent)]
#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize)]
pub(crate) struct Signature(GenericArray<u8, U64>);

bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize)]
    pub struct ShredFlags:u8 {
        const SHRED_TICK_REFERENCE_MASK = 0b0011_1111;
        const DATA_COMPLETE_SHRED       = 0b0100_0000;
        const LAST_SHRED_IN_SLOT        = 0b1100_0000;
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub(crate) enum ShredType {
    Data = 0b1010_0101,
    Code = 0b0101_1010,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum ShredVariant {
    LegacyCode,
    LegacyData,
    MerkleCode {
        proof_size: u8,
        chained: bool,
        resigned: bool,
    },
    MerkleData {
        proof_size: u8,
        chained: bool,
        resigned: bool,
    },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ErasureMeta {
    num_data_shreds: usize,
    num_coding_shreds: usize,
    first_coding_index: u32,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Shred<'a> {
    shred_type: ShredType,
    proof_size: u8,
    chained: bool,
    resigned: bool,
    payload: &'a [u8],
}

impl<'a> Shred<'a> {
    const SIZE_OF_CODING_PAYLOAD: usize = 1228;
    const SIZE_OF_DATA_PAYLOAD: usize =
        Self::SIZE_OF_CODING_PAYLOAD - Self::SIZE_OF_CODING_HEADERS + SIZE_OF_SIGNATURE;
    const SIZE_OF_CODING_HEADERS: usize = 89;
    const SIZE_OF_DATA_HEADERS: usize = 88;

    pub(crate) fn new_from_payload(payload: &'a [u8]) -> Result<Self, SlashingError> {
        match Self::get_shred_variant(payload)? {
            ShredVariant::LegacyCode | ShredVariant::LegacyData => Err(SlashingError::LegacyShreds),
            ShredVariant::MerkleCode {
                proof_size,
                chained,
                resigned,
            } => Ok(Self {
                shred_type: ShredType::Code,
                proof_size,
                chained,
                resigned,
                payload,
            }),
            ShredVariant::MerkleData {
                proof_size,
                chained,
                resigned,
            } => Ok(Self {
                shred_type: ShredType::Data,
                proof_size,
                chained,
                resigned,
                payload,
            }),
        }
    }

    fn pod_from_bytes<const OFFSET: usize, const SIZE: usize, T: Pod>(
        &self,
    ) -> Result<&T, SlashingError> {
        let end_index: usize = OFFSET
            .checked_add(SIZE)
            .ok_or(SlashingError::ShredDeserializationError)?;
        bytemuck::try_from_bytes(
            self.payload
                .get(OFFSET..end_index)
                .ok_or(SlashingError::ShredDeserializationError)?,
        )
        .map_err(|_| SlashingError::ShredDeserializationError)
    }

    fn get_shred_variant(payload: &'a [u8]) -> Result<ShredVariant, SlashingError> {
        let Some(&shred_variant) = payload.get(OFFSET_OF_SHRED_VARIANT) else {
            return Err(SlashingError::ShredDeserializationError);
        };
        ShredVariant::try_from(shred_variant).map_err(|_| SlashingError::InvalidShredVariant)
    }

    pub(crate) fn slot(&self) -> Result<Slot, SlashingError> {
        self.pod_from_bytes::<OFFSET_OF_SLOT, SIZE_OF_SLOT, PodU64>()
            .map(|x| u64::from(*x))
    }

    pub(crate) fn index(&self) -> Result<u32, SlashingError> {
        self.pod_from_bytes::<OFFSET_OF_INDEX, SIZE_OF_INDEX, PodU32>()
            .map(|x| u32::from(*x))
    }

    pub(crate) fn version(&self) -> Result<u16, SlashingError> {
        self.pod_from_bytes::<OFFSET_OF_VERSION, SIZE_OF_VERSION, PodU16>()
            .map(|x| u16::from(*x))
    }

    pub(crate) fn fec_set_index(&self) -> Result<u32, SlashingError> {
        self.pod_from_bytes::<OFFSET_OF_FEC_SET_INDEX, SIZE_OF_FEC_SET_INDEX, PodU32>()
            .map(|x| u32::from(*x))
    }

    pub(crate) fn shred_type(&self) -> ShredType {
        self.shred_type
    }

    pub(crate) fn last_in_slot(&self) -> Result<bool, SlashingError> {
        debug_assert!(self.shred_type == ShredType::Data);
        let Some(&flags) = self.payload.get(OFFSET_OF_DATA_SHRED_FLAGS) else {
            return Err(SlashingError::ShredDeserializationError);
        };

        let flags: ShredFlags =
            bincode::deserialize(&[flags]).map_err(|_| SlashingError::InvalidShredVariant)?;
        Ok(flags.contains(ShredFlags::LAST_SHRED_IN_SLOT))
    }

    fn num_data_shreds(&self) -> Result<usize, SlashingError> {
        debug_assert!(self.shred_type == ShredType::Code);
        self.pod_from_bytes::<OFFSET_OF_CODING_NUM_DATA_SHREDS, SIZE_OF_NUM_DATA_SHREDS, PodU16>()
            .map(|x| u16::from(*x) as usize)
    }

    fn num_coding_shreds(&self) -> Result<usize, SlashingError> {
        debug_assert!(self.shred_type == ShredType::Code);
        self.pod_from_bytes::<OFFSET_OF_CODING_NUM_CODING_SHREDS, SIZE_OF_NUM_CODING_SHREDS, PodU16>()
            .map(|x| u16::from(*x) as usize)
    }

    fn position(&self) -> Result<usize, SlashingError> {
        debug_assert!(self.shred_type == ShredType::Code);
        self.pod_from_bytes::<OFFSET_OF_CODING_POSITION, SIZE_OF_POSITION, PodU16>()
            .map(|x| u16::from(*x) as usize)
    }

    pub(crate) fn next_fec_set_index(&self) -> Result<u32, SlashingError> {
        debug_assert!(self.shred_type == ShredType::Code);
        let num_data = u32::try_from(self.num_data_shreds()?)
            .map_err(|_| SlashingError::ShredDeserializationError)?;
        self.fec_set_index()?
            .checked_add(num_data)
            .ok_or(SlashingError::ShredDeserializationError)
    }

    pub(crate) fn erasure_meta(&self) -> Result<ErasureMeta, SlashingError> {
        debug_assert!(self.shred_type == ShredType::Code);
        let num_data_shreds = self.num_data_shreds()?;
        let num_coding_shreds = self.num_coding_shreds()?;
        let first_coding_index = self
            .index()?
            .checked_sub(
                u32::try_from(self.position()?)
                    .map_err(|_| SlashingError::ShredDeserializationError)?,
            )
            .ok_or(SlashingError::ShredDeserializationError)?;
        Ok(ErasureMeta {
            num_data_shreds,
            num_coding_shreds,
            first_coding_index,
        })
    }

    fn erasure_batch_index(&self) -> Result<usize, SlashingError> {
        match self.shred_type {
            ShredType::Data => self
                .index()?
                .checked_sub(self.fec_set_index()?)
                .and_then(|x| usize::try_from(x).ok())
                .ok_or(SlashingError::ShredDeserializationError),
            ShredType::Code => self
                .num_data_shreds()?
                .checked_add(self.position()?)
                .ok_or(SlashingError::ShredDeserializationError),
        }
    }

    pub(crate) fn merkle_root(&self) -> Result<Hash, SlashingError> {
        let (proof_offset, proof_size) = self.get_proof_offset_and_size()?;
        let proof_end = proof_offset
            .checked_add(proof_size)
            .ok_or(SlashingError::ShredDeserializationError)?;
        let index = self.erasure_batch_index()?;

        let proof = self
            .payload
            .get(proof_offset..proof_end)
            .ok_or(SlashingError::InvalidMerkleShred)?
            .chunks(SIZE_OF_MERKLE_PROOF_ENTRY)
            .map(<&MerkleProofEntry>::try_from)
            .map(Result::unwrap);
        let node = self
            .payload
            .get(SIZE_OF_SIGNATURE..proof_offset)
            .ok_or(SlashingError::InvalidMerkleShred)?;
        let node = hashv(&[MERKLE_HASH_PREFIX_LEAF, node]);

        Self::get_merkle_root(index, node, proof)
    }

    fn get_proof_offset_and_size(&self) -> Result<(usize, usize), SlashingError> {
        let (header_size, payload_size) = match self.shred_type {
            ShredType::Code => (Self::SIZE_OF_CODING_HEADERS, Self::SIZE_OF_CODING_PAYLOAD),
            ShredType::Data => (Self::SIZE_OF_DATA_HEADERS, Self::SIZE_OF_DATA_PAYLOAD),
        };
        let proof_size = usize::from(self.proof_size)
            .checked_mul(SIZE_OF_MERKLE_PROOF_ENTRY)
            .ok_or(SlashingError::ShredDeserializationError)?;
        let bytes_past_end = header_size
            .checked_add(if self.chained { SIZE_OF_MERKLE_ROOT } else { 0 })
            .and_then(|x| x.checked_add(proof_size))
            .and_then(|x| x.checked_add(if self.resigned { SIZE_OF_SIGNATURE } else { 0 }))
            .ok_or(SlashingError::ShredDeserializationError)?;

        let capacity = payload_size
            .checked_sub(bytes_past_end)
            .ok_or(SlashingError::ShredDeserializationError)?;
        let proof_offset = header_size
            .checked_add(capacity)
            .and_then(|x| x.checked_add(if self.chained { SIZE_OF_MERKLE_ROOT } else { 0 }))
            .ok_or(SlashingError::ShredDeserializationError)?;
        Ok((proof_offset, proof_size))
    }

    // Obtains parent's hash by joining two sibiling nodes in merkle tree.
    fn join_nodes<S: AsRef<[u8]>, T: AsRef<[u8]>>(node: S, other: T) -> Hash {
        let node = &node.as_ref()[..SIZE_OF_MERKLE_PROOF_ENTRY];
        let other = &other.as_ref()[..SIZE_OF_MERKLE_PROOF_ENTRY];
        hashv(&[MERKLE_HASH_PREFIX_NODE, node, other])
    }

    // Recovers root of the merkle tree from a leaf node
    // at the given index and the respective proof.
    fn get_merkle_root<'b, I>(index: usize, node: Hash, proof: I) -> Result<Hash, SlashingError>
    where
        I: IntoIterator<Item = &'b MerkleProofEntry>,
    {
        let (index, root) = proof
            .into_iter()
            .fold((index, node), |(index, node), other| {
                let parent = if index % 2 == 0 {
                    Self::join_nodes(node, other)
                } else {
                    Self::join_nodes(other, node)
                };
                (index >> 1, parent)
            });
        (index == 0)
            .then_some(root)
            .ok_or(SlashingError::InvalidMerkleShred)
    }

    /// Returns true if the other shred has the same (slot, index,
    /// shred-type), but different payload.
    /// Retransmitter's signature is ignored when comparing payloads.
    pub(crate) fn is_shred_duplicate(&self, other: &Shred) -> bool {
        if (self.slot(), self.index(), self.shred_type())
            != (other.slot(), other.index(), other.shred_type())
        {
            return false;
        }
        fn get_payload<'a>(shred: &Shred<'a>) -> &'a [u8] {
            let Ok((proof_offset, proof_size)) = shred.get_proof_offset_and_size() else {
                return shred.payload;
            };
            if !shred.resigned {
                return shred.payload;
            }
            let Some(offset) = proof_offset.checked_add(proof_size) else {
                return shred.payload;
            };
            shred.payload.get(..offset).unwrap_or(shred.payload)
        }
        get_payload(self) != get_payload(other)
    }

    /// Returns true if the erasure metas of the other shred matches ours.
    /// Assumes that other shred has the same fec set index as ours.
    pub(crate) fn check_erasure_consistency(&self, other: &Shred) -> Result<bool, SlashingError> {
        debug_assert!(self.fec_set_index() == other.fec_set_index());
        debug_assert!(self.shred_type == ShredType::Code);
        debug_assert!(other.shred_type == ShredType::Code);
        Ok(self.erasure_meta()? == other.erasure_meta()?)
    }
}

impl TryFrom<u8> for ShredVariant {
    type Error = SlashingError;
    fn try_from(shred_variant: u8) -> Result<Self, Self::Error> {
        if shred_variant == u8::from(ShredType::Code) {
            Ok(ShredVariant::LegacyCode)
        } else if shred_variant == u8::from(ShredType::Data) {
            Ok(ShredVariant::LegacyData)
        } else {
            let proof_size = shred_variant & 0x0F;
            match shred_variant & 0xF0 {
                0x40 => Ok(ShredVariant::MerkleCode {
                    proof_size,
                    chained: false,
                    resigned: false,
                }),
                0x60 => Ok(ShredVariant::MerkleCode {
                    proof_size,
                    chained: true,
                    resigned: false,
                }),
                0x70 => Ok(ShredVariant::MerkleCode {
                    proof_size,
                    chained: true,
                    resigned: true,
                }),
                0x80 => Ok(ShredVariant::MerkleData {
                    proof_size,
                    chained: false,
                    resigned: false,
                }),
                0x90 => Ok(ShredVariant::MerkleData {
                    proof_size,
                    chained: true,
                    resigned: false,
                }),
                0xb0 => Ok(ShredVariant::MerkleData {
                    proof_size,
                    chained: true,
                    resigned: true,
                }),
                _ => Err(SlashingError::InvalidShredVariant),
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use {
        super::Shred,
        crate::shred::ShredType,
        rand::Rng,
        solana_entry::entry::Entry,
        solana_ledger::shred::{
            ProcessShredsStats, ReedSolomonCache, Shred as SolanaShred, Shredder,
        },
        solana_sdk::{hash::Hash, pubkey::Pubkey, signature::Keypair, system_transaction},
        std::sync::Arc,
    };

    pub(crate) fn new_rand_data_shred<R: Rng>(
        rng: &mut R,
        next_shred_index: u32,
        shredder: &Shredder,
        keypair: &Keypair,
        merkle_variant: bool,
        is_last_in_slot: bool,
    ) -> SolanaShred {
        let (mut data_shreds, _) = new_rand_shreds(
            rng,
            next_shred_index,
            next_shred_index,
            5,
            merkle_variant,
            shredder,
            keypair,
            is_last_in_slot,
        );
        data_shreds.pop().unwrap()
    }

    pub(crate) fn new_rand_coding_shreds<R: Rng>(
        rng: &mut R,
        next_shred_index: u32,
        num_entries: usize,
        shredder: &Shredder,
        keypair: &Keypair,
        merkle_variant: bool,
    ) -> Vec<SolanaShred> {
        let (_, coding_shreds) = new_rand_shreds(
            rng,
            next_shred_index,
            next_shred_index,
            num_entries,
            merkle_variant,
            shredder,
            keypair,
            true,
        );
        coding_shreds
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_rand_shreds<R: Rng>(
        rng: &mut R,
        next_shred_index: u32,
        next_code_index: u32,
        num_entries: usize,
        merkle_variant: bool,
        shredder: &Shredder,
        keypair: &Keypair,
        is_last_in_slot: bool,
    ) -> (Vec<SolanaShred>, Vec<SolanaShred>) {
        let entries: Vec<_> = std::iter::repeat_with(|| {
            let tx = system_transaction::transfer(
                &Keypair::new(),       // from
                &Pubkey::new_unique(), // to
                rng.gen(),             // lamports
                Hash::new_unique(),    // recent blockhash
            );
            Entry::new(
                &Hash::new_unique(), // prev_hash
                1,                   // num_hashes,
                vec![tx],            // transactions
            )
        })
        .take(num_entries)
        .collect();
        shredder.entries_to_shreds(
            keypair,
            &entries,
            is_last_in_slot,
            // chained_merkle_root
            Some(Hash::new_from_array(rng.gen())),
            next_shred_index,
            next_code_index, // next_code_index
            merkle_variant,
            &ReedSolomonCache::default(),
            &mut ProcessShredsStats::default(),
        )
    }

    #[test]
    fn test_solana_shred_parity() {
        // Verify that the deserialization functions match solana shred format
        for _ in 0..300 {
            let mut rng = rand::thread_rng();
            let leader = Arc::new(Keypair::new());
            let slot = rng.gen_range(1..u64::MAX);
            let parent_slot = slot - 1;
            let reference_tick = 0;
            let version = rng.gen_range(0..u16::MAX);
            let shredder = Shredder::new(slot, parent_slot, reference_tick, version).unwrap();
            let next_shred_index = rng.gen_range(0..671);
            let next_code_index = rng.gen_range(0..781);
            let is_last_in_slot = rng.gen_bool(0.5);
            let (data_solana_shreds, coding_solana_shreds) = new_rand_shreds(
                &mut rng,
                next_shred_index,
                next_code_index,
                10,
                true,
                &shredder,
                &leader,
                is_last_in_slot,
            );

            for solana_shred in data_solana_shreds
                .into_iter()
                .chain(coding_solana_shreds.into_iter())
            {
                let payload = solana_shred.payload().as_slice();
                let shred = Shred::new_from_payload(payload).unwrap();

                assert_eq!(shred.slot().unwrap(), solana_shred.slot());
                assert_eq!(shred.index().unwrap(), solana_shred.index());
                assert_eq!(shred.version().unwrap(), solana_shred.version());
                assert_eq!(
                    u8::from(shred.shred_type()),
                    u8::from(solana_shred.shred_type())
                );
                if shred.shred_type() == ShredType::Data {
                    assert_eq!(shred.last_in_slot().unwrap(), solana_shred.last_in_slot());
                } else {
                    let erasure_meta = shred.erasure_meta().unwrap();
                    assert_eq!(
                        erasure_meta.num_data_shreds,
                        shred.num_data_shreds().unwrap()
                    );
                    assert_eq!(
                        erasure_meta.num_coding_shreds,
                        shred.num_coding_shreds().unwrap()
                    );
                    // We cannot verify first_coding_index until visibility is
                    // changed in agave
                }
                assert_eq!(
                    shred.merkle_root().unwrap(),
                    solana_shred.merkle_root().unwrap()
                );
                assert_eq!(&shred.payload, solana_shred.payload());
            }
        }
    }
}
