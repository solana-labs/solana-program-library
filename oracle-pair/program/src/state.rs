/// Program states.
#[repr(C)]
#[derive(Debug, Default, PartialEq)]
pub struct OraclePair {
    /// Initialized state.
    pub is_initialized: bool,

    /// Nonce used in program address.
    pub nonce: u8,

    /// Program ID of the tokens
    pub token_program_id: Pubkey,

    /// Token Pass
    pub token_pass: Pubkey,
    /// Token Fail
    pub token_fail: Pubkey,

    /// Account to deposit into
    pub deposit_account: Pubkey,

    /// Mint information for token Pass
    pub token_pass_mint: Pubkey,
    /// Mint information for token Fail
    pub token_fail_mint: Pubkey,

    /// Deposit account to receive minting fees
    pub deposit_fee_account: Pubkey,

    /// All fee information
    pub fees: Fees,
}
