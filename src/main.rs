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
mod tinydancer;
use tinydancer::{RpcEndpoint, TinyDancer};
mod sampler;

#[tokio::main]
async fn main() {
    let sampler = TinyDancer::new(RpcEndpoint::Localnet).await;
    sampler.join().await;
}
