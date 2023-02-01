//! Sampler struct - incharge of sampling shreds
// use rayon::prelude::*;

use std::thread::Result;

// use tokio::time::Duration;
use crate::sampler::{SampleService, SampleServiceConfig};
use async_trait::async_trait;
use std::error::Error;
use tokio::task::JoinError;
// use std::{thread, thread::JoinHandle, time::Duration};

#[async_trait]
pub trait ClientService<T> {
    type ServiceError: std::error::Error;
    fn new(config: T) -> Self;
    async fn join(self) -> std::result::Result<(), Self::ServiceError>;
}
pub struct TinyDancer {
    rpc_endpoint: Cluster,
    sampler: SampleService,
    sample_qty: u64,
}
pub struct TinyDancerConfig {
    pub rpc_endpoint: Cluster,
    pub sample_qty: u64,
}
impl TinyDancer {
    pub async fn new(config: TinyDancerConfig) -> Self {
        let TinyDancerConfig {
            rpc_endpoint,
            sample_qty,
        } = config;
        let config = SampleServiceConfig {
            cluster: rpc_endpoint.clone(),
        };
        let sampler = SampleService::new(config);

        Self {
            sample_qty,
            rpc_endpoint,
            sampler,
        }
    }
    pub async fn join(self) {
        self.sampler
            .join()
            .await
            .expect("error in sample service thread");
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub enum Cluster {
    Mainnet,
    Devnet,
    Localnet,
    Custom(String),
}
pub fn endpoint(cluster: Cluster) -> String {
    let cluster = cluster.clone();
    match cluster {
        Cluster::Mainnet => String::from("https://api.mainnet-beta.solana.com"),
        Cluster::Devnet => String::from("https://api.devnet.solana.com"),
        Cluster::Localnet => String::from("http://0.0.0.0:8899"),
        Cluster::Custom(cluster) => cluster.to_string(),
    }
}
