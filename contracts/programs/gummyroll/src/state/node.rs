use anchor_lang::prelude::*;
use std::ops::Deref;
use std::ops::DerefMut;

pub const EMPTY: Node = Node {
    inner: [0 as u8; 32],
};

#[derive(Debug, Copy, Clone, AnchorDeserialize, AnchorSerialize, Default, PartialEq)]
pub struct Node {
    pub inner: [u8; 32],
}

impl Node {
    pub fn new(inner: [u8; 32]) -> Self {
        Self { inner }
    }
}

impl Deref for Node {
    type Target = [u8; 32];
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl AsRef<[u8; 32]> for Node {
    fn as_ref(&self) -> &[u8; 32] {
        &self.inner
    }
}

impl From<[u8; 32]> for Node {
    fn from(inner: [u8; 32]) -> Self {
        Self { inner }
    }
}
