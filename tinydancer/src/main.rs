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
#![feature(mutex_unlock)]
mod tinydancer;
use std::{
    f32::consts::E,
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use crossterm::style::Stylize;
use sampler::{pull_and_verify_shreds, ArchiveConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use spinoff::{spinners, Color, Spinner};
use tinydancer::{endpoint, Cluster, TinyDancer, TinyDancerConfig};
mod macros;
use colored::Colorize;
mod rpc_wrapper;
mod sampler;
mod ui;

use clap::{ArgGroup, Parser, Subcommand, *};
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]

struct Args {
    /// Subcommands to run
    #[clap(subcommand)]
    command: Commands,
}
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the local light client
    Start {
        /// If you want to enable the status monitor
        #[clap(long, short, default_value_t = true)]
        enable_ui_service: bool,

        /// If you want to enable detailed tui to monitor
        #[clap(long, short, default_value_t = false)]
        tui_monitor: bool,

        /// Amount of shreds you want to sample per slot
        #[clap(long, short, default_value_t = 10)]
        sample_qty: u64,
        /// Rocks db path for storing shreds
        #[clap(required = false)]
        archive_path: Option<String>,

        /// Duration after which shreds will be purged
        #[clap(required = false, default_value_t = 10000000)]
        shred_archive_duration: u64,
    },
    /// Verify the samples for a single slot
    Verify {
        #[clap(long, required = false, default_value = "0")]
        slot: usize,
    },
    /// Stream the client logs to your terminal
    Logs {
        #[clap(long, required = false, default_value = "client.log")]
        log_path: String,
    },
    /// Edit your client config
    #[clap(subcommand)]
    Config(ConfigSubcommands),
}
#[derive(Debug, Subcommand)]
pub enum ConfigSubcommands {
    Set {
        #[clap(long, required = false, default_value = "client.log")]
        log_path: String,
        /// The cluster you want to run the client on (Mainnet, Localnet,Devnet, <custom-url>)
        #[clap(long, short, required = false, default_value = "Localnet")]
        cluster: String,
    },
    Get,
}
// ~/.config/
#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Logs { log_path } => {
            std::process::Command::new("tail")
                .arg("-f")
                .arg(log_path)
                .output()
                .expect("log command failed");
        }
        Commands::Start {
            enable_ui_service,
            sample_qty,
            archive_path,
            shred_archive_duration,
            tui_monitor,
        } => {
            let mut config_file = {
                let home_path = std::env::var("HOME").unwrap();

                // println!("path {:?}", path);
                let text =
                    std::fs::read_to_string(home_path + "/.config/tinydancer/config.json").unwrap();

                serde_json::from_str::<ConfigSchema>(&text)
            };
            match config_file {
                Ok(config_file) => {
                    let config = TinyDancerConfig {
                        enable_ui_service,
                        rpc_endpoint: get_cluster(config_file.cluster),
                        sample_qty,
                        tui_monitor,
                        log_path: config_file.log_path,
                        archive_config: {
                            if let Some(path) = archive_path {
                                Some(ArchiveConfig {
                                    shred_archive_duration,
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
                Err(e) => {
                    println!("error: {:?}", e);
                    std::process::Command::new("echo")
                        .arg("\"Please set a config first using tindancer config set\"")
                        .spawn()
                        .expect("Config not set");
                }
            }
        }
        Commands::Config(sub_config) => match sub_config {
            ConfigSubcommands::Get => {
                let home_path = std::env::var("HOME").unwrap();
                let is_existing = home_path.clone() + "/.config/tinydancer/config.json";
                let path = Path::new(&is_existing);
                if path.exists() {
                    std::process::Command::new("cat")
                        .arg(home_path + "/.config/tinydancer/config.json")
                        .spawn()
                        .expect("Config not set");
                } else {
                    println!(
                        "{} {}",
                        "Initialise a config first using:".to_string().yellow(),
                        "tinydancer set config".to_string().green()
                    );
                }
            }
            ConfigSubcommands::Set { log_path, cluster } => {
                // println!("{:?}", fs::create_dir_all("~/.config/tinydancer"));

                let home_path = std::env::var("HOME").unwrap();
                let is_existing = home_path.clone() + "/.config/tinydancer";
                let path = Path::new(&is_existing);
                if !path.exists() {
                    std::process::Command::new("mkdir")
                        .arg(home_path.clone() + "/.config/tinydancer")
                        .stdout(std::process::Stdio::null())
                        .spawn()
                        .expect("couldnt make dir");
                }
                let is_existing = home_path.clone() + "/.config/tinydancer/config.json";
                let path = Path::new(&is_existing);
                if !path.exists() {
                    std::process::Command::new("touch")
                        .arg(home_path.clone() + "/.config/tinydancer/config.json")
                        .stdout(std::process::Stdio::null())
                        .spawn()
                        .expect("couldnt make file");
                }
                sleep(Duration::from_secs(1));
                loop {
                    let mut config_file = {
                        let home_path = std::env::var("HOME").unwrap();

                        // println!("path {:?}", path);
                        let text =
                            std::fs::read_to_string(home_path + "/.config/tinydancer/config.json")
                                .unwrap();

                        serde_json::from_str::<ConfigSchema>(&text)
                    };

                    match config_file {
                        Ok(mut config_file) => {
                            config_file.log_path = log_path.clone();
                            config_file.cluster = cluster.clone();
                            std::fs::write(
                                home_path.clone() + "/.config/tinydancer/config.json",
                                serde_json::to_string_pretty(&config_file).unwrap(),
                            )
                            .unwrap();
                            break;
                        }
                        Err(_) => {
                            std::fs::write(
                                home_path.clone() + "/.config/tinydancer/config.json",
                                serde_json::to_string_pretty(&serde_json::json!({
                                    "cluster":"Localnet",
                                    "logPath":"client.log"
                                }))
                                .unwrap(),
                            )
                            .unwrap();
                            break;
                        }
                    }
                }
            }
        },
        Commands::Verify { slot } => {
            let mut spinner = Spinner::new(
                spinners::Dots,
                format!("Verifying Shreds for Slot {}", slot),
                Color::Green,
            );
            let home_path = std::env::var("HOME").unwrap();
            let is_existing = home_path.clone() + "/.config/tinydancer/config.json";
            let path = Path::new(&is_existing);
            if path.exists() {
                let mut config_file = {
                    let home_path = std::env::var("HOME").unwrap();

                    let text =
                        std::fs::read_to_string(home_path + "/.config/tinydancer/config.json")
                            .unwrap();

                    serde_json::from_str::<ConfigSchema>(&text)
                };
                // println!("path {:?}", config_file);
                match config_file {
                    Ok(config_file) => {
                        let is_verified =
                            pull_and_verify_shreds(slot, get_endpoint(config_file.cluster)).await;

                        if is_verified {
                            println!(
                                "Slot {} is {} ✓",
                                slot.to_string().yellow(),
                                "Valid".to_string().green()
                            );
                        } else {
                            println!(
                                "Slot {} is not {} ❌",
                                slot.to_string().yellow(),
                                "Valid".to_string().red()
                            );
                        }
                    }
                    Err(e) => {
                        tiny_logger::logs::error!("{}", e);
                        println!("e {:?}", e);
                    }
                };
            } else {
                println!(
                    "{} {}",
                    "Initialise a config first using:".to_string().yellow(),
                    "tinydancer set config".to_string().green()
                );
            }
        }
    }
}

pub fn get_cluster(cluster: String) -> Cluster {
    match cluster.as_str() {
        "Mainnet" => Cluster::Mainnet,
        "Devnet" => Cluster::Devnet,
        "Localnet" => Cluster::Localnet,
        _ => Cluster::Custom(cluster),
    }
}
pub fn get_endpoint(cluster: String) -> String {
    match cluster.as_str() {
        "Mainnet" => "https://api.mainnet-beta.solana.com".to_owned(),
        "Devnet" => "https://api.devnet.solana.com".to_owned(),
        "Localnet" => "http://0.0.0.0:8899".to_owned(),
        _ => "http://0.0.0.0:8899".to_owned(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSchema {
    pub log_path: String,
    pub cluster: String,
}
