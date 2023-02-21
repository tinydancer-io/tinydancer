use super::packet;
use super::ping_pong;
use super::protocol::Protocol;
use crate::tinydancer::ClientService;
use async_trait::async_trait;
use core::str;
use crossbeam::channel::unbounded;
use solana_sdk::sanitize::Sanitize;
use std::net::SocketAddr;
use std::time::Duration;
use std::{net::AddrParseError, sync::Arc};
use tiny_logger::logs::{error, info};
use tokio::{net::UdpSocket, task::JoinHandle};
pub struct GossipService {
    pub gossip_handle: JoinHandle<()>,
}
pub struct GossipConfig {
    pub gossip_socket: String,
}

#[async_trait]
impl ClientService<GossipConfig> for GossipService {
    type ServiceError = tokio::task::JoinError;
    fn new(config: GossipConfig) -> Self {
        let addr = config.gossip_socket.clone();
        let gossip_handle = tokio::spawn(async move {
            // message receiver
            //message listener - process packets
            // message sender

            let socket = UdpSocket::bind(addr.as_str().parse::<SocketAddr>().unwrap())
                .await
                .unwrap();
            let gossip_socket = Arc::new(socket);
            let (receiver_tx, receiver_rx) = unbounded::<packet::Packet>();
            let (listener_tx, listener_rx) = unbounded::<(SocketAddr, Protocol)>();
            // let (sender_tx, sender_rx) = unbounded::<packet::Packet>();
            let rx_socket = Arc::clone(&gossip_socket);
            // let listener_socket = Arc::clone(&gossip_socket);
            let sender_socket = Arc::clone(&gossip_socket);
            let receiver = tokio::spawn(async move {
                rx_socket.set_broadcast(true).expect("err");

                println!("Broadcast: {:?}", rx_socket.broadcast());
                loop {
                    let mut buf = [0u8; 32];
                    while let Ok((n, addr)) = rx_socket.recv_from(&mut buf).await {
                        println!("{:?} bytes response from {:?}", n, addr);
                        let packet = packet::Packet::from_data(Some(&addr), buf);
                        match packet {
                            Ok(packet) => receiver_tx.send(packet).expect("send err"),
                            Err(e) => println!("error creating packet from network{:?}", e),
                        }
                    }
                }
            });
            let deser_and_verify_packet = |packet: packet::Packet| {
                let protocol: Protocol = packet.deserialize_slice(..).ok()?;
                protocol.sanitize().ok()?;
                let protocol = protocol.par_verify()?; // @TODO: add stats here
                Some((packet.meta().socket_addr(), protocol))
            };
            let listener = tokio::spawn(async move {
                loop {
                    while let Ok(packet) = receiver_rx.recv() {
                        let verified_packet = deser_and_verify_packet(packet);
                        match verified_packet {
                            Some(packet) => listener_tx.send(packet).expect("send err"),
                            None => error!(
                                "error verifying and deserializing packet {:?}",
                                verified_packet
                            ),
                        }
                    }
                }
            });
            let sender = tokio::spawn(async move {
                sender_socket.set_broadcast(true).expect("err");
                loop {
                    while let Ok((a, p)) = listener_rx.recv() {
                        match sender_socket.send_to(p.serialize().as_slice(), a).await {
                            Ok(r) => {}
                            Err(e) => error!("socket send err{:?}", e),
                        }
                    }
                }
            });
            let thread_handles = vec![receiver, listener, sender];
            for handle in thread_handles {
                handle.await;
            }
        });
        Self { gossip_handle }
    }
    async fn join(self) -> std::result::Result<(), Self::ServiceError> {
        self.gossip_handle.await
    }
}

#[cfg(test)]
mod tests {

    use std::net::SocketAddr;

    use rand::{rngs::ThreadRng, Rng};
    use rand_chacha::{rand_core::SeedableRng, ChaChaRng};
    use solana_sdk::signature::Keypair;
    use tokio::net::UdpSocket;

    use super::{ping_pong, Protocol};
    #[tokio::test]
    async fn send_ping() {
        let addr = String::from("0.0.0.0:4444");
        let socket = UdpSocket::bind(addr.as_str().parse::<SocketAddr>().unwrap())
            .await
            .unwrap();
        const SEED: [u8; 32] = [0x55; 32];
        let mut rng = ChaChaRng::from_seed(SEED);
        let kp = Keypair::generate(&mut rng);
        let token = [69u8; 32];
        let ping = Protocol::PingMessage(ping_pong::Ping::new(token, &kp).unwrap());
        socket.set_broadcast(true).expect("shut up");
        let res = socket
            .send_to(ping.serialize().as_slice(), "0.0.0.0:5555")
            .await;
        println!("res {:?}", res);
    }
}
