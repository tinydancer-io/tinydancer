use crate::tinydancer::{endpoint, ClientService, ClientStatus, Cluster};
use crate::sampler::{ArchiveConfig, SlotSubscribeResponse};
use crate::{convert_to_websocket, send_rpc_call, try_coerce_shred};
use anyhow::anyhow;
use async_trait::async_trait;
use crossbeam::channel::{Receiver, Sender};
use futures::Sink;
use itertools::Itertools;
use rand::distributions::Uniform;
use rand::prelude::*;
use rayon::prelude::*;
use reqwest::Request;
use rocksdb::{ColumnFamily, Options as RocksOptions, DB};
use serde::de::DeserializeOwned;
use solana_ledger::shred::{ShredId, ShredType};
use solana_ledger::{
ancestor_iterator::{AncestorIterator, AncestorIteratorWithHash},
blockstore::Blockstore,
// blockstore_db::columns::ShredCode,
shred::{Nonce, Shred, ShredCode, ShredData, ShredFetchStats, SIZE_OF_NONCE},
};
use solana_sdk::hash::hashv;
use solana_sdk::{
clock::Slot,
genesis_config::ClusterType,
hash::{Hash, HASH_BYTES},
packet::PACKET_DATA_SIZE,
pubkey::{Pubkey, PUBKEY_BYTES},
signature::{Signable, Signature, Signer, SIGNATURE_BYTES},
signer::keypair::Keypair,
timing::{duration_as_ms, timestamp},
};
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::{error::Error, ops::Add};
use std::{
net::{SocketAddr, UdpSocket},
thread::Builder,
};
use tiny_logger::logs::{debug, error, info};
use tokio::{
sync::mpsc::UnboundedSender,
task::{JoinError, JoinHandle},
sync::Mutex as TokioMutex,
};
use tungstenite::{connect, Message};
use url::Url;
use serde_derive::Deserialize;
use serde_derive::Serialize;

pub struct ConsensusService {
    consensus_indices: Vec<u64>,
    consensus_handler: JoinHandle<()>,
}

pub struct ConsensusServiceConfig {
    pub cluster: Cluster,
    pub archive_config: ArchiveConfig,
    pub instance: Arc<rocksdb::DB>,
    pub status_consensus: Arc<TokioMutex<ClientStatus>>,
    pub sample_qty: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
    pub struct RpcBlockCommitment<T> {
    pub commitment: Option<T>,
    pub total_stake: u64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
    pub struct GetCommittmentResponse {
    pub jsonrpc: String,
    pub result: RpcBlockCommitment<BlockCommitmentArray>,
    pub id: i64,
}

pub const MAX_LOCKOUT_HISTORY: usize = 31;
pub type BlockCommitmentArray = [u64; MAX_LOCKOUT_HISTORY + 1];

pub const VOTE_THRESHOLD_SIZE: f64 = 2f64 / 3f64;

#[async_trait]
impl ClientService<ConsensusServiceConfig> for ConsensusService {
type ServiceError = tokio::task::JoinError;

fn new(config: ConsensusServiceConfig) -> Self {
    let consensus_handler = tokio::spawn(async move {
        let rpc_url = endpoint(config.cluster);
        let pub_sub = convert_to_websocket!(rpc_url);

        let mut threads = Vec::default();

        let (slot_update_tx, slot_update_rx) = crossbeam::channel::unbounded::<u64>();

        let status_arc = config.status_consensus.clone();

        // waits on new slots => triggers slot_verify_loop
        threads.push(tokio::spawn(slot_update_loop(
            slot_update_tx,
            pub_sub,
            config.status_consensus,
        )));

        // verify slot votes
        threads.push(tokio::spawn(slot_verify_loop(
            slot_update_rx,
            rpc_url,
            status_arc,
        )));


        for thread in threads {
            thread.await;
        }
    });

    Self {
        consensus_handler,
        consensus_indices: Vec::default(),
    }
}

async fn join(self) -> std::result::Result<(), Self::ServiceError> {
    self.consensus_handler.await
}
}

pub async fn slot_update_loop(
    slot_update_tx: Sender<u64>,
    pub_sub: String,
    status_sampler: Arc<TokioMutex<ClientStatus>>,
) -> anyhow::Result<()> {
let result = match connect(Url::parse(pub_sub.as_str()).unwrap()) {
    Ok((socket, _response)) => Some((socket, _response)),
    Err(_) => {
        let mut status = status_sampler.lock().await;
        *status = ClientStatus::Crashed(String::from("Client can't connect to socket"));
        None
    }
};

if result.is_none() {
    return Err(anyhow!(""));
}

let (mut socket, _response) = result.unwrap();

socket.write_message(Message::Text(
    r#"{ "jsonrpc": "2.0", "id": 1, "method": "slotSubscribe" }"#.into(),
))?;

loop {
    match socket.read_message() {
        Ok(msg) => {
            let res = serde_json::from_str::<SlotSubscribeResponse>(msg.to_string().as_str());

            // info!("res: {:?}", msg.to_string().as_str());
            if let Ok(res) = res {
                match slot_update_tx.send(res.params.result.root as u64) {
                    Ok(_) => {
                        info!("slot updated: {:?}", res.params.result.root);
                    }
                    Err(e) => {
                        info!("error here: {:?} {:?}", e, res.params.result.root as u64);
                        continue; // @TODO: we should add retries here incase send fails for some reason
                    }
                }
            }
        }
        Err(e) => info!("err: {:?}", e),
    }
}
}

// verifies the total vote on the slot > 2/3
fn verify_slot(slot_commitment: RpcBlockCommitment<BlockCommitmentArray>) -> bool {
    let commitment_array = &slot_commitment.commitment;
    let total_stake = &slot_commitment.total_stake;
    let sum: u64 = commitment_array.iter().flatten().sum();

    if (sum as f64 / *total_stake as f64) > VOTE_THRESHOLD_SIZE {
        true
    } else {
        false
    }
}

pub async fn slot_verify_loop(
    slot_update_rx: Receiver<u64>,
    endpoint: String,
    status_sampler: Arc<tokio::sync::Mutex<ClientStatus>>,
) -> anyhow::Result<()> {
loop {
        let mut status = status_sampler.lock().await;
        if let ClientStatus::Crashed(_) = &*status {
            return Err(anyhow!("Client crashed"));
        } else {
            *status = ClientStatus::Active(String::from(
                "Monitoring Tinydancer: Verifying consensus",
            ));
        }
        if let Ok(slot) = slot_update_rx.recv() {
            let slot_commitment_result = request_slot_voting(slot, &endpoint).await;
            
            if let Err(e) = slot_commitment_result {
                println!("Error {}", e);
                info!("{}", e);
                continue;
            }

            let slot_commitment = slot_commitment_result.unwrap();

            let verified = verify_slot(slot_commitment.result);

            if verified {
                info!("slot {:?} verified ", slot);
            } else {
                info!("slot {:?} failed to verified ", slot);
                info!("sample INVALID for slot : {:?}", slot);
            }
        }
    }
}

pub async fn request_slot_voting(
    slot: u64,
    endpoint: &String,
) -> Result<GetCommittmentResponse, serde_json::Error> {

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBlockCommitment",
        "params": [
            slot
        ]
    })
    .to_string();

    let res = send_rpc_call!(endpoint, request);

    serde_json::from_str::<GetCommittmentResponse>(&res)
}
