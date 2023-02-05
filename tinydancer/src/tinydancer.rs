//! Sampler struct - incharge of sampling shreds
// use rayon::prelude::*;

use std::thread::Result;

// use tokio::time::Duration;
use crate::{
    block_on,
    sampler::{SampleService, SampleServiceConfig},
    ui::{UiConfig, UiService},
};
use async_trait::async_trait;
// use log::info;
// use log4rs;
use std::error::Error;
use tokio::{runtime::Runtime, task::JoinError};
// use std::{thread, thread::JoinHandle, time::Duration};

#[async_trait]
pub trait ClientService<T> {
    type ServiceError: std::error::Error;

    fn new(config: T) -> Self;
    async fn join(self) -> std::result::Result<(), Self::ServiceError>;
}
pub struct TinyDancer {
    sample_service: SampleService,
    ui_service: Option<UiService>,
    sample_qty: u64,
    config: TinyDancerConfig,
}
#[derive(Clone)]
pub struct TinyDancerConfig {
    pub rpc_endpoint: Cluster,
    pub sample_qty: u64,
    pub enable_ui_service: bool,
}
impl TinyDancer {
    pub async fn new(config: TinyDancerConfig) -> Self {
        tiny_logger::setup_file_with_default("../log/client.log", "error");
        let TinyDancerConfig {
            enable_ui_service,
            rpc_endpoint,
            sample_qty,
        } = config.clone();
        let sample_service_config = SampleServiceConfig {
            cluster: rpc_endpoint.clone(),
        };
        let sample_service = SampleService::new(sample_service_config);
        let ui_service = if enable_ui_service {
            Some(UiService::new(UiConfig {}))
        } else {
            None
        };
        Self {
            config,
            ui_service,
            sample_qty,
            sample_service,
        }
    }
    pub async fn join(self) {
        self.sample_service
            .join()
            .await
            .expect("error in sample service thread");
        if let Some(ui_service) = self.ui_service {
            block_on!(async { ui_service.join().await }, "Ui Service Error");
        }
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
