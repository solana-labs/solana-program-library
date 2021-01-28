pub const UNINITIALIZED_VERSION: u8 = 0;

/// Program states.
#[repr(C)]
#[derive(Debug, Default, PartialEq)]
pub struct Pool {
    /// Initialized state.
    pub version: u8,

    /// Nonce used in program address.
    pub nonce: u8,

    /// Program ID of the tokens
    pub token_program_id: Pubkey,

    /// Account to deposit into
    pub deposit_account: Pubkey,

    /// Mint information for token Pass
    pub token_pass_mint: Pubkey,

    /// Mint information for token Fail
    pub token_fail_mint: Pubkey,

    /// decider key
    pub decider: Pubkey,

    /// decision boolean
    pub decision: Option<bool>,
}

impl Sealed for LendingMarket {}
impl IsInitialized for Pool {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}
