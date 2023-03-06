use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use solana_ledger::shred::{self, Shred, ShredId, Slot};
use solana_sdk::{
    pubkey::Pubkey,
    sanitize::{Sanitize, SanitizeError},
};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct NodeShreds {
    pub shred_table: HashMap<Slot, Vec<ShredId>>,
    pub wallclock: u64,
    pub node_pubkey: Pubkey,
}

impl Sanitize for NodeShreds {
    fn sanitize(&self) -> std::result::Result<(), SanitizeError> {
        Ok(())
    }
}

impl NodeShreds {
    fn new(shreds: Vec<Shred>, node_pubkey: Pubkey) -> Self {
        let slot = shreds[0].slot();
        assert!(shreds.iter().all(|s| s.slot() == slot));
        let shred_ids = shreds.into_iter().map(|s| s.id()).collect::<Vec<ShredId>>();
        let mut shred_table: HashMap<Slot, Vec<ShredId>> = HashMap::new();
        shred_table.insert(slot, shred_ids);
        let start = SystemTime::now();
        let wallclock = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64;
        Self {
            shred_table,
            wallclock,
            node_pubkey,
        }
    }
}
