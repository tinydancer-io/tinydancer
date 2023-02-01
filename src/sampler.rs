use async_trait::async_trait;
use crossbeam::channel::{Receiver, Sender};
use futures::{AsyncSink, Sink};
use rand::distributions::Uniform;
use rand::prelude::*;
use reqwest::Request;
use solana_ledger::{
    ancestor_iterator::{AncestorIterator, AncestorIteratorWithHash},
    blockstore::Blockstore,
    shred::{Nonce, Shred, ShredFetchStats, SIZE_OF_NONCE},
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
use std::error::Error;
use std::{
    net::{SocketAddr, UdpSocket},
    thread::Builder,
};
use tokio::{
    sync::mpsc::UnboundedSender,
    task::{JoinError, JoinHandle},
};
use tungstenite::{connect, Message};
use url::Url;

use crate::convert_to_websocket;
use crate::tinydancer::{endpoint, ClientService, Cluster};
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
            println!("log {}", pub_sub);
            let (slot_update_tx, slot_update_rx) = crossbeam::channel::unbounded::<u64>();

            threads.push(tokio::spawn(slot_update_loop(slot_update_tx, pub_sub)));
            threads.push(tokio::spawn(async move {
                while let Ok(slot) = slot_update_rx.recv() {
                    println!("GOT = {}", slot);
                }
            }));

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
pub fn gen_random_indices(max_shreds_per_slot: u64, sample_qty: u64) -> Vec<u64> {
    let mut rng = StdRng::from_entropy();
    let vec = (0..max_shreds_per_slot)
        .map(|_| rng.gen())
        .collect::<Vec<u64>>();
    vec.as_slice()[0..(sample_qty as usize)].to_vec()
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
async fn slot_update_loop(tx: Sender<u64>, pub_sub: String) {
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
                    match tx.send(res.params.result.slot as u64) {
                        Ok(r) => {
                            println!("sent mssg:{:?}", r);
                        }
                        Err(e) => {
                            println!("error here: {:?}", e);
                            continue; // @TODO: we should add retries here incase send fails for some reason
                        }
                    }
                }
            }
            Err(e) => println!("err: {:?}", e),
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
    pub params: Params,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Params {
    pub result: Result,
    pub subscription: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Result {
    pub parent: i64,
    pub root: i64,
    pub slot: i64,
}
