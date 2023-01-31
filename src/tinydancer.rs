//! Sampler struct - incharge of sampling shreds
// use rayon::prelude::*;

// use tokio::time::Duration;
use std::{thread, thread::JoinHandle, time::Duration};

use crate::sampler::SampleService;
pub struct TinyDancer {
    rpc_endpoint: RpcEndpoint,
    sampler: SampleService,
}

impl TinyDancer {
    pub async fn new(rpc_endpoint: RpcEndpoint) -> Self {
        let sampler = SampleService::new();

        Self {
            rpc_endpoint,
            sampler,
        }
    }
    pub async fn join(self) {
        self.sampler.join().await.expect("sample service error");
    }
}

pub enum RpcEndpoint {
    Mainnet,
    Devnet,
    Localnet,
    Custom(String),
}
