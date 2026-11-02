use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use futures_util::{SinkExt, StreamExt};
use nostr::filter::MatchEventOptions;
use nostr::message::relay::RelayMessage;
use nostr::message::ClientMessage;
use nostr::prelude::*;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;

static NEXT_SUB_ID: AtomicUsize = AtomicUsize::new(1);

struct Subscription {
    id: SubscriptionId,
    filters: Vec<Filter>,
    sender: mpsc::UnboundedSender<Message>,
}

#[derive(Clone)]
pub struct RelayStore {
    events: Arc<RwLock<Vec<Event>>>,
    subs: Arc<RwLock<HashMap<usize, Subscription>>>,
    inbound_event_tx: mpsc::UnboundedSender<(Event, String)>,
}

impl RelayStore {
    pub fn new(inbound_event_tx: mpsc::UnboundedSender<(Event, String)>) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::with_capacity(1000))),
            subs: Arc::new(RwLock::new(HashMap::new())),
            inbound_event_tx,
        }
    }

    pub async fn add_event(&self, event: Event) -> Result<bool, &'static str> {
        if event.verify().is_err() {
            return Err("invalid signature");
        }

        let mut events = self.events.write().await;
        if events.iter().any(|e| e.id == event.id) {
            return Ok(false);
        }

        if events.len() >= 2000 {
            events.remove(0);
        }
        events.push(event.clone());
        drop(events);

        let _ = self.inbound_event_tx.send((event.clone(), "embedded".to_string()));
        self.broadcast_event(&event).await;

        Ok(true)
    }

    pub async fn register_sub(&self, sub_id: SubscriptionId, filters: Vec<Filter>, tx: mpsc::UnboundedSender<Message>) -> usize {
        let internal_id = NEXT_SUB_ID.fetch_add(1, Ordering::Relaxed);

        let events = self.events.read().await;
        let match_opts = MatchEventOptions::new();
        for event in events.iter() {
            let matched = filters.is_empty() || filters.iter().any(|f| f.match_event(event, match_opts));
            if matched {
                let msg = RelayMessage::event(sub_id.clone(), event.clone());
                let _ = tx.send(Message::text(msg.as_json()));
            }
        }
        let eose = RelayMessage::eose(sub_id.clone());
        let _ = tx.send(Message::text(eose.as_json()));

        let mut subs = self.subs.write().await;
        subs.insert(internal_id, Subscription {
            id: sub_id,
            filters,
            sender: tx,
        });

        internal_id
    }

    pub async fn unregister_sub(&self, internal_id: usize) {
        let mut subs = self.subs.write().await;
        subs.remove(&internal_id);
    }

    pub async fn close_sub_by_name(&self, sub_id: &SubscriptionId) {
        let mut subs = self.subs.write().await;
        subs.retain(|_, s| &s.id != sub_id);
    }

    pub async fn broadcast_event(&self, event: &Event) {
        let subs = self.subs.read().await;
        let match_opts = MatchEventOptions::new();
        for sub in subs.values() {
            let matched = sub.filters.is_empty() || sub.filters.iter().any(|f| f.match_event(event, match_opts));
            if matched {
                let msg = RelayMessage::event(sub.id.clone(), event.clone());
                let _ = sub.sender.send(Message::text(msg.as_json()));
            }
        }
    }
}

#[allow(dead_code)]
pub struct EmbeddedRelay {
    pub bind_addr: SocketAddr,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl EmbeddedRelay {
    pub async fn start(port: u16, inbound_event_tx: mpsc::UnboundedSender<(Event, String)>) -> Result<(Self, RelayStore), Box<dyn std::error::Error + Send + Sync>> {
        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
        let listener = TcpListener::bind(addr).await?;
        let actual_addr = listener.local_addr()?;
        let store = RelayStore::new(inbound_event_tx);
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let store_clone = store.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        break;
                    }
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((stream, peer_addr)) => {
                                let conn_store = store_clone.clone();
                                tokio::spawn(handle_connection(stream, peer_addr, conn_store));
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        });

        Ok((Self {
            bind_addr: actual_addr,
            shutdown_tx: Some(shutdown_tx),
        }, store))
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

async fn handle_connection(stream: TcpStream, _peer_addr: SocketAddr, store: RelayStore) {
    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(_) => return,
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut registered_ids = Vec::new();

    while let Some(msg_res) = ws_receiver.next().await {
        let msg = match msg_res {
            Ok(m) => m,
            Err(_) => break,
        };

        if msg.is_close() {
            break;
        }

        if let Ok(text) = msg.to_text() {
            if let Ok(client_msg) = ClientMessage::from_json(text) {
                match client_msg {
                    ClientMessage::Event(ev) => {
                        let ev_id = ev.id;
                        match store.add_event(ev.into_owned()).await {
                            Ok(_) => {
                                let ok_msg = RelayMessage::ok(ev_id, true, "");
                                let _ = tx.send(Message::text(ok_msg.as_json()));
                            }
                            Err(e) => {
                                let ok_msg = RelayMessage::ok(ev_id, false, e);
                                let _ = tx.send(Message::text(ok_msg.as_json()));
                            }
                        }
                    }
                    ClientMessage::Req { subscription_id, filters } => {
                        let owned_filters: Vec<Filter> = filters.into_iter().map(|f| f.into_owned()).collect();
                        let sub_id = subscription_id.into_owned();
                        let int_id = store.register_sub(sub_id, owned_filters, tx.clone()).await;
                        registered_ids.push(int_id);
                    }
                    ClientMessage::Close(sub_id) => {
                        store.close_sub_by_name(&sub_id).await;
                    }
                    _ => {}
                }
            }
        }
    }

    for id in registered_ids {
        store.unregister_sub(id).await;
    }

    send_task.abort();
}
