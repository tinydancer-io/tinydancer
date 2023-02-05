use crate::tinydancer::{endpoint, ClientService, Cluster};
use crate::{convert_to_websocket, send_rpc_call, try_coerce_shred};
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
use std::{error::Error, ops::Add};
use std::{
    net::{SocketAddr, UdpSocket},
    thread::Builder,
};
use tiny_logger::logs::debug;
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
    // let res = serde_json::json!("{\"jsonrpc\":\"2.0\",\"result\":[{\"ShredData\":{\"Merkle\":{\"common_header\":{\"fec_set_index\":0,\"index\":0,\"shred_variant\":133,\"signature\":[177,101,220,159,243,113,65,54,237,233,96,152,170,160,69,65,176,24,59,150,252,108,83,109,75,142,175,233,41,35,237,125,221,216,225,65,168,47,58,48,48,255,220,141,220,2,35,6,197,46,213,108,107,155,231,15,111,247,23,161,195,254,172,2],\"slot\":100887,\"version\":37419},\"data_header\":{\"flags\":{\"bits\":72},\"parent_offset\":1,\"size\":911},\"payload\":[177,101,220,159,243,113,65,54,237,233,96,152,170,160,69,65,176,24,59,150,252,108,83,109,75,142,175,233,41,35,237,125,221,216,225,65,168,47,58,48,48,255,220,141,220,2,35,6,197,46,213,108,107,155,231,15,111,247,23,161,195,254,172,2,133,23,138,1,0,0,0,0,0,0,0,0,0,43,146,0,0,0,0,1,0,72,143,3,9,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,71,107,75,72,14,208,74,162,171,251,242,56,81,211,93,123,236,57,85,203,112,107,57,168,243,133,56,4,241,121,163,136,1,0,0,0,0,0,0,0,2,83,226,137,56,69,198,68,237,114,181,98,131,24,126,192,220,22,80,96,22,233,129,36,54,133,209,22,195,230,74,31,184,130,230,251,204,33,245,75,141,229,3,196,242,9,89,87,211,172,171,148,10,150,19,217,40,43,156,210,25,131,141,1,12,38,62,166,18,213,233,104,108,231,170,252,51,145,230,60,76,135,213,53,130,239,34,254,82,78,198,143,196,192,191,10,14,236,116,84,109,2,150,123,199,124,220,221,59,170,184,62,133,210,213,165,60,165,70,195,190,167,76,178,179,167,241,56,12,2,0,1,3,11,51,209,150,204,26,238,155,223,38,45,43,59,70,56,39,137,167,125,2,54,164,70,36,39,243,129,58,34,167,175,122,199,131,184,4,172,255,25,179,239,180,61,65,92,234,211,61,155,131,251,65,86,159,20,132,198,77,237,152,216,181,229,88,7,97,72,29,53,116,116,187,124,77,118,36,235,211,189,179,216,53,94,115,209,16,67,252,13,163,83,128,0,0,0,0,6,106,60,180,223,161,39,147,188,254,224,215,134,58,226,116,55,149,202,24,170,67,208,79,144,144,193,105,183,207,58,102,1,2,2,1,1,116,12,0,0,0,247,137,1,0,0,0,0,0,31,1,31,1,30,1,29,1,28,1,27,1,26,1,25,1,24,1,23,1,22,1,21,1,20,1,19,1,18,1,17,1,16,1,15,1,14,1,13,1,12,1,11,1,10,1,9,1,8,1,7,1,6,1,5,1,4,1,3,1,2,1,1,222,162,139,122,214,209,101,216,37,98,98,210,147,130,203,189,70,30,91,33,108,125,93,199,243,129,62,191,103,170,103,242,1,228,99,223,99,0,0,0,0,1,0,0,0,0,0,0,0,144,52,122,31,114,217,178,73,51,186,87,227,141,30,113,223,95,248,199,20,137,207,98,251,106,229,21,18,207,241,177,5,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,205,241,123,63,87,73,161,108,120,85,19,8,177,14,206,126,47,68,226,70,68,50,21,86,99,83,206,65,26,46,209,246,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,126,57,43,14,180,179,219,13,111,62,233,12,75,5,102,26,149,211,152,168,6,198,49,245,64,254,254,207,155,42,94,105,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,87,102,199,190,229,162,134,141,128,96,40,74,99,179,108,183,84,29,74,98,21,105,76,217,211,218,131,54,13,242,75,215,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,167,0,163,158,94,127,122,79,95,253,37,1,249,193,75,78,185,87,237,48,169,125,246,118,20,251,31,145,121,168,50,149,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,218,38,134,24,71,84,38,85,121,239,92,122,231,206,172,255,229,181,122,209,20,146,98,216,115,67,8,135,246,120,60,15,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,13,148,27,145,94,71,252,200,150,85,228,7,30,249,95,58,130,209,93,2,188,80,104,144,233,171,200,199,92,192,114,191,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,132,121,224,215,201,18,190,205,202,27,95,196,184,255,99,83,54,224,143,214,227,65,173,133,246,238,239,80,196,49,225,21,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,70,106,126,46,18,123,102,209,125,154,170,51,53,198,232,189,128,144,98,89,250,69,46,36,48,96,92,128,121,107,80,111,56,195,76,163,154,146,71,124,109,139,188,87,127,209,237,43,234,122,141,234,73,145,154,247,100,125,205,30,183,152,206,35,95,127,197,149,198,37,175,132,156,142,93,71,178,22,115,119,248,60,9,3,228,108,51,169,205,9,133,44,175,193,151,61,38,241,35,124,86,134,202,55,179,77,39,84,245,2,23,51,117,70,143,237,167,77,61,38]}}},{\"ShredCode\":{\"Merkle\":{\"coding_header\":{\"num_coding_shreds\":17,\"num_data_shreds\":1,\"position\":0},\"common_header\":{\"fec_set_index\":0,\"index\":0,\"shred_variant\":69,\"signature\":[177,101,220,159,243,113,65,54,237,233,96,152,170,160,69,65,176,24,59,150,252,108,83,109,75,142,175,233,41,35,237,125,221,216,225,65,168,47,58,48,48,255,220,141,220,2,35,6,197,46,213,108,107,155,231,15,111,247,23,161,195,254,172,2],\"slot\":100887,\"version\":37419},\"payload\":[177,101,220,159,243,113,65,54,237,233,96,152,170,160,69,65,176,24,59,150,252,108,83,109,75,142,175,233,41,35,237,125,221,216,225,65,168,47,58,48,48,255,220,141,220,2,35,6,197,46,213,108,107,155,231,15,111,247,23,161,195,254,172,2,69,23,138,1,0,0,0,0,0,0,0,0,0,43,146,0,0,0,0,1,0,17,0,0,0,133,23,138,1,0,0,0,0,0,0,0,0,0,43,146,0,0,0,0,1,0,72,143,3,9,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,71,107,75,72,14,208,74,162,171,251,242,56,81,211,93,123,236,57,85,203,112,107,57,168,243,133,56,4,241,121,163,136,1,0,0,0,0,0,0,0,2,83,226,137,56,69,198,68,237,114,181,98,131,24,126,192,220,22,80,96,22,233,129,36,54,133,209,22,195,230,74,31,184,130,230,251,204,33,245,75,141,229,3,196,242,9,89,87,211,172,171,148,10,150,19,217,40,43,156,210,25,131,141,1,12,38,62,166,18,213,233,104,108,231,170,252,51,145,230,60,76,135,213,53,130,239,34,254,82,78,198,143,196,192,191,10,14,236,116,84,109,2,150,123,199,124,220,221,59,170,184,62,133,210,213,165,60,165,70,195,190,167,76,178,179,167,241,56,12,2,0,1,3,11,51,209,150,204,26,238,155,223,38,45,43,59,70,56,39,137,167,125,2,54,164,70,36,39,243,129,58,34,167,175,122,199,131,184,4,172,255,25,179,239,180,61,65,92,234,211,61,155,131,251,65,86,159,20,132,198,77,237,152,216,181,229,88,7,97,72,29,53,116,116,187,124,77,118,36,235,211,189,179,216,53,94,115,209,16,67,252,13,163,83,128,0,0,0,0,6,106,60,180,223,161,39,147,188,254,224,215,134,58,226,116,55,149,202,24,170,67,208,79,144,144,193,105,183,207,58,102,1,2,2,1,1,116,12,0,0,0,247,137,1,0,0,0,0,0,31,1,31,1,30,1,29,1,28,1,27,1,26,1,25,1,24,1,23,1,22,1,21,1,20,1,19,1,18,1,17,1,16,1,15,1,14,1,13,1,12,1,11,1,10,1,9,1,8,1,7,1,6,1,5,1,4,1,3,1,2,1,1,222,162,139,122,214,209,101,216,37,98,98,210,147,130,203,189,70,30,91,33,108,125,93,199,243,129,62,191,103,170,103,242,1,228,99,223,99,0,0,0,0,1,0,0,0,0,0,0,0,144,52,122,31,114,217,178,73,51,186,87,227,141,30,113,223,95,248,199,20,137,207,98,251,106,229,21,18,207,241,177,5,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,205,241,123,63,87,73,161,108,120,85,19,8,177,14,206,126,47,68,226,70,68,50,21,86,99,83,206,65,26,46,209,246,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,126,57,43,14,180,179,219,13,111,62,233,12,75,5,102,26,149,211,152,168,6,198,49,245,64,254,254,207,155,42,94,105,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,87,102,199,190,229,162,134,141,128,96,40,74,99,179,108,183,84,29,74,98,21,105,76,217,211,218,131,54,13,242,75,215,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,167,0,163,158,94,127,122,79,95,253,37,1,249,193,75,78,185,87,237,48,169,125,246,118,20,251,31,145,121,168,50,149,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,218,38,134,24,71,84,38,85,121,239,92,122,231,206,172,255,229,181,122,209,20,146,98,216,115,67,8,135,246,120,60,15,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,13,148,27,145,94,71,252,200,150,85,228,7,30,249,95,58,130,209,93,2,188,80,104,144,233,171,200,199,92,192,114,191,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,132,121,224,215,201,18,190,205,202,27,95,196,184,255,99,83,54,224,143,214,227,65,173,133,246,238,239,80,196,49,225,21,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,70,106,126,46,18,123,102,209,125,154,170,51,53,198,232,189,128,144,98,89,93,144,127,83,4,229,238,245,33,32,32,75,235,253,244,198,31,170,83,89,109,139,188,87,127,209,237,43,234,122,141,234,73,145,154,247,100,125,205,30,183,152,206,35,95,127,197,149,198,37,175,132,156,142,93,71,178,22,115,119,248,60,9,3,228,108,51,169,205,9,133,44,175,193,151,61,38,241,35,124,86,134,202,55,179,77,39,84,245,2,23,51,117,70,143,237,167,77,61,38]}}}],\"id\":1}");
    // debug!("this is a debug {}", res.to_string());
    serde_json::from_str::<GetShredResponse>(res.to_string().as_str())
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
    shred_tx: Sender<Vec<Option<Shred>>>,
) {
    loop {
        if let Ok(slot) = slot_update_rx.recv() {
            let shred_for_one = request_shreds(slot as usize, vec![0], endpoint.clone()).await;
            let shred_indices_for_slot = match shred_for_one {
                Ok(first_shred) => {
                    let first_shred = &first_shred.result[1].clone().unwrap(); // add some check later
                    let max_shreds_per_slot = match (
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
                    };
                    // } else {
                    //     println!("shred: {:?}", first_shred);
                    //     None
                    // };
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
            if let Some(shred_indices_for_slot) = shred_indices_for_slot {
                let shreds_for_slot =
                    request_shreds(slot as usize, shred_indices_for_slot, endpoint.clone()).await;
                println!("made 2nd req");
                if let Ok(shreds_for_slot) = shreds_for_slot {
                    println!("get shred for slot in 2nd req");
                    let shreds = shreds_for_slot
                        .result
                        .iter()
                        .map(|s| try_coerce_shred!(s))
                        .collect();
                    shred_tx.send(shreds).expect("shred tx send error");
                }
            }
        }
    }
}

pub async fn shred_verify_loop(shred_rx: Receiver<Vec<Option<Shred>>>) {
    loop {
        if let Ok(shreds) = shred_rx.recv() {
            shreds.par_iter().for_each(|sh| match sh {
                Some(sh) => debug!("id of shred{:?}", sh.id()),
                None => {}
            });
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
    pub result: Vec<Option<GetShredResult>>,
    pub id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetShredResult {
    #[serde(rename = "ShredData")]
    pub shred_data: Option<ShredData>,
    #[serde(rename = "ShredCode")]
    pub shred_code: Option<ShredCode>,
}
