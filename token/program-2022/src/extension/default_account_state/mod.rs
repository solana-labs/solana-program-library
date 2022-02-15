use {
    crate::extension::{Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
};

/// Default Account state extension instructions
pub mod instruction;

/// Default Account state extension processor
pub mod processor;

/// Default Account::state extension data for mints.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct DefaultAccountState {
    /// Default Account::state in which new Accounts should be initialized
    pub state: PodAccountState,
}
impl Extension for DefaultAccountState {
    const TYPE: ExtensionType = ExtensionType::DefaultAccountState;
}

type PodAccountState = u8;
