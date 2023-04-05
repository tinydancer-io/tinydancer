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
    consensus::{ConsensusService, ConsensusServiceConfig},
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
use tokio::{runtime::Runtime, task::JoinError, try_join, sync::Mutex as TokioMutex,};
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
    pub consensus_mode: bool,
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
        let status_clone = status.clone();
        let client_status = Arc::new(Mutex::new(status_clone));
        let status_sampler = client_status.clone();

        let consensus_client_status = Arc::new(TokioMutex::new(status.clone()));
        let status_consensus = consensus_client_status.clone();

        let TinyDancerConfig {
            enable_ui_service,
            rpc_endpoint,
            sample_qty,
            tui_monitor,
            log_path,
            archive_config,
            consensus_mode,
        } = config.clone();
        std::env::set_var("RUST_LOG", "info");
        // tiny_logger::setup_file_with_default(&log_path, "RUST_LOG");

        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.set_error_if_exists(false);
        opts.create_missing_column_families(true);

        // setup db
        let db = rocksdb::DB::open_cf(&opts, archive_config.clone().archive_path, vec![SHRED_CF])
            .unwrap();
        let db = Arc::new(db);


        if consensus_mode {
            println!("Running in consensus_mode");

            let consensus_service_config = ConsensusServiceConfig {
                cluster: rpc_endpoint.clone(),
                archive_config,
                instance: db.clone(),
                status_consensus: status_consensus.clone(),
                sample_qty,
            };

            let consensus_service = ConsensusService::new(consensus_service_config);
                    
            // run the sampling service
            consensus_service
            .join()
            .await
            .expect("error in consensus service thread");
        }

        else{

            let sample_service_config = SampleServiceConfig {
                cluster: rpc_endpoint.clone(),
                archive_config,
                instance: db.clone(),
                status_sampler,
                sample_qty,
            };

            let sample_service = SampleService::new(sample_service_config);
                    
            // run the sampling service
            sample_service
            .join()
            .await
            .expect("error in sample service thread");
        }

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
#[derive(Clone, PartialEq, Debug)]
pub enum ClientStatus {
    Initializing(String),
    SearchingForRPCService(String),
    Active(String),
    Crashed(String),
    ShuttingDown(String),
}
