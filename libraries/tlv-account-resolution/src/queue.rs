//! Because the seed configurations for an `AccountMetaPda` can reference
//! the public key of another account in either the instruction's accounts
//! or the required extra accounts, we have to collect all accounts
//! first as a `RequiredAccount` and then resolve them to `AccountMeta`s
//! afterwards, when we know which ones to grab first.
//!
//! This is due to the fact that, for example, a PDA at index 3 could
//! require the pubkey of another PDA at index 5 (after it).
//!
//! To solve this, we use a queue!

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
    std::collections::{BTreeMap, HashMap, HashSet},
};

/// A queue for resolving account metas in order based on dependent
/// account keys
pub struct AccountResolutionQueue {
    root: Node,
    required_accounts_map: HashMap<usize, RequiredAccount>,
    initial_instruction_accounts_len: usize,
}
impl AccountResolutionQueue {
    /// Create a new queue
    fn new(instruction_accounts: &Vec<AccountMeta>) -> Result<Self, ProgramError> {
        let mut required_accounts_map = HashMap::new();
        let initial_instruction_accounts_len = instruction_accounts.len();

        // Build the queue from the instruction accounts
        let mut root = Node::new(0, None);
        let mut current_node = &mut root;
        for (index, account) in instruction_accounts.iter().enumerate() {
            required_accounts_map.insert(index, RequiredAccount::from(account));
            if index == 0 {
                continue;
            }
            current_node.insert_next(index, None);
            current_node = current_node.next.as_mut().unwrap();
        }

        Ok(Self {
            root,
            required_accounts_map,
            initial_instruction_accounts_len,
        })
    }

    fn update_recursive(
        node: &mut Node,
        index: usize,
        dependencies: HashSet<usize>,
        unfound_dependencies: &mut HashSet<usize>,
    ) -> Result<(), ProgramError> {
        if node.index == index {
            if node.pop() {
                return Self::update_recursive(node, index, dependencies, unfound_dependencies);
            }
        } else {
            if dependencies.contains(&node.index) {
                unfound_dependencies.remove(&node.index);
            }
            if node.has_dependency(index) {
                if unfound_dependencies.is_empty() {
                    node.insert_before(index, Some(dependencies));
                } else if node.search_for_set(unfound_dependencies) {
                    return Err(AccountResolutionError::CircularReference.into());
                } else {
                    node.insert_before(index, Some(dependencies));
                }
                return Ok(());
            }
            if let Some(ref mut next) = node.next {
                return Self::update_recursive(next, index, dependencies, unfound_dependencies);
            } else {
                node.insert_next(index, Some(dependencies));
            }
        }
        Ok(())
    }

    /// Adds a new node to the queue
    fn add(&mut self, index: usize, required_account: RequiredAccount) -> Result<(), ProgramError> {
        let accounts_list_index = index + self.initial_instruction_accounts_len;
        match &required_account {
            RequiredAccount::Account { .. } => {
                self.root.insert_before(accounts_list_index, None);
            }
            RequiredAccount::Pda { seeds, .. } => {
                if let Some(dependencies) = Seed::get_account_key_indices(seeds) {
                    let mut unfound_dependencies = dependencies.clone();
                    Self::update_recursive(
                        &mut self.root,
                        accounts_list_index,
                        dependencies,
                        &mut unfound_dependencies,
                    )?;
                    for new_node in unfound_dependencies {
                        self.root.insert_before(new_node, None);
                    }
                } else {
                    self.root.insert_before(accounts_list_index, None);
                }
            }
        }
        self.required_accounts_map
            .insert(accounts_list_index, required_account);

        Ok(())
    }

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

    fn resolve_recursive(
        resolved_metas: &mut BTreeMap<usize, AccountMeta>,
        node: &Node,
        required_accounts_map: &HashMap<usize, RequiredAccount>,
        instruction_data: &[u8],
        program_id: &Pubkey,
    ) -> Result<(), ProgramError> {
        if let Some(acct) = required_accounts_map.get(&node.index) {
            let meta = if let RequiredAccount::Pda {
                seeds,
                is_signer,
                is_writable,
            } = acct
            {
                let pubkey =
                    Self::resolve_pda(resolved_metas, instruction_data, program_id, seeds)?;
                AccountMeta {
                    pubkey,
                    is_signer: *is_signer,
                    is_writable: *is_writable,
                }
            } else {
                AccountMeta::try_from(acct)?
            };
            resolved_metas.insert(node.index, meta);
        } else {
            return Err(AccountResolutionError::AccountNotFound.into());
        }
        if let Some(next) = &node.next {
            Self::resolve_recursive(
                resolved_metas,
                next,
                required_accounts_map,
                instruction_data,
                program_id,
            )
        } else {
            Ok(())
        }
    }

    /// Resolve the entire queue into a list of `AccountMeta`s
    fn resolve_metas(&mut self, instruction: &mut Instruction) -> Result<(), ProgramError> {
        let mut resolved_metas = BTreeMap::new();
        Self::resolve_recursive(
            &mut resolved_metas,
            &self.root,
            &self.required_accounts_map,
            &instruction.data,
            &instruction.program_id,
        )?;

        for account_meta in &mut resolved_metas.values_mut() {
            // This is a little tricky to read, but the idea is to see if
            // this account is marked as writable or signer anywhere in
            // the instruction at the start. If so, DON'T escalate it to
            // be a writer or signer
            let maybe_highest_privileges = &instruction
                .accounts
                .iter()
                .filter(|&x| x.pubkey == account_meta.pubkey)
                .map(|x| (x.is_signer, x.is_writable))
                .reduce(|acc, x| (acc.0 || x.0, acc.1 || x.1));
            // If `Some`, then the account was found somewhere in the instruction
            if let Some((is_signer, is_writable)) = maybe_highest_privileges {
                if !is_signer && is_signer != &account_meta.is_signer {
                    // Existing account is *NOT* a signer already,
                    // so de-escalate to not be a signer
                    account_meta.is_signer = false;
                }
                if !is_writable && is_writable != &account_meta.is_writable {
                    // Existing account is *NOT* writable already,
                    // so de-escalate to not be writable
                    account_meta.is_writable = false;
                }
            }
        }

        instruction.accounts = resolved_metas.values().cloned().collect();

        Ok(())
    }

    /// Resolve the extra account metas for an instruction
    pub fn resolve<T: SplDiscriminate>(
        instruction: &mut Instruction,
        data: &[u8],
    ) -> Result<(), ProgramError> {
        let mut queue = Self::new(&instruction.accounts)?;

        let state = TlvStateBorrowed::unpack(data)?;
        let bytes = state.get_bytes::<T>()?;
        let extra_account_metas = PodSlice::<PodAccountMeta>::unpack(bytes)?;

        for (index, account_meta) in extra_account_metas.data().iter().enumerate() {
            let required_account = RequiredAccount::try_from(account_meta)?;
            queue.add(index, required_account)?;
        }

        queue.resolve_metas(instruction)?;

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Node {
    /// The index of the account in the total accounts list
    index: usize,
    /// The indices of the accounts whose keys this account's PDA depends on
    dependencies: Option<HashSet<usize>>,
    next: Option<Box<Node>>,
}
impl Node {
    fn new(index: usize, dependencies: Option<HashSet<usize>>) -> Self {
        Self {
            index,
            dependencies,
            next: None,
        }
    }

    fn new_with_next(index: usize, dependencies: Option<HashSet<usize>>, next: Node) -> Self {
        Self {
            index,
            dependencies,
            next: Some(Box::new(next)),
        }
    }

    fn has_next(&self) -> bool {
        self.next.is_some()
    }

    fn next(&self) -> &Self {
        self.next.as_ref().unwrap()
    }

    fn search(&self, index: usize) -> bool {
        if self.index == index {
            return true;
        }
        if self.has_next() {
            self.next().search(index)
        } else {
            false
        }
    }

    fn search_for_set(&self, indices: &HashSet<usize>) -> bool {
        if indices.is_empty() {
            return true;
        }
        let mut node = self;
        while node.has_next() {
            for index in indices.iter() {
                if node.index == *index {
                    return true;
                }
            }
            node = node.next();
        }
        false
    }

    fn pop(&mut self) -> bool {
        if self.has_next() {
            let next = self.next.take().unwrap();
            *self = *next;
            return true;
        }
        false
    }

    fn insert_before(&mut self, index: usize, dependencies: Option<HashSet<usize>>) {
        if !self.search(index) {
            *self = Self::new_with_next(index, dependencies, self.clone());
        }
    }

    fn insert_next(&mut self, index: usize, dependencies: Option<HashSet<usize>>) {
        if self.has_next() {
            let prev_next = self.next.clone().unwrap();
            let new_next = Self::new_with_next(index, dependencies, *prev_next);
            *self = Self::new_with_next(self.index, self.dependencies.clone(), new_next);
        } else {
            self.next = Some(Box::new(Self::new(index, dependencies)));
        }
    }

    fn has_dependency(&self, index: usize) -> bool {
        if let Some(dependencies) = &self.dependencies {
            dependencies.contains(&index)
        } else {
            false
        }
    }
}
