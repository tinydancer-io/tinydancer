use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::bail;
use dashmap::DashMap;
use tiny_logger::logs::{info, warn};

use solana_transaction_status::TransactionStatus;
use tokio::{
    sync::Semaphore,
    sync::{mpsc::UnboundedReceiver, OwnedSemaphorePermit},
    task::JoinHandle,
};

use crate::rpc_wrapper::tpu_manager::TpuManager;

pub type WireTransaction = Vec<u8>;
const NUMBER_OF_TX_SENDERS: usize = 5;

/// Retry transactions to a maximum of `u16` times, keep a track of confirmed transactions
#[derive(Clone)]
pub struct TxSender {
    /// Tx(s) forwarded to tpu
    pub txs_sent_store: Arc<DashMap<String, TxProps>>,
    /// TpuClient to call the tpu port
    pub tpu_manager: Arc<TpuManager>,
}

/// Transaction Properties
pub struct TxProps {
    pub status: Option<TransactionStatus>,
    /// Time at which transaction was forwarded
    pub sent_at: Instant,
}

impl Default for TxProps {
    fn default() -> Self {
        Self {
            status: Default::default(),
            sent_at: Instant::now(),
        }
    }
}

impl TxSender {
    pub fn new(tpu_manager: Arc<TpuManager>) -> Self {
        Self {
            tpu_manager,
            txs_sent_store: Default::default(),
        }
    }

    /// retry enqued_tx(s)
    async fn forward_txs(
        &self,
        sigs_and_slots: Vec<(String, u64)>,
        txs: Vec<WireTransaction>,
        permit: OwnedSemaphorePermit,
    ) {
        assert_eq!(sigs_and_slots.len(), txs.len());

        if sigs_and_slots.is_empty() {
            return;
        }

        let start = Instant::now();

        let tpu_client = self.tpu_manager.clone();
        let txs_sent = self.txs_sent_store.clone();

        for (sig, _) in &sigs_and_slots {
            txs_sent.insert(sig.to_owned(), TxProps::default());
        }

        let _quic_response = match tpu_client.try_send_wire_transaction_batch(txs).await {
            Ok(_) => 1,
            Err(err) => {
                warn!("{err}");
                0
            }
        };
        drop(permit);

        info!(
            "It took {} ms to send a batch of {} transaction(s)",
            start.elapsed().as_millis(),
            sigs_and_slots.len()
        );
    }

    /// retry and confirm transactions every 2ms (avg time to confirm tx)
    pub fn execute(
        self,
        mut recv: UnboundedReceiver<(String, WireTransaction, u64)>,
        tx_batch_size: usize,
        tx_send_interval: Duration,
    ) -> JoinHandle<anyhow::Result<()>> {
        tokio::spawn(async move {
            info!(
                "Batching tx(s) with batch size of {tx_batch_size} every {}ms",
                tx_send_interval.as_millis()
            );
            let semaphore = Arc::new(Semaphore::new(NUMBER_OF_TX_SENDERS));
            loop {
                let mut sigs_and_slots = Vec::with_capacity(tx_batch_size);
                let mut txs = Vec::with_capacity(tx_batch_size);
                let mut permit = None;

                while txs.len() <= tx_batch_size {
                    match tokio::time::timeout(tx_send_interval, recv.recv()).await {
                        Ok(value) => match value {
                            Some((sig, tx, slot)) => {
                                sigs_and_slots.push((sig, slot));
                                txs.push(tx);
                            }
                            None => {
                                bail!("Channel Disconnected");
                            }
                        },
                        Err(_) => {
                            permit = semaphore.clone().try_acquire_owned().ok();
                            if permit.is_some() {
                                // we have a permit we can send collected transaction batch
                                break;
                            }
                        }
                    }
                }
                assert_eq!(sigs_and_slots.len(), txs.len());

                if sigs_and_slots.is_empty() {
                    continue;
                }

                let permit = match permit {
                    Some(permit) => permit,
                    None => {
                        // get the permit
                        semaphore.clone().acquire_owned().await.unwrap()
                    }
                };

                if !txs.is_empty() {
                    let tx_sender = self.clone();
                    tokio::spawn(async move {
                        tx_sender.forward_txs(sigs_and_slots, txs, permit).await;
                    });
                }
            }
        })
    }
}
