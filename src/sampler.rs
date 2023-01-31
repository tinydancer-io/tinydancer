#![feature(async_closure)]

use rand::distributions::Uniform;
use rand::prelude::*;
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
use std::{
    net::{SocketAddr, UdpSocket},
    thread::{Builder, JoinHandle},
};
use tokio::task::JoinError;

use solana_ledger::{
    ancestor_iterator::{AncestorIterator, AncestorIteratorWithHash},
    blockstore::Blockstore,
    shred::{Nonce, Shred, ShredFetchStats, SIZE_OF_NONCE},
};

pub struct SampleService {
    // sample_index: Uniform<u64>,
    // peers: Vec<(Pubkey, SocketAddr)>,
    sampler_handle: tokio::task::JoinHandle<()>,
}

impl SampleService {
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
    pub fn new() -> Self {
        let sampler_handle = tokio::spawn(async {
            //add algo here

            tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
        });
        Self { sampler_handle }
    }
    pub async fn join(self) -> core::result::Result<(), JoinError> {
        self.sampler_handle.await
    }
}
