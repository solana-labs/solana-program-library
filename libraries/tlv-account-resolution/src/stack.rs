//! Stack for resolving accounts for an instruction

use {
    crate::{
        account::{PodAccountMeta, RequiredAccount},
        error::AccountResolutionError,
        seeds::Seed,
    },
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_discriminator::SplDiscriminate,
    spl_type_length_value::{
        pod::PodSlice,
        state::{TlvState, TlvStateBorrowed},
    },
    std::collections::BTreeMap,
};

#[derive(Clone, Debug, PartialEq)]
struct Node {
    /// The index of the account in the total accounts list
    index: usize,
    /// The indices of the accounts whose keys this account's
    /// PDA depends on (if any)
    dependencies: Vec<usize>,
    /// The required account itself as a `RequiredAccount`
    required_account: RequiredAccount,
}
impl Node {
    fn new(index: usize, dependencies: Vec<usize>, required_account: RequiredAccount) -> Self {
        Self {
            index,
            dependencies,
            required_account,
        }
    }

    fn has_dependency(&self, index: usize) -> bool {
        self.dependencies.contains(&index)
    }
}

/// Because the seed configurations for an `AccountMetaPda` can reference
/// the public key of another account in either the instruction's accounts
/// or the required extra accounts, we have to collect all accounts
/// first as a `RequiredAccount` and then resolve them to `AccountMeta`s
/// afterwards, when we know which ones to grab first.
///
/// This is due to the fact that, for example, a PDA at index 3 could
/// require the pubkey of another PDA at index 5 (after it).
///
/// To solve this, we use a stack!
#[derive(Debug)]
pub struct AccountResolutionStack(Option<(Node, Box<AccountResolutionStack>)>);
impl AccountResolutionStack {
    fn new() -> Self {
        Self(None)
    }

    fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    fn has_next(&self) -> bool {
        if let Some((_, next)) = &self.0 {
            next.0.is_some()
        } else {
            false
        }
    }

    fn next(&self) -> &Self {
        self.0.as_ref().unwrap().1.as_ref()
    }

    fn next_mut(&mut self) -> &mut Self {
        self.0.as_mut().unwrap().1.as_mut()
    }

    fn has_dependency(&self, index: usize) -> bool {
        if let Some((node, _)) = &self.0 {
            node.has_dependency(index)
        } else {
            false
        }
    }

    fn search(&self, indices: &[usize]) -> bool {
        if indices.is_empty() {
            return true;
        }
        let mut node = self;
        while node.has_next() {
            for index in indices.iter() {
                if node.0.as_ref().unwrap().0.index == *index {
                    return true;
                }
            }
            node = node.next();
        }
        false
    }

    fn pop(&mut self) -> Node {
        let (node, next) = self.0.take().unwrap();
        *self = *next;
        node
    }

    fn push_before(&mut self, node: Node) -> Result<(), ProgramError> {
        if !self.search(&[node.index]) {
            let next = self.0.take();
            self.0 = Some((node, Box::new(AccountResolutionStack(next))));
            return Ok(());
        }
        Err(AccountResolutionError::CircularReference.into())
    }

    fn push_next(&mut self, node: Node) -> Result<(), ProgramError> {
        if self.has_next() {
            self.next_mut().push_before(node)?;
        } else {
            let next =
                AccountResolutionStack(Some((node, Box::new(AccountResolutionStack::new()))));
            self.0.as_mut().unwrap().1 = Box::new(next);
        }
        Ok(())
    }

    fn update_recursive(
        stack: &mut AccountResolutionStack,
        node: Node,
    ) -> Result<(), ProgramError> {
        // If the new node has no dependencies, add it to the front.
        // This account can be resolved first.
        if stack.is_empty() || node.dependencies.is_empty() {
            stack.push_before(node)
        }
        // If a node is found that dependends on the new node,
        // stop and check the rest of the stack for any dependencies.
        else if stack.has_dependency(node.index) {
            // If any are found, throw a circular reference error.
            // The configuration can't be resolved.
            if stack.search(&node.dependencies) {
                return Err(AccountResolutionError::CircularReference.into());
            }
            // If not, add the new node before the dependent node.
            // This account must be resolved before the dependent node.
            stack.push_before(node)
        }
        // If the end of the stack is reached, add the new node to the end.
        // This account can be resolved last, since no other accounts depend
        // on it.
        else if !stack.has_next() {
            stack.push_next(node)
        } else {
            Self::update_recursive(stack.next_mut(), node)
        }
    }

    /// Insert a required account and preserve proper ordering of the stack
    fn insert(
        &mut self,
        index: usize,
        required_account: RequiredAccount,
    ) -> Result<(), ProgramError> {
        let dependencies = match &required_account {
            RequiredAccount::Account { .. } => vec![],
            RequiredAccount::Pda { seeds, .. } => Seed::get_account_key_indices(seeds),
        };
        let node = Node::new(index, dependencies, required_account);
        Self::update_recursive(self, node)
    }

    /// De-escalate an account meta if necessary
    fn de_escalate_account_meta(account_meta: &mut AccountMeta, account_metas: &[AccountMeta]) {
        // This is a little tricky to read, but the idea is to see if
        // this account is marked as writable or signer anywhere in
        // the instruction at the start. If so, DON'T escalate it to
        // be a writer or signer in the CPI
        let maybe_highest_privileges = account_metas
            .iter()
            .filter(|&x| x.pubkey == account_meta.pubkey)
            .map(|x| (x.is_signer, x.is_writable))
            .reduce(|acc, x| (acc.0 || x.0, acc.1 || x.1));
        // If `Some`, then the account was found somewhere in the instruction
        if let Some((is_signer, is_writable)) = maybe_highest_privileges {
            if !is_signer && is_signer != account_meta.is_signer {
                // Existing account is *NOT* a signer already, but the CPI
                // wants it to be, so de-escalate to not be a signer
                account_meta.is_signer = false;
            }
            if !is_writable && is_writable != account_meta.is_writable {
                // Existing account is *NOT* writable already, but the CPI
                // wants it to be, so de-escalate to not be writable
                account_meta.is_writable = false;
            }
        }
    }

    /// Resolve a program-derived address (PDA) from the instruction data
    /// and the accounts that have already been resolved
    fn resolve_pda(
        resolved_metas: &BTreeMap<usize, AccountMeta>,
        instruction_data: &[u8],
        program_id: &Pubkey,
        seeds: &[Seed],
    ) -> Result<Pubkey, ProgramError> {
        let mut pda_seeds: Vec<&[u8]> = vec![];
        for config in seeds {
            match config {
                Seed::Literal { bytes } => pda_seeds.push(bytes),
                Seed::InstructionArg { index, ty } => {
                    let arg_start = *index as usize;
                    let arg_end = arg_start + ty.arg_size() as usize;
                    pda_seeds.push(&instruction_data[arg_start..arg_end]);
                }
                Seed::AccountKey { index } => {
                    let account_index = *index as usize;
                    let account_meta = resolved_metas
                        .get(&account_index)
                        .ok_or::<ProgramError>(AccountResolutionError::AccountNotFound.into())?;
                    pda_seeds.push(account_meta.pubkey.as_ref());
                }
            }
        }
        Ok(Pubkey::find_program_address(&pda_seeds, program_id).0)
    }

    /// Resolve the stack and populate the instruction with required accounts
    fn resolve_metas(&mut self, instruction: &mut Instruction) -> Result<(), ProgramError> {
        let mut resolved_metas = BTreeMap::new();

        while !self.is_empty() {
            let node = self.pop();
            let mut account_meta = if let RequiredAccount::Pda {
                seeds,
                is_signer,
                is_writable,
            } = node.required_account
            {
                AccountMeta {
                    pubkey: Self::resolve_pda(
                        &resolved_metas,
                        &instruction.data,
                        &instruction.program_id,
                        &seeds,
                    )?,
                    is_signer,
                    is_writable,
                }
            } else {
                AccountMeta::try_from(&node.required_account)?
            };
            Self::de_escalate_account_meta(&mut account_meta, &instruction.accounts);
            resolved_metas.insert(node.index, account_meta);
        }

        instruction.accounts = resolved_metas.values().cloned().collect();

        Ok(())
    }

    /// Resolve the extra account metas for an instruction
    pub fn resolve<T: SplDiscriminate>(
        instruction: &mut Instruction,
        data: &[u8],
    ) -> Result<(), ProgramError> {
        let mut stack = Self::new();

        let state = TlvStateBorrowed::unpack(data)?;
        let bytes = state.get_bytes::<T>()?;
        let extra_account_metas = PodSlice::<PodAccountMeta>::unpack(bytes)?;

        for (index, ix_account_meta) in instruction.accounts.iter().enumerate() {
            stack.insert(index, RequiredAccount::from(ix_account_meta))?;
        }

        for (index, account_meta) in extra_account_metas.data().iter().enumerate() {
            stack.insert(
                index + instruction.accounts.len(),
                RequiredAccount::try_from(account_meta)?,
            )?;
        }

        stack.resolve_metas(instruction)?;

        Ok(())
    }
}
