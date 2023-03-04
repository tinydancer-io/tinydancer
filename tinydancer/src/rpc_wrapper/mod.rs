pub mod rpc;
pub mod config;
//pub use config;
pub mod tpu_manager;
pub mod encoding;
pub mod bridge;
pub mod workers;
pub mod cli;
pub mod block_store;
use async_trait::async_trait;
use const_env::from_env;
use solana_transaction_status::TransactionConfirmationStatus;
use tokio::task::JoinHandle;

use crate::tinydancer::{Cluster, ClientService};

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
    tx_handle: JoinHandle<()>,
}

pub struct TransactionServiceConfig {
    pub cluster: Cluster,
}
#[async_trait]
impl ClientService<TransactionServiceConfig> for TransactionService{
    type ServiceError = tokio::io::Error;
   fn new(config: TransactionServiceConfig) -> Self{
        
    }
   async fn join(self) -> std::result::Result<(), Self::ServiceError> {
        self.tx_handle.await;
        Ok(())
    }
}