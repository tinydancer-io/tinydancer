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

use sampler::ArchiveConfig;
use tinydancer::{Cluster, TinyDancer, TinyDancerConfig};
mod macros;
mod sampler;
mod ui;
mod rpc_wrapper;

use clap::{ArgGroup, Parser, Subcommand};
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]

struct Args {
    /// The cluster you want to run the client on (Mainnet, Localnet,Devnet, <custom-url>)
    #[clap(long, short, required = true)]
    cluster: String,

    /// If you want to enable the monitoring ui
    #[clap(long, short, default_value_t = false)]
    enable_ui_service: bool,

    /// Amount of shreds you want to sample per slot
    #[clap(long, short, default_value_t = 10)]
    sample_qty: u64,
    /// Rocks db path for toring shreds
    #[clap(required = false)]
    archive_path: Option<String>,

    /// Duration after which shreds will be purged
    #[clap(required = false, default_value_t = 10000000)]
    shred_archive_duration: u64,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let config = TinyDancerConfig {
        enable_ui_service: args.enable_ui_service,
        rpc_endpoint: get_cluster(args.cluster),
        sample_qty: args.sample_qty,

        archive_config: {
            if let Some(path) = args.archive_path {
                Some(ArchiveConfig {
                    shred_archive_duration: args.shred_archive_duration,
                    archive_path: path,
                })
            } else {
                None
            }
        },
    };
    let client = TinyDancer::new(config).await;
    client.join().await;
}

pub fn get_cluster(cluster: String) -> Cluster {
    match cluster.as_str() {
        "Mainnet" => Cluster::Mainnet,
        "Devnet" => Cluster::Devnet,
        "Localnet" => Cluster::Localnet,
        _ => Cluster::Custom(cluster),
    }
}
