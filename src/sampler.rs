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

use crate::{
    tinydancer::{GenericService, RpcEndpoint},
    ClientError,
};

pub struct SampleService {
    sample_index: Uniform<u64>,
    // peers: Vec<(Pubkey, SocketAddr)>,
    sampler_handle: tokio::task::JoinHandle<()>,
}
pub struct SampleServiceConfig {
    pub rpc_endpoint: RpcEndpoint,
}

#[async_trait]
impl GenericService<SampleServiceConfig> for SampleService {
    fn new(config: SampleServiceConfig) -> Self {
        let sampler_handle = tokio::spawn(async {
            //add algo here

            tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
        });
        let sample_index = Uniform::from(0..1000);
        Self {
            sampler_handle,
            sample_index,
        }
    }
    async fn join<ClientError>(self) -> std::result::Result<(), ClientError> {
        match self.sampler_handle.await {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("error: {}", e);
                ClientError::SampleServiceError(""))
            }
        }
    }
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
