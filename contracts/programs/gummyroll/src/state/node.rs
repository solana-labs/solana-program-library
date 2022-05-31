use anchor_lang::prelude::*;
use std::ops::Deref;
use std::ops::DerefMut;

#[derive(Debug, Copy, Clone, AnchorDeserialize, AnchorSerialize, Default, PartialEq)]
pub struct Node {
    pub inner: [u8; 32],
}
