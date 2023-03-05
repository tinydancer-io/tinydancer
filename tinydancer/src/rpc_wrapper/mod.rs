pub mod rpc;
pub mod config;
//pub use config;
pub mod tpu_manager;
pub mod encoding;
pub mod bridge;
pub mod workers;
pub mod cli;
pub mod block_store;
use crate::rpc_wrapper::bridge::LiteBridge;
use std::{time::Duration, env};
use anyhow::bail;
use log::info;
use solana_sdk::signer::keypair::Keypair;
use async_trait::async_trait;
use clap::Parser;
use const_env::from_env;
use solana_transaction_status::TransactionConfirmationStatus;
use tokio::task::JoinHandle;
use dotenv::dotenv;
use crate::tinydancer::{Cluster, ClientService};

use self::cli::Args;

#[from_env]
pub const DEFAULT_RPC_ADDR: &str = "http://0.0.0.0:8899";
#[from_env]
pub const DEFAULT_LITE_RPC_ADDR: &str = "http://0.0.0.0:8890";
#[from_env]
pub const DEFAULT_WS_ADDR: &str = "ws://0.0.0.0:8900";
#[from_env]
pub const DEFAULT_TX_MAX_RETRIES: u16 = 1;
#[from_env]
pub const DEFAULT_TX_BATCH_SIZE: usize = 128;
#[from_env]
pub const DEFAULT_FANOUT_SIZE: u64 = 32;
#[from_env]
pub const DEFAULT_TX_BATCH_INTERVAL_MS: u64 = 1;
#[from_env]
pub const DEFAULT_CLEAN_INTERVAL_MS: u64 = 5 * 60 * 1000; // five minute
#[from_env]
pub const DEFAULT_TX_SENT_TTL_S: u64 = 12;
pub const DEFAULT_TRANSACTION_CONFIRMATION_STATUS: TransactionConfirmationStatus =
    TransactionConfirmationStatus::Finalized;

pub struct TransactionService{
    tx_handle: JoinHandle<Result<(), anyhow::Error>>,
}

pub struct TransactionServiceConfig {
    pub cluster: Cluster,
}


async fn get_identity_keypair(identity_from_cli: &String) -> Keypair {
    if let Some(identity_env_var) = env::var("IDENTITY").ok() {
        if let Ok(identity_bytes) = serde_json::from_str::<Vec<u8>>(identity_env_var.as_str()) {
            print!("HASIII TO HASU");
            Keypair::from_bytes(identity_bytes.as_slice()).unwrap()
        } else {
            // must be a file
            let identity_file = tokio::fs::read_to_string(identity_env_var.as_str())
                .await
                .expect("Cannot find the identity file provided");
            let identity_bytes: Vec<u8> = serde_json::from_str(&identity_file).unwrap();
            Keypair::from_bytes(identity_bytes.as_slice()).unwrap()
        }
    } else if identity_from_cli.is_empty() {
        Keypair::new()
    } else {
        let identity_file = tokio::fs::read_to_string(identity_from_cli.as_str())
            .await
            .expect("Cannot find the identity file provided");
        let identity_bytes: Vec<u8> = serde_json::from_str(&identity_file).unwrap();
        Keypair::from_bytes(identity_bytes.as_slice()).unwrap()
    }
}

#[async_trait]
impl ClientService<TransactionServiceConfig> for TransactionService{
    type ServiceError = tokio::io::Error;
   fn new(config: TransactionServiceConfig) -> Self{
        let transaction_handle = tokio::spawn( async {
            let Args {
                rpc_addr,
                ws_addr,
                tx_batch_size,
                lite_rpc_ws_addr,
                lite_rpc_http_addr,
                tx_batch_interval_ms,
                clean_interval_ms,
                fanout_size,
                prometheus_addr,
                identity_keypair,
            } = Args::parse();

        dotenv().ok();

        let identity = get_identity_keypair(&identity_keypair).await;

        let tx_batch_interval_ms = Duration::from_millis(tx_batch_interval_ms);
        let clean_interval_ms = Duration::from_millis(clean_interval_ms);

        let light_bridge = LiteBridge::new(rpc_addr, ws_addr, fanout_size, identity).await?;

        let services = light_bridge
            .start_services(
                lite_rpc_http_addr,
                lite_rpc_ws_addr,
                tx_batch_size,
                tx_batch_interval_ms,
                clean_interval_ms,
                prometheus_addr,
            )
            .await?;

        let services = futures::future::try_join_all(services);

        let ctrl_c_signal = tokio::signal::ctrl_c();

        tokio::select! {
            _ = services => {
                bail!("Services quit unexpectedly");
            }
            _ = ctrl_c_signal => {
                info!("Received ctrl+c signal");
                Ok(())
            }
        }});
        Self {
            tx_handle: transaction_handle
        }
    }
   async fn join(self) -> std::result::Result<(), Self::ServiceError> {
        let _ = self.tx_handle.await;
        Ok(())
    }
}