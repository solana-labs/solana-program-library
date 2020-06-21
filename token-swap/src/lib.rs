/// Instructions supported by the token program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    /// Creates a new TokenSwap
    ///
    /// # Accounts expected by this instruction:
    ///
    ///   0. `[writable, signer]` New Token-swap to create.
    ///   1. `[signer]` Authority
    ///   2. `[readable]` $instance_id/authority
    ///   3. `[readable]` $instance_id/tokenA
    ///   4. `[readable]` $instance_id/tokenB
    ///   5. `[readable]` tokenA account
    ///   6. `[readable]` tokenB account
    Init,
    ///   0. `[writable]` Token-swap
    ///   1.  Token assigned to "token(A|B)/authority" program address
    ///   2.  The token to deposit into
    ///
    ///   Amount swapped is always based on existing curve: A*B = K
    Swap,
    ///   Reassigns the authority on tokenA and tokenB to Authority.
    ///   
    ///   0. `[writable]` Token-swap
    ///   1. `[signer]` Authority
    ///   2. `[writable]` Token source (must be owned by Swap)
    ///   5. `[writable]` Token destination
    ///   userdata: The amount of each token to withdraw
    Withdraw(u64),
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum State {
    /// Unallocated state, may be initialized into another state.
    Unallocated,
    Init(TokenSwap),
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct TokenSwap {
    authority: Pubkey,
}

struct Invariant {
    tokenA: u64,
    tokenB: u64,
}

impl Invariant {
    pub fn swap(&self, tokenA: u64) -> Option<u64> {
        let invariant = self.tokenA.checked_mul(self.tokenB)?;
        let newA = self.tokenA.checked_add(tokenA)?;
        let newB = invariant.checked_div(newA)?;
        let remove = self.tokenB.checked_sub(newB)?;
        self,tokenA = newA;
        self,tokenB = newB;
        Some(remove)
    }
}

impl State {
    pub fn deserialize(input: &'a [u8]) -> Result<Self, ProgramError> {
        if input.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(match input[0] {
            0 => Self::Unallocated,
            1 => {
                if input.len() < size_of::<u8>() + size_of::<TokenSwap>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                #[allow(clippy::cast_ptr_alignment)]
                let swap: &TokenSwap = unsafe { &*(&input[1] as *const u8 as *const TokenSwap) };
                Self::Init(*swap)
            }
            _ => return Err(ProgramError::InvalidAccountData),
        })
    }

    pub fn serialize(self: &Self, output: &mut [u8]) -> ProgramResult {
        if output.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        match self {
            Self::Unallocated => output[0] = 0,
            Self::Init(swap) => {
                if output.len() < size_of::<u8>() + size_of::<TokenSwap>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                output[0] = 1;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut TokenSwap) };
                *value = *token;
            }
        }
        Ok(())
    }

    pub fn create_token_account(
        kind: &str,
        instance_id: &Pubkey,
        program_id: &Pubkey,
        token_account: &AccountInfo,
    ) -> ProgramResult {
        // create token A account
        let instruction_data = vec![];
        let instruction = token::Instruction::NewAccount;
        instruction.serialize(&mut instruction_data)?;

        let account_addr = Pubkey::create_program_address(&[instance_id, kind], program_id)?;
        let authority = Pubkey::create_program_address(&[instance_id, "authority"], program_id)?;
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
        let instance_id = token_swap_info.key;
        let authority = next_account_info(account_info_iter)?;
        if !authority.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let tokenA = next_account_info(account_info_iter)?;

        // create token A account
        create_token_account("tokenA", tokenA)?;

        // create token B account
        let tokenB = next_account_info(account_info_iter)?;
        create_token_account("tokenB", tokenB)?;

        let obj = State;:Init(TokenSwap { authority });
        obj.serialize(&mut token_swap_info.data.borrow_mut())
    }

    pub fn transfer_token(
        instance_id: &Pubkey,
        source: &Pubkey,
        destination: &Pubkey,
        amount: u64,
    ) -> ProgramResult {
        let signers = &[&[instance_id, "authority"]],
        let authority = Pubkey::create_program_address(&[instance_id, "authority"], program_id)?;
        let source = Pubkey::create_program_address(&[instance_id, kind], program_id)?;
        let instruction_data = vec![];
        let instruction = token::Instruction::Transfer(amount);
        instruction.serialize(&mut instruction_data)?;
        let invoked_instruction = create_instruction(
            token_account.owner,
            &[
                (authority, false, true),
                (source, true, false),
                (destination, true, false),
            ],
            instruction_data,
        );
        invoke_signed(
            &invoked_instruction,
            accounts,
            signers,
        )
    }
    pub fn process_swap<I: Iterator<Item = &'a AccountInfo<'a>>>(
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
        let instance_id = token_swap_account.key;
        let token_authority = Pubkey::create_program_address(&[instance_id, "authority"], program_id)?;

        let tokenA_account = Pubkey::create_program_address(&[instance_id, "tokenA"], program_id)?;

        //tokenA
        let tokenA_info = next_account_info(account_info_iter)?;
        if tokenA_info.key != tokenA_account {
            return Err(Error::InvalidTokenAAccount);
        }
        let tokenA_account = token::Account::deserialize(tokenA_info.data)?;

        let tokenB_account = Pubkey::create_program_address(&[instance_id, "tokenB"], program_id)?;

        //tokenB
        let tokenB_info = next_account_info(account_info_iter)?;
        let tokenB_account = token::Account::deserialize(tokenB_info.data)?;
        if tokenB_info.key != tokenB_account {
            return Err(Error::InvalidTokenBAccount);
        }

        //input token
        let input_token_info = next_account_info(account_info_iter)?;
        let input_account = token::Account::deserialize(input.data)?;

        //incoming token should be delegated to the TokenSwap intance authority
        if input_account.authority != token_authority {
            return Err(Error::InvalidTokenAuthority);
        }

        let output_token_info = next_account_info(account_info_iter)?;
        if input_account.token == tokenA_account.token {
            let invariant = Invariant { tokenA: tokenA.amount, tokenB: tokenB.amount};
            let exchange = invariant.swap(input_account.amount)?;
            Self::transfer_token(instance_id, input_account, tokenA_account, input_account.amount)?;
            Self::transfer_token(instance_id, tokenB_account, output_token_info.key, exchange)?;
        } else {
            let invariant = Invariant { tokenA: tokenB.amount, tokenB: tokenA.amount};
            let exchange = invariant.swap(input_account.amount)?;
            Self::transfer_token(instance_id, input_account, tokenB_account, input_account.amount)?;
            Self::transfer_token(instance_id, tokenA_account, output_token_info.key, exchange)?;
        }
        Ok(())
    } 

    pub fn process_withdraw<I: Iterator<Item = &'a AccountInfo<'a>>>(
        program_id: &Pubkey,
        amount: u64,
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
        let instance_id = token_swap_account.key;
        let token_authority = Pubkey::create_program_address(&[instance_id, "authority"], program_id)?;

        let token_info = next_account_info(account_info_iter)?;
        let tokenA_account = Pubkey::create_program_address(&[instance_id, "tokenA"], program_id)?;
        let tokenB_account = Pubkey::create_program_address(&[instance_id, "tokenB"], program_id)?;
        let destination = next_account_info(account_info_iter)?;

        if token_info.key == tokenA_account {
            Self::transfer_token(instance_id, tokenA_account, destination, amount)?;
        } else if token_info.key == tokenB_account {
            Self::transfer_token(instance_id, tokenB_account, destination, amount)?;
        } else {
            return Err(Error::InvalidTokenAAccount);
        }
        Ok(())
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
            Instruction::Swap => {
                info!("Instruction: Swap");
                Self::process_swap(program_id, account_info_iter)
            },
            Instruction::Withdraw => {
                info!("Instruction: Withdraw");
                Self::process_withdraw(program_id, account_info_iter)
            }
        }
    }
}


