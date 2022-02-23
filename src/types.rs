#![allow(clippy::enum_variant_names)]

use std::collections::HashMap;

#[derive(CandidType)]
pub struct MetadataPart<'a> {
    pub purpose: MetadataPurpose,
    pub key_val_data: HashMap<&'static str, MetadataVal>,
    pub data: &'a [u8],
}

#[derive(CandidType)]
#[allow(dead_code)]
pub enum MetadataPurpose {
    Preview,
    Rendered,
}

#[derive(CandidType)]
#[allow(dead_code)]
pub enum MetadataVal {
    TextContent(String),
    BlobContent(Vec<u8>),
    NatContent(u128),
    Nat8Content(u8),
    Nat16Content(u16),
    Nat32Content(u32),
    Nat64Content(u64),
}

pub use MetadataVal::*;

#[derive(CandidType, Deserialize, PartialEq)]
pub enum InterfaceId {
    Approval,
    TransactionHistory,
    Mint,
    Burn,
    TransferNotification,
}

#[derive(CandidType, Deserialize, Error, Debug)]
pub enum MintError {
    #[error("You aren't authorized as a custodian of that canister.")]
    Unauthorized,
}

#[derive(CandidType, Deserialize)]
pub struct MintReceipt {
    pub id: u128,
    pub token_id: u64,
}
