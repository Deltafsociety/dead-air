use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use futures_util::{SinkExt, StreamExt};
use nostr::message::relay::RelayMessage;
use nostr::message::ClientMessage;
use nostr::prelude::*;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;

#[derive(Clone, Debug)]
pub struct RelayStatus {
    pub url: String,
    pub connected: bool,
    pub latency_ms: Option<u64>,
    pub events_received: usize,
    pub last_seen: Option<Instant>,
}

pub struct ClientPool {
    relays: Arc<RwLock<HashMap<String, RelayStatus>>>,
    publish_broadcast_tx: tokio::sync::broadcast::Sender<Event>,
    inbound_event_tx: mpsc::UnboundedSender<Event>,
}

impl ClientPool {
    pub fn new(inbound_event_tx: mpsc::UnboundedSender<Event>) -> Self {
        let (publish_broadcast_tx, _) = tokio::sync::broadcast::channel(256);
        Self {
            relays: Arc::new(RwLock::new(HashMap::new())),
            publish_broadcast_tx,
            inbound_event_tx,
        }
    }

    pub fn add_relay(&self, url: String) {
        let relays_map = Arc::clone(&self.relays);
        let inbound_tx = self.inbound_event_tx.clone();
        let publish_rx = self.publish_broadcast_tx.subscribe();

        tokio::spawn(async move {
            {
                let mut map = relays_map.write().await;
                if map.contains_key(&url) {
                    return;
                }
                map.insert(url.clone(), RelayStatus {
                    url: url.clone(),
                    connected: false,
                    latency_ms: None,
                    events_received: 0,
                    last_seen: None,
                });
            }

            relay_worker(url, relays_map, inbound_tx, publish_rx).await;
        });
    }

    pub fn publish(&self, event: Event) {
        let _ = self.publish_broadcast_tx.send(event);
    }

    pub async fn statuses(&self) -> Vec<RelayStatus> {
        let map = self.relays.read().await;
        map.values().cloned().collect()
    }
}

async fn relay_worker(
    url: String,
    relays_map: Arc<RwLock<HashMap<String, RelayStatus>>>,
    inbound_tx: mpsc::UnboundedSender<Event>,
    mut publish_rx: tokio::sync::broadcast::Receiver<Event>,
) {
    loop {
        let start = Instant::now();
        match tokio_tungstenite::connect_async(&url).await {
            Ok((ws_stream, _)) => {
                let latency = start.elapsed().as_millis() as u64;
                {
                    let mut map = relays_map.write().await;
                    if let Some(st) = map.get_mut(&url) {
                        st.connected = true;
                        st.latency_ms = Some(latency);
                        st.last_seen = Some(Instant::now());
                    }
                }

                let (mut ws_sender, mut ws_receiver) = ws_stream.split();

                let sub_req = ClientMessage::req(
                    SubscriptionId::new("feed"),
                    vec![Filter::new().kind(Kind::TextNote).limit(200)],
                );
                let _ = ws_sender.send(Message::text(sub_req.as_json())).await;

                loop {
                    tokio::select! {
                        pub_res = publish_rx.recv() => {
                            match pub_res {
                                Ok(event) => {
                                    let msg = ClientMessage::event(event);
                                    if ws_sender.send(Message::text(msg.as_json())).await.is_err() {
                                        break;
                                    }
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                                Err(_) => break,
                            }
                        }
                        recv_res = ws_receiver.next() => {
                            match recv_res {
                                Some(Ok(ws_msg)) => {
                                    if ws_msg.is_close() {
                                        break;
                                    }
                                    if let Ok(text) = ws_msg.to_text() {
                                        if let Ok(relay_msg) = RelayMessage::from_json(text) {
                                            if let RelayMessage::Event { event, .. } = relay_msg {
                                                let ev = event.into_owned();
                                                let _ = inbound_tx.send(ev);
                                                let mut map = relays_map.write().await;
                                                if let Some(st) = map.get_mut(&url) {
                                                    st.events_received += 1;
                                                    st.last_seen = Some(Instant::now());
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => break,
                            }
                        }
                    }
                }

                {
                    let mut map = relays_map.write().await;
                    if let Some(st) = map.get_mut(&url) {
                        st.connected = false;
                    }
                }
            }
            Err(_) => {
                {
                    let mut map = relays_map.write().await;
                    if let Some(st) = map.get_mut(&url) {
                        st.connected = false;
                        st.latency_ms = None;
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
