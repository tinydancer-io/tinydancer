use std::borrow::Cow;

use bincode::serialize;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    sanitize::{Sanitize, SanitizeError},
    signature::{Signable, Signature},
};

use super::{
    ping_pong::{self, Pong},
    CrdsValue, CrdsValueLabel, MAX_WALLCLOCK,
};
const GOSSIP_PING_TOKEN_SIZE: usize = 32;
#[derive(Serialize, Deserialize, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Protocol {
    /// Gossip protocol messages
    PullRequest(CrdsFilter, CrdsValue),
    PullResponse(Pubkey, Vec<CrdsValue>),
    PushMessage(Pubkey, Vec<CrdsValue>),
    // TODO: Remove the redundant outer pubkey here,
    // and use the inner PruneData.pubkey instead.
    PruneMessage(Pubkey, PruneData),
    PingMessage(Ping),
    PongMessage(Pong),
    // Update count_packets_received if new variants are added here.
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CrdsFilter {
    pub filter: CrdsValueLabel,
}
pub type Ping = ping_pong::Ping<[u8; GOSSIP_PING_TOKEN_SIZE]>;
impl Protocol {
    pub fn par_verify(self) -> Option<Self> {
        match self {
            Protocol::PullRequest(_, ref caller) => {
                if caller.verify() {
                    Some(self)
                } else {
                    // stats.gossip_pull_request_verify_fail.add_relaxed(1);
                    None
                }
            }
            Protocol::PullResponse(from, data) => {
                let size = data.len();
                let data: Vec<_> = data.into_par_iter().filter(Signable::verify).collect();
                if size != data.len() {
                    // stats
                    //     .gossip_pull_response_verify_fail
                    //     .add_relaxed((size - data.len()) as u64);
                }
                if data.is_empty() {
                    None
                } else {
                    Some(Protocol::PullResponse(from, data))
                }
            }
            Protocol::PushMessage(from, data) => {
                let size = data.len();
                let data: Vec<_> = data.into_par_iter().filter(Signable::verify).collect();
                if size != data.len() {
                    // stats
                    //     .gossip_push_msg_verify_fail
                    //     .add_relaxed((size - data.len()) as u64);
                }
                if data.is_empty() {
                    None
                } else {
                    Some(Protocol::PushMessage(from, data))
                }
            }
            Protocol::PruneMessage(_, ref data) => {
                if data.verify() {
                    Some(self)
                } else {
                    // stats.gossip_prune_msg_verify_fail.add_relaxed(1);
                    None
                }
            }
            Protocol::PingMessage(ref ping) => {
                if ping.verify() {
                    Some(self)
                } else {
                    // stats.gossip_ping_msg_verify_fail.add_relaxed(1);
                    None
                }
            }
            Protocol::PongMessage(ref pong) => {
                if pong.verify() {
                    Some(self)
                } else {
                    // stats.gossip_pong_msg_verify_fail.add_relaxed(1);
                    None
                }
            }
        }
    }
    pub fn serialize(self) -> Vec<u8> {
        bincode::serialize(&self).expect("error serializing Protocol")
    }
}

impl Sanitize for Protocol {
    fn sanitize(&self) -> Result<(), SanitizeError> {
        match self {
            Protocol::PullRequest(_filter, val) => {
                // filter.sanitize()?;
                val.sanitize()
            }
            Protocol::PullResponse(_, val) => val.sanitize(),
            Protocol::PushMessage(_, val) => val.sanitize(),
            Protocol::PruneMessage(from, val) => {
                if *from != val.pubkey {
                    Err(SanitizeError::InvalidValue)
                } else {
                    val.sanitize()
                }
            }
            Protocol::PingMessage(ping) => ping.sanitize(),
            Protocol::PongMessage(pong) => pong.sanitize(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PruneData {
    /// Pubkey of the node that sent this prune data
    pubkey: Pubkey,
    /// Pubkeys of nodes that should be pruned
    prunes: Vec<Pubkey>,
    /// Signature of this Prune Message
    signature: Signature,
    /// The Pubkey of the intended node/destination for this message
    destination: Pubkey,
    /// Wallclock of the node that generated this message
    wallclock: u64,
}

// impl PruneData {
//     /// New random PruneData for tests and benchmarks.
//     #[cfg(test)]
//     fn new_rand<R: rand::Rng>(
//         rng: &mut R,
//         self_keypair: &Keypair,
//         num_nodes: Option<usize>,
//     ) -> Self {
//         use super::crds;

//         let wallclock = crds::new_rand_timestamp(rng);
//         let num_nodes = num_nodes.unwrap_or_else(|| rng.gen_range(0, MAX_PRUNE_DATA_NODES + 1));
//         let prunes = std::iter::repeat_with(Pubkey::new_unique)
//             .take(num_nodes)
//             .collect();
//         let mut prune_data = PruneData {
//             pubkey: self_keypair.pubkey(),
//             prunes,
//             signature: Signature::default(),
//             destination: Pubkey::new_unique(),
//             wallclock,
//         };
//         prune_data.sign(self_keypair);
//         prune_data
//     }
// }

impl Sanitize for PruneData {
    fn sanitize(&self) -> Result<(), SanitizeError> {
        if self.wallclock >= MAX_WALLCLOCK {
            return Err(SanitizeError::ValueOutOfBounds);
        }
        Ok(())
    }
}

impl Signable for PruneData {
    fn pubkey(&self) -> Pubkey {
        self.pubkey
    }

    fn signable_data(&self) -> Cow<[u8]> {
        #[derive(Serialize)]
        struct SignData<'a> {
            pubkey: &'a Pubkey,
            prunes: &'a [Pubkey],
            destination: &'a Pubkey,
            wallclock: u64,
        }
        let data = SignData {
            pubkey: &self.pubkey,
            prunes: &self.prunes,
            destination: &self.destination,
            wallclock: self.wallclock,
        };
        Cow::Owned(serialize(&data).expect("serialize PruneData"))
    }

    fn get_signature(&self) -> Signature {
        self.signature
    }

    fn set_signature(&mut self, signature: Signature) {
        self.signature = signature
    }
}
