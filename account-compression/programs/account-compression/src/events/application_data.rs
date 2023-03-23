use anchor_lang::prelude::*;

#[derive(AnchorDeserialize, AnchorSerialize)]
#[repr(C)]
pub enum ApplicationDataEvent {
    V1(ApplicationDataEventV1),
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct ApplicationDataEventV1 {
    pub application_data: Vec<u8>,
}
