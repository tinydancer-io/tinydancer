use crate::tinydancer::{endpoint, ClientService, Cluster};
use crate::{convert_to_websocket, send_rpc_call};
use async_trait::async_trait;
use crossbeam::channel::{Receiver, Sender};
use futures::{AsyncSink, Sink};
use rand::distributions::Uniform;
use rand::prelude::*;
use rayon::prelude::*;
use reqwest::Request;
use solana_ledger::{
    ancestor_iterator::{AncestorIterator, AncestorIteratorWithHash},
    blockstore::Blockstore,
    blockstore_db::columns::ShredCode,
    shred::{Nonce, Shred, ShredData, ShredFetchStats, SIZE_OF_NONCE},
};
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
use std::{error::Error, ops::Add};
use std::{
    net::{SocketAddr, UdpSocket},
    thread::Builder,
};
// use tiny_logger::{debug, error, info, log_enabled, Level};
use tokio::{
    sync::mpsc::UnboundedSender,
    task::{JoinError, JoinHandle},
};
use tungstenite::{connect, Message};
use url::Url;
pub struct SampleService {
    sample_indices: Vec<u64>,
    // peers: Vec<(Pubkey, SocketAddr)>,
    sampler_handle: JoinHandle<()>,
}
pub struct SampleServiceConfig {
    pub cluster: Cluster,
}

#[async_trait]
impl ClientService<SampleServiceConfig> for SampleService {
    type ServiceError = tokio::task::JoinError;
    fn new(config: SampleServiceConfig) -> Self {
        let sampler_handle = tokio::spawn(async {
            let rpc_url = endpoint(config.cluster);
            let pub_sub = convert_to_websocket!(rpc_url);
            let mut threads = Vec::default();

            let (slot_update_tx, slot_update_rx) = crossbeam::channel::unbounded::<u64>();
            let (shred_tx, shred_rx) = crossbeam::channel::unbounded();
            threads.push(tokio::spawn(slot_update_loop(slot_update_tx, pub_sub)));
            threads.push(tokio::spawn(shred_update_loop(
                slot_update_rx,
                rpc_url,
                shred_tx,
            )));
            threads.push(tokio::spawn(shred_verify_loop(shred_rx)));

            for thread in threads {
                thread.await;
            }
        });
        let sample_indices: Vec<u64> = Vec::default();
        Self {
            sampler_handle,
            sample_indices,
        }
    }
    async fn join(self) -> std::result::Result<(), Self::ServiceError> {
        self.sampler_handle.await
    }
}
pub fn gen_random_indices(max_shreds_per_slot: usize, sample_qty: usize) -> Vec<usize> {
    let mut rng = StdRng::from_entropy();
    let vec = (0..max_shreds_per_slot)
        .map(|_| rng.gen_range(0..max_shreds_per_slot))
        .collect::<Vec<usize>>();
    vec.as_slice()[0..(sample_qty as usize)].to_vec()
}
pub async fn request_shreds(
    slot: usize,
    indices: Vec<usize>,
    endpoint: String,
) -> Result<GetShredResponse, serde_json::Error> {
    let request = serde_json::json!(  {"jsonrpc": "2.0","id":1,"method":"getShreds","params":[slot,&indices]}) // getting one shred just to get max shreds per slot, can maybe randomize the selection here
        .to_string();
    let res = send_rpc_call!(endpoint, request);
    // datapoint!("this is a debug {}", res);
    serde_json::from_str::<GetShredResponse>(res.as_str())
}
// fn sample<R: Rng>(
//     &self,
//     rng: &mut R,
//     //index of the shred we're interested to start from
//     sample_index: u64,
//     /*index of the last shred, it would vary per block*/
//     last_shred_index: u64,
// ) {
//     todo!()
// }
async fn slot_update_loop(slot_update_tx: Sender<u64>, pub_sub: String) {
    let (mut socket, _response) =
        connect(Url::parse(pub_sub.as_str()).unwrap()).expect("Can't connect to websocket");
    socket
        .write_message(Message::Text(
            r#"{ "jsonrpc": "2.0", "id": 1, "method": "slotSubscribe" }"#.into(),
        ))
        .unwrap();

    loop {
        match socket.read_message() {
            Ok(msg) => {
                let res = serde_json::from_str::<SlotSubscribeResponse>(msg.to_string().as_str());
                // println!("res: {:?}", msg.to_string().as_str());
                if let Ok(res) = res {
                    match slot_update_tx.send(res.params.result.root as u64) {
                        Ok(_) => {
                            println!("slot updated: {:?}", res.params.result.root);
                        }
                        Err(e) => {
                            println!("error here: {:?} {:?}", e, res.params.result.root as u64);
                            continue; // @TODO: we should add retries here incase send fails for some reason
                        }
                    }
                }
            }
            Err(e) => println!("err: {:?}", e),
        }
    }
}

async fn shred_update_loop(
    slot_update_rx: Receiver<u64>,
    endpoint: String,
    shred_tx: Sender<Vec<Shred>>,
) {
    loop {
        if let Ok(slot) = slot_update_rx.recv() {
            let shred_for_one = request_shreds(slot as usize, vec![0], endpoint.clone()).await;
            let shred_indices_for_slot = match shred_for_one {
                Ok(first_shred) => {
                    let first_shred = &first_shred.result[1];
                    let max_shreds_per_slot = if first_shred.is_code() {
                        Some((
                            first_shred
                                .num_data_shreds()
                                .expect("num data shreds error"),
                            first_shred
                                .num_coding_shreds()
                                .expect("num code shreds error"),
                        ))
                    } else {
                        println!("shred: {:?}", first_shred);
                        None
                    };
                    println!("max_shreds_per_slot {:?}", max_shreds_per_slot);

                    if let Some((d, c)) = max_shreds_per_slot {
                        let indices = gen_random_indices(d.add(c) as usize, 10); // unwrap only temporary
                        Some(indices)
                    } else {
                        None
                    }
                }
                Err(_) => {
                    //@TODO: add logger here

                    None
                }
            };
            println!("indices of: {:?}", shred_indices_for_slot);
            if let Some(shred_indices_for_slot) = shred_indices_for_slot {
                let shreds_for_slot =
                    request_shreds(slot as usize, shred_indices_for_slot, endpoint.clone()).await;
                println!("2nd req: {:?}", shreds_for_slot);
                if let Ok(shreds_for_slot) = shreds_for_slot {
                    println!("shred for slot: {:?}", shreds_for_slot);
                    shred_tx
                        .send(shreds_for_slot.result)
                        .expect("shred tx send error");
                }
            }
        }
    }
}

pub async fn shred_verify_loop(shred_rx: Receiver<Vec<Shred>>) {
    loop {
        if let Ok(shreds) = shred_rx.recv() {
            shreds
                .par_iter()
                .for_each(|sh| println!("id of shred{:?}", sh.id()));
        }
    }
}
use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotSubscribeResponse {
    pub jsonrpc: String,
    pub method: String,
    pub params: SlotSubscribeParams,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotSubscribeParams {
    pub result: SlotSubscribeResult,
    pub subscription: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotSubscribeResult {
    pub parent: i64,
    pub root: i64,
    pub slot: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetShredResponse {
    pub jsonrpc: String,
    pub result: Vec<Shred>,
    pub id: i64,
}

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct GetShredResult {
//     #[serde(rename = "ShredData")]
//     pub shred_data: Option<ShredData>,
//     #[serde(rename = "ShredCode")]
//     pub shred_code: Option<ShredCode>,
// }
