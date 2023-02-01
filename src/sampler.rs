use async_trait::async_trait;
use rand::distributions::Uniform;
use rand::prelude::*;
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
    thread::{Builder, JoinHandle},
};
use tokio::task::JoinError;

use crate::tinydancer::{ClientService, Cluster};

pub struct SampleService {
    sample_indices: Vec<u64>,
    // peers: Vec<(Pubkey, SocketAddr)>,
    sampler_handle: tokio::task::JoinHandle<()>,
}
pub struct SampleServiceConfig {
    pub rpc_endpoint: Cluster,
}

#[async_trait]
impl ClientService<SampleServiceConfig> for SampleService {
    type ServiceError = tokio::task::JoinError;
    fn new(config: SampleServiceConfig) -> Self {
        let sampler_handle = tokio::spawn(async {
            //add algo here

            for _ in 0..10 {
                println!("running");
                tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
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
