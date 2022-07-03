//! On-chain reference database

use {
    crate::{
        pack::{
            as64_deserialize, as64_serialize, check_data_len, pack_array_string64,
            unpack_array_string64,
        },
        string::ArrayString64,
        traits::*,
    },
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    num_enum::TryFromPrimitive,
    serde::{Deserialize, Serialize},
    solana_program::{program_error::ProgramError, pubkey::Pubkey},
    std::mem::size_of,
};

/// Whether to init refdb accounts from on-chain program or off-chain.
/// Main Router admin key is used as the Base address to derive refdb address
/// if off-chain initialization is selected.
/// Off-chain is required for accounts with data size > 10K.
/// This is temporary solution until realloc is implemented.
pub const REFDB_ONCHAIN_INIT: bool = false;

/// Storage Header, one per account
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct Header {
    pub counter: u32,
    pub active_records: u32,
    pub reference_type: ReferenceType,
    #[serde(
        serialize_with = "as64_serialize",
        deserialize_with = "as64_deserialize"
    )]
    pub name: ArrayString64,
}

impl Header {
    pub const LEN: usize = 73;
    const REF_TYPE_OFFSET: usize = 8;
    const NAME_OFFSET: usize = 9;

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, Header::LEN)?;

        let output = array_mut_ref![output, 0, Header::LEN];

        let (counter_out, active_records_out, reference_type_out, name_out) =
            mut_array_refs![output, 4, 4, 1, 64];
        *counter_out = self.counter.to_le_bytes();
        *active_records_out = self.active_records.to_le_bytes();
        reference_type_out[0] = self.reference_type as u8;
        pack_array_string64(&self.name, name_out);

        Ok(Header::LEN)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; Header::LEN] = [0; Header::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    pub fn unpack(input: &[u8]) -> Result<Header, ProgramError> {
        check_data_len(input, Header::LEN)?;

        let input = array_ref![input, 0, Header::LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (counter, active_records, reference_type, name) = array_refs![input, 4, 4, 1, 64];

        Ok(Self {
            counter: u32::from_le_bytes(*counter),
            active_records: u32::from_le_bytes(*active_records),
            reference_type: ReferenceType::try_from_primitive(reference_type[0])
                .or(Err(ProgramError::InvalidAccountData))?,
            name: unpack_array_string64(name)?,
        })
    }
}

/// Reference is a short, fixed size data field, used to store homogeneous value
/// or a link to the account with more detailed data
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum Reference {
    Pubkey { data: Pubkey },
    U8 { data: u8 },
    U16 { data: u16 },
    U32 { data: u32 },
    U64 { data: u64 },
    F64 { data: f64 },
    Empty,
}

impl Reference {
    pub const MAX_LEN: usize = 32;
    pub const PUBKEY_LEN: usize = size_of::<Pubkey>();
    pub const U8_LEN: usize = size_of::<u8>();
    pub const U16_LEN: usize = size_of::<u16>();
    pub const U32_LEN: usize = size_of::<u32>();
    pub const U64_LEN: usize = size_of::<u64>();
    pub const F64_LEN: usize = size_of::<f64>();

    pub const fn get_type(&self) -> ReferenceType {
        match self {
            Reference::Pubkey { .. } => ReferenceType::Pubkey,
            Reference::U8 { .. } => ReferenceType::U8,
            Reference::U16 { .. } => ReferenceType::U16,
            Reference::U32 { .. } => ReferenceType::U32,
            Reference::U64 { .. } => ReferenceType::U64,
            Reference::F64 { .. } => ReferenceType::F64,
            Reference::Empty => ReferenceType::Empty,
        }
    }
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum ReferenceType {
    Pubkey,
    U8,
    U16,
    U32,
    U64,
    F64,
    Empty,
}

impl ReferenceType {
    pub const fn get_size(&self) -> usize {
        match self {
            ReferenceType::Pubkey => size_of::<Pubkey>(),
            ReferenceType::U8 => size_of::<u8>(),
            ReferenceType::U16 => size_of::<u16>(),
            ReferenceType::U32 => size_of::<u32>(),
            ReferenceType::U64 => size_of::<u64>(),
            ReferenceType::F64 => size_of::<f64>(),
            ReferenceType::Empty => 0,
        }
    }
}

impl std::fmt::Display for ReferenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ReferenceType::Pubkey => write!(f, "Pubkey"),
            ReferenceType::U8 => write!(f, "U8"),
            ReferenceType::U16 => write!(f, "U16"),
            ReferenceType::U32 => write!(f, "U32"),
            ReferenceType::U64 => write!(f, "U64"),
            ReferenceType::F64 => write!(f, "F64"),
            ReferenceType::Empty => write!(f, "Empty"),
        }
    }
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum StorageType {
    Program,
    Vault,
    Pool,
    Farm,
    Token,
    Other,
}

impl StorageType {
    pub const fn get_default_size(storage_type: StorageType) -> usize {
        match storage_type {
            StorageType::Program => 25000usize,
            StorageType::Vault => 25000usize,
            StorageType::Pool => 50000usize,
            StorageType::Farm => 25000usize,
            StorageType::Token => 500000usize,
            _ => 0usize,
        }
    }

    pub const fn get_default_max_records(
        storage_type: StorageType,
        reference_type: ReferenceType,
    ) -> usize {
        let record_size = Record::get_size_with_reference(reference_type);
        (StorageType::get_default_size(storage_type) - Header::LEN) / record_size
    }

    pub const fn get_storage_size_for_records(
        reference_type: ReferenceType,
        records_num: usize,
    ) -> usize {
        if records_num > u32::MAX as usize {
            return 0;
        }
        let record_size = Record::get_size_with_reference(reference_type);
        records_num * record_size + Header::LEN
    }

    pub const fn get_storage_size_for_max_records(
        storage_type: StorageType,
        reference_type: ReferenceType,
    ) -> usize {
        StorageType::get_storage_size_for_records(
            reference_type,
            StorageType::get_default_max_records(storage_type, reference_type),
        )
    }
}

impl std::fmt::Display for StorageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            StorageType::Program => write!(f, "Program"),
            StorageType::Vault => write!(f, "Vault"),
            StorageType::Pool => write!(f, "Pool"),
            StorageType::Farm => write!(f, "Farm"),
            StorageType::Token => write!(f, "Token"),
            StorageType::Other => write!(f, "Other"),
        }
    }
}

impl std::str::FromStr for StorageType {
    type Err = ProgramError;

    fn from_str(s: &str) -> Result<Self, ProgramError> {
        match s {
            "Program" => Ok(StorageType::Program),
            "Vault" => Ok(StorageType::Vault),
            "Pool" => Ok(StorageType::Pool),
            "Farm" => Ok(StorageType::Farm),
            "Token" => Ok(StorageType::Token),
            "Other" => Ok(StorageType::Other),
            _ => Err(ProgramError::InvalidArgument),
        }
    }
}

/// Data record; All records have the same reference type for single storage
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Record {
    // index is the record location index [0..total_records-1] and is not stored on-chain,
    // but returned to the reader for more efficient consecutive read/writes.
    // if index is set to None record will be looked up by name with linear search.
    pub index: Option<u32>,
    pub counter: u16,
    pub tag: u16,
    #[serde(
        serialize_with = "as64_serialize",
        deserialize_with = "as64_deserialize"
    )]
    pub name: ArrayString64,
    pub reference: Reference,
}

impl Named for Record {
    fn name(&self) -> ArrayString64 {
        self.name
    }
}

impl Record {
    pub const NO_REF_LEN: usize = 68;
    pub const MAX_LEN: usize = Record::NO_REF_LEN + Reference::MAX_LEN;

    pub const fn get_size(&self) -> usize {
        match self.reference {
            Reference::Pubkey { .. } => Record::NO_REF_LEN + size_of::<Pubkey>(),
            Reference::U8 { .. } => Record::NO_REF_LEN + size_of::<u8>(),
            Reference::U16 { .. } => Record::NO_REF_LEN + size_of::<u16>(),
            Reference::U32 { .. } => Record::NO_REF_LEN + size_of::<u32>(),
            Reference::U64 { .. } => Record::NO_REF_LEN + size_of::<u64>(),
            Reference::F64 { .. } => Record::NO_REF_LEN + size_of::<f64>(),
            Reference::Empty => Record::NO_REF_LEN,
        }
    }

    pub const fn get_size_with_reference(reference_type: ReferenceType) -> usize {
        Record::NO_REF_LEN + reference_type.get_size()
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        let record_size = self.get_size();
        check_data_len(output, record_size)?;

        match self.reference {
            Reference::Pubkey { data } => self.pack_with_pubkey(output, &data),
            Reference::U8 { data } => self.pack_with_u8(output, data),
            Reference::U16 { data } => self.pack_with_u16(output, data),
            Reference::U32 { data } => self.pack_with_u32(output, data),
            Reference::U64 { data } => self.pack_with_u64(output, data),
            Reference::F64 { data } => self.pack_with_f64(output, data),
            Reference::Empty => self.pack_with_empty(output),
        }

        Ok(record_size)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; Record::MAX_LEN] = [0; Record::MAX_LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    pub fn unpack(
        input: &[u8],
        reference_type: ReferenceType,
        index: Option<u32>,
    ) -> Result<Record, ProgramError> {
        let record_size = Record::NO_REF_LEN + reference_type.get_size();
        check_data_len(input, record_size)?;

        match reference_type {
            ReferenceType::Pubkey => Record::unpack_with_pubkey(input, index),
            ReferenceType::U8 => Record::unpack_with_u8(input, index),
            ReferenceType::U16 => Record::unpack_with_u16(input, index),
            ReferenceType::U32 => Record::unpack_with_u32(input, index),
            ReferenceType::U64 => Record::unpack_with_u64(input, index),
            ReferenceType::F64 => Record::unpack_with_f64(input, index),
            ReferenceType::Empty => Record::unpack_with_empty(input, index),
        }
    }

    pub fn unpack_counter(input: &[u8]) -> Result<usize, ProgramError> {
        check_data_len(input, Record::NO_REF_LEN)?;
        let counter = array_ref![input, 0, 2];
        Ok(u16::from_le_bytes(*counter) as usize)
    }

    pub fn unpack_counter_and_name(input: &[u8]) -> Result<(usize, ArrayString64), ProgramError> {
        check_data_len(input, Record::NO_REF_LEN)?;
        let counter = array_ref![input, 0, 2];
        let name = array_ref![input, 4, 64];
        Ok((
            u16::from_le_bytes(*counter) as usize,
            unpack_array_string64(name)?,
        ))
    }

    fn pack_with_pubkey(&self, output: &mut [u8], data: &Pubkey) {
        let output = array_mut_ref![output, 0, Record::NO_REF_LEN + Reference::PUBKEY_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (counter_out, tag_out, name_out, reference_out) =
            mut_array_refs![output, 2, 2, 64, Reference::PUBKEY_LEN];
        *counter_out = self.counter.to_le_bytes();
        *tag_out = self.tag.to_le_bytes();
        pack_array_string64(&self.name, name_out);
        reference_out.copy_from_slice(data.as_ref());
    }

    fn pack_with_u8(&self, output: &mut [u8], data: u8) {
        let output = array_mut_ref![output, 0, Record::NO_REF_LEN + Reference::U8_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (counter_out, tag_out, name_out, reference_out) =
            mut_array_refs![output, 2, 2, 64, Reference::U8_LEN];
        *counter_out = self.counter.to_le_bytes();
        *tag_out = self.tag.to_le_bytes();
        pack_array_string64(&self.name, name_out);
        *reference_out = data.to_le_bytes();
    }

    fn pack_with_u16(&self, output: &mut [u8], data: u16) {
        let output = array_mut_ref![output, 0, Record::NO_REF_LEN + Reference::U16_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (counter_out, tag_out, name_out, reference_out) =
            mut_array_refs![output, 2, 2, 64, Reference::U16_LEN];
        *counter_out = self.counter.to_le_bytes();
        *tag_out = self.tag.to_le_bytes();
        pack_array_string64(&self.name, name_out);
        *reference_out = data.to_le_bytes();
    }

    fn pack_with_u32(&self, output: &mut [u8], data: u32) {
        let output = array_mut_ref![output, 0, Record::NO_REF_LEN + Reference::U32_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (counter_out, tag_out, name_out, reference_out) =
            mut_array_refs![output, 2, 2, 64, Reference::U32_LEN];
        *counter_out = self.counter.to_le_bytes();
        *tag_out = self.tag.to_le_bytes();
        pack_array_string64(&self.name, name_out);
        *reference_out = data.to_le_bytes();
    }

    fn pack_with_u64(&self, output: &mut [u8], data: u64) {
        let output = array_mut_ref![output, 0, Record::NO_REF_LEN + Reference::U64_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (counter_out, tag_out, name_out, reference_out) =
            mut_array_refs![output, 2, 2, 64, Reference::U64_LEN];
        *counter_out = self.counter.to_le_bytes();
        *tag_out = self.tag.to_le_bytes();
        pack_array_string64(&self.name, name_out);
        *reference_out = data.to_le_bytes();
    }

    fn pack_with_f64(&self, output: &mut [u8], data: f64) {
        let output = array_mut_ref![output, 0, Record::NO_REF_LEN + Reference::F64_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (counter_out, tag_out, name_out, reference_out) =
            mut_array_refs![output, 2, 2, 64, Reference::F64_LEN];
        *counter_out = self.counter.to_le_bytes();
        *tag_out = self.tag.to_le_bytes();
        pack_array_string64(&self.name, name_out);
        *reference_out = data.to_le_bytes();
    }

    fn pack_with_empty(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, Record::NO_REF_LEN];
        let (counter_out, tag_out, name_out) = mut_array_refs![output, 2, 2, 64];
        *counter_out = self.counter.to_le_bytes();
        *tag_out = self.tag.to_le_bytes();
        pack_array_string64(&self.name, name_out);
    }

    fn unpack_with_pubkey(input: &[u8], index: Option<u32>) -> Result<Record, ProgramError> {
        let input = array_ref![input, 0, Record::NO_REF_LEN + Reference::PUBKEY_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (counter, tag, name, reference) = array_refs![input, 2, 2, 64, Reference::PUBKEY_LEN];
        Ok(Self {
            index,
            counter: u16::from_le_bytes(*counter),
            tag: u16::from_le_bytes(*tag),
            name: unpack_array_string64(name)?,
            reference: Reference::Pubkey {
                data: Pubkey::new_from_array(*reference),
            },
        })
    }

    fn unpack_with_u8(input: &[u8], index: Option<u32>) -> Result<Record, ProgramError> {
        let input = array_ref![input, 0, Record::NO_REF_LEN + Reference::U8_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (counter, tag, name, reference) = array_refs![input, 2, 2, 64, Reference::U8_LEN];
        Ok(Self {
            index,
            counter: u16::from_le_bytes(*counter),
            tag: u16::from_le_bytes(*tag),
            name: unpack_array_string64(name)?,
            reference: Reference::U8 { data: reference[0] },
        })
    }

    fn unpack_with_u16(input: &[u8], index: Option<u32>) -> Result<Record, ProgramError> {
        let input = array_ref![input, 0, Record::NO_REF_LEN + Reference::U16_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (counter, tag, name, reference) = array_refs![input, 2, 2, 64, Reference::U16_LEN];
        Ok(Self {
            index,
            counter: u16::from_le_bytes(*counter),
            tag: u16::from_le_bytes(*tag),
            name: unpack_array_string64(name)?,
            reference: Reference::U16 {
                data: u16::from_le_bytes(*reference),
            },
        })
    }

    fn unpack_with_u32(input: &[u8], index: Option<u32>) -> Result<Record, ProgramError> {
        let input = array_ref![input, 0, Record::NO_REF_LEN + Reference::U32_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (counter, tag, name, reference) = array_refs![input, 2, 2, 64, Reference::U32_LEN];
        Ok(Self {
            index,
            counter: u16::from_le_bytes(*counter),
            tag: u16::from_le_bytes(*tag),
            name: unpack_array_string64(name)?,
            reference: Reference::U32 {
                data: u32::from_le_bytes(*reference),
            },
        })
    }

    fn unpack_with_u64(input: &[u8], index: Option<u32>) -> Result<Record, ProgramError> {
        let input = array_ref![input, 0, Record::NO_REF_LEN + Reference::U64_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (counter, tag, name, reference) = array_refs![input, 2, 2, 64, Reference::U64_LEN];
        Ok(Self {
            index,
            counter: u16::from_le_bytes(*counter),
            tag: u16::from_le_bytes(*tag),
            name: unpack_array_string64(name)?,
            reference: Reference::U64 {
                data: u64::from_le_bytes(*reference),
            },
        })
    }

    fn unpack_with_f64(input: &[u8], index: Option<u32>) -> Result<Record, ProgramError> {
        let input = array_ref![input, 0, Record::NO_REF_LEN + Reference::F64_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (counter, tag, name, reference) = array_refs![input, 2, 2, 64, Reference::F64_LEN];
        Ok(Self {
            index,
            counter: u16::from_le_bytes(*counter),
            tag: u16::from_le_bytes(*tag),
            name: unpack_array_string64(name)?,
            reference: Reference::F64 {
                data: f64::from_le_bytes(*reference),
            },
        })
    }

    fn unpack_with_empty(input: &[u8], index: Option<u32>) -> Result<Record, ProgramError> {
        let input = array_ref![input, 0, Record::NO_REF_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (counter, tag, name) = array_refs![input, 2, 2, 64];
        Ok(Self {
            index,
            counter: u16::from_le_bytes(*counter),
            tag: u16::from_le_bytes(*tag),
            name: unpack_array_string64(name)?,
            reference: Reference::Empty,
        })
    }
}

/// RefDB manages homogeneous records in a given continuous storage
pub struct RefDB {}

impl RefDB {
    /// Initializes the storage, must be called before first read/write
    pub fn init(
        data: &mut [u8],
        name: &ArrayString64,
        reference_type: ReferenceType,
    ) -> Result<(), ProgramError> {
        if RefDB::is_initialized(data) {
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        let record_size = Record::NO_REF_LEN + reference_type.get_size();
        check_data_len(data, Header::LEN + record_size)?;
        let header = Header {
            counter: 1,
            active_records: 0,
            reference_type,
            name: *name,
        };
        header.pack(data)?;
        Ok(())
    }

    /// Clears out underlying storage
    pub fn drop(data: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(data, Header::LEN)?;
        if data.len() > 2000 {
            Err(ProgramError::Custom(431))
        } else {
            data.fill(0);
            Ok(data.len())
        }
    }

    /// Checks if the storage is empty
    pub fn is_empty(data: &[u8]) -> Result<bool, ProgramError> {
        Ok(RefDB::get_active_records(data)? == 0)
    }

    /// Checks if the storage is full
    pub fn is_full(data: &[u8]) -> Result<bool, ProgramError> {
        Ok(RefDB::get_free_records(data)? == 0)
    }

    /// Checks if the storage has been updated
    pub fn is_updated(data: &[u8], last_counter: usize) -> Result<bool, ProgramError> {
        Ok(RefDB::get_storage_counter(data)? > last_counter)
    }

    /// Checks if data storage has been initialized.
    /// It can return false positives if storage is not managed by RefDB.
    pub fn is_initialized(data: &[u8]) -> bool {
        if let Ok(header) = Header::unpack(data) {
            if let Ok(rec_size) = RefDB::get_record_size(data) {
                if header.counter > 0
                    && header.active_records as usize <= (data.len() - Header::LEN) / rec_size
                {
                    return true;
                }
            }
        }
        false
    }

    /// Returns unpacked storage header
    pub fn get_storage_header(data: &[u8]) -> Result<Header, ProgramError> {
        Header::unpack(data)
    }

    /// Returns the storage updates counter
    pub fn get_storage_counter(data: &[u8]) -> Result<usize, ProgramError> {
        check_data_len(data, Header::LEN)?;
        let counter = u32::from_le_bytes(*array_ref![data, 0, 4]) as usize;
        if counter > 0 {
            Ok(counter)
        } else {
            Err(ProgramError::UninitializedAccount)
        }
    }

    /// Sets the storage counter to the new value
    pub fn set_storage_counter(data: &mut [u8], counter: usize) -> Result<usize, ProgramError> {
        if counter == 0 {
            return Err(ProgramError::InvalidArgument);
        }
        check_data_len(data, Header::LEN)?;

        let counter_out = array_mut_ref![data, 0, 4];
        *counter_out = (counter as u32).to_le_bytes();

        Ok(counter)
    }

    /// Returns the number of active records (not marked as deleted)
    pub fn get_active_records(data: &[u8]) -> Result<usize, ProgramError> {
        check_data_len(data, Header::LEN)?;
        Ok(u32::from_le_bytes(*array_ref![data, 4, 4]) as usize)
    }

    /// Sets the number of active records to the new value
    pub fn set_active_records(data: &mut [u8], records: usize) -> Result<usize, ProgramError> {
        check_data_len(data, Header::LEN)?;

        let records_out = array_mut_ref![data, 4, 4];
        *records_out = (records as u32).to_le_bytes();

        Ok(records)
    }

    /// Returns the number of free records
    pub fn get_free_records(data: &[u8]) -> Result<usize, ProgramError> {
        let rec_size = RefDB::get_record_size(data)?;
        Ok((data.len() - RefDB::get_active_records(data)? * rec_size - Header::LEN) / rec_size)
    }

    /// Returns total number of allocated records
    pub fn get_total_records(data: &[u8]) -> Result<usize, ProgramError> {
        Ok((data.len() - Header::LEN) / RefDB::get_record_size(data)?)
    }

    /// Returns the length of a single record
    pub fn get_record_size(data: &[u8]) -> Result<usize, ProgramError> {
        Ok(Record::NO_REF_LEN + RefDB::get_reference_type(data)?.get_size())
    }

    /// Returns the type of reference data
    pub fn get_reference_type(data: &[u8]) -> Result<ReferenceType, ProgramError> {
        check_data_len(data, Header::LEN)?;
        ReferenceType::try_from_primitive(data[Header::REF_TYPE_OFFSET])
            .or(Err(ProgramError::InvalidAccountData))
    }

    /// Returns the name of the DB
    pub fn get_name(data: &[u8]) -> Result<ArrayString64, ProgramError> {
        check_data_len(data, Header::LEN)?;
        let name = array_ref![data, Header::NAME_OFFSET, 64];
        unpack_array_string64(name)
    }

    /// Returns the record associated with the given name
    pub fn read(data: &[u8], name: &ArrayString64) -> Result<Option<Record>, ProgramError> {
        if let Some(index) = RefDB::find_index(data, name)? {
            RefDB::read_at(data, index)
        } else {
            Ok(None)
        }
    }

    /// Returns the record at the given index
    pub fn read_at(data: &[u8], index: usize) -> Result<Option<Record>, ProgramError> {
        let offset = RefDB::get_offset(data, index)?;
        let ref_type = RefDB::get_reference_type(data)?;
        let record = Record::unpack(&data[offset..], ref_type, Some(index as u32))?;
        if record.counter > 0 {
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    /// Returns the record only if it has been updated
    pub fn read_if_changed(
        data: &[u8],
        name: &ArrayString64,
        last_counter: usize,
    ) -> Result<Option<Record>, ProgramError> {
        if let Some(index) = RefDB::find_index(data, name)? {
            RefDB::read_at_if_changed(data, index, last_counter)
        } else {
            Ok(None)
        }
    }

    /// Returns the record at the given index only if it has been updated
    pub fn read_at_if_changed(
        data: &[u8],
        index: usize,
        last_counter: usize,
    ) -> Result<Option<Record>, ProgramError> {
        let offset = RefDB::get_offset(data, index)?;
        let counter = RefDB::get_record_counter(data, offset)?;
        if counter > last_counter {
            RefDB::read_at(data, index)
        } else {
            Ok(None)
        }
    }

    /// Returns all active records in the storage
    pub fn read_all(data: &[u8]) -> Result<Vec<Record>, ProgramError> {
        let rec_num = RefDB::get_total_records(data)?;
        let active_rec = RefDB::get_active_records(data)?;
        let mut vec: Vec<Record> = vec![];
        if active_rec == 0 {
            return Ok(vec);
        }
        for i in 0..rec_num {
            if let Some(rec) = RefDB::read_at(data, i)? {
                vec.push(rec);
                if vec.len() == active_rec {
                    return Ok(vec);
                }
            }
        }
        Err(ProgramError::InvalidAccountData)
    }

    /// Returns all active records in the storage if any of them was updated
    pub fn read_all_if_changed(
        data: &[u8],
        last_counter: usize,
    ) -> Result<Vec<Record>, ProgramError> {
        if RefDB::get_storage_counter(data)? > last_counter {
            RefDB::read_all(data)
        } else {
            Ok(Vec::<Record>::default())
        }
    }

    /// Writes the record to the storage.
    /// Uses the index if provided or searches the record by name.
    /// If counter is provided it must be equal to stored value or error is returned.
    pub fn write(data: &mut [u8], record: &Record) -> Result<usize, ProgramError> {
        let offset = if let Some(idx) = record.index {
            // if the index was specified we will update existing record
            RefDB::get_offset(data, idx as usize)?
        } else {
            // otherwise either find a record with the supplied name or first available slot
            RefDB::find_write_offset(data, &record.name)?
        };
        let (cur_counter, cur_name) = RefDB::get_record_counter_and_name(data, offset)?;
        if cur_counter > 0 {
            // if the counter was specified we check that value is equal to stored,
            // to make sure there were no intermediate updates
            if record.counter > 0 && cur_counter != record.counter as usize {
                return Err(ProgramError::Custom(409));
            }
            // check that we are either writing to an empty slot or record name matches
            if record.index.is_some() && record.name != cur_name {
                return Err(ProgramError::Custom(409));
            }
        }
        // check that reference data type matches storage
        if RefDB::get_reference_type(data)? != record.reference.get_type() {
            return Err(ProgramError::Custom(409));
        }
        // update storage counters
        let storage_counter = RefDB::get_storage_counter(data)?;
        if (storage_counter as u32) < u32::MAX {
            RefDB::set_storage_counter(data, storage_counter + 1)?;
        } else {
            RefDB::set_storage_counter(data, 1)?;
        }
        if cur_counter == 0 {
            let active_records = RefDB::get_active_records(data)?;
            if (active_records as u32) < u32::MAX {
                RefDB::set_active_records(data, active_records + 1)?;
            }
        }
        // write record
        let res = record.pack(&mut data[offset..]);
        // update record counter
        if (cur_counter as u16) < u16::MAX {
            RefDB::set_record_counter(data, offset, cur_counter + 1);
        } else {
            RefDB::set_record_counter(data, offset, 1);
        }
        res
    }

    /// Updates the reference value in the storage for the record with the given name.
    /// It doesn't validate storage type, counter or name. Should be used only if
    /// record is active (i.e. to update existing record), you are certain that
    /// the storage and index are correct, and you don't care if the record was
    /// updated since last read time.
    pub fn update(
        data: &mut [u8],
        name: &ArrayString64,
        reference: &Reference,
    ) -> Result<usize, ProgramError> {
        if let Some(index) = RefDB::find_index(data, name)? {
            RefDB::update_at(data, index, reference)
        } else {
            Err(ProgramError::Custom(404))
        }
    }

    /// Updates the reference value in the storage at the given index.
    /// It doesn't validate storage type, counter or name. Should be used only if
    /// record is active (i.e. to update existing record), you are certain that
    /// the storage and index are correct, and you don't care if the record was
    /// updated since last read time.
    pub fn update_at(
        data: &mut [u8],
        index: usize,
        reference: &Reference,
    ) -> Result<usize, ProgramError> {
        let offset = RefDB::get_offset(data, index)?;
        let mut cur_record = Record::unpack(&data[offset..], reference.get_type(), None)?;
        // update storage counters
        let storage_counter = RefDB::get_storage_counter(data)?;
        if (storage_counter as u32) < u32::MAX {
            RefDB::set_storage_counter(data, storage_counter + 1)?;
        } else {
            RefDB::set_storage_counter(data, 1)?;
        }
        if cur_record.counter == 0 {
            return Err(ProgramError::UninitializedAccount);
        }
        // write record
        if (cur_record.counter as u16) < u16::MAX {
            cur_record.counter += 1;
        } else {
            cur_record.counter = 1;
        }
        cur_record.reference = *reference;
        cur_record.pack(&mut data[offset..])
    }

    /// Deletes the record from the storage.
    /// Uses the index if provided or searches the record by name.
    /// If counter is provided it checks that it is equal to stored or error is returned.
    pub fn delete(data: &mut [u8], record: &Record) -> Result<usize, ProgramError> {
        let offset = if let Some(idx) = record.index {
            // if the index was specified we will update existing record
            RefDB::get_offset(data, idx as usize)?
        } else {
            // otherwise either find a record with the supplied name or first available slot
            RefDB::find_write_offset(data, &record.name)?
        };
        let data_end = offset + record.get_size();
        check_data_len(data, data_end)?;
        let (stored_counter, stored_name) = RefDB::get_record_counter_and_name(data, offset)?;
        if stored_counter == 0 {
            return Ok(0);
        }
        // if the counter was specified we check that value is equal to stored,
        // to make sure there were no intermediate updates
        if record.counter > 0 && stored_counter != record.counter as usize {
            return Err(ProgramError::Custom(409));
        }
        // check that we are deleting record with matching name
        if record.name != stored_name {
            return Err(ProgramError::Custom(409));
        }
        // update data and counters
        let counter = RefDB::get_storage_counter(data)?;
        if (counter as u32) < u32::MAX {
            RefDB::set_storage_counter(data, counter + 1)?;
        } else {
            RefDB::set_storage_counter(data, 1)?;
        }
        let active_records = RefDB::get_active_records(data)?;
        if active_records > 0 {
            RefDB::set_active_records(data, active_records - 1)?;
        }
        data[offset..data_end].fill(0);

        Ok(record.get_size())
    }

    /// Deletes the record from the storage using the name only.
    pub fn delete_with_name(data: &mut [u8], name: &ArrayString64) -> Result<usize, ProgramError> {
        RefDB::delete(
            data,
            &Record {
                index: None,
                counter: 0,
                tag: 0,
                name: *name,
                reference: Reference::Empty,
            },
        )
    }

    /// Returns index of the record with the given name or None if not found
    pub fn find_index(data: &[u8], name: &ArrayString64) -> Result<Option<usize>, ProgramError> {
        let rec_active = RefDB::get_active_records(data)?;
        if rec_active == 0 {
            return Ok(None);
        }
        let rec_num = RefDB::get_total_records(data)?;
        let rec_size = RefDB::get_record_size(data)?;
        let mut offset = Header::LEN;
        let mut checked = 0;
        for index in 0..rec_num {
            let (counter, rec_name) = RefDB::get_record_counter_and_name(data, offset)?;
            if counter > 0 {
                if rec_name == *name {
                    return Ok(Some(index));
                }
                checked += 1;
                if checked == rec_active {
                    return Ok(None);
                }
            }
            offset += rec_size;
        }
        Ok(None)
    }

    /// Returns the index of the first empty record at the back of the storage,
    /// i.e. there will be no active records after the index
    pub fn find_last_index(data: &[u8]) -> Result<u32, ProgramError> {
        let rec_active = RefDB::get_active_records(data)?;
        if rec_active == 0 {
            return Ok(0);
        }
        let rec_num = RefDB::get_total_records(data)?;
        let rec_size = RefDB::get_record_size(data)?;
        let mut offset = Header::LEN;
        let mut checked = 0;
        for index in 0..rec_num {
            let counter = RefDB::get_record_counter(data, offset)?;
            if counter > 0 {
                checked += 1;
                if checked == rec_active {
                    return Ok((index + 1) as u32);
                }
            }
            offset += rec_size;
        }
        Err(ProgramError::InvalidAccountData)
    }

    /// Returns the index of the next empty record to write to in the storage
    pub fn find_next_index(data: &[u8]) -> Result<u32, ProgramError> {
        let rec_active = RefDB::get_active_records(data)?;
        if rec_active == 0 {
            return Ok(0);
        }
        let rec_num = RefDB::get_total_records(data)?;
        let rec_size = RefDB::get_record_size(data)?;
        let mut offset = Header::LEN;
        let mut checked = 0;
        for index in 0..rec_num {
            let counter = RefDB::get_record_counter(data, offset)?;
            if counter == 0 {
                return Ok(index as u32);
            } else {
                checked += 1;
                if checked == rec_active {
                    return Ok((index + 1) as u32);
                }
            }
            offset += rec_size;
        }
        Err(ProgramError::InvalidAccountData)
    }

    // private helpers

    /// Returns offset of the record given its index
    fn get_offset(data: &[u8], index: usize) -> Result<usize, ProgramError> {
        let rec_size = RefDB::get_record_size(data)?;
        let offset = Header::LEN + index * rec_size;
        check_data_len(data, offset)?;
        Ok(offset)
    }

    fn find_write_offset(data: &[u8], name: &ArrayString64) -> Result<usize, ProgramError> {
        let rec_active = RefDB::get_active_records(data)?;
        if rec_active == 0 {
            return Ok(Header::LEN);
        }
        let rec_num = RefDB::get_total_records(data)?;
        let rec_size = RefDB::get_record_size(data)?;
        let mut offset = Header::LEN;
        let mut checked = 0;
        let mut free_slot = 0;
        for _ in 0..rec_num {
            let (counter, rec_name) = RefDB::get_record_counter_and_name(data, offset)?;
            if counter > 0 {
                if rec_name == *name {
                    return Ok(offset);
                }
                checked += 1;
                if checked == rec_active {
                    offset += rec_size;
                    break;
                }
            } else if free_slot == 0 {
                free_slot = offset;
            }
            offset += rec_size;
        }
        if free_slot > 0 {
            Ok(free_slot)
        } else {
            Ok(offset)
        }
    }

    fn get_record_counter(data: &[u8], offset: usize) -> Result<usize, ProgramError> {
        Record::unpack_counter(&data[offset..])
    }

    fn set_record_counter(data: &mut [u8], offset: usize, counter: usize) {
        if counter == 0 {
            return;
        }
        let counter_out = array_mut_ref![data, offset, 2];
        *counter_out = (counter as u16).to_le_bytes();
    }

    fn get_record_counter_and_name(
        data: &[u8],
        offset: usize,
    ) -> Result<(usize, ArrayString64), ProgramError> {
        Record::unpack_counter_and_name(&data[offset..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_test() {
        let mut data = vec![0; Header::LEN + Record::MAX_LEN * 6];
        assert!(!RefDB::is_initialized(data.as_slice()));

        assert!(RefDB::init(
            data.as_mut_slice(),
            &ArrayString64::from_utf8("test").unwrap(),
            ReferenceType::Pubkey
        )
        .is_ok());

        assert!(RefDB::is_initialized(data.as_slice()));
        assert_eq!(
            Header {
                counter: 1,
                active_records: 0,
                reference_type: ReferenceType::Pubkey,
                name: ArrayString64::from_utf8("test").unwrap()
            },
            RefDB::get_storage_header(data.as_slice()).unwrap()
        );
        assert_eq!(RefDB::get_storage_counter(data.as_slice()).unwrap(), 1);
        assert_eq!(RefDB::get_active_records(data.as_slice()).unwrap(), 0);
        assert_eq!(
            RefDB::get_reference_type(data.as_slice()).unwrap(),
            ReferenceType::Pubkey
        );
        assert!(RefDB::init(
            data.as_mut_slice(),
            &ArrayString64::from_utf8("test").unwrap(),
            ReferenceType::Pubkey
        )
        .is_err());
        let _ = RefDB::drop(data.as_mut_slice());
        assert!(!RefDB::is_initialized(data.as_slice()));

        assert!(RefDB::init(
            data.as_mut_slice(),
            &ArrayString64::from_utf8("test2").unwrap(),
            ReferenceType::U8
        )
        .is_ok());
        assert_eq!(RefDB::get_storage_counter(data.as_slice()).unwrap(), 1);
        assert_eq!(RefDB::get_active_records(data.as_slice()).unwrap(), 0);
        assert_eq!(
            RefDB::get_reference_type(data.as_slice()).unwrap(),
            ReferenceType::U8
        );

        assert_eq!(
            Header {
                counter: 1,
                active_records: 0,
                reference_type: ReferenceType::U8,
                name: ArrayString64::from_utf8("test2").unwrap()
            },
            RefDB::get_storage_header(data.as_slice()).unwrap()
        );
    }

    #[test]
    fn read_write_test() {
        // init
        let mut data = vec![0; Header::LEN + Record::MAX_LEN * 3];
        assert!(RefDB::init(
            data.as_mut_slice(),
            &ArrayString64::from_utf8("test").unwrap(),
            ReferenceType::Pubkey
        )
        .is_ok());

        // write
        let mut record = Record {
            index: Some(1),
            counter: 0,
            tag: 123,
            name: ArrayString64::from_utf8("test record").unwrap(),
            reference: Reference::Pubkey {
                data: Pubkey::new_unique(),
            },
        };
        assert_eq!(
            RefDB::get_record_size(data.as_slice()).unwrap(),
            Record::NO_REF_LEN + Reference::PUBKEY_LEN
        );
        assert_eq!(RefDB::get_total_records(data.as_slice()).unwrap(), 3);
        assert_eq!(RefDB::get_free_records(data.as_slice()).unwrap(), 3);
        assert_eq!(RefDB::get_active_records(data.as_slice()).unwrap(), 0);
        assert_eq!(RefDB::get_storage_counter(data.as_slice()).unwrap(), 1);
        assert!(RefDB::write(data.as_mut_slice(), &record).is_ok());
        assert_eq!(RefDB::get_total_records(data.as_slice()).unwrap(), 3);
        assert_eq!(RefDB::get_free_records(data.as_slice()).unwrap(), 2);
        assert_eq!(RefDB::get_active_records(data.as_slice()).unwrap(), 1);
        assert_eq!(RefDB::get_storage_counter(data.as_slice()).unwrap(), 2);

        let read = RefDB::read(
            data.as_slice(),
            &ArrayString64::from_utf8("test record").unwrap(),
        )
        .unwrap()
        .unwrap();

        record.index = Some(1);
        record.counter = 1;
        assert_eq!(read, record);

        // update
        record.tag = 321;
        record.reference = Reference::Pubkey {
            data: Pubkey::new_unique(),
        };
        RefDB::write(data.as_mut_slice(), &record).unwrap();
        assert_eq!(RefDB::get_total_records(data.as_slice()).unwrap(), 3);
        assert_eq!(RefDB::get_free_records(data.as_slice()).unwrap(), 2);
        assert_eq!(RefDB::get_active_records(data.as_slice()).unwrap(), 1);
        assert_eq!(RefDB::get_storage_counter(data.as_slice()).unwrap(), 3);

        let read = RefDB::read(
            data.as_slice(),
            &ArrayString64::from_utf8("test record").unwrap(),
        )
        .unwrap()
        .unwrap();

        record.counter = 2;
        assert_eq!(read, record);

        // fast update
        let new_ref = Reference::Pubkey {
            data: Pubkey::new_unique(),
        };
        assert!(
            RefDB::update(
                data.as_mut_slice(),
                &ArrayString64::from_utf8("test record").unwrap(),
                &new_ref
            )
            .unwrap()
                > 0
        );
        let read = RefDB::read(
            data.as_slice(),
            &ArrayString64::from_utf8("test record").unwrap(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(read.reference, new_ref);

        // update should fail if counter is stale
        record.counter = 1;
        assert!(RefDB::write(data.as_mut_slice(), &record).is_err());
        assert_eq!(RefDB::get_total_records(data.as_slice()).unwrap(), 3);
        assert_eq!(RefDB::get_free_records(data.as_slice()).unwrap(), 2);
        assert_eq!(RefDB::get_active_records(data.as_slice()).unwrap(), 1);
        assert_eq!(RefDB::get_storage_counter(data.as_slice()).unwrap(), 4);

        // write another record
        let mut record2 = Record {
            index: None,
            counter: 0,
            tag: 123,
            name: ArrayString64::from_utf8("test record2").unwrap(),
            reference: Reference::U8 { data: 0 },
        };
        // update should fail if reference type mismatch
        assert!(RefDB::write(data.as_mut_slice(), &record2).is_err());

        record2.reference = Reference::Pubkey {
            data: Pubkey::new_unique(),
        };
        RefDB::write(data.as_mut_slice(), &record2).unwrap();
        assert_eq!(RefDB::get_total_records(data.as_slice()).unwrap(), 3);
        assert_eq!(RefDB::get_free_records(data.as_slice()).unwrap(), 1);
        assert_eq!(RefDB::get_active_records(data.as_slice()).unwrap(), 2);
        assert_eq!(RefDB::get_storage_counter(data.as_slice()).unwrap(), 5);

        let read = RefDB::read(
            data.as_slice(),
            &ArrayString64::from_utf8("test record2").unwrap(),
        )
        .unwrap()
        .unwrap();

        record2.index = Some(0);
        record2.counter = 1;
        assert_eq!(read, record2);

        // check old record is still there
        let read = RefDB::read(
            data.as_slice(),
            &ArrayString64::from_utf8("test record").unwrap(),
        )
        .unwrap()
        .unwrap();

        record.counter = 3;
        record.reference = new_ref;
        assert_eq!(read, record);

        // update record with index
        record2.tag = 567;
        RefDB::write(data.as_mut_slice(), &record2).unwrap();
        let read = RefDB::read_at(data.as_slice(), record2.index.unwrap() as usize)
            .unwrap()
            .unwrap();
        record2.counter = 2;
        assert_eq!(read, record2);

        // write another
        let mut record3 = Record {
            index: None,
            counter: 0,
            tag: 3,
            name: ArrayString64::from_utf8("test record3").unwrap(),
            reference: Reference::Pubkey {
                data: Pubkey::new_unique(),
            },
        };
        RefDB::write(data.as_mut_slice(), &record3).unwrap();
        assert_eq!(RefDB::get_total_records(data.as_slice()).unwrap(), 3);
        assert_eq!(RefDB::get_free_records(data.as_slice()).unwrap(), 0);
        assert_eq!(RefDB::get_active_records(data.as_slice()).unwrap(), 3);
        assert_eq!(RefDB::get_storage_counter(data.as_slice()).unwrap(), 7);

        // full storage write
        record3.name = ArrayString64::from_utf8("test record4").unwrap();
        assert!(RefDB::write(data.as_mut_slice(), &record3).is_err());

        // delete record
        assert!(RefDB::delete_with_name(
            data.as_mut_slice(),
            &ArrayString64::from_utf8("test record4").unwrap()
        )
        .is_err());
        assert_eq!(RefDB::get_total_records(data.as_slice()).unwrap(), 3);
        assert_eq!(RefDB::get_free_records(data.as_slice()).unwrap(), 0);
        assert_eq!(RefDB::get_active_records(data.as_slice()).unwrap(), 3);
        assert_eq!(RefDB::get_storage_counter(data.as_slice()).unwrap(), 7);

        assert!(RefDB::delete_with_name(
            data.as_mut_slice(),
            &ArrayString64::from_utf8("test record2").unwrap()
        )
        .is_ok());
        assert_eq!(RefDB::get_total_records(data.as_slice()).unwrap(), 3);
        assert_eq!(RefDB::get_free_records(data.as_slice()).unwrap(), 1);
        assert_eq!(RefDB::get_active_records(data.as_slice()).unwrap(), 2);
        assert_eq!(RefDB::get_storage_counter(data.as_slice()).unwrap(), 8);

        assert!(
            RefDB::read_at(data.as_slice(), record2.index.unwrap() as usize)
                .unwrap()
                .is_none()
        );
        record2.index = None;
        assert!(RefDB::read(
            data.as_slice(),
            &ArrayString64::from_utf8("test record2").unwrap(),
        )
        .unwrap()
        .is_none());

        // write again
        record2.counter = 0;
        assert!(RefDB::write(data.as_mut_slice(), &record2).is_ok());
        assert_eq!(RefDB::get_total_records(data.as_slice()).unwrap(), 3);
        assert_eq!(RefDB::get_free_records(data.as_slice()).unwrap(), 0);
        assert_eq!(RefDB::get_active_records(data.as_slice()).unwrap(), 3);
        assert_eq!(RefDB::get_storage_counter(data.as_slice()).unwrap(), 9);

        let read = RefDB::read(
            data.as_slice(),
            &ArrayString64::from_utf8("test record2").unwrap(),
        )
        .unwrap()
        .unwrap();

        record2.index = Some(0);
        record2.counter = 1;
        assert_eq!(read, record2);
    }
}
