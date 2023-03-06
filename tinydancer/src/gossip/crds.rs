use std::borrow::{Borrow, Cow};

use super::{contact_info::ContactInfo, node_shreds::NodeShreds, version::Version};
use bincode::{serialize, serialized_size};
use indexmap::{map::Entry, IndexMap, IndexSet};
use rand::Rng;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    blake3::hash,
    pubkey::Pubkey,
    sanitize::{Sanitize, SanitizeError},
    signature::{Keypair, Signable, Signature},
    slot_history::Slot,
    timing::timestamp,
};
pub const MAX_WALLCLOCK: u64 = 1_000_000_000_000_000;
pub struct Crds {
    table: IndexMap<CrdsValueLabel, VersionedCrdsValue>,
    cursor: Cursor,         // Next insert ordinal location.
    nodes: IndexSet<usize>, // Indices of nodes' ContactInfo.
}

impl Default for Crds {
    fn default() -> Self {
        Crds {
            table: IndexMap::default(),
            cursor: Cursor::default(),

            nodes: IndexSet::default(),
        }
    }
}
#[derive(PartialEq, Eq, Debug)]
pub enum CrdsError {
    DuplicatePush(/*num dups:*/ u8),
    InsertFailed,
    UnknownStakes,
}
impl Crds {
    pub fn insert(&mut self, value: CrdsValue, now: u64) -> Result<(), CrdsError> {
        let label = value.label();
        let pubkey = value.pubkey();
        let value = VersionedCrdsValue::new(value, self.cursor, now);
        match self.table.entry(label) {
            Entry::Vacant(entry) => {
                // self.stats.lock().unwrap().record_insert(&value, route);
                let entry_index = entry.index();
                // self.shards.insert(entry_index, &value);
                match &value.value.data {
                    CrdsData::ContactInfo(node) => {
                        self.nodes.insert(entry_index);
                        // self.shred_versions.insert(pubkey, node.shred_version);
                    }
                    // CrdsData::Vote(_, _) => {
                    //     self.votes.insert(value.ordinal, entry_index);
                    // }
                    // CrdsData::EpochSlots(_, _) => {
                    //     self.epoch_slots.insert(value.ordinal, entry_index);
                    // }
                    // CrdsData::DuplicateShred(_, _) => {
                    //     self.duplicate_shreds.insert(value.ordinal, entry_index);
                    // }
                    _ => (),
                };
                // self.entries.insert(value.ordinal, entry_index);
                // self.records.entry(pubkey).or_default().insert(entry_index);
                self.cursor.consume(value.ordinal);
                entry.insert(value);
                Ok(())
            }
            Entry::Occupied(mut entry) if overrides(&value.value, entry.get()) => {
                // self.stats.lock().unwrap().record_insert(&value, route);
                let entry_index = entry.index();
                // self.shards.remove(entry_index, entry.get());
                // self.shards.insert(entry_index, &value);
                match &value.value.data {
                    CrdsData::ContactInfo(node) => {
                        // self.shred_versions.insert(pubkey, node.shred_version);
                        // self.nodes does not need to be updated since the
                        // entry at this index was and stays contact-info.
                        // debug_assert_matches!(entry.get().value.data, CrdsData::ContactInfo(_));
                    }
                    // CrdsData::Vote(_, _) => {
                    //     self.votes.remove(&entry.get().ordinal);
                    //     self.votes.insert(value.ordinal, entry_index);
                    // }
                    // CrdsData::EpochSlots(_, _) => {
                    //     self.epoch_slots.remove(&entry.get().ordinal);
                    //     self.epoch_slots.insert(value.ordinal, entry_index);
                    // }
                    // CrdsData::DuplicateShred(_, _) => {
                    //     self.duplicate_shreds.remove(&entry.get().ordinal);
                    //     self.duplicate_shreds.insert(value.ordinal, entry_index);
                    // }
                    _ => (),
                }
                // self.entries.remove(&entry.get().ordinal);
                // self.entries.insert(value.ordinal, entry_index);
                // As long as the pubkey does not change, self.records
                // does not need to be updated.
                debug_assert_eq!(entry.get().value.pubkey(), pubkey);
                self.cursor.consume(value.ordinal);
                // self.purged.push_back((entry.get().value_hash, now));
                entry.insert(value);
                Ok(())
            }
            Entry::Occupied(mut entry) => {
                // self.stats.lock().unwrap().record_fail(&value, route);
                // trace!(
                //     "INSERT FAILED data: {} new.wallclock: {}",
                //     value.value.label(),
                //     value.value.wallclock(),
                // );
                // Identify if the message is outdated (as opposed to
                // duplicate) by comparing value hashes.
                if entry.get().value_hash != value.value_hash {
                    self.purged.push_back((value.value_hash, now));
                    Err(CrdsError::InsertFailed)
                } else if matches!(route, GossipRoute::PushMessage) {
                    let entry = entry.get_mut();
                    entry.num_push_dups = entry.num_push_dups.saturating_add(1);
                    Err(CrdsError::DuplicatePush(entry.num_push_dups))
                } else {
                    Err(CrdsError::InsertFailed)
                }
            }
        }
    }
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
impl VersionedCrdsValue {
    fn new(value: CrdsValue, cursor: Cursor, local_timestamp: u64) -> Self {
        let value_hash = hash(&serialize(&value).unwrap());
        VersionedCrdsValue {
            ordinal: cursor.ordinal(),
            value,
            local_timestamp,
            value_hash,
            // num_push_dups: 0u8,
        }
    }
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
    // Version(Version),
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

#[derive(PartialEq, Hash, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum CrdsValueLabel {
    ContactInfo(Pubkey),
    NodeShreds(Pubkey),
    // Version(Pubkey),
}
impl CrdsValueLabel {
    pub fn pubkey(&self) -> Pubkey {
        match self {
            CrdsValueLabel::ContactInfo(p) => *p,
            CrdsValueLabel::NodeShreds(p) => *p,
            // CrdsValueLabel::Vote(_, p) => *p,
            // CrdsValueLabel::LowestSlot(p) => *p,
            // CrdsValueLabel::SnapshotHashes(p) => *p,
            // CrdsValueLabel::EpochSlots(_, p) => *p,
            // CrdsValueLabel::AccountsHashes(p) => *p,
            // CrdsValueLabel::LegacyVersion(p) => *p,
            // CrdsValueLabel::Version(p) => *p,
            // CrdsValueLabel::NodeInstance(p) => *p,
            // CrdsValueLabel::DuplicateShred(_, p) => *p,
            // CrdsValueLabel::IncrementalSnapshotHashes(p) => *p,
        }
    }
}

impl CrdsValue {
    pub fn new_unsigned(data: CrdsData) -> Self {
        Self {
            signature: Signature::default(),
            data,
        }
    }

    pub fn new_signed(data: CrdsData, keypair: &Keypair) -> Self {
        let mut value = Self::new_unsigned(data);
        value.sign(keypair);
        value
    }

    /// New random CrdsValue for tests and benchmarks.
    // pub fn new_rand<R: Rng>(rng: &mut R, keypair: Option<&Keypair>) -> CrdsValue {
    //     match keypair {
    //         None => {
    //             let keypair = Keypair::new();
    //             let data = CrdsData::new_rand(rng, Some(keypair.pubkey()));
    //             Self::new_signed(data, &keypair)
    //         }
    //         Some(keypair) => {
    //             let data = CrdsData::new_rand(rng, Some(keypair.pubkey()));
    //             Self::new_signed(data, keypair)
    //         }
    //     }
    // }

    /// Totally unsecure unverifiable wallclock of the node that generated this message
    /// Latest wallclock is always picked.
    /// This is used to time out push messages.
    pub fn wallclock(&self) -> u64 {
        match &self.data {
            CrdsData::ContactInfo(contact_info) => contact_info.wallclock,
            CrdsData::NodeShreds(ns) => ns.wallclock,
            // CrdsData::Vote(_, vote) => vote.wallclock,
            // CrdsData::LowestSlot(_, obj) => obj.wallclock,
            // CrdsData::SnapshotHashes(hash) => hash.wallclock,
            // CrdsData::AccountsHashes(hash) => hash.wallclock,
            // CrdsData::EpochSlots(_, p) => p.wallclock,
            // CrdsData::LegacyVersion(version) => version.wallclock,
            // CrdsData::Version(version) => version.wallclock,
            // CrdsData::NodeInstance(node) => node.wallclock,
            // CrdsData::DuplicateShred(_, shred) => shred.wallclock,
            // CrdsData::IncrementalSnapshotHashes(hash) => hash.wallclock,
        }
    }
    pub fn pubkey(&self) -> Pubkey {
        match &self.data {
            CrdsData::ContactInfo(contact_info) => contact_info.id,
            CrdsData::NodeShreds(ns) => ns.node_pubkey
            // CrdsData::Vote(_, vote) => vote.from,
            // CrdsData::LowestSlot(_, slots) => slots.from,
            // CrdsData::SnapshotHashes(hash) => hash.from,
            // CrdsData::AccountsHashes(hash) => hash.from,
            // CrdsData::EpochSlots(_, p) => p.from,
            // CrdsData::LegacyVersion(version) => version.from,
            // CrdsData::Version(version) => version.from,
            // CrdsData::NodeInstance(node) => node.from,
            // CrdsData::DuplicateShred(_, shred) => shred.from,
            // CrdsData::IncrementalSnapshotHashes(hash) => hash.from,
        }
    }
    pub fn label(&self) -> CrdsValueLabel {
        match &self.data {
            CrdsData::ContactInfo(_) => CrdsValueLabel::ContactInfo(self.pubkey()),
            CrdsData::NodeShreds(ns) => CrdsValueLabel::NodeShreds(self.pubkey(), ns), // CrdsData::Vote(ix, _) => CrdsValueLabel::Vote(*ix, self.pubkey()),
                                                                                       // CrdsData::LowestSlot(_, _) => CrdsValueLabel::LowestSlot(self.pubkey()),
                                                                                       // CrdsData::SnapshotHashes(_) => CrdsValueLabel::SnapshotHashes(self.pubkey()),
                                                                                       // CrdsData::AccountsHashes(_) => CrdsValueLabel::AccountsHashes(self.pubkey()),
                                                                                       // CrdsData::EpochSlots(ix, _) => CrdsValueLabel::EpochSlots(*ix, self.pubkey()),
                                                                                       // CrdsData::LegacyVersion(_) => CrdsValueLabel::LegacyVersion(self.pubkey()),
                                                                                       // CrdsData::Version(_) => CrdsValueLabel::Version(self.pubkey()),
                                                                                       // CrdsData::NodeInstance(node) => CrdsValueLabel::NodeInstance(node.from),
                                                                                       // CrdsData::DuplicateShred(ix, shred) => CrdsValueLabel::DuplicateShred(*ix, shred.from),
                                                                                       // CrdsData::IncrementalSnapshotHashes(_) => {
                                                                                       //     CrdsValueLabel::IncrementalSnapshotHashes(self.pubkey())
                                                                                       // }
        }
    }
    pub fn contact_info(&self) -> Option<&ContactInfo> {
        match &self.data {
            CrdsData::ContactInfo(contact_info) => Some(contact_info),
            _ => None,
        }
    }

    // pub(crate) fn accounts_hash(&self) -> Option<&SnapshotHashes> {
    //     match &self.data {
    //         CrdsData::AccountsHashes(slots) => Some(slots),
    //         _ => None,
    //     }
    // }

    /// Returns the size (in bytes) of a CrdsValue
    pub fn size(&self) -> u64 {
        serialized_size(&self).expect("unable to serialize contact info")
    }
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
