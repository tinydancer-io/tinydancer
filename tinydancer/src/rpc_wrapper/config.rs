use serde::{Deserialize, Serialize};
use solana_sdk::commitment_config::{CommitmentLevel, CommitmentConfig};
use solana_sdk::pubkey::Pubkey;
use crate::rpc_wrapper::encoding::BinaryEncoding;
use solana_transaction_status::UiTransactionEncoding;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcShredConfig {
    #[serde(flatten)]
    pub commitment: Option<CommitmentConfig>,
}


#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendTransactionConfig {
    //    #[serde(default)]
    //    pub skip_preflight: bool,
    //    #[serde(default)]
    //    pub preflight_commitment: CommitmentLevel,
    #[serde(default)]
    pub encoding: BinaryEncoding,
    pub max_retries: Option<u16>,
    //    pub min_context_slot: Option<Slot>,
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IsBlockHashValidConfig {
    pub commitment: Option<CommitmentLevel>,
    //    pub minContextSlot: Option<u64>,
}






#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct BlockHeader {
    pub vote_signature: Vec<Option<String>>,
    pub validator_identity: Vec<Option<Pubkey>>,
    pub validator_stake: Vec<Option<u64>>,
}