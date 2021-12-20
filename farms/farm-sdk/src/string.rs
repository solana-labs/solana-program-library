//! Fixed length string types

//use arrayvec::ArrayString;
use {
    arraystring::{typenum::U64, ArrayString},
    serde::Serialize,
    solana_program::{instruction::Instruction, program_error::ProgramError, pubkey::Pubkey},
    std::collections::HashMap,
};

/// Fixed size array to store UTF-8 strings on blockchain.
pub type ArrayString64 = ArrayString<U64>;

pub fn to_pretty_json<T>(object: &T) -> Result<String, serde_json::Error>
where
    T: ?Sized + Serialize,
{
    serde_json::to_string_pretty(&object)
}

// Custom serializer that prints base58 addresses instead of arrays
pub fn instruction_to_string(inst: &Instruction) -> String {
    let len = 145 + inst.data.len() * 4 + inst.accounts.len() * 40;
    let mut s = String::with_capacity(len);
    s += format!("{{\"program_id\":\"{}\",\"accounts\":[", inst.program_id).as_str();
    let mut first_object = true;
    for val in &inst.accounts {
        if !first_object {
            s += ",";
        } else {
            first_object = false;
        }
        s += format!(
            "{{\"pubkey\":\"{}\",\"is_signer\":{},\"is_writable\":{}}}",
            val.pubkey, val.is_signer, val.is_writable
        )
        .as_str();
    }
    s += format!("],\"data\":{:?}}}", inst.data).as_str();
    s
}

// Custom serializer that prints base58 addresses instead of arrays
pub fn pubkey_map_to_string(map: &HashMap<String, Pubkey>) -> String {
    if map.is_empty() {
        return "{}".to_string();
    }
    let mut len = 1;
    for key in map.keys() {
        len += key.len() + 50;
    }
    let mut s = String::with_capacity(len);
    s += "{";
    for (key, val) in map {
        if s.len() != 1 {
            s += ",";
        }
        s += format!("\"{}\":\"{}\"", key, val.to_string()).as_str();
    }
    s += "}";
    s
}

pub fn str_to_as64(input: &str) -> Result<ArrayString64, ProgramError> {
    ArrayString64::try_from_str(input).or(Err(ProgramError::InvalidArgument))
}
