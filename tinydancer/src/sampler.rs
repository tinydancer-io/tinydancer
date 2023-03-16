use crate::tinydancer::{endpoint, ClientService, ClientStatus, Cluster};
use crate::{convert_to_websocket, send_rpc_call, try_coerce_shred};
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
};
use tungstenite::{connect, Message};
use url::Url;
use anyhow::anyhow;

pub const SHRED_CF: &str = "archived_shreds";

pub struct SampleService {
    sample_indices: Vec<u64>,
    // peers: Vec<(Pubkey, SocketAddr)>,
    sampler_handle: JoinHandle<()>,
}
pub struct SampleServiceConfig {
    pub cluster: Cluster,
    pub archive_config: Option<ArchiveConfig>,
    pub instance: Arc<rocksdb::DB>,
    pub status_sampler: Arc<Mutex<ClientStatus>>,
}

#[derive(Clone, Debug)]
pub struct ArchiveConfig {
    pub shred_archive_duration: u64,
    pub archive_path: String,
}

#[async_trait]
impl ClientService<SampleServiceConfig> for SampleService {
    type ServiceError = tokio::task::JoinError;

    fn new(config: SampleServiceConfig) -> Self {
        let sampler_handle = tokio::spawn(async move {
            let rpc_url = endpoint(config.cluster);
            let pub_sub = convert_to_websocket!(rpc_url);

            let mut threads = Vec::default();

            let (slot_update_tx, slot_update_rx) = crossbeam::channel::unbounded::<u64>();
            let (shred_tx, shred_rx) = crossbeam::channel::unbounded();
            let (verified_shred_tx, verified_shred_rx) = crossbeam::channel::unbounded();

            let status_arc = config.status_sampler.clone();

            // waits on new slots => triggers shred_update_loop
            threads.push(tokio::spawn(slot_update_loop(
                slot_update_tx,
                pub_sub,
                config.status_sampler,
            )));

            // sample shreds from new slot
            // verify each shred in shred_verify_loop
            threads.push(tokio::spawn(shred_update_loop(
                slot_update_rx,
                rpc_url,
                shred_tx,
                status_arc,
            )));

            // verify shreds + store in db in shred_archiver
            threads.push(tokio::spawn(shred_verify_loop(shred_rx, verified_shred_tx)));

            if let Some(archive_config) = config.archive_config {
                threads.push(tokio::spawn(shred_archiver(
                    verified_shred_rx,
                    archive_config,
                    config.instance,
                )));
            }

            for thread in threads {
                thread.await.unwrap().unwrap();
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
    let vec = (0..sample_qty)
        .map(|_| rng.gen_range(0..max_shreds_per_slot))
        .collect::<Vec<usize>>();
    vec
}

pub async fn request_shreds(
    slot: usize,
    indices: Vec<usize>,
    endpoint: String,
) -> Result<GetShredResponse, serde_json::Error> {
    let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getShreds",
            "params":[
                slot,
                indices,
                { "commitment": "confirmed" }
            ]
        }) // getting one shred just to get max shreds per slot, can maybe randomize the selection here
        .to_string();

    let res = send_rpc_call!(endpoint, request);
    // info!("{:?}", res);
    serde_json::from_str::<GetShredResponse>(&res)
}

async fn slot_update_loop(
    slot_update_tx: Sender<u64>,
    pub_sub: String,
    status_sampler: Arc<Mutex<ClientStatus>>,
) -> anyhow::Result<()> {
    let (mut socket, _response) = match connect(Url::parse(pub_sub.as_str()).unwrap()) {
        Ok((socket, _response)) => Some((socket, _response)),
        Err(_) => {
            let mut status = status_sampler.lock().unwrap();
            *status = ClientStatus::Crashed(String::from("Client can't connect to socket"));
            None
        }
    }.unwrap(); 

    socket.write_message(Message::Text(
        r#"{ "jsonrpc": "2.0", "id": 1, "method": "slotSubscribe" }"#.into(),
    ))?;

    loop {
        match socket.read_message() {
            Ok(msg) => {
                let res =
                    serde_json::from_str::<SlotSubscribeResponse>(msg.to_string().as_str());

                // info!("res: {:?}", msg.to_string().as_str());
                if let Ok(res) = res {
                    match slot_update_tx.send(res.params.result.root as u64) {
                        Ok(_) => {
                            info!("slot updated: {:?}", res.params.result.root);
                        }
                        Err(e) => {
                            info!(
                                "error here: {:?} {:?}",
                                e, res.params.result.root as u64
                            );
                            continue; // @TODO: we should add retries here incase send fails for some reason
                        }
                    }
                }
            }
            Err(e) => info!("err: {:?}", e),
        }
    }
}

macro_rules! unwrap_or_return {
    (Result $var:ident) => {
        if let Err(e) = $var { 
            return Err(e.into());
        } else { 
            $var.unwrap()
        }
    };
    (Option $var:ident $err:expr) => {
        if $var.is_none() { 
            return Err(anyhow!($err));
        } else { 
            $var.unwrap()
        }
    };
    (OptionRef $var:ident $err:expr) => {
        if $var.is_none() { 
            return Err(anyhow!($err));
        } else { 
            $var.as_ref().unwrap()
        }
    };
}

async fn get_shreds_and_leader_for_slot(
    slot: u64,
    endpoint: &String
) -> anyhow::Result<(Vec<Option<Shred>>, Pubkey)> { 

    // get shred length (max_shreds_per_slot)
    let first_shred = request_shreds(slot as usize, vec![0], endpoint.clone()).await;
    let first_shred = unwrap_or_return!(Result first_shred);

    let first_shred = &first_shred.result.shreds[1]; 
    let first_shred = unwrap_or_return!(OptionRef first_shred "first shred not found");

    let max_shreds_per_slot = {
        if let Some(data_shred) = &first_shred.shred_data { 
            Shred::ShredData(data_shred.clone())
                .num_data_shreds()
                .expect("num data shreds error")
        } else if let Some(code_shred) = &first_shred.shred_code { 
            Shred::ShredCode(code_shred.clone())
                    .num_coding_shreds()
                    .expect("num code shreds error")
        } else {
            // todo
            return Err(anyhow!("shred isnt either data or code type"));
        }
    };

    // get a random sample of shreds
    let mut shred_indices_for_slot = gen_random_indices(max_shreds_per_slot as usize, 10); // unwrap only temporary
    shred_indices_for_slot.push(0_usize);
    info!("indices of: {:?} {:?}", shred_indices_for_slot, slot);

    let shreds_for_slot = request_shreds(
        slot as usize,
        shred_indices_for_slot.clone(),
        endpoint.clone(),
    )
    .await;
    let shreds_for_slot = unwrap_or_return!(Result shreds_for_slot);

    info!("get shred for slot in 2nd req");
    let mut shreds: Vec<Option<Shred>> = shreds_for_slot
        .result
        .shreds
        .par_iter()
        .map(|s| try_coerce_shred!(s))
        .collect();

    // info!("before leader");
    let leader = solana_ledger::shred::Pubkey::from_str(
        shreds_for_slot.result.leader.as_str(),
    )?;

    // info!("leader {:?}", leader);
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

    Ok((shreds, leader))
}

async fn shred_update_loop(
    slot_update_rx: Receiver<u64>,
    endpoint: String,
    shred_tx: Sender<(Vec<Option<Shred>>, solana_ledger::shred::Pubkey)>,
    status_sampler: Arc<Mutex<ClientStatus>>,
) -> anyhow::Result<()> {
    loop {
        {
            let mut status = status_sampler.lock().unwrap();

            if let ClientStatus::Crashed(_) = &*status { } else {
                *status = ClientStatus::Active(String::from(
                    "Monitoring Tinydancer: Actively Sampling Shreds",
                ));
            }
        }

        if let Ok(slot) = slot_update_rx.recv() {
            let shreds = get_shreds_and_leader_for_slot(slot, &endpoint).await;
            if let Err(e) = shreds { 
                info!("{}", e);
                continue;
            }
            let (shreds, leader) = shreds.unwrap();

            shred_tx
                .send((shreds, leader))
                .expect("shred tx send error");
        }
    }
}

// use solana_ledger::shred::dispatch;

// verifies the merkle proof of the shread 
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
    .all(|s| *s);
    verified
}

pub async fn shred_verify_loop(
    shred_rx: Receiver<(Vec<Option<Shred>>, solana_ledger::shred::Pubkey)>,
    verified_shred_tx: Sender<(Shred, solana_ledger::shred::Pubkey)>,
) -> anyhow::Result<()> {
    loop {
        if let Ok((shreds, leader)) = shred_rx.recv() {
            shreds.par_iter().for_each(|sh| match sh {
                Some(shred) => {
                    let verified = verify_sample(shred, leader);
                    match verified {
                        true => {
                            info!(
                                "sample {:?} verified for slot: {:?}",
                                shred.index(),
                                shred.slot()
                            );
                            match verified_shred_tx.send((shred.clone(), leader)) {
                                Ok(_) => {}
                                Err(e) => error!("Error verified_shred_tx: {}", e),
                            }
                        }
                        false => info!("sample INVALID for slot : {:?}", shred.slot()),
                    }
                }
                None => {
                    // info!("none")
                }
            });
        } else {
            // info!("None")
        }
    }
}


// store verified shreds in db
pub async fn shred_archiver(
    verified_shred_rx: Receiver<(Shred, solana_ledger::shred::Pubkey)>,
    _archive_config: ArchiveConfig,
    instance: Arc<rocksdb::DB>,
) -> anyhow::Result<()> {
    loop {
        if let Ok((verified_shred, leader)) = verified_shred_rx.recv() {
            let mut opts = RocksOptions::default();
            opts.create_if_missing(true);
            opts.set_error_if_exists(false);
            opts.create_missing_column_families(true);

            let key = hashv(&[
                &verified_shred.slot().to_le_bytes(),
                &u8::from(verified_shred.shred_type()).to_le_bytes(),
                &verified_shred.index().to_le_bytes(),
            ])
            .to_bytes();
            // info!("archiver {:?}", verified_shred.slot(),);
            // let cfs =
            //     rocksdb::DB::list_cf(&opts, archive_config.archive_path.clone()).unwrap_or(vec![]);
            // let shred_cf = cfs.clone().into_iter().find(|cf| cf.as_str() == SHRED_CF);
            // let instance =
            //     DB::open_cf(&opts, archive_config.archive_path.clone(), vec![SHRED_CF]).unwrap();
            // match shred_cf {
            //     Some(cf_name) => {
            let cf = instance.cf_handle(SHRED_CF).unwrap();
            let put_response = put_serialized(&instance, cf, key, &verified_shred);
            match put_response {
                Ok(_) => info!("Saved Shred {:?} to db", verified_shred.id().seed(&leader)),
                Err(e) => info!("{:?}", e),
            }
            //     }
            //     None => instance
            //         .create_cf(SHRED_CF, &RocksOptions::default())
            //         .unwrap(),
            // }
        }
    }
}


pub async fn pull_and_verify_shreds(slot: usize, endpoint: String) -> bool {
    let shreds = get_shreds_and_leader_for_slot(slot as u64, &endpoint).await;
    if let Err(e) = shreds { 
        info!("{}", e);
        return false;
    }
    let (shreds, leader) = shreds.unwrap();

    let sampled = shreds
        .par_iter()
        .flatten()
        .all(|s| verify_sample(s, leader));

    info!("pull and verify {:?}", sampled);
    sampled
}

pub fn put_serialized<T: serde::Serialize + std::fmt::Debug>(
    instance: &rocksdb::DB,
    cf: &ColumnFamily,
    key: [u8; 32],
    value: &T,
) -> Result<(), String> {
    match serde_json::to_string(&value) {
        Ok(serialized) => instance
            .put_cf(cf, key, serialized.into_bytes())
            .map_err(|err| format!("Failed to put to ColumnFamily:{:?}", err)),
        Err(err) => Err(format!(
            "Failed to serialize to String. T: {:?}, err: {:?}",
            value, err
        )),
    }
}
pub fn get_serialized<T: DeserializeOwned>(
    instance: &rocksdb::DB,
    cf: &ColumnFamily,
    key: [u8; 32],
) -> Result<Option<T>, String> {
    match instance.get_cf(cf, key) {
        Ok(opt) => match opt {
            Some(found) => match String::from_utf8(found) {
                Ok(s) => match serde_json::from_str::<T>(&s) {
                    Ok(t) => Ok(Some(t)),
                    Err(err) => Err(format!("Failed to deserialize: {:?}", err)),
                },
                Err(err) => Err(format!("Failed to convert to String: {:?}", err)),
            },
            None => Ok(None),
        },
        Err(err) => Err(format!("Failed to get from ColumnFamily: {:?}", err)),
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

#[cfg(test)]
mod tests {
    use super::{get_serialized, SHRED_CF};
    use rocksdb::{Options as RocksOptions, DB};
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_ledger::shred::{hashv, Shred, ShredType, Signer};
    use solana_sdk::signer::keypair::Keypair;
    use tiny_logger::logs::info;

    #[test]
    fn get_shred_from_db() {
        let mut opts = RocksOptions::default();
        opts.create_if_missing(true);
        opts.set_error_if_exists(false);
        opts.create_missing_column_families(true);
        let instance = DB::open_cf(&opts, "/tmp", vec![SHRED_CF]).unwrap();
        let slot: u64 = 1963754;
        let _index: u32 = 11;
        let key = hashv(&[
            &slot.to_le_bytes(),
            &u8::from(ShredType::Data).to_le_bytes(),
            &0_u32.to_le_bytes(), // can be random
        ])
        .to_bytes();
        let cf = instance.cf_handle(SHRED_CF).unwrap();
        let shred = get_serialized::<Shred>(&instance, cf, key);
        println!("shred {:?}", shred);
        assert!(
            shred.is_ok(),
            "error retrieving and serializing shred from db"
        );
    }

    #[tokio::test]
    async fn call_lite_rpc() {
        let rpc_client = RpcClient::new("http://0.0.0.0:8890".to_string());

        //let identity = get_identity_keypair(&identity_keypair).await;
        // let blockhash = rpc_client.get_latest_blockhash().await.unwrap();
        let payer = Keypair::new();
        let airdrop_sign = rpc_client
            .request_airdrop(&payer.try_pubkey().unwrap(), 2000000000)
            .await
            .unwrap();
        println!("AIRDROP CONFIRMED:{}", airdrop_sign);
    }
}
