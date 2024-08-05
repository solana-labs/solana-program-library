use solana_program::keccak::hashv;

/// Abstract type for 32 byte leaf data
pub type Node = [u8; 32];

/// An empty node is a 32 byte array of zeroes
pub const EMPTY: Node = [0_u8; 32];

/// Calculates the hash of empty nodes up to level i
pub fn empty_node(level: u32) -> Node {
    empty_node_cached::<0>(level, &[])
}

/// Calculates the hash of empty nodes up to level i using an existing cache
pub fn empty_node_cached<const N: usize>(level: u32, cache: &[Node; N]) -> Node {
    let mut data = EMPTY;
    if level != 0 {
        let target = (level - 1) as usize;
        let lower_empty = if target < cache.len() && cache[target] != EMPTY {
            cache[target]
        } else {
            empty_node(target as u32)
        };
        let hash = hashv(&[lower_empty.as_ref(), lower_empty.as_ref()]);
        data.copy_from_slice(hash.as_ref());
    }
    data
}

/// Calculates and caches the hash of empty nodes up to level i
pub fn empty_node_cached_mut<const N: usize>(level: u32, cache: &mut [Node; N]) -> Node {
    let mut data = EMPTY;
    if level != 0 {
        let target = (level - 1) as usize;
        let lower_empty = if target < cache.len() && cache[target] != EMPTY {
            cache[target]
        } else {
            empty_node(target as u32)
        };
        let hash = hashv(&[lower_empty.as_ref(), lower_empty.as_ref()]);
        data.copy_from_slice(hash.as_ref());
    }
    cache[level as usize] = data;
    data
}
