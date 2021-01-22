//! Instruction types
use crate::state::Fees;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::IsInitialized;
use solana_program::program_pack::{Pack, Sealed};

/// Instructions supported by the MarginPoolInfo program.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum MarginPoolInstruction {
    ///   Initializes a new MarginPool.
    ///
    ///   0. `[writable, signer]` New MarginPool to create.
    ///   1. `[]` $authority derived from `create_program_address(&[MarginPool account])`
    ///   4. `[]` Swap Market for token_A and token_B
    ///   2. `[]` token_LP Account. Must be empty, owned by $authority.
    ///   2. `[]` token_A Account. Must be empty, owned by $authority.
    ///   3. `[]` token_B Account. Must be empty, owned by $authority.
    ///   5. `[writable]` Pool Token Mint. Must be empty, owned by $authority.
    ///   8. '[]` Token program id
    ///   9. '[]` Swap program id
    Initialize {
        /// nonce used to create valid program address
        nonce: u8,
        /// Fees for the pool
        fees: Fees,
    },

    ///   Fundn a position.
    ///
    ///   0. `[]` MarginPool
    ///   1. `[]` $authority
    ///   4. `[]` Swap Market
    ///   4. `[writable]` MarginPool::Position state, uninitialized on first use.
    ///   4. `[writable]` Position mint
    ///   2. `[writable]` token_X SOURCE Account, amount is transferable by $authority.
    ///   3. `[writable]` token_LP LP account to borrow from.
    ///   4. `[writable]` token_Y Base Account to deposit into, owned by $authority.
    ///   4. `[writable]` Position token to deposit into, owned by user.
    ///   8. '[]` Token program id
    ///   9. '[]` Token swap program id
    FundPosition {
        /// SOURCE amount
        amount_in: u64,
        /// Minimum amount DESTINATION token to output, prevents excessive slippage
        minimum_amount_out: u64,
    },

    ///   Reduce a position.
    ///
    ///   0. `[]` MarginPool
    ///   1. `[]` $authority
    ///   4. `[]` Swap Market for token_A and token_A
    ///   4. `[writable]` Initialized MarginPool::Position state.
    ///   3. `[writable]` token_LP LP account.
    ///   3. `[writable]` token_Y Base Account.
    ///   4. `[writable]` token_X DESTINATION Account.
    ///   8. '[]` Token program id
    ReducePosition {
        /// SOURCE amount to transfer, output to DESTINATION is based on the exchange rate
        amount_in: u64,
        /// Minimum amount of DESTINATION token to output, prevents excessive slippage
        minimum_amount_out: u64,
    },

    /// Force position liquidation
    Liquidate,

    ///   Deposit some tokens into the pool.  The output is a "pool" token representing ownership
    ///   into the pool.
    ///
    ///   0. `[]` Margin pool
    ///   1. `[]` $authority
    ///   2. `[writable]` LP token account $authority can transfer `deposit_amount`
    ///   3. `[writable]` Pool LP token account to deposit into
    ///   4. `[writable]` Pool Mint account, $authority is the owner
    ///   5. `[writable]` Pool Account to deposit the generated tokens, user is the owner
    ///   6. '[]` Token program id
    Deposit {
        /// Amount of LP tokens to deposit to the pool
        deposit_amount: u64,
    },

    ///   Burn pool tokens and withdraw LP tokens from the pool
    ///
    ///   0. `[]` Margin pool
    ///   1. `[]` $authority
    ///   2. `[writable]` Pool mint account, $authority is the owner
    ///   3. `[writable]` Pool Account to burn from, `burn_amount` is transferable by $authority
    ///   4. `[writable]` Pool LP token account to withdraw from
    ///   5. `[writable]` User LP token account to credit
    ///   6. `[writable]` Pool fee account, to receive withdrawal fees
    ///   7. '[]` Token program id
    Withdraw {
        /// Amount of pool tokens to burn. User received LP tokens according to the current pool ratio
        burn_amount: u64,
    },
}

impl Pack for MarginPoolInstruction {
    const LEN: usize = 291;
    fn unpack_from_slice(_input: &[u8]) -> Result<Self, ProgramError> {
        unimplemented!();
    }
    fn pack_into_slice(&self, _output: &mut [u8]) {
        unimplemented!();
    }
}

impl Sealed for MarginPoolInstruction {}
impl IsInitialized for MarginPoolInstruction {
    fn is_initialized(&self) -> bool {
        true
    }
}
