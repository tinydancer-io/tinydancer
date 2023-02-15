use std::borrow::{Borrow, Cow};

use super::{contact_info::ContactInfo, node_shreds::NodeShreds, version::Version};
use bincode::serialize;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    sanitize::{Sanitize, SanitizeError},
    signature::{Signable, Signature},
    timing::timestamp,
};
pub const MAX_WALLCLOCK: u64 = 1_000_000_000_000_000;
pub struct Crds {
    table: IndexMap<CrdsValueLabel, VersionedCrdsValue>,
    cursor: Cursor,         // Next insert ordinal location.
    nodes: IndexSet<usize>, // Indices of nodes' ContactInfo.
}

/// This structure stores some local metadata associated with the CrdsValue
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct VersionedCrdsValue {
    /// Ordinal index indicating insert order.
    ordinal: u64,
    pub value: CrdsValue,
    /// local time when updated
    pub(crate) local_timestamp: u64,
    /// value hash
    pub(crate) value_hash: solana_sdk::blake3::Hash,
    // /// Number of times duplicates of this value are recevied from gossip push.
    // num_push_dups: u8,
}
// CrdsValue that is replicated across the cluster
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CrdsValue {
    pub signature: Signature,
    pub data: CrdsData,
}

impl Sanitize for CrdsValue {
    fn sanitize(&self) -> Result<(), SanitizeError> {
        self.signature.sanitize()?;
        self.data.sanitize()
    }
}

impl Signable for CrdsValue {
    fn pubkey(&self) -> Pubkey {
        self.pubkey()
    }

    fn signable_data(&self) -> Cow<[u8]> {
        Cow::Owned(serialize(&self.data).expect("failed to serialize CrdsData"))
    }

    fn get_signature(&self) -> Signature {
        self.signature
    }

    fn set_signature(&mut self, signature: Signature) {
        self.signature = signature
    }

    fn verify(&self) -> bool {
        self.get_signature()
            .verify(self.pubkey().as_ref(), self.signable_data().borrow())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum CrdsData {
    ContactInfo(ContactInfo),
    Version(Version),
    NodeShreds(NodeShreds),
}

impl Sanitize for CrdsData {
    fn sanitize(&self) -> Result<(), SanitizeError> {
        match self {
            CrdsData::ContactInfo(val) => val.sanitize(),
            CrdsData::Version(version) => version.sanitize(),
            CrdsData::NodeShreds(node_shreds) => node_shreds.sanitize(),
        }
    }
}

#[derive(PartialEq, Hash, Eq, Clone, Debug)]
pub enum CrdsValueLabel {
    ContactInfo(Pubkey),
    Version(Pubkey),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CrdsVersion {
    pub from: Pubkey,
    pub wallclock: u64,
    pub version: Version,
}

impl Sanitize for CrdsVersion {
    fn sanitize(&self) -> Result<(), SanitizeError> {
        sanitize_wallclock(self.wallclock)?;
        self.from.sanitize()?;
        self.version.sanitize()
    }
}

impl CrdsVersion {
    pub fn new(from: Pubkey) -> Self {
        Self {
            from,
            wallclock: timestamp(),
            version: Version::default(),
        }
    }
}

pub(crate) fn sanitize_wallclock(wallclock: u64) -> Result<(), SanitizeError> {
    if wallclock >= MAX_WALLCLOCK {
        Err(SanitizeError::ValueOutOfBounds)
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Default)]
pub struct Cursor(u64);

impl Cursor {
    fn ordinal(&self) -> u64 {
        self.0
    }

    // Updates the cursor position given the ordinal index of value consumed.
    #[inline]
    fn consume(&mut self, ordinal: u64) {
        self.0 = self.0.max(ordinal + 1);
    }
}
