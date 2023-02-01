#[allow(dead_code)]
enum TreeLoad {
    Immutable,
    Mutable,
}

/// This macro applies functions on a ConcurrentMerkleT:ee and emits leaf information
/// needed to sync the merkle tree state with off-chain indexers.
macro_rules! _merkle_tree_depth_size_apply_fn {
    ($max_depth:literal, $max_size:literal, $id:ident, $bytes:ident, $func:ident, TreeLoad::Mutable, $($arg:tt)*)
     => {
        match ConcurrentMerkleTree::<$max_depth, $max_size>::load_mut_bytes($bytes) {
            Ok(merkle_tree) => {
                match merkle_tree.$func($($arg)*) {
                    Ok(_) => {
                        Ok(Box::<ChangeLogEvent>::from((merkle_tree.get_change_log(), $id, merkle_tree.sequence_number)))
                    }
                    Err(err) => {
                        msg!("Error using concurrent merkle tree: {}", err);
                        err!(AccountCompressionError::ConcurrentMerkleTreeError)
                    }
                }
            }
            Err(err) => {
                msg!("Error zero copying concurrent merkle tree: {}", err);
                err!(AccountCompressionError::ZeroCopyError)
            }
        }
    };
    ($max_depth:literal, $max_size:literal, $id:ident, $bytes:ident, $func:ident, TreeLoad::Immutable, $($arg:tt)*) => {
        match ConcurrentMerkleTree::<$max_depth, $max_size>::load_bytes($bytes) {
            Ok(merkle_tree) => {
                match merkle_tree.$func($($arg)*) {
                    Ok(_) => {
                        Ok(Box::<ChangeLogEvent>::from((merkle_tree.get_change_log(), $id, merkle_tree.sequence_number)))
                    }
                    Err(err) => {
                        msg!("Error using concurrent merkle tree: {}", err);
                        err!(AccountCompressionError::ConcurrentMerkleTreeError)
                    }
                }
            }
            Err(err) => {
                msg!("Error zero copying concurrent merkle tree: {}", err);
                err!(AccountCompressionError::ZeroCopyError)
            }
        }
    };
}

/// This applies a given function on a ConcurrentMerkleTree by
/// allowing the compiler to infer the size of the tree based
/// upon the header information stored on-chain
macro_rules! _merkle_tree_apply_fn {
    ($header:ident, $($arg:tt)*) => {
        // Note: max_buffer_size MUST be a power of 2
        match ($header.get_max_depth(), $header.get_max_buffer_size()) {
            (3, 8) => _merkle_tree_depth_size_apply_fn!(3, 8, $($arg)*),
            (5, 8) => _merkle_tree_depth_size_apply_fn!(5, 8, $($arg)*),
            (14, 64) => _merkle_tree_depth_size_apply_fn!(14, 64, $($arg)*),
            (14, 256) => _merkle_tree_depth_size_apply_fn!(14, 256, $($arg)*),
            (14, 1024) => _merkle_tree_depth_size_apply_fn!(14, 1024, $($arg)*),
            (14, 2048) => _merkle_tree_depth_size_apply_fn!(14, 2048, $($arg)*),
            (15, 64) => _merkle_tree_depth_size_apply_fn!(15, 64, $($arg)*),
            (16, 64) => _merkle_tree_depth_size_apply_fn!(16, 64, $($arg)*),
            (17, 64) => _merkle_tree_depth_size_apply_fn!(17, 64, $($arg)*),
            (18, 64) => _merkle_tree_depth_size_apply_fn!(18, 64, $($arg)*),
            (19, 64) => _merkle_tree_depth_size_apply_fn!(19, 64, $($arg)*),
            (20, 64) => _merkle_tree_depth_size_apply_fn!(20, 64, $($arg)*),
            (20, 256) => _merkle_tree_depth_size_apply_fn!(20, 256, $($arg)*),
            (20, 1024) => _merkle_tree_depth_size_apply_fn!(20, 1024, $($arg)*),
            (20, 2048) => _merkle_tree_depth_size_apply_fn!(20, 2048, $($arg)*),
            (24, 64) => _merkle_tree_depth_size_apply_fn!(24, 64, $($arg)*),
            (24, 256) => _merkle_tree_depth_size_apply_fn!(24, 256, $($arg)*),
            (24, 512) => _merkle_tree_depth_size_apply_fn!(24, 512, $($arg)*),
            (24, 1024) => _merkle_tree_depth_size_apply_fn!(24, 1024, $($arg)*),
            (24, 2048) => _merkle_tree_depth_size_apply_fn!(24, 2048, $($arg)*),
            (26, 512) => _merkle_tree_depth_size_apply_fn!(26, 512, $($arg)*),
            (26, 1024) => _merkle_tree_depth_size_apply_fn!(26, 1024, $($arg)*),
            (26, 2048) => _merkle_tree_depth_size_apply_fn!(26, 2048, $($arg)*),
            (30, 512) => _merkle_tree_depth_size_apply_fn!(30, 512, $($arg)*),
            (30, 1024) => _merkle_tree_depth_size_apply_fn!(30, 1024, $($arg)*),
            (30, 2048) => _merkle_tree_depth_size_apply_fn!(30, 2048, $($arg)*),
            _ => {
                msg!("Failed to apply {} on concurrent merkle tree with max depth {} and max buffer size {}",
                    stringify!($func),
                    $header.get_max_depth(),
                    $header.get_max_buffer_size()
                );
                err!(AccountCompressionError::ConcurrentMerkleTreeConstantsError)
            }
        }
    };
}

/// This applies a given function on a mutable ConcurrentMerkleTree
#[macro_export]
macro_rules! merkle_tree_apply_fn_mut {
    ($header:ident, $id:ident, $bytes:ident, $func:ident, $($arg:tt)*) => {
        _merkle_tree_apply_fn!($header, $id, $bytes, $func, TreeLoad::Mutable, $($arg)*)
    };
}

/// This applies a given function on a read-only ConcurrentMerkleTree
#[macro_export]
macro_rules! merkle_tree_apply_fn {
    ($header:ident, $id:ident, $bytes:ident, $func:ident, $($arg:tt)*) => {
        _merkle_tree_apply_fn!($header, $id, $bytes, $func, TreeLoad::Immutable, $($arg)*)
    };
}
