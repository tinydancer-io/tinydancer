//! client struct
//! new -> self
//! join -> JoinHandle
//!
//!
//! things happening at the same time:
//! sampling -> pull data -> run sampling algo
//! monitoring system -> slot number | shreds req/rec | sampling progress | connected nodes
//! ui -> display stats
//!
//! let (rx,tx) = channel();
//! spawn(move{
//!     let sampler = Sampler::new(tx)
//!     sampler.join();         
//! })
//!
//! spawn(move{
//!     let monitor = Monitor::new(config, rx)
//! });
//! let rx2 = rx.clone();
//! spawn(move{
//!     let ui = Ui::new(...)
//!  while let r = rx2.recv(){
//!     
//! }
//! })
#![feature(async_closure)]
#![allow(unused_imports)]
#![allow(dead_code)]
mod tinydancer;
use std::io;

use tinydancer::{Cluster, TinyDancer, TinyDancerConfig};
mod sampler;
// mod ui;

#[tokio::main]
async fn main() {
    let config = TinyDancerConfig {
        rpc_endpoint: Cluster::Localnet,
        sample_qty: 10,
    };
    let client = TinyDancer::new(config).await;
    client.join().await;
}
