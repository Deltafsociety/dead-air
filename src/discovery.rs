use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, RwLock};

pub const DEFAULT_BEACON_PORT: u16 = 48690;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct PeerNode {
    pub ip: String,
    pub relay_port: u16,
    pub name: String,
    pub ws_url: String,
    pub last_seen: Instant,
}

pub struct DiscoveryManager {
    peers: Arc<RwLock<HashMap<String, PeerNode>>>,
    discovered_tx: mpsc::UnboundedSender<PeerNode>,
}

impl DiscoveryManager {
    pub fn new(discovered_tx: mpsc::UnboundedSender<PeerNode>) -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            discovered_tx,
        }
    }

    pub fn start(
        self: Arc<Self>,
        node_name: String,
        relay_port: u16,
        my_npub: String,
    ) {
        let mgr_listener = Arc::clone(&self);
        let my_npub_filter = my_npub.clone();
        tokio::spawn(async move {
            let socket = match UdpSocket::bind(format!("0.0.0.0:{}", DEFAULT_BEACON_PORT)).await {
                Ok(s) => s,
                Err(_) => return,
            };
            let _ = socket.set_broadcast(true);

            let mut buf = [0u8; 1024];
            loop {
                let (len, src) = match socket.recv_from(&mut buf).await {
                    Ok(res) => res,
                    Err(_) => continue,
                };

                if let Ok(msg) = std::str::from_utf8(&buf[..len]) {
                    if let Some(peer) = parse_beacon(msg, src, &my_npub_filter) {
                        let mut peers = mgr_listener.peers.write().await;
                        peers.insert(peer.ws_url.clone(), peer.clone());
                        let _ = mgr_listener.discovered_tx.send(peer);
                    }
                }
            }
        });

        tokio::spawn(async move {
            let socket = match UdpSocket::bind("0.0.0.0:0").await {
                Ok(s) => s,
                Err(_) => return,
            };
            let _ = socket.set_broadcast(true);
            let target = format!("255.255.255.255:{}", DEFAULT_BEACON_PORT);

            let short_npub = if my_npub.len() > 12 {
                &my_npub[..12]
            } else {
                &my_npub
            };
            let payload = format!("DEADAIR:{}:{}:{}", node_name, relay_port, short_npub);
            let bytes = payload.as_bytes();

            loop {
                let _ = socket.send_to(bytes, &target).await;
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        });
    }

    #[allow(dead_code)]
    pub async fn active_peers(&self) -> Vec<PeerNode> {
        let peers = self.peers.read().await;
        let now = Instant::now();
        peers
            .values()
            .filter(|p| now.duration_since(p.last_seen) < Duration::from_secs(30))
            .cloned()
            .collect()
    }
}

fn parse_beacon(msg: &str, src: SocketAddr, my_npub: &str) -> Option<PeerNode> {
    let parts: Vec<&str> = msg.split(':').collect();
    if parts.len() < 4 || parts[0] != "DEADAIR" {
        return None;
    }

    let node_name = parts[1].to_string();
    let relay_port: u16 = parts[2].parse().ok()?;
    let beacon_npub = parts[3];

    if my_npub.starts_with(beacon_npub) {
        return None;
    }

    let ip = src.ip().to_string();
    let ws_url = format!("ws://{}:{}", ip, relay_port);

    Some(PeerNode {
        ip,
        relay_port,
        name: node_name,
        ws_url,
        last_seen: Instant::now(),
    })
}
