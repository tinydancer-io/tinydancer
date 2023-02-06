use crate::tinydancer::{endpoint, ClientService, Cluster};
use crate::{convert_to_websocket, send_rpc_call, try_coerce_shred};
use async_trait::async_trait;
use crossbeam::channel::{Receiver, Sender};
use futures::{AsyncSink, Sink};
use itertools::Itertools;
use rand::distributions::Uniform;
use rand::prelude::*;
use rayon::prelude::*;
use reqwest::Request;
use solana_ledger::{
    ancestor_iterator::{AncestorIterator, AncestorIteratorWithHash},
    blockstore::Blockstore,
    // blockstore_db::columns::ShredCode,
    shred::{Nonce, Shred, ShredCode, ShredData, ShredFetchStats, SIZE_OF_NONCE},
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
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::{error::Error, ops::Add};
use std::{
    net::{SocketAddr, UdpSocket},
    thread::Builder,
};
use tiny_logger::logs::{debug, info};
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
    let request =
        serde_json::json!(  {"jsonrpc": "2.0","id":1,"method":"getShreds","params":[slot,&indices,{
          "commitment": "confirmed"
        }]}) // getting one shred just to get max shreds per slot, can maybe randomize the selection here
        .to_string();
    let res = send_rpc_call!(endpoint, request);
    // println!("why not result {:?}", res);
    serde_json::from_str::<GetShredResponse>(res.to_string().as_str())
}

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
    shred_tx: Sender<(Vec<Option<Shred>>, solana_ledger::shred::Pubkey)>,
) {
    loop {
        if let Ok(slot) = slot_update_rx.recv() {
            let shred_for_one = request_shreds(slot as usize, vec![0], endpoint.clone()).await;
            // println!("res {:?}", shred_for_one);
            let shred_indices_for_slot = match shred_for_one {
                Ok(first_shred) => {
                    let first_shred = &first_shred.result.shreds[1].clone(); // add some check later

                    let max_shreds_per_slot = if let Some(first_shred) = first_shred {
                        match (
                            first_shred.clone().shred_data,
                            first_shred.clone().shred_code,
                        ) {
                            (Some(data_shred), None) => {
                                Some(
                                    Shred::ShredData(data_shred)
                                        .num_data_shreds()
                                        .expect("num data shreds error"),
                                )
                                // Some(data_shred. ().expect("num data shreds error"))
                            }
                            (None, Some(coding_shred)) => Some(
                                Shred::ShredCode(coding_shred)
                                    .num_coding_shreds()
                                    .expect("num code shreds error"),
                            ),
                            _ => None,
                        }
                    } else {
                        println!("shred: {:?}", first_shred);
                        None
                    };
                    println!("max_shreds_per_slot {:?}", max_shreds_per_slot);

                    if let Some(max_shreds_per_slot) = max_shreds_per_slot {
                        let indices = gen_random_indices(max_shreds_per_slot as usize, 10); // unwrap only temporary
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
            if let Some(shred_indices_for_slot) = shred_indices_for_slot.clone() {
                let shreds_for_slot = request_shreds(
                    slot as usize,
                    shred_indices_for_slot.clone(),
                    endpoint.clone(),
                )
                .await;
                // println!("made 2nd req: {:?}", shreds_for_slot);
                if let Ok(shreds_for_slot) = shreds_for_slot {
                    println!("get shred for slot in 2nd req");
                    let mut shreds: Vec<Option<Shred>> = shreds_for_slot
                        .result
                        .shreds
                        .par_iter()
                        .map(|s| try_coerce_shred!(s))
                        .collect();
                    // println!("before leader");
                    let leader = solana_ledger::shred::Pubkey::from_str(
                        shreds_for_slot.result.leader.as_str(),
                    )
                    .unwrap();
                    // println!("leader {:?}", leader);
                    let mut fullfill_count = AtomicU32::new(0u32);
                    shreds.dedup();
                    shreds.iter().for_each(|f| {
                        if let Some(s) = f {
                            info!("{:?}", s.index());
                        }
                    });
                    shreds.par_iter().for_each(|s| {
                        if let Some(s) = s {
                            match shred_indices_for_slot.contains(&(s.index() as usize)) {
                                true => {
                                    fullfill_count.fetch_add(1, Ordering::Relaxed);
                                    info!(
                                        "Received requested shred: {:?} for slot: {:?}",
                                        s.index(),
                                        s.slot()
                                    )
                                }
                                false => info!(
                                    "Received unrequested shred index: {:?} for slot: {:?}",
                                    s.index(),
                                    s.slot()
                                ),
                            }
                        } else {
                            info!("Received empty")
                        }
                    });
                    if (fullfill_count.get_mut().to_owned() as usize) < shred_indices_for_slot.len()
                    {
                        info!("Received incomplete number of shreds, requested {:?} shreds for slot {:?} and received {:?}", shred_indices_for_slot.len(),slot, fullfill_count);
                    }
                    shred_tx
                        .send((shreds, leader))
                        .expect("shred tx send error");
                }
            }
        }
    }
}
// use solana_ledger::shred::dispatch;
pub fn verify_sample(shred: &Shred, leader: solana_ledger::shred::Pubkey) -> bool {
    // @TODO fix error handling here
    let verify_merkle_root = match shred {
        Shred::ShredData(ShredData::Merkle(shred)) => Some(shred.verify_merkle_proof()),

        Shred::ShredCode(ShredCode::Merkle(shred)) => Some(shred.verify_merkle_proof()),
        _ => None,
    };

    let verified = vec![shred.verify(&leader), {
        if let Some(proof) = verify_merkle_root {
            match proof {
                Ok(validated) => validated,
                Err(e) => panic!("{}", e),
            }
        } else {
            panic!("This was not a merkle shred"); // @TODO figure out how to handle this properly
        }
    }]
    .iter()
    .all(|s| *s == true);
    verified
}
pub async fn shred_verify_loop(
    shred_rx: Receiver<(Vec<Option<Shred>>, solana_ledger::shred::Pubkey)>,
) {
    loop {
        let rx = shred_rx.recv();

        if let Ok((shreds, leader)) = rx {
            shreds.par_iter().for_each(|sh| match sh {
                Some(shred) => {
                    let verified = verify_sample(shred, leader);
                    match verified {
                        true => info!(
                            "sample {:?} verified for slot: {:?}",
                            shred.index(),
                            shred.slot()
                        ),
                        false => info!("sample INVALID for slot : {:?}", shred.slot()),
                    }
                }
                None => {
                    println!("none")
                }
            });
        } else {
            println!("None")
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
    pub result: GetShredResult,
    pub id: i64,
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetShredResult {
    pub leader: String,
    pub shreds: Vec<Option<RpcShred>>, // This has to be an option
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcShred {
    #[serde(rename = "ShredData")]
    pub shred_data: Option<ShredData>,
    #[serde(rename = "ShredCode")]
    pub shred_code: Option<ShredCode>,
}
