use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nostr::event::FinalizeEvent;
use nostr::nips::nip01::Metadata;
use nostr::prelude::*;
use tokio::sync::mpsc;

use crate::client::{ClientPool, RelayStatus};
use crate::discovery::{DiscoveryManager, PeerNode};
use crate::identity::Identity;
use crate::outbox::Outbox;
use crate::relay::{EmbeddedRelay, RelayStore};

pub const DEFAULT_PUBLIC_RELAYS: &[&str] = &[
    "wss://relay.damus.io",
    "wss://nos.lol",
    "wss://relay.primal.net",
];

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum InputMode {
    Normal,
    Editing,
}

pub struct App {
    pub identity: Identity,
    pub relay_port: u16,
    pub embedded_relay: Option<EmbeddedRelay>,
    pub relay_store: Option<RelayStore>,
    pub client_pool: ClientPool,
    pub _discovery: Arc<DiscoveryManager>,
    pub outbox: Outbox,
    pub timeline: Vec<Event>,
    pub profiles: HashMap<PublicKey, String>,
    pub discovered_peers: Vec<PeerNode>,
    pub relay_statuses: Vec<RelayStatus>,
    pub public_relays_active: bool,
    pub input_buffer: String,
    pub input_mode: InputMode,
    pub status_toast: Option<(String, Instant)>,
    pub should_quit: bool,
    inbound_event_tx: mpsc::UnboundedSender<Event>,
    inbound_event_rx: mpsc::UnboundedReceiver<Event>,
    peer_rx: mpsc::UnboundedReceiver<PeerNode>,
}

impl App {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let identity = Identity::load_or_create()?;
        let relay_port = 4869;

        let (inbound_event_tx, inbound_event_rx) = mpsc::unbounded_channel::<Event>();
        let (peer_tx, peer_rx) = mpsc::unbounded_channel::<PeerNode>();

        let client_pool = ClientPool::new(inbound_event_tx.clone());
        let discovery = Arc::new(DiscoveryManager::new(peer_tx));

        discovery.clone().start(
            identity.name.clone(),
            relay_port,
            identity.npub.clone(),
        );

        let outbox = Outbox::new();

        let (embedded_relay, relay_store) = match EmbeddedRelay::start(relay_port, inbound_event_tx.clone()).await {
            Ok((relay, store)) => {
                client_pool.add_relay(format!("ws://127.0.0.1:{}", relay_port));
                (Some(relay), Some(store))
            }
            Err(_) => (None, None),
        };

        let mut profiles = HashMap::new();
        profiles.insert(identity.keys.public_key(), identity.name.clone());

        let mut app = Self {
            identity,
            relay_port,
            embedded_relay,
            relay_store,
            client_pool,
            _discovery: discovery,
            outbox,
            timeline: Vec::new(),
            profiles,
            discovered_peers: Vec::new(),
            relay_statuses: Vec::new(),
            public_relays_active: false,
            input_buffer: String::new(),
            input_mode: InputMode::Editing,
            status_toast: Some(("Node initialized on intranet".to_string(), Instant::now())),
            should_quit: false,
            inbound_event_tx,
            inbound_event_rx,
            peer_rx,
        };

        app.broadcast_metadata().await;
        app.flush_outbox().await;

        Ok(app)
    }

    pub async fn tick(&mut self) {
        while let Ok(event) = self.inbound_event_rx.try_recv() {
            if event.kind == Kind::Metadata {
                if let Ok(meta) = Metadata::from_json(&event.content) {
                    if let Some(name) = meta.name.or(meta.display_name) {
                        self.profiles.insert(event.pubkey, name);
                    }
                }
            } else if event.kind == Kind::TextNote {
                if !self.timeline.iter().any(|e| e.id == event.id) {
                    self.outbox.remove(&event.id);
                    self.timeline.push(event);
                    self.timeline.sort_by_key(|e| e.created_at);
                }
            }
        }

        while let Ok(peer) = self.peer_rx.try_recv() {
            let exists = self.discovered_peers.iter().any(|p| p.ws_url == peer.ws_url);
            if !exists {
                self.client_pool.add_relay(peer.ws_url.clone());
                self.status_toast = Some((
                    format!("Discovered peer {} [{}]", peer.ws_url, peer.name),
                    Instant::now(),
                ));
                self.discovered_peers.push(peer);
            }
        }

        self.relay_statuses = self.client_pool.statuses().await;

        if let Some((_, time)) = self.status_toast {
            if time.elapsed().as_secs() > 4 {
                self.status_toast = None;
            }
        }
    }

    pub async fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('i') | KeyCode::Enter => self.input_mode = InputMode::Editing,
                KeyCode::Char('r') | KeyCode::F(2) => self.toggle_relay().await,
                KeyCode::Char('p') => self.toggle_public_relays().await,
                KeyCode::Char('c') => {
                    self.timeline.clear();
                    self.status_toast = Some(("Feed cleared".to_string(), Instant::now()));
                }
                KeyCode::Tab => self.input_mode = InputMode::Editing,
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Enter => self.submit_input().await,
                KeyCode::Esc => self.input_mode = InputMode::Normal,
                KeyCode::Tab => self.input_mode = InputMode::Normal,
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                }
                _ => {}
            },
        }
    }

    async fn submit_input(&mut self) {
        let text = self.input_buffer.trim().to_string();
        if text.is_empty() {
            return;
        }
        self.input_buffer.clear();

        if text.starts_with('/') {
            self.handle_command(&text).await;
            return;
        }

        let event = match EventBuilder::new(Kind::TextNote, &text).finalize(&self.identity.keys) {
            Ok(ev) => ev,
            Err(_) => return,
        };

        if let Some(store) = &self.relay_store {
            let _ = store.add_event(event.clone()).await;
        }

        self.client_pool.publish(event.clone());

        let connected_count = self.relay_statuses.iter().filter(|r| r.connected).count();
        if connected_count == 0 && self.embedded_relay.is_none() {
            self.outbox.push(event.clone());
            self.status_toast = Some(("Saved to offline outbox".to_string(), Instant::now()));
        } else {
            self.status_toast = Some(("Transmitted note".to_string(), Instant::now()));
        }

        if !self.timeline.iter().any(|e| e.id == event.id) {
            self.timeline.push(event);
            self.timeline.sort_by_key(|e| e.created_at);
        }
    }

    async fn handle_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        match parts.as_slice() {
            ["/name", new_name] => {
                let name = new_name.to_string();
                if let Ok(()) = self.identity.set_name(name.clone()) {
                    self.profiles.insert(self.identity.keys.public_key(), name.clone());
                    self.broadcast_metadata().await;
                    self.status_toast = Some((format!("Station name set to '{}'", name), Instant::now()));
                }
            }
            ["/relay", "add", url] => {
                let relay_url = url.to_string();
                self.client_pool.add_relay(relay_url.clone());
                self.status_toast = Some((format!("Added relay {}", relay_url), Instant::now()));
            }
            ["/relay", "public"] => {
                self.toggle_public_relays().await;
            }
            ["/help"] => {
                self.status_toast = Some(("Commands: /name <nick> | /relay add <url> | /relay public".to_string(), Instant::now()));
            }
            _ => {
                self.status_toast = Some(("Unknown command. Type /help".to_string(), Instant::now()));
            }
        }
    }

    pub async fn toggle_public_relays(&mut self) {
        self.public_relays_active = !self.public_relays_active;
        if self.public_relays_active {
            for &relay in DEFAULT_PUBLIC_RELAYS {
                self.client_pool.add_relay(relay.to_string());
            }
            self.status_toast = Some(("Connecting to public internet relays...".to_string(), Instant::now()));
        } else {
            self.status_toast = Some(("Public internet relays disabled".to_string(), Instant::now()));
        }
    }

    async fn broadcast_metadata(&mut self) {
        let meta = Metadata::new().name(&self.identity.name);
        if let Ok(ev) = meta.finalize(&self.identity.keys) {
            if let Some(store) = &self.relay_store {
                let _ = store.add_event(ev.clone()).await;
            }
            self.client_pool.publish(ev);
        }
    }

    pub async fn toggle_relay(&mut self) {
        if let Some(mut relay) = self.embedded_relay.take() {
            relay.stop();
            self.relay_store = None;
            self.status_toast = Some(("Embedded relay stopped".to_string(), Instant::now()));
        } else {
            match EmbeddedRelay::start(self.relay_port, self.inbound_event_tx.clone()).await {
                Ok((relay, store)) => {
                    self.client_pool.add_relay(format!("ws://127.0.0.1:{}", self.relay_port));
                    self.embedded_relay = Some(relay);
                    self.relay_store = Some(store);
                    self.status_toast = Some((
                        format!("Embedded relay running on :{}", self.relay_port),
                        Instant::now(),
                    ));
                    self.flush_outbox().await;
                }
                Err(_) => {
                    self.status_toast = Some(("Failed to bind relay port".to_string(), Instant::now()));
                }
            }
        }
    }

    async fn flush_outbox(&mut self) {
        let pending = self.outbox.pending();
        if pending.is_empty() {
            return;
        }

        for ev in pending {
            if let Some(store) = &self.relay_store {
                let _ = store.add_event(ev.clone()).await;
            }
            self.client_pool.publish(ev);
        }
    }
}
