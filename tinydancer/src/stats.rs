use async_trait::async_trait;
use solana_ledger::shred::Shred;
use crate::tinydancer::{endpoint, ClientService, Cluster};
use tokio::task::JoinHandle;
use std::collections::{HashMap, HashSet};
use std::ops::AddAssign;
use solana_sdk::{
    clock::Slot, epoch_schedule::EpochSchedule, hash::Hash, pubkey::Pubkey,
    signer::keypair::Keypair,
};
use solana_metrics::datapoint_info;
use crate::sampler::{request_shreds, verify_sample};


pub struct StatsService {
    stat_handle: JoinHandle<()>,
}

pub struct StatsServiceConfig{
    pub cluster: Cluster,
}
#[async_trait]
impl ClientService<StatsServiceConfig> for StatsService{
    type ServiceError = tokio::task::JoinError;
    fn new(config: StatsServiceConfig) -> Self{
        todo!()
    }

    async fn join(self) -> std::result::Result<(), Self::ServiceError> {
        self.stat_handle.await
    }


}
//not being used atm as we're only connected to a single node
#[derive(Clone)]
pub struct SampleInfo{
    request_counter: u64,
    peer_light_nodes: Option<HashSet<Pubkey>>,
    full_nodes: Option<HashSet<Pubkey>>,
}

#[derive(Clone,Copy, Default, Debug)]
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
#[derive(Copy, Clone, Default, Debug)]
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

#[derive(Default,Clone, Copy, Debug)]
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

//alternate
// #[derive(Default)]
// pub(crate) struct SampleStatistics{
//     total_requests: usize,
//     whitelisted_requests: Option<usize>, //setting it to an option for now
//     verified: usize, //no of shreds processed or verified
//     verification_err: usize, //no of shreds errored
// }
// make a report stats fn here

// fn report_sample_stats(){
    
// }