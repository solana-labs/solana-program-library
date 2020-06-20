#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct TokenSwap {
    authority: Pubkey,
}
pub enum State {
    /// Unallocated state, may be initialized into another state.
    Unallocated,
    Init(TokenSwap),
    Invalid,
}



impl TokenSwap {
    pub fn a_to_b(&mut self, tokenA: u64) -> Option<u64> {
        let newA = self.tokenA.checked_add(tokenA)?;
        let newB = self.invariant.checked_div(newA)?;
        let remove = self.tokenB.checked_sub(newB)?;
        self,tokenA = newA;
        self,tokenB = newB;
        Some(remove)
    }
    pub fn b_to_a(&mut self, tokenB: u64) -> Option<u64> {
        let newB = self.tokenB.checked_add(tokenB)?;
        let newA = self.invariant.checked_div(newB)?;
        let remove = self.tokenA.checked_sub(newA)?;
        self,tokenA = newA;
        self,tokenB = newB;
        Some(remove)

    }
}
impl State {
    pub fn create_token(
        kind: &str,
        token_account: &AccountInfo,
    ) -> ProgramResult {
        // create token A account
        let instruction_data = vec![];
        let instruction = token::Instruction::NewAccount;
        instruction.serialize(&mut instruction_data)?;

        let account_addr = Pubkey::create_program_address(&[kind, "account"], program_id)?;
        let authority = Pubkey::create_program_address(&[kind, "authority"], program_id)?;
        let invoked_instruction = create_instruction(
            token_account.owner,
            &[
                (token_account.key, false, false),
                (account_addr, true, true),
                (authority, false, true),
            ],
            instruction_data,
        );
        invoke_signed(
            &invoked_instruction,
            accounts,
            &[&[kind, "account"], &[kind, "authority"]],
        )
    }


    pub fn process_init<I: Iterator<Item = &'a AccountInfo<'a>>>(
        account_info_iter: &mut I,
    ) -> ProgramResult {
        let token_swap_info = next_account_info(account_info_iter)?;
        if State::Unallocated != State::deserialize(&token_swap_info.data.borrow())? {
            return Err(ProgramError::Unallocated);
        }
        let authority = next_account_info(account_info_iter)?;
        if !authority.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let tokenA = next_account_info(account_info_iter)?;

        // create token A account
        create_token("tokenA", tokenA)?;

        let tokenB = next_account_info(account_info_iter)?;
        create_token("tokenB", tokenA)?;

        let obj = State;:Init(TokenSwap { authority });
        obj.serialize(&mut token_swap_info.data.borrow_mut())
    }

    pub fn process_deposit<I: Iterator<Item = &'a AccountInfo<'a>>>(
        kind: &str,
        program_id: &Pubkey,
        account_info_iter: &mut I,
    ) -> ProgramResult {
        let token_swap_account = next_account_info(account_info_iter)?;
        let state = State::deserialize(&token_swap_account.data.borrow())?;
        let swap = state.swap()?;

        let authority = next_account_info(account_info_iter)?;
        if !authority.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !authority.pubkey != swap.authority {
            return Err(ProgramError::InvalidAuthority);
        }
        let token_auth = Pubkey::create_program_address(&[kind, "authority"], program_id)?;
        let token_account = Pubkey::create_program_address(&[kind, "account"], program_id)?;
    }


    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'a>],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = Instruction::deserialize(input)?;
        let account_info_iter = &mut accounts.iter();
        match instruction {
            Instruction::Init => {
                info!("Instruction: Init");
                Self::process_init(account_info_iter)
            },
            Instruction::DepositA => {
                info!("Instruction: Deposit");
                Self::process_deposit("tokenA", program_id, account_info_iter)
            }
        }
    }

}

/// Instructions supported by the token program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    /// Creates a new TokenSwap
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[writable, signer]` New token-swap to create.
    ///   1. `[signer]` Authority
    Init,
    /// Deposit tokens and update the invariant and fee.
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Token-swap
    ///   1. `[signer]` Authority
    ///   2.  Token assigned to "tokenA/authority" program address
    ///   3.  Token assigned to "tokenB/authority" program address
    Deposit,
    ///   Reassigns the authority on tokenA and tokenB to Authority.
    ///   Transfers lamports to authority.
    ///   
    ///   0. `[writable]` Token-swap
    ///   1. `[signer]` Authority
    Close,
    ///   0. `[writable]` Token-swap
    ///   1.  Token assigned to "token(A|B)/authority" program address
    ///   2.  Token to deposit into
    Swap,
}
