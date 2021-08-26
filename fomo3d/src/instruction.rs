use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub enum GameInstruction {
    /// Ix0 - Initializes a new version of Fomo3D.
    /// Accounts expected:
    /// 0 `[s]` Game creator's personal account.  
    /// 1 `[w]` Game state PDA. Uninitialized.
    /// 2 `[]` Token account for community share. Initialized.    
    /// 3 `[]` Token account for p3d share. Initialized.    
    /// 4 `[]` Token mint account. Initialized.    
    /// 5 `[]` SystemProgram account.    
    InitializeGame(InitGameParams),
    /// Ix1 - Initializes a new round of Fomo3D.
    /// Accounts expected:
    /// 0 `[s]` Funder account to pay any associated fees.
    /// 1 `[w]` Game state PDA. Initialized.
    /// 2 `[w]` Current round state PDA. Uninitialized.
    /// 3 `[w]` Token account for the current round's money pot. Uninitialized.
    /// 4 `[]` Token mint account. Initialized.
    /// 5 `[]` Rent account.
    /// 6 `[]` SystemProgram account.
    /// 7 `[]` TokenProgram account.
    /// The next two are passed if the round is not the 1st round for current program version:
    /// 8 `[w]` (optional) Previous round state PDA. Initialized.
    /// 9 `[w]` (optional) Token account for the previous round's money pot. Initialized.
    InitializeRound,
    /// Ix2 - Purchase a number of keys to participate in the game.
    /// Accounts expected:
    /// 0 `[s]` Player's personal account.
    /// 1 `[]` Game state PDA. Initialized.
    /// 2 `[w]` Round state PDA. Initialized.
    /// 3 `[w]` Player-round state PDA. Un/Initialized.
    /// 4 `[w]` Token account for the round's money pot. Initialized.
    /// 5 `[w]` Player's token account. Initialized.
    /// 6 `[]` SystemProgram account.
    /// 7 `[]` TokenProgram account.
    /// The next two are passed if the user wants to credit an existing/new affiliate.
    /// 8 `[w]` Affiliate-round state PDA. Un/Initialized.
    /// 9 `[]` Affiliate owner's account.
    PurchaseKeys(PurchaseKeysParams),
    /// Ix3 - Withdraw any accumulated Tokens in player's name.
    /// 0 `[s]` Player's personal account.
    /// 1 `[]` Game state PDA. Initialized.
    /// 2 `[]` Round state PDA. Initialized.
    /// 3 `[w]` Player-round state PDA. Initialized.
    /// 4 `[w]` Token account for the round's money pot. Initialized.
    /// 5 `[w]` Player's token account. Initialized.
    /// 6 `[]` SystemProgram account.
    /// 7 `[]` TokenProgram account.
    WithdrawSol(WithdrawParams),
    /// Ix4 - End the current game's round. Can be run by anyone, not just the creator.
    /// 0 `[]` Game state PDA. Initialized.
    /// 1 `[w]` Round state PDA. Initialized.
    /// 2 `[w]` Winner-round state PDA. Initialized.
    EndRound,
    /// Ix5 - Withdraw community rewards. Can be run by whoever controls the community token wallet.
    /// 0 `[]` Game state PDA. Initialized.
    /// 1 `[w]` Round state PDA. Initialized.
    /// 2 `[w]` Token account for the round's money pot. Initialized.
    /// 3 `[w]` Community's token account. Initialized.
    /// 4 `[s]` Community wallet owner.
    /// 5 `[]` TokenProgram account.
    WithdrawCommunityRewards(WithdrawParams),
    /// Ix6 - Withdraw p3d rewards.
    /// (!) Unimplemented - see explanation in processor.
    WithdrawP3DRewards(WithdrawParams),
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct InitGameParams {
    pub version: u64,
    //time (in seconds) for the initial window when a new round starts
    //in original Fomo3D: 1h
    pub round_init_time: i64,
    //time (in seconds) by which each key purchase bumps round end time
    //in original Fomo3D: 30s
    pub round_inc_time_per_key: i64,
    //time (in seconds) for max possible window
    //in original Fomo3D: 24h
    pub round_max_time: i64,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct PurchaseKeysParams {
    pub sol_to_be_added: u128,
    pub team: u8,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct WithdrawParams {
    //user should be able to specify which round they want to withdraw for
    pub withdraw_for_round: u64,
}
