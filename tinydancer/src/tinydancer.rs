//! Sampler struct - incharge of sampling shreds
// use rayon::prelude::*;

use std::{
    env,
    sync::{Arc, Mutex, MutexGuard},
    thread::Result,
};

// use tokio::time::Duration;
use crate::{
    block_on,
    rpc_wrapper::{TransactionService, TransactionServiceConfig},
    sampler::{ArchiveConfig, SampleService, SampleServiceConfig, SHRED_CF},
    ui::{UiConfig, UiService},
};
use async_trait::async_trait;
use tiny_logger::logs::info;
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
    transaction_service: TransactionService,
}
#[derive(Clone)]
pub struct TinyDancerConfig {
    pub rpc_endpoint: Cluster,
    pub sample_qty: u64,
    pub enable_ui_service: bool,
    pub archive_config: Option<ArchiveConfig>,
}

use solana_metrics::datapoint_info;
use std::ffi::OsString;
use std::fs::read_dir;
use std::io;
use std::io::ErrorKind;
use std::path::PathBuf;
// use tiny_logger::logs::info;
pub fn get_project_root() -> io::Result<PathBuf> {
    let path = env::current_dir()?;
    let mut path_ancestors = path.as_path().ancestors();

    while let Some(p) = path_ancestors.next() {
        let has_cargo = read_dir(p)?
            .into_iter()
            .any(|p| p.unwrap().file_name() == OsString::from("Cargo.lock"));
        if has_cargo {
            let mut path = PathBuf::from(p);
            // path.push("log");
            path.push("client.log");
            return Ok(path);
        }
    }
    Err(io::Error::new(
        ErrorKind::NotFound,
        "Ran out of places to find Cargo.toml",
    ))
}
impl TinyDancer {
    pub async fn new(config: TinyDancerConfig) -> Self {
        let path = get_project_root().unwrap();
        let status = ClientStatus::Initializing(String::from("Starting Up Tinydancer"));
        let client_status = Arc::new(Mutex::new(status));
        let status_sampler = Arc::clone(&client_status);
        // datapoint_info!("log", ("test", "testvalue", String));
        info!("{:?}", path);
        tiny_logger::setup_file_with_default(path.to_str().unwrap(), "RUST_LOG");
        let TinyDancerConfig {
            enable_ui_service,
            rpc_endpoint,
            sample_qty,
            archive_config,
        } = config.clone();
        let rpc_cluster = rpc_endpoint.clone();

        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.set_error_if_exists(false);
        opts.create_missing_column_families(true);
        let instance = rocksdb::DB::open_cf(
            &opts,
            archive_config.clone().unwrap().archive_path,
            vec![SHRED_CF],
        )
        .unwrap();
        let instance = Arc::new(instance);
        let sample_instance = Arc::clone(&instance);

        let rpc_instance = Arc::clone(&instance);
        let sample_service_config = SampleServiceConfig {
            cluster: rpc_endpoint,
            archive_config,
            instance: sample_instance,
            status_sampler,
        };
        let sample_service = SampleService::new(sample_service_config);
        let transaction_service = TransactionService::new(TransactionServiceConfig {
            cluster: rpc_cluster,
            db_instance: rpc_instance,
        });
        let ui_service = if enable_ui_service {
            Some(UiService::new(UiConfig { client_status }))
        } else {
            None
        };
        Self {
            config,
            ui_service,
            sample_qty,
            sample_service,
            transaction_service,
        }
    }
    pub async fn join(self) {
        self.sample_service
            .join()
            .await
            .expect("error in sample service thread");
        self.transaction_service
            .join()
            .await
            .expect("ERROR IN SIMPLE PAYMENT SERVICE");
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
pub enum ClientStatus {
    Initializing(String),
    SearchingForRPCService(String),
    Active(String),
    Crashed(String),
    ShuttingDown(String),
}
