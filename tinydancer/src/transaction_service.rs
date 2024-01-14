use std::{str::FromStr, sync::Arc};

use crate::{
    send_rpc_call,
    tinydancer::{endpoint, ClientService, Cluster},
    ValidatorSet,
};
use anyhow::anyhow;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::{Signable, Signature};
use tokio::{sync::Mutex, task::JoinHandle};

pub struct TransactionService {
    pub handler: JoinHandle<()>,
    // pub status: Arc<Mutex<ClientStatus>>,
}

pub struct TransactionServiceConfig {
    pub cluster: Cluster,
    pub validator_set: Arc<Mutex<Vec<(String, u64)>>>,
    pub slot: u64,
    pub current_epoch: u64,
}

#[async_trait]
impl ClientService<TransactionServiceConfig> for TransactionService {
    type ServiceError = tokio::task::JoinError;

    fn new(config: TransactionServiceConfig) -> Self {
        // let validator_set = Arc::new(
        //     read_validator_set(&config.validator_set_path).unwrap_or(ValidatorSet::default()),
        // );
        let validator_set = config.validator_set;
        // let slot = config.slot;

        let handler = tokio::spawn(async move {
            let rpc_url = endpoint(config.cluster);
            let vote_pubkeys: Vec<String> = validator_set
                .lock()
                .await
                .iter()
                .map(|item| item.0.clone())
                .collect();
            println!("keys: {:?}", vote_pubkeys);
            let vote_signatures = request_vote_signatures(config.slot, rpc_url, vote_pubkeys).await;
            println!("votes: {:?}", vote_signatures);
            let signatures: Vec<Signature> = vote_signatures
                .unwrap()
                .result
                .vote_signature
                .iter()
                .map(|sig| Signature::from_str(sig.as_str()))
                .flatten()
                .collect();
        });
        Self { handler }
    }
    async fn join(self) -> std::result::Result<(), Self::ServiceError> {
        self.handler.await
    }
}

pub async fn request_vote_signatures(
    slot: u64,
    endpoint: String,
    vote_pubkeys: Vec<String>,
) -> Result<GetVoteSignaturesResponse, serde_json::Error> {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getVoteSignatures",
        "params":[
            slot,
            {
                "votePubkey": vote_pubkeys,
                "commitment": "confirmed"
            }
        ]
    }) // getting one shred just to get max shreds per slot, can maybe randomize the selection here
    .to_string();

    let res = send_rpc_call!(endpoint, request);
    // info!("{:?}", res);
    serde_json::from_str::<GetVoteSignaturesResponse>(&res)
}

pub fn read_validator_set(path: &str) -> anyhow::Result<ValidatorSet> {
    match std::fs::read_to_string(path) {
        Ok(data) => {
            let validator_set: Result<ValidatorSet, serde_json::Error> =
                serde_json::from_str(&data);
            validator_set.map_err(|e| anyhow!(e.to_string()))
        }
        Err(e) => Err(anyhow!(e.to_string())),
    }
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVoteSignaturesResponse {
    pub jsonrpc: String,
    pub result: GetVoteSignaturesResult,
    pub id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVoteSignaturesResult {
    pub vote_signature: Vec<String>,
}
