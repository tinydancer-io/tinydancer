use std::collections::HashMap;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use solana_ledger::shred::{self, Shred, ShredId, Slot};
use solana_sdk::sanitize::{Sanitize, SanitizeError};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct NodeShreds {
    pub shred_table: HashMap<Slot, Vec<ShredId>>,
}

impl Sanitize for NodeShreds {
    fn sanitize(&self) -> std::result::Result<(), SanitizeError> {
        Ok(())
    }
}

impl NodeShreds {
    fn new(shreds: Vec<Shred>) -> Self {
        let slot = shreds[0].slot();
        assert!(shreds.iter().all(|s| s.slot() == slot));
        let shred_ids = shreds.into_iter().map(|s| s.id()).collect::<Vec<ShredId>>();
        let mut shred_table: HashMap<Slot, Vec<ShredId>> = HashMap::new();
        shred_table.insert(slot, shred_ids);
        Self { shred_table }
    }
}
