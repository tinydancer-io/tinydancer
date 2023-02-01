//! Sampler struct - incharge of sampling shreds
// use rayon::prelude::*;

use std::thread::Result;

// use tokio::time::Duration;
use crate::sampler::{SampleService, SampleServiceConfig};
use async_trait::async_trait;
use std::error::Error;
use tokio::task::JoinError;
// use std::{thread, thread::JoinHandle, time::Duration};
pub struct TinyDancer {
    rpc_endpoint: RpcEndpoint,
    sampler: SampleService,
}

#[async_trait]
pub trait GenericService<T> {
    fn new(config: T) -> Self;
    async fn join<E: Error>(self) -> std::result::Result<(), E>;
}
impl TinyDancer {
    pub async fn new(rpc_endpoint: RpcEndpoint) -> Self {
        let config = SampleServiceConfig { rpc_endpoint };
        let sampler = SampleService::new(config);

        Self {
            rpc_endpoint,
            sampler,
        }
    }
    pub async fn join(self) {
        self.sampler.join().await;
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum RpcEndpoint {
    Mainnet,
    Devnet,
    Localnet,
    Custom,
}
