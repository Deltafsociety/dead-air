use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nostr::event::FinalizeEvent;
use nostr::nips::nip01::Metadata;
use nostr::prelude::*;
use tokio::sync::mpsc;

use crate::bookmarks::BookmarkManager;
use crate::client::{ClientPool, RelayStatus};
use crate::discovery::{DiscoveryManager, PeerNode};
use crate::identity::Identity;
use crate::net_info;
use crate::outbox::Outbox;
use crate::relay::{EmbeddedRelay, RelayStore};

pub const DEFAULT_PUBLIC_RELAYS: &[&str] = &[
    "wss://relay.damus.io",
    "wss://nos.lol",
    "wss://relay.primal.net",
];

#[derive(Clone, Debug, Default)]
pub struct ProfileInfo {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub nip05: Option<String>,
    pub about: Option<String>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ActiveTab {
    Station,
    Public,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
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
    pub bookmarks: BookmarkManager,
    pub detected_ip: String,
    pub share_address: String,
    pub active_tab: ActiveTab,
    pub timeline: Vec<Event>,
    pub public_timeline: Vec<Event>,
    pub profiles: HashMap<PublicKey, String>,
    pub profile_details: HashMap<PublicKey, ProfileInfo>,
    pub post_likes: HashMap<EventId, usize>,
    pub post_replies: HashMap<EventId, usize>,
    pub post_reposts: HashMap<EventId, usize>,
    pub my_likes: HashSet<EventId>,
    pub selected_public_index: Option<usize>,
    pub replying_to: Option<Event>,
    pub discovered_peers: Vec<PeerNode>,
    pub relay_statuses: Vec<RelayStatus>,
    pub public_relays_active: bool,
    pub input_buffer: String,
    pub input_mode: InputMode,
    pub status_toast: Option<(String, Instant)>,
    pub should_quit: bool,
    inbound_event_tx: mpsc::UnboundedSender<(Event, String)>,
    inbound_event_rx: mpsc::UnboundedReceiver<(Event, String)>,
    peer_rx: mpsc::UnboundedReceiver<PeerNode>,
    ip_rx: mpsc::UnboundedReceiver<(String, String)>,
    ip_tx: mpsc::UnboundedSender<(String, String)>,
}

impl App {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let identity = Identity::load_or_create()?;
        let relay_port = 4869;

        let (inbound_event_tx, inbound_event_rx) = mpsc::unbounded_channel::<(Event, String)>();
        let (peer_tx, peer_rx) = mpsc::unbounded_channel::<PeerNode>();
        let (ip_tx, ip_rx) = mpsc::unbounded_channel::<(String, String)>();

        let client_pool = ClientPool::new(inbound_event_tx.clone());
        let discovery = Arc::new(DiscoveryManager::new(peer_tx));

        discovery.clone().start(
            identity.name.clone(),
            relay_port,
            identity.npub.clone(),
        );

        let outbox = Outbox::new();
        let bookmarks = BookmarkManager::load();

        let (embedded_relay, relay_store) = match EmbeddedRelay::start(relay_port, inbound_event_tx.clone()).await {
            Ok((relay, store)) => {
                client_pool.add_relay(format!("ws://127.0.0.1:{}", relay_port));
                (Some(relay), Some(store))
            }
            Err(_) => (None, None),
        };

        for url in bookmarks.auto_connect_urls() {
            client_pool.add_relay(url);
        }

        let mut profiles = HashMap::new();
        profiles.insert(identity.keys.public_key(), identity.name.clone());

        let initial_ip = "detecting...".to_string();
        let initial_share = format!("ws://127.0.0.1:{}", relay_port);

        let ip_tx_spawn = ip_tx.clone();
        tokio::spawn(async move {
            let res = net_info::resolve_best_address(relay_port).await;
            let _ = ip_tx_spawn.send(res);
        });

        let mut app = Self {
            identity,
            relay_port,
            embedded_relay,
            relay_store,
            client_pool,
            _discovery: discovery,
            outbox,
            bookmarks,
            detected_ip: initial_ip,
            share_address: initial_share,
            active_tab: ActiveTab::Station,
            timeline: Vec::new(),
            public_timeline: Vec::new(),
            profiles,
            profile_details: HashMap::new(),
            post_likes: HashMap::new(),
            post_replies: HashMap::new(),
            post_reposts: HashMap::new(),
            my_likes: HashSet::new(),
            selected_public_index: None,
            replying_to: None,
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
            ip_rx,
            ip_tx,
        };

        app.broadcast_metadata().await;
        app.flush_outbox().await;

        Ok(app)
    }

    pub async fn tick(&mut self) {
        while let Ok((ip, share_url)) = self.ip_rx.try_recv() {
            self.detected_ip = ip;
            self.share_address = share_url;
            self.status_toast = Some((
                format!("Detected domestic address: {}", self.share_address),
                Instant::now(),
            ));
        }

        while let Ok((event, source_url)) = self.inbound_event_rx.try_recv() {
            if event.kind == Kind::Metadata {
                if let Ok(meta) = Metadata::from_json(&event.content) {
                    let display = meta.display_name.clone().or_else(|| meta.name.clone());
                    if let Some(name) = display {
                        self.profiles.insert(event.pubkey, name);
                    }
                    self.profile_details.insert(event.pubkey, ProfileInfo {
                        name: meta.name,
                        display_name: meta.display_name,
                        nip05: meta.nip05,
                        about: meta.about,
                    });
                }
            } else if event.kind == Kind::Reaction {
                for target_id in event.tags.event_ids() {
                    *self.post_likes.entry(target_id).or_insert(0) += 1;
                }
            } else if event.kind == Kind::Repost {
                for target_id in event.tags.event_ids() {
                    *self.post_reposts.entry(target_id).or_insert(0) += 1;
                }
            } else if event.kind == Kind::TextNote {
                self.outbox.remove(&event.id);

                for target_id in event.tags.event_ids() {
                    *self.post_replies.entry(target_id).or_insert(0) += 1;
                }

                if self.is_deadair_event(&event, Some(&source_url)) {
                    if !self.timeline.iter().any(|e| e.id == event.id) {
                        self.timeline.push(event);
                        self.timeline.sort_by_key(|e| e.created_at);
                        if self.timeline.len() > 1000 {
                            self.timeline.remove(0);
                        }
                    }
                } else {
                    if !self.public_timeline.iter().any(|e| e.id == event.id) {
                        self.public_timeline.push(event);
                        self.public_timeline.sort_by_key(|e| e.created_at);
                        if self.public_timeline.len() > 1000 {
                            self.public_timeline.remove(0);
                        }
                    }
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

    pub fn is_deadair_event(&self, event: &Event, source_url: Option<&str>) -> bool {
        // 1. Authored by the local user
        if event.pubkey == self.identity.keys.public_key() {
            return true;
        }

        // 2. Contains standard DEADAIR client tag
        let has_deadair_tag = event.tags.iter().any(|t| {
            let slice = t.as_slice();
            slice.len() >= 2 && slice[0].eq_ignore_ascii_case("client") && slice[1].eq_ignore_ascii_case("deadair")
        });
        if has_deadair_tag {
            return true;
        }

        // 3. Authored by a known LAN peer discovered via UDP beacon
        if self.discovered_peers.iter().any(|p| {
            p.name == event.pubkey.to_hex()
                || self.profiles.get(&event.pubkey).map_or(false, |name| name == &p.name)
        }) {
            return true;
        }

        // 4. Delivered from domestic/embedded source (embedded relay, localhost, or domestic bookmark)
        if let Some(url) = source_url {
            if url == "embedded" || url.contains("127.0.0.1") || url.contains("localhost") {
                return true;
            }
            let is_public = DEFAULT_PUBLIC_RELAYS.contains(&url);
            if !is_public {
                return true;
            }
        }

        false
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
                KeyCode::Char('r') => {
                    if self.active_tab == ActiveTab::Public && !self.public_timeline.is_empty() {
                        self.start_reply_to_selected();
                    } else {
                        self.toggle_relay().await;
                    }
                }
                KeyCode::Char('R') | KeyCode::F(2) => self.toggle_relay().await,
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    self.like_selected_post().await;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if self.active_tab == ActiveTab::Public && !self.public_timeline.is_empty() {
                        match self.selected_public_index {
                            None => {
                                self.selected_public_index = Some(self.public_timeline.len().saturating_sub(1));
                            }
                            Some(cur) => {
                                if cur + 1 < self.public_timeline.len() {
                                    self.selected_public_index = Some(cur + 1);
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if self.active_tab == ActiveTab::Public && !self.public_timeline.is_empty() {
                        match self.selected_public_index {
                            None => {
                                self.selected_public_index = Some(self.public_timeline.len().saturating_sub(2));
                            }
                            Some(cur) => {
                                if cur > 0 {
                                    self.selected_public_index = Some(cur - 1);
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('g') | KeyCode::Home => {
                    if self.active_tab == ActiveTab::Public {
                        self.selected_public_index = None;
                        self.status_toast = Some(("Resumed live stream".to_string(), Instant::now()));
                    }
                }
                KeyCode::Char('G') | KeyCode::End => {
                    if self.active_tab == ActiveTab::Public && !self.public_timeline.is_empty() {
                        self.selected_public_index = Some(0);
                    }
                }
                KeyCode::Char('p') => self.toggle_public_relays().await,
                KeyCode::Char('b') => self.connect_bookmarks().await,
                KeyCode::Char('s') => self.show_share_toast(),
                KeyCode::Char('c') => {
                    match self.active_tab {
                        ActiveTab::Station => {
                            self.timeline.clear();
                            self.status_toast = Some(("Station feed cleared".to_string(), Instant::now()));
                        }
                        ActiveTab::Public => {
                            self.public_timeline.clear();
                            self.selected_public_index = None;
                            self.status_toast = Some(("Public feed cleared".to_string(), Instant::now()));
                        }
                    }
                }
                KeyCode::Tab | KeyCode::Char('t') => {
                    self.active_tab = match self.active_tab {
                        ActiveTab::Station => ActiveTab::Public,
                        ActiveTab::Public => ActiveTab::Station,
                    };
                    self.status_toast = Some((
                        format!("Switched to {} tab", match self.active_tab {
                            ActiveTab::Station => "DEADAIR Station",
                            ActiveTab::Public => "Public Relays",
                        }),
                        Instant::now(),
                    ));
                }
                KeyCode::Char('1') | KeyCode::Left => {
                    self.active_tab = ActiveTab::Station;
                    self.status_toast = Some(("Switched to DEADAIR Station tab".to_string(), Instant::now()));
                }
                KeyCode::Char('2') | KeyCode::Right => {
                    self.active_tab = ActiveTab::Public;
                    self.status_toast = Some(("Switched to Public Relays tab".to_string(), Instant::now()));
                }
                KeyCode::Esc => {
                    if self.selected_public_index.is_some() {
                        self.selected_public_index = None;
                        self.status_toast = Some(("Resumed live stream".to_string(), Instant::now()));
                    }
                }
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Enter => self.submit_input().await,
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.replying_to = None;
                }
                KeyCode::Tab => {
                    self.input_mode = InputMode::Normal;
                }
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

    pub async fn like_selected_post(&mut self) {
        let target_event = match self.active_tab {
            ActiveTab::Public => {
                if let Some(idx) = self.selected_public_index {
                    self.public_timeline.get(idx).cloned()
                } else {
                    self.public_timeline.last().cloned()
                }
            }
            ActiveTab::Station => self.timeline.last().cloned(),
        };

        if let Some(target) = target_event {
            if self.my_likes.contains(&target.id) {
                self.status_toast = Some(("You already liked this note".to_string(), Instant::now()));
                return;
            }

            let reaction = match EventBuilder::new(Kind::Reaction, "❤️")
                .tag(Tag::event(target.id))
                .tag(Tag::public_key(target.pubkey))
                .finalize(&self.identity.keys)
            {
                Ok(ev) => ev,
                Err(_) => return,
            };

            if let Some(store) = &self.relay_store {
                let _ = store.add_event(reaction.clone()).await;
            }
            self.client_pool.publish(reaction);
            self.my_likes.insert(target.id);
            *self.post_likes.entry(target.id).or_insert(0) += 1;

            let author_name = self.profiles.get(&target.pubkey).cloned().unwrap_or_else(|| {
                let hex = target.pubkey.to_hex();
                format!("{}..{}", &hex[..6], &hex[hex.len() - 4..])
            });
            self.status_toast = Some((format!("Liked note by {} ❤️", author_name), Instant::now()));
        }
    }

    pub fn start_reply_to_selected(&mut self) {
        let target_event = match self.active_tab {
            ActiveTab::Public => {
                if let Some(idx) = self.selected_public_index {
                    self.public_timeline.get(idx).cloned()
                } else {
                    self.public_timeline.last().cloned()
                }
            }
            ActiveTab::Station => self.timeline.last().cloned(),
        };

        if let Some(target) = target_event {
            let author_name = self.profiles.get(&target.pubkey).cloned().unwrap_or_else(|| {
                let hex = target.pubkey.to_hex();
                format!("{}..{}", &hex[..6], &hex[hex.len() - 4..])
            });
            self.replying_to = Some(target);
            self.input_mode = InputMode::Editing;
            self.input_buffer.clear();
            self.status_toast = Some((format!("Replying to {} (type message & press Enter)", author_name), Instant::now()));
        }
    }

    async fn send_note(&mut self, text: String) {
        let deadair_tag = Tag::parse(["client", "deadair"]).unwrap_or_else(|_| Tag::hashtag("deadair"));
        let mut builder = EventBuilder::new(Kind::TextNote, &text).tag(deadair_tag);

        if let Some(target) = self.replying_to.take() {
            builder = builder
                .tag(Tag::event(target.id))
                .tag(Tag::public_key(target.pubkey));
            *self.post_replies.entry(target.id).or_insert(0) += 1;
        }

        let event = match builder.finalize(&self.identity.keys) {
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
            self.status_toast = Some(("Transmitted note to network".to_string(), Instant::now()));
        }

        if !self.timeline.iter().any(|e| e.id == event.id) {
            self.timeline.push(event);
            self.timeline.sort_by_key(|e| e.created_at);
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

        self.send_note(text).await;
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
            ["/share"] => {
                self.show_share_toast();
            }
            ["/ip"] => {
                let ip_tx_spawn = self.ip_tx.clone();
                let port = self.relay_port;
                tokio::spawn(async move {
                    let res = net_info::resolve_best_address(port).await;
                    let _ = ip_tx_spawn.send(res);
                });
                self.status_toast = Some(("Detecting domestic network address...".to_string(), Instant::now()));
            }
            ["/tab"] => {
                self.active_tab = match self.active_tab {
                    ActiveTab::Station => ActiveTab::Public,
                    ActiveTab::Public => ActiveTab::Station,
                };
                self.status_toast = Some((
                    format!("Switched to {} tab", match self.active_tab {
                        ActiveTab::Station => "DEADAIR Station",
                        ActiveTab::Public => "Public Relays",
                    }),
                    Instant::now(),
                ));
            }
            ["/station"] | ["/tab", "station"] | ["/tab", "1"] | ["/local"] => {
                self.active_tab = ActiveTab::Station;
                self.status_toast = Some(("Switched to DEADAIR Station tab".to_string(), Instant::now()));
            }
            ["/public"] | ["/tab", "public"] | ["/tab", "2"] | ["/global"] => {
                self.active_tab = ActiveTab::Public;
                self.status_toast = Some(("Switched to Public Relays tab".to_string(), Instant::now()));
            }
            ["/clear"] => {
                match self.active_tab {
                    ActiveTab::Station => {
                        self.timeline.clear();
                        self.status_toast = Some(("Station feed cleared".to_string(), Instant::now()));
                    }
                    ActiveTab::Public => {
                        self.public_timeline.clear();
                        self.status_toast = Some(("Public feed cleared".to_string(), Instant::now()));
                    }
                }
            }
            ["/bookmark", "add", url] => {
                let name = format!("node-{}", &url[url.len().saturating_sub(6)..]);
                self.bookmarks.add(url.to_string(), name);
                self.client_pool.add_relay(url.to_string());
                self.status_toast = Some((format!("Added bookmark {}", url), Instant::now()));
            }
            ["/bookmark", "add", url, name] => {
                self.bookmarks.add(url.to_string(), name.to_string());
                self.client_pool.add_relay(url.to_string());
                self.status_toast = Some((format!("Added bookmark {} [{}]", url, name), Instant::now()));
            }
            ["/bookmark", "remove", key] => {
                if self.bookmarks.remove(key) {
                    self.status_toast = Some((format!("Removed bookmark {}", key), Instant::now()));
                } else {
                    self.status_toast = Some((format!("Bookmark not found: {}", key), Instant::now()));
                }
            }
            ["/bookmark", "connect"] => {
                self.connect_bookmarks().await;
            }
            ["/relay", "add", url] => {
                let relay_url = url.to_string();
                self.client_pool.add_relay(relay_url.clone());
                self.bookmarks.add(relay_url.clone(), "manual-node".to_string());
                self.status_toast = Some((format!("Added and bookmarked relay {}", relay_url), Instant::now()));
            }
            ["/relay", "public"] => {
                self.toggle_public_relays().await;
            }
            ["/like"] => {
                self.like_selected_post().await;
            }
            ["/reply"] => {
                self.start_reply_to_selected();
            }
            _ if cmd.starts_with("/reply ") => {
                let text = cmd.strip_prefix("/reply ").unwrap_or("").trim().to_string();
                if !text.is_empty() {
                    self.start_reply_to_selected();
                    self.send_note(text).await;
                }
            }
            ["/help"] => {
                self.status_toast = Some(("Commands: /tab [station|public] | /like | /reply <msg> | /share | /name <nick> | /clear".to_string(), Instant::now()));
            }
            _ => {
                self.status_toast = Some(("Unknown command. Type /help".to_string(), Instant::now()));
            }
        }
    }

    pub fn show_share_toast(&mut self) {
        self.status_toast = Some((
            format!("Share with peers: {} [{}]", self.share_address, self.identity.name),
            Instant::now(),
        ));
    }

    pub async fn connect_bookmarks(&mut self) {
        let urls = self.bookmarks.auto_connect_urls();
        let count = urls.len();
        for url in urls {
            self.client_pool.add_relay(url);
        }
        self.status_toast = Some((
            format!("Connecting to {} domestic bookmarked relays", count),
            Instant::now(),
        ));
    }

    pub async fn toggle_public_relays(&mut self) {
        self.public_relays_active = !self.public_relays_active;
        if self.public_relays_active {
            for &relay in DEFAULT_PUBLIC_RELAYS {
                self.client_pool.add_relay(relay.to_string());
            }
            self.status_toast = Some(("Public relays active (viewing in [2] Public Relays tab)".to_string(), Instant::now()));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_classification_and_tagging() {
        let app = App::new().await.expect("App should initialize");

        // 1. Local event authored by user
        let my_event = EventBuilder::new(Kind::TextNote, "hello from me")
            .finalize(&app.identity.keys)
            .unwrap();
        assert!(app.is_deadair_event(&my_event, Some("wss://relay.damus.io")));

        // 2. Foreign event with client=deadair tag
        let other_keys = Keys::generate();
        let tag = Tag::parse(["client", "deadair"]).unwrap();
        let deadair_foreign_event = EventBuilder::new(Kind::TextNote, "hello from deadair friend")
            .tag(tag)
            .finalize(&other_keys)
            .unwrap();
        assert!(app.is_deadair_event(&deadair_foreign_event, Some("wss://relay.damus.io")));

        // 3. Foreign event from domestic/embedded relay without deadair tag
        let domestic_foreign_event = EventBuilder::new(Kind::TextNote, "domestic lan node")
            .finalize(&other_keys)
            .unwrap();
        assert!(app.is_deadair_event(&domestic_foreign_event, Some("embedded")));
        assert!(app.is_deadair_event(&domestic_foreign_event, Some("ws://127.0.0.1:4869")));
        assert!(app.is_deadair_event(&domestic_foreign_event, Some("ws://192.168.1.50:4869")));

        // 4. Foreign event from public internet relay without deadair tag
        let public_event = EventBuilder::new(Kind::TextNote, "global public nostr tweet")
            .finalize(&other_keys)
            .unwrap();
        assert!(!app.is_deadair_event(&public_event, Some("wss://relay.damus.io")));
        assert!(!app.is_deadair_event(&public_event, Some("wss://nos.lol")));
        assert!(!app.is_deadair_event(&public_event, Some("wss://relay.primal.net")));
    }

    #[tokio::test]
    async fn test_tab_switching_command() {
        let mut app = App::new().await.expect("App should initialize");
        assert_eq!(app.active_tab, ActiveTab::Station);

        app.handle_command("/tab public").await;
        assert_eq!(app.active_tab, ActiveTab::Public);

        app.handle_command("/tab station").await;
        assert_eq!(app.active_tab, ActiveTab::Station);

        app.handle_command("/tab").await;
        assert_eq!(app.active_tab, ActiveTab::Public);
    }

    #[tokio::test]
    async fn test_social_browsing_like_and_reply() {
        let mut app = App::new().await.expect("App should initialize");
        app.active_tab = ActiveTab::Public;

        let other_keys = Keys::generate();
        let ev1 = EventBuilder::new(Kind::TextNote, "First public post")
            .finalize(&other_keys)
            .unwrap();
        let ev2 = EventBuilder::new(Kind::TextNote, "Second public post")
            .finalize(&other_keys)
            .unwrap();

        app.public_timeline.push(ev1.clone());
        app.public_timeline.push(ev2.clone());
        app.input_mode = InputMode::Normal;

        // Press 'k' to browse older posts (from live stream to index 0)
        app.handle_key(KeyEvent::from(KeyCode::Char('k'))).await;
        assert_eq!(app.selected_public_index, Some(0));

        // Press 'j' to move forward to index 1
        app.handle_key(KeyEvent::from(KeyCode::Char('j'))).await;
        assert_eq!(app.selected_public_index, Some(1));

        // Press 'l' to like the selected post (ev2)
        app.handle_key(KeyEvent::from(KeyCode::Char('l'))).await;
        assert!(app.my_likes.contains(&ev2.id));
        assert_eq!(app.post_likes.get(&ev2.id).copied(), Some(1));

        // Press 'r' to start replying to ev2
        app.handle_key(KeyEvent::from(KeyCode::Char('r'))).await;
        assert_eq!(app.input_mode, InputMode::Editing);
        assert_eq!(app.replying_to.as_ref().map(|e| e.id), Some(ev2.id));

        // Send the reply note
        app.send_note("Replying to second note".to_string()).await;
        assert_eq!(app.replying_to, None);
        assert_eq!(app.post_replies.get(&ev2.id).copied(), Some(1));

        // Verify sent reply has correct 'e' and 'p' tags
        let sent_reply = app.timeline.last().expect("Reply should be in timeline");
        let has_e_tag = sent_reply.tags.event_ids().any(|id| id == ev2.id);
        assert!(has_e_tag, "Reply event should tag parent event id");

        // Press Esc to return to Normal mode
        app.handle_key(KeyEvent::from(KeyCode::Esc)).await;
        assert_eq!(app.input_mode, InputMode::Normal);

        // Press 'g' to return to live stream
        app.handle_key(KeyEvent::from(KeyCode::Char('g'))).await;
        assert_eq!(app.selected_public_index, None);
    }

    #[tokio::test]
    async fn test_social_metrics_aggregation() {
        let mut app = App::new().await.expect("App should initialize");
        let other_keys = Keys::generate();
        let base_note = EventBuilder::new(Kind::TextNote, "Base public post")
            .finalize(&other_keys)
            .unwrap();
        app.public_timeline.push(base_note.clone());

        // Simulate incoming Kind::Reaction (Kind 7)
        let reaction = EventBuilder::new(Kind::Reaction, "+")
            .tag(Tag::event(base_note.id))
            .finalize(&other_keys)
            .unwrap();

        app.inbound_event_tx
            .send((reaction, "wss://nos.lol".to_string()))
            .unwrap();
        app.tick().await;
        assert_eq!(app.post_likes.get(&base_note.id).copied(), Some(1));

        // Simulate incoming Kind::Repost (Kind 6)
        let repost = EventBuilder::new(Kind::Repost, "")
            .tag(Tag::event(base_note.id))
            .finalize(&other_keys)
            .unwrap();

        app.inbound_event_tx
            .send((repost, "wss://nos.lol".to_string()))
            .unwrap();
        app.tick().await;
        assert_eq!(app.post_reposts.get(&base_note.id).copied(), Some(1));
    }
}
