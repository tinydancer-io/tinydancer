use async_trait::async_trait;
use crossbeam::channel::Receiver;
use solana_ledger::shred::Shred;
use crate::tinydancer::{endpoint, ClientService, Cluster};
use tokio::task::JoinHandle;
use std::collections::{HashMap, HashSet};
use std::ops::AddAssign;
use solana_sdk::{
    clock::Slot, epoch_schedule::EpochSchedule, hash::Hash, pubkey::Pubkey,
    signer::keypair::Keypair,
};
use rocksdb::{ColumnFamily, Options as RocksOptions, DB};
use solana_metrics::datapoint_info;
use crate::sampler::{request_shreds, verify_sample, put_serialized};
use serde::{ Serialize, Deserialize};
use serde::de::DeserializeOwned;
use solana_sdk::hash::hashv;
use tiny_logger::logs::info;
pub struct StatsService {
    stat_handle: JoinHandle<()>,
}

pub struct StatsServiceConfig{
    pub cluster: Cluster,
}

const SLOT_STATS: &'static str = "Slot_stats";
const SAMPLE_STATS: &'static str = "Sample_stats";
const VERIFIED_STATS: &'static str = "Verified_stats";

// #[async_trait]
// impl ClientService<StatsServiceConfig> for StatsService{
//     type ServiceError = tokio::task::JoinError;
//     fn new(config: StatsServiceConfig) -> Self{
    
//     }

//     async fn join(self) -> std::result::Result<(), Self::ServiceError> {
//         self.stat_handle.await
//     }


// }
//not being used atm as we're only connected to a single node
// #[derive(Clone)]
// pub struct SampleInfo{
//     request_counter: u64,
//     peer_light_nodes: Option<HashSet<Pubkey>>,
//     full_nodes: Option<HashSet<Pubkey>>,
// }



#[derive(Clone)]
pub struct StatDBConfig {
    pub archive_duration: u64,

    pub archive_path: String,
}
#[derive(Clone,Copy, Default, Debug,Serialize, Deserialize)]
pub struct SlotUpdateStats{
   // pub separator: String,
    pub slots: usize,
}
impl SlotUpdateStats{
    pub fn new(slot: usize) -> Self{
        SlotUpdateStats{           
             slots: slot,
        }
    }

}
#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub struct PerRequestSampleStats {
   pub slot: Slot,
   pub total_sampled: usize,
   pub num_data_shreds: usize,
   pub num_coding_shreds: usize,
   //pub num_requested_shreds: usize,
}
impl PerRequestSampleStats{
 fn update(
        &mut self,
        slot: Slot,
       // total_sampled: u64,
        num_data_shreds: usize,
        num_coding_shreds: usize,
        shreds: Vec<Option<Shred>>,
    ) {
        let total_samples = shreds.len() as usize;
        self.slot = slot;
        self.total_sampled = total_samples;

        self.num_data_shreds = num_data_shreds;
        self.num_coding_shreds = num_coding_shreds;
    }
    
}

#[derive(Default,Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PerRequestVerificationStats{
    pub slot: Slot,
    pub num_verified: usize,
    pub num_failed: usize,
}
impl AddAssign for PerRequestVerificationStats{
    fn add_assign(&mut self, rhs: Self) {
        self.num_verified += rhs.num_verified;
        self.num_failed += rhs.num_failed;
    }
}

impl PerRequestVerificationStats{
    fn update(
        &mut self,
        num_verified: usize,
        num_failed: usize,
    ){
        self.num_verified = num_verified;
        self.num_failed = num_failed;
    }
}


pub async fn store_stats(
    stat_config: StatDBConfig,
    slot_db_rx: Receiver<SlotUpdateStats>,
    sample_stats_rx: Receiver<PerRequestSampleStats>,
    verified_stats_rx: Receiver<PerRequestVerificationStats>, 
){
 loop{
    let (sx, rx, vx) = (slot_db_rx.recv(), sample_stats_rx.recv(), verified_stats_rx.recv());
    if sx.is_ok() && rx.is_ok() && vx.is_ok() {
        let mut s_vec = vec![];
        let mut r_vec = vec![];
        let mut v_vec = vec![];
        s_vec.push(sx.unwrap().slots);
        r_vec.push((
                rx.unwrap().slot as usize,
                rx.unwrap().total_sampled,
                rx.unwrap().num_data_shreds,
                rx.unwrap().num_coding_shreds,
        ));
        v_vec.push((
                    vx.unwrap().slot as usize,
                    vx.unwrap().num_verified,
                    vx.unwrap().num_failed,
        ));
        let mut opts = RocksOptions::default();
            opts.create_if_missing(true);
            opts.set_error_if_exists(false);
            opts.create_missing_column_families(true);
        let instance = DB::open_cf(&opts, stat_config.archive_path.clone(), vec![SLOT_STATS, SAMPLE_STATS, VERIFIED_STATS]).unwrap();
        let cf_one = instance.cf_handle(SLOT_STATS).unwrap();
        let cf_two = instance.cf_handle(SAMPLE_STATS).unwrap();
        let cf_three = instance.cf_handle(VERIFIED_STATS).unwrap();
        let key = hashv(&[&sx.unwrap().slots.to_le_bytes()]);

        let put_response_one = put_serialized(&instance, cf_one,key.to_bytes(), &s_vec);
        let put_response_two = put_serialized(&instance, cf_two, key.to_bytes(), &r_vec);
        let put_response_three = put_serialized(&instance, cf_three, key.to_bytes(), &v_vec);
        match (put_response_one, put_response_two, put_response_three){
            (Ok(_), Ok(_), Ok(_)) => info!("stored"),
            _ => info!("error in storage")
        }

    }
  }
}
