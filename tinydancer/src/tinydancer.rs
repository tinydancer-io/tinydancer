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
use anyhow::anyhow;
use async_trait::async_trait;
use futures::{future::join_all, TryFutureExt};
use rand::seq::index::sample;
use tiny_logger::logs::info;
// use log::info;
// use log4rs;
use std::error::Error;
use tokio::{runtime::Runtime, task::JoinError, try_join};
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
    pub sample_qty: usize,
    pub enable_ui_service: bool,
    pub archive_config: ArchiveConfig,
    pub tui_monitor: bool,
    pub log_path: String,
}

use solana_metrics::datapoint_info;
use std::ffi::OsString;
use std::fs::read_dir;
use std::io;
use std::io::ErrorKind;
use std::path::PathBuf;

impl TinyDancer {
    pub async fn start(config: TinyDancerConfig) -> Result<()> {
        let status = ClientStatus::Initializing(String::from("Starting Up Tinydancer"));

        let client_status = Arc::new(Mutex::new(status));
        let status_sampler = client_status.clone();

        let TinyDancerConfig {
            enable_ui_service,
            rpc_endpoint,
            sample_qty,
            tui_monitor,
            log_path,
            archive_config,
        } = config.clone();
        std::env::set_var("RUST_LOG", "info");
        tiny_logger::setup_file_with_default(&log_path, "RUST_LOG");

        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.set_error_if_exists(false);
        opts.create_missing_column_families(true);

        // setup db
        let db = rocksdb::DB::open_cf(&opts, archive_config.clone().archive_path, vec![SHRED_CF])
            .unwrap();
        let db = Arc::new(db);

        let sample_service_config = SampleServiceConfig {
            cluster: rpc_endpoint.clone(),
            archive_config,
            instance: db.clone(),
            status_sampler,
            sample_qty,
        };
        let sample_service = SampleService::new(sample_service_config);

        let transaction_service = TransactionService::new(TransactionServiceConfig {
            cluster: rpc_endpoint.clone(),
            db_instance: db.clone(),
        });

        let ui_service = if enable_ui_service || tui_monitor {
            Some(UiService::new(UiConfig {
                client_status,
                enable_ui_service,
                tui_monitor,
            }))
        } else {
            None
        };

        // run
        sample_service
            .join()
            .await
            .expect("error in sample service thread");

        transaction_service
            .join()
            .await
            .expect("ERROR IN SIMPLE PAYMENT SERVICE");

        if let Some(ui_service) = ui_service {
            block_on!(async { ui_service.join().await }, "Ui Service Error");
        }

        Ok(())
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
    let cluster = cluster;
    match cluster {
        Cluster::Mainnet => String::from("https://api.mainnet-beta.solana.com"),
        Cluster::Devnet => String::from("https://api.devnet.solana.com"),
        Cluster::Localnet => String::from("http://0.0.0.0:8899"),
        Cluster::Custom(cluster) => cluster,
    }
}
pub enum ClientStatus {
    Initializing(String),
    SearchingForRPCService(String),
    Active(String),
    Crashed(String),
    ShuttingDown(String),
}
