use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
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
use crate::ticket::StationTicket;

#[derive(Clone, Debug, Default)]
pub struct ProfileInfo {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub nip05: Option<String>,
    pub about: Option<String>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ActiveTab {
    Main,
    Direct,
    Groups,
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
    pub safe_ticket: String,
    pub active_tab: ActiveTab,
    pub timeline: Vec<Event>,
    pub direct_messages: Vec<Event>,
    pub group_messages: Vec<Event>,
    pub current_group: String,
    pub selected_direct_peer: Option<PublicKey>,
    pub profiles: HashMap<PublicKey, String>,
    pub profile_details: HashMap<PublicKey, ProfileInfo>,
    pub post_likes: HashMap<EventId, usize>,
    pub post_replies: HashMap<EventId, usize>,
    pub post_reposts: HashMap<EventId, usize>,
    pub my_likes: HashSet<EventId>,
    pub selected_note_index: Option<usize>,
    pub replying_to: Option<Event>,
    pub discovered_peers: Vec<PeerNode>,
    pub relay_statuses: Vec<RelayStatus>,
    pub input_buffer: String,
    pub input_mode: InputMode,
    pub status_toast: Option<(String, Instant)>,
    pub last_presence_time: Instant,
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
        let safe_ticket = StationTicket::new(&initial_share, &identity.name, Some(identity.short_npub.clone())).encode();

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
            safe_ticket,
            active_tab: ActiveTab::Main,
            timeline: Vec::new(),
            direct_messages: Vec::new(),
            group_messages: Vec::new(),
            current_group: "general".to_string(),
            selected_direct_peer: None,
            profiles,
            profile_details: HashMap::new(),
            post_likes: HashMap::new(),
            post_replies: HashMap::new(),
            post_reposts: HashMap::new(),
            my_likes: HashSet::new(),
            selected_note_index: None,
            replying_to: None,
            discovered_peers: Vec::new(),
            relay_statuses: Vec::new(),
            input_buffer: String::new(),
            input_mode: InputMode::Editing,
            status_toast: Some(("DEADAIR station initialized on sovereign mesh".to_string(), Instant::now())),
            last_presence_time: Instant::now(),
            should_quit: false,
            inbound_event_tx,
            inbound_event_rx,
            peer_rx,
            ip_rx,
            ip_tx,
        };

        app.broadcast_metadata().await;
        app.broadcast_presence().await;
        app.flush_outbox().await;

        Ok(app)
    }

    pub fn current_notes(&self) -> &[Event] {
        match self.active_tab {
            ActiveTab::Main => &self.timeline,
            ActiveTab::Direct => &self.direct_messages,
            ActiveTab::Groups => &self.group_messages,
        }
    }

    pub fn current_notes_mut(&mut self) -> &mut Vec<Event> {
        match self.active_tab {
            ActiveTab::Main => &mut self.timeline,
            ActiveTab::Direct => &mut self.direct_messages,
            ActiveTab::Groups => &mut self.group_messages,
        }
    }

    pub async fn tick(&mut self) {
        while let Ok((ip, share_url)) = self.ip_rx.try_recv() {
            self.detected_ip = ip;
            self.share_address = share_url;
            self.safe_ticket = StationTicket::new(
                &self.share_address,
                &self.identity.name,
                Some(self.identity.short_npub.clone()),
            ).encode();
            self.status_toast = Some((
                format!("Detected domestic address: {}", self.share_address),
                Instant::now(),
            ));
            self.broadcast_presence().await;
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

                // Check for mesh peer presence gossip (discovers peers outside local network)
                let is_presence = event.tags.iter().any(|t| {
                    let s = t.as_slice();
                    s.len() >= 2 && s[0] == "type" && s[1] == "presence"
                });

                if is_presence {
                    let mut ws_addr = None;
                    let mut peer_name = None;
                    let mut ticket_str = None;

                    for t in event.tags.iter() {
                        let s = t.as_slice();
                        if s.len() >= 2 {
                            if s[0] == "address" { ws_addr = Some(s[1].to_string()); }
                            if s[0] == "name" { peer_name = Some(s[1].to_string()); }
                            if s[0] == "ticket" { ticket_str = Some(s[1].to_string()); }
                        }
                    }

                    if let Some(t_str) = ticket_str {
                        if let Some(decoded) = StationTicket::decode(&t_str) {
                            ws_addr = Some(decoded.ws);
                            peer_name = Some(decoded.name);
                        }
                    }

                    if let Some(addr) = ws_addr {
                        if event.pubkey != self.identity.keys.public_key()
                            && !addr.contains("127.0.0.1")
                            && !self.discovered_peers.iter().any(|p| p.ws_url == addr)
                        {
                            let name = peer_name.unwrap_or_else(|| "mesh-node".to_string());
                            let ip_part = addr.replace("ws://", "").replace("wss://", "");
                            let ip = ip_part.split(':').next().unwrap_or("0.0.0.0").to_string();
                            self.client_pool.add_relay(addr.clone());
                            self.discovered_peers.push(PeerNode {
                                ip,
                                relay_port: 4869,
                                ws_url: addr.clone(),
                                name: name.clone(),
                                last_seen: Instant::now(),
                            });
                            self.status_toast = Some((
                                format!("Mesh peer discovered: {} [{}]", addr, name),
                                Instant::now(),
                            ));
                        }
                    }
                }

                // Strictly filter for DEADAIR network users only
                if self.is_deadair_event(&event, Some(&source_url)) {
                    let is_direct = event.tags.iter().any(|t| {
                        let s = t.as_slice();
                        s.len() >= 2 && s[0] == "type" && s[1] == "direct"
                    });

                    let is_group = event.tags.iter().any(|t| {
                        let s = t.as_slice();
                        s.len() >= 2 && (s[0] == "g" || s[0] == "group" || (s[0] == "type" && s[1] == "group"))
                    });

                    if is_direct {
                        if !self.direct_messages.iter().any(|e| e.id == event.id) {
                            self.direct_messages.push(event.clone());
                            self.direct_messages.sort_by_key(|e| e.created_at);
                            if self.direct_messages.len() > 1000 {
                                self.direct_messages.remove(0);
                            }
                        }
                    } else if is_group {
                        if !self.group_messages.iter().any(|e| e.id == event.id) {
                            self.group_messages.push(event.clone());
                            self.group_messages.sort_by_key(|e| e.created_at);
                            if self.group_messages.len() > 1000 {
                                self.group_messages.remove(0);
                            }
                        }
                    } else {
                        if !self.timeline.iter().any(|e| e.id == event.id) {
                            self.timeline.push(event.clone());
                            self.timeline.sort_by_key(|e| e.created_at);
                            if self.timeline.len() > 1000 {
                                self.timeline.remove(0);
                            }
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
                    format!("Discovered LAN peer {} [{}]", peer.ws_url, peer.name),
                    Instant::now(),
                ));
                self.discovered_peers.push(peer);
            }
        }

        self.relay_statuses = self.client_pool.statuses().await;

        // Periodic presence gossip (every 30 seconds) so remote nodes find each other
        if self.last_presence_time.elapsed().as_secs() >= 30 {
            self.last_presence_time = Instant::now();
            self.broadcast_presence().await;
        }

        if let Some((_, time)) = self.status_toast {
            if time.elapsed().as_secs() > 4 {
                self.status_toast = None;
            }
        }
    }

    /// Determines if an event is authentic DEADAIR traffic:
    /// - Authored by the local user, OR
    /// - Contains explicit DEADAIR client tag, OR
    /// - Authored by a known LAN/mesh peer
    pub fn is_deadair_event(&self, event: &Event, _source_url: Option<&str>) -> bool {
        if event.pubkey == self.identity.keys.public_key() {
            return true;
        }

        let has_deadair_tag = event.tags.iter().any(|t| {
            let slice = t.as_slice();
            slice.len() >= 2 && slice[0].eq_ignore_ascii_case("client") && slice[1].eq_ignore_ascii_case("deadair")
        });
        if has_deadair_tag {
            return true;
        }

        if self.discovered_peers.iter().any(|p| {
            p.name == event.pubkey.to_hex()
                || self.profiles.get(&event.pubkey).map_or(false, |name| name == &p.name)
        }) {
            return true;
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
                KeyCode::Tab | KeyCode::BackTab => {
                    self.active_tab = match self.active_tab {
                        ActiveTab::Main => ActiveTab::Direct,
                        ActiveTab::Direct => ActiveTab::Groups,
                        ActiveTab::Groups => ActiveTab::Main,
                    };
                    self.selected_note_index = None;
                    let tab_name = match self.active_tab {
                        ActiveTab::Main => "Main Mesh",
                        ActiveTab::Direct => "Direct Messages",
                        ActiveTab::Groups => "Groups",
                    };
                    self.status_toast = Some((format!("Switched to {}", tab_name), Instant::now()));
                }
                KeyCode::Char('1') => {
                    self.active_tab = ActiveTab::Main;
                    self.selected_note_index = None;
                }
                KeyCode::Char('2') => {
                    self.active_tab = ActiveTab::Direct;
                    self.selected_note_index = None;
                }
                KeyCode::Char('3') => {
                    self.active_tab = ActiveTab::Groups;
                    self.selected_note_index = None;
                }
                KeyCode::Char('r') => {
                    if self.selected_note_index.is_some() || !self.current_notes().is_empty() {
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
                    let total = self.current_notes().len();
                    if total > 0 {
                        match self.selected_note_index {
                            None => {
                                self.selected_note_index = Some(total.saturating_sub(1));
                            }
                            Some(cur) => {
                                if cur + 1 < total {
                                    self.selected_note_index = Some(cur + 1);
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    let total = self.current_notes().len();
                    if total > 0 {
                        match self.selected_note_index {
                            None => {
                                self.selected_note_index = Some(total.saturating_sub(2));
                            }
                            Some(cur) => {
                                if cur > 0 {
                                    self.selected_note_index = Some(cur - 1);
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('g') | KeyCode::Home => {
                    self.selected_note_index = None;
                    self.status_toast = Some(("Resumed live stream".to_string(), Instant::now()));
                }
                KeyCode::Char('G') | KeyCode::End => {
                    if !self.current_notes().is_empty() {
                        self.selected_note_index = Some(0);
                    }
                }
                KeyCode::Char('b') => self.connect_bookmarks().await,
                KeyCode::Char('s') => self.show_share_toast(),
                KeyCode::Char('c') => {
                    self.current_notes_mut().clear();
                    self.selected_note_index = None;
                    self.status_toast = Some(("View cleared".to_string(), Instant::now()));
                }
                KeyCode::Esc => {
                    if self.selected_note_index.is_some() {
                        self.selected_note_index = None;
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
        let target_event = match self.selected_note_index {
            Some(idx) => self.current_notes().get(idx).cloned(),
            None => self.current_notes().last().cloned(),
        };

        if let Some(target) = target_event {
            if self.my_likes.contains(&target.id) {
                self.status_toast = Some(("You already liked this note".to_string(), Instant::now()));
                return;
            }

            let reaction = match EventBuilder::new(Kind::Reaction, "+")
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
            self.status_toast = Some((format!("Liked note by {} [+1]", author_name), Instant::now()));
        }
    }

    pub fn start_reply_to_selected(&mut self) {
        let target_event = match self.selected_note_index {
            Some(idx) => self.current_notes().get(idx).cloned(),
            None => self.current_notes().last().cloned(),
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

    pub async fn send_note(&mut self, text: String) {
        let deadair_tag = Tag::parse(["client", "deadair"]).unwrap_or_else(|_| Tag::hashtag("deadair"));
        let mut builder = EventBuilder::new(Kind::TextNote, &text).tag(deadair_tag);

        match self.active_tab {
            ActiveTab::Main => {}
            ActiveTab::Direct => {
                builder = builder.tag(Tag::parse(["type", "direct"]).unwrap());
                if let Some(target_pk) = self.selected_direct_peer {
                    builder = builder.tag(Tag::public_key(target_pk));
                }
            }
            ActiveTab::Groups => {
                builder = builder
                    .tag(Tag::parse(["type", "group"]).unwrap())
                    .tag(Tag::parse(["g", &self.current_group]).unwrap());
            }
        }

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
            self.status_toast = Some(("Transmitted note to DEADAIR mesh".to_string(), Instant::now()));
        }

        match self.active_tab {
            ActiveTab::Main => {
                if !self.timeline.iter().any(|e| e.id == event.id) {
                    self.timeline.push(event);
                    self.timeline.sort_by_key(|e| e.created_at);
                }
            }
            ActiveTab::Direct => {
                if !self.direct_messages.iter().any(|e| e.id == event.id) {
                    self.direct_messages.push(event);
                    self.direct_messages.sort_by_key(|e| e.created_at);
                }
            }
            ActiveTab::Groups => {
                if !self.group_messages.iter().any(|e| e.id == event.id) {
                    self.group_messages.push(event);
                    self.group_messages.sort_by_key(|e| e.created_at);
                }
            }
        }
    }

    pub async fn send_image(&mut self, path_str: &str, caption: Option<&str>) {
        let path = std::path::Path::new(path_str);
        if !path.exists() {
            self.status_toast = Some((format!("File not found: {}", path_str), Instant::now()));
            return;
        }

        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.status_toast = Some((format!("Failed to read file: {}", e), Instant::now()));
                return;
            }
        };

        if bytes.len() > 2_000_000 {
            self.status_toast = Some(("Image too large (>2MB). Please choose a smaller picture.".to_string(), Instant::now()));
            return;
        }

        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("png").to_lowercase();
        let mime = match ext.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            _ => "image/png",
        };

        let b64 = BASE64.encode(&bytes);
        let data_uri = format!("data:{};base64,{}", mime, b64);
        let cap = caption.unwrap_or_else(|| path.file_name().and_then(|s| s.to_str()).unwrap_or("image"));
        let content = format!("![{}]({})", cap, data_uri);

        self.send_note(content).await;
        self.status_toast = Some(("Transmitted picture to mesh".to_string(), Instant::now()));
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
                    self.safe_ticket = StationTicket::new(
                        &self.share_address,
                        &self.identity.name,
                        Some(self.identity.short_npub.clone()),
                    ).encode();
                    self.broadcast_metadata().await;
                    self.broadcast_presence().await;
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
            ["/clear"] => {
                self.current_notes_mut().clear();
                self.selected_note_index = None;
                self.status_toast = Some(("View cleared".to_string(), Instant::now()));
            }
            ["/tab", "main"] | ["/tab", "1"] => {
                self.active_tab = ActiveTab::Main;
                self.status_toast = Some(("Switched to Main Mesh tab".to_string(), Instant::now()));
            }
            ["/tab", "direct"] | ["/tab", "dm"] | ["/tab", "2"] => {
                self.active_tab = ActiveTab::Direct;
                self.status_toast = Some(("Switched to Direct Messages tab".to_string(), Instant::now()));
            }
            ["/tab", "groups"] | ["/tab", "group"] | ["/tab", "3"] => {
                self.active_tab = ActiveTab::Groups;
                self.status_toast = Some((format!("Switched to Groups tab (#{})", self.current_group), Instant::now()));
            }
            ["/group", name] | ["/join", name] => {
                self.current_group = name.trim_start_matches('#').to_string();
                self.active_tab = ActiveTab::Groups;
                self.status_toast = Some((format!("Active group set to #{}", self.current_group), Instant::now()));
            }
            ["/dm", target, msg @ ..] => {
                let target_name = target.to_string();
                let message_body = msg.join(" ");
                let target_pk = self.profiles.iter().find_map(|(pk, name)| {
                    if name.eq_ignore_ascii_case(&target_name) || pk.to_hex().starts_with(&target_name) {
                        Some(*pk)
                    } else {
                        None
                    }
                });

                if let Some(pk) = target_pk {
                    self.selected_direct_peer = Some(pk);
                    self.active_tab = ActiveTab::Direct;
                    if !message_body.is_empty() {
                        self.send_note(message_body).await;
                    } else {
                        self.status_toast = Some((format!("Direct chat ready with {}", target_name), Instant::now()));
                    }
                } else {
                    self.status_toast = Some((format!("Contact '{}' not found in known peers", target_name), Instant::now()));
                }
            }
            ["/image", path, caption @ ..] => {
                let cap = if !caption.is_empty() { Some(caption.join(" ")) } else { None };
                self.send_image(path, cap.as_deref()).await;
            }
            ["/encode", url] => {
                let ticket = StationTicket::new(*url, &self.identity.name, Some(self.identity.short_npub.clone())).encode();
                self.status_toast = Some((format!("Safe Ticket: {}", ticket), Instant::now()));
            }
            ["/encode", url, name] => {
                let ticket = StationTicket::new(*url, *name, None).encode();
                self.status_toast = Some((format!("Safe Ticket: {}", ticket), Instant::now()));
            }
            ["/decode", code] => {
                if let Some(ticket) = StationTicket::decode(code) {
                    self.status_toast = Some((format!("Decoded: {} [{}]", ticket.ws, ticket.name), Instant::now()));
                } else {
                    self.status_toast = Some(("Failed to decode ticket".to_string(), Instant::now()));
                }
            }
            ["/bookmark", "add", target] | ["/connect", target] => {
                if let Some(ticket) = StationTicket::decode(target) {
                    self.bookmarks.add(ticket.ws.clone(), ticket.name.clone());
                    self.client_pool.add_relay(ticket.ws.clone());
                    self.status_toast = Some((format!("Connected & bookmarked safe ticket: {} [{}]", ticket.ws, ticket.name), Instant::now()));
                } else {
                    let name = format!("node-{}", &target[target.len().saturating_sub(6)..]);
                    self.bookmarks.add(target.to_string(), name);
                    self.client_pool.add_relay(target.to_string());
                    self.status_toast = Some((format!("Added bookmark {}", target), Instant::now()));
                }
            }
            ["/bookmark", "add", url, name] => {
                if let Some(ticket) = StationTicket::decode(url) {
                    self.bookmarks.add(ticket.ws.clone(), name.to_string());
                    self.client_pool.add_relay(ticket.ws.clone());
                    self.status_toast = Some((format!("Added bookmark {} [{}]", ticket.ws, name), Instant::now()));
                } else {
                    self.bookmarks.add(url.to_string(), name.to_string());
                    self.client_pool.add_relay(url.to_string());
                    self.status_toast = Some((format!("Added bookmark {} [{}]", url, name), Instant::now()));
                }
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
            ["/relay", "add", target] => {
                if let Some(ticket) = StationTicket::decode(target) {
                    self.client_pool.add_relay(ticket.ws.clone());
                    self.bookmarks.add(ticket.ws.clone(), ticket.name.clone());
                    self.status_toast = Some((format!("Added and bookmarked relay {} [{}]", ticket.ws, ticket.name), Instant::now()));
                } else {
                    let relay_url = target.to_string();
                    self.client_pool.add_relay(relay_url.clone());
                    self.bookmarks.add(relay_url.clone(), "manual-node".to_string());
                    self.status_toast = Some((format!("Added and bookmarked relay {}", relay_url), Instant::now()));
                }
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
                self.status_toast = Some(("Commands: /name | /tab <1|2|3> | /dm <user> <msg> | /group <name> | /image <file> | /share | /bookmark add <code>".to_string(), Instant::now()));
            }
            _ => {
                self.status_toast = Some(("Unknown command. Type /help".to_string(), Instant::now()));
            }
        }
    }

    pub fn show_share_toast(&mut self) {
        let ticket = StationTicket::new(
            &self.share_address,
            &self.identity.name,
            Some(self.identity.short_npub.clone()),
        ).encode();
        self.safe_ticket = ticket.clone();
        self.status_toast = Some((
            format!("Address: {} | Safe Ticket: {}", self.share_address, ticket),
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

    async fn broadcast_metadata(&mut self) {
        let meta = Metadata::new().name(&self.identity.name);
        if let Ok(ev) = meta.finalize(&self.identity.keys) {
            if let Some(store) = &self.relay_store {
                let _ = store.add_event(ev.clone()).await;
            }
            self.client_pool.publish(ev);
        }
    }

    pub async fn broadcast_presence(&mut self) {
        let deadair_tag = Tag::parse(["client", "deadair"]).unwrap_or_else(|_| Tag::hashtag("deadair"));
        let presence_tag = Tag::parse(["type", "presence"]).unwrap();
        let addr_tag = Tag::parse(["address", &self.share_address]).unwrap();
        let name_tag = Tag::parse(["name", &self.identity.name]).unwrap();
        let ticket_tag = Tag::parse(["ticket", &self.safe_ticket]).unwrap();

        let builder = EventBuilder::new(Kind::TextNote, format!("DEADAIR Node Online: {}", self.identity.name))
            .tag(deadair_tag)
            .tag(presence_tag)
            .tag(addr_tag)
            .tag(name_tag)
            .tag(ticket_tag);

        if let Ok(ev) = builder.finalize(&self.identity.keys) {
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
                    self.broadcast_presence().await;
                    self.flush_outbox().await;
                }
                Err(_) => {
                    self.status_toast = Some(("Failed to bind relay port".to_string(), Instant::now()));
                }
            }
        }
    }

    async fn flush_outbox(&mut self) {
        let events = self.outbox.pending();
        let count = events.len();
        if count > 0 {
            for event in events {
                if let Some(store) = &self.relay_store {
                    let _ = store.add_event(event.clone()).await;
                }
                self.client_pool.publish(event);
            }
            self.status_toast = Some((
                format!("Flushed {} queued notes to relays", count),
                Instant::now(),
            ));
        }
    }
}
