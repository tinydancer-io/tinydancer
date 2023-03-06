use std::{
    net::{SocketAddr, UdpSocket},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use solana_sdk::pubkey::Pubkey;
use tokio::sync::RwLock;

use super::contact_info::ContactInfo;

// pub struct Sockets {
//     pub gossip_socket: UdpSocket,
//     pub repair: UdpSocket,
// }
pub struct Node {
    pub gossip_nodes: Vec<SocketAddr>,
}

impl Node {
    pub fn new(gossip_nodes: Vec<SocketAddr>) -> Self {
        Self { gossip_nodes }
    }
    // pub fn new(
    //     pubkey: Pubkey,
    //     repair: SocketAddr,
    //     gossip: SocketAddr,
    //     rpc: SocketAddr,
    //     rpc_pubsub: SocketAddr,
    //     serve_repair: SocketAddr,
    // ) -> Self {
    //     let start = SystemTime::now();
    //     let since_the_epoch = start
    //         .duration_since(UNIX_EPOCH)
    //         .expect("Time went backwards");
    //     let contact_info = ContactInfo {
    //         id: pubkey,
    //         repair,
    //         gossip,
    //         rpc,
    //         rpc_pubsub,
    //         serve_repair,
    //         wallclock: since_the_epoch.as_millis() as u64,
    //         shred_version: 0, // later have to fetch and get latest version
    //     };
    //     let repair = UdpSocket::bind(repair).expect("failed to bind");
    //     let gossip_socket = UdpSocket::bind(gossip).expect("failed to bind");

    //     Self {
    //         contact_info,
    //         sockets: Sockets {
    //             repair,
    //             gossip_socket,
    //         },
    //     }
    // }
}
