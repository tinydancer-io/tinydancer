use std::collections::HashMap;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use solana_ledger::shred::{Shred, ShredId, Slot};
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
