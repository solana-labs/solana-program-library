<p align="center">
  <a href="https://solana.com">
    <img alt="Solana" src="https://i.imgur.com/IKyzQ6T.png" width="250" />
  </a>
</p>

# SPL Account Compression Rust SDK

## Overview

The SPL Account Compression Rust SDK provides tools and interfaces for implementing account compression on the Solana blockchain. This technology enables efficient storage and verification of large-scale data structures, particularly beneficial for NFT collections and other data-intensive applications.

## Key Features

- **Concurrent Merkle Tree Implementation**: Efficient on-chain data structure for verifying off-chain data
- **Compressed NFT Support**: Primary integration with Metaplex Bubblegum for compressed NFTs
- **Optimized Storage**: Significantly reduced on-chain storage costs
- **Scalable Architecture**: Designed for handling large collections efficiently

## Primary Use Cases

1. **Compressed NFTs**
   - Mint NFTs at a fraction of the usual cost
   - Manage large NFT collections efficiently
   - Enable scalable NFT operations

2. **Data Verification**
   - On-chain verification of off-chain data
   - Secure state management
   - Efficient proof validation

## Integration

### Prerequisites
- Rust 1.68 or higher
- Solana CLI tools
- Anchor framework (optional but recommended)

### Installation

Add this to your `Cargo.toml`:
```toml
[dependencies]
spl-account-compression = { version = "0.4.2", features = ["cpi"] }
```

## Documentation

For detailed documentation and implementation examples, visit:
- [Account Compression Program Documentation](https://github.com/solana-labs/solana-program-library/tree/master/account-compression)
- [Metaplex Compressed NFTs (Bubblegum)](https://github.com/metaplex-foundation/mpl-bubblegum)

## Architecture

The SDK consists of several key components:
1. **Concurrent Merkle Tree**: Core data structure for efficient storage
2. **State Management**: Handles on-chain state transitions
3. **Proof Verification**: Validates off-chain data modifications
4. **Integration Interfaces**: APIs for common use cases

## Important Notes

- This implementation is primarily targeted at supporting Metaplex Compressed NFTs
- The SDK requires an indexer service for tracking off-chain data
- Features and APIs may evolve as the technology matures

## Example Usage

```rust
// 1. Basic imports for tree creation
use spl_account_compression::{
    state::ConcurrentMerkleTree,  // For working with merkle tree structure
    instruction::create_tree,      // For creating new merkle trees
};

// Example usage:
fn create_new_tree() {
    // Initialize a new merkle tree
    let tree = ConcurrentMerkleTree::new();
    // Create tree instruction
    let ix = create_tree(/* params */);
}

// 2. Simple Node import
use spl_account_compression::Node;  // For working with individual tree nodes

// Example usage:
fn work_with_nodes() {
    let node = Node::default();
    // Work with individual node operations
}

// 3. Application data wrapper import
use spl_account_compression::wrap_application_data_v1;  // For wrapping application data

// Example usage:
fn wrap_data() {
    let data = vec![1, 2, 3];
    let wrapped = wrap_application_data_v1(&data);
    // Use wrapped data in tree operations
}

// 4. Full program imports with Noop
use spl_account_compression::{
    program::SplAccountCompression,  // Main program interface
    wrap_application_data_v1,        // Data wrapper function
    Noop,                           // No-operation program for logging
};

// Example usage:
fn full_program_usage() {
    // Access program ID
    let program_id = SplAccountCompression::id();
    
    // Use Noop for logging
    let noop = Noop::id();
    
    // Wrap data
    let data = vec![1, 2, 3];
    let wrapped = wrap_application_data_v1(&data);
    
    // Create program instruction
    // Note: This is a simplified example
    let ix = SplAccountCompression::new_instruction(
        program_id,
        &wrapped,
        &noop,
    );
}

// Common patterns and their uses:

// Pattern 1: Basic Tree Operations
// When you only need to work with merkle trees
use spl_account_compression::{state::ConcurrentMerkleTree, instruction::create_tree};

// Pattern 2: Node Operations
// When working with individual nodes in the tree
use spl_account_compression::Node;

// Pattern 3: Data Wrapping
// When you need to prepare data for the tree
use spl_account_compression::wrap_application_data_v1;

// Pattern 4: Full Program Integration
// When building complete applications
use spl_account_compression::{
    program::SplAccountCompression,
    wrap_application_data_v1,
    Noop,
};

// (Please refer to docs for complete usage)
```

## Related Projects

- [Metaplex Bubblegum](https://github.com/metaplex-foundation/mpl-bubblegum): Compressed NFT standard
- [Solana Program Library](https://github.com/solana-labs/solana-program-library): Core SPL programs
- [Noop Program](https://github.com/solana-labs/solana-program-library/tree/master/noop): Supporting program for log handling

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## Security

If you discover any security issues, please report them via [Solana's Security Policy](https://github.com/solana-labs/solana-program-library#security).

## License

The SPL Account Compression SDK is licensed under the Apache License, Version 2.0.