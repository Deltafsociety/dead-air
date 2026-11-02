use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use deadair::app::ProfileInfo;
use deadair::bookmarks::BookmarkManager;
use deadair::client::ClientPool;
use deadair::discovery::{DiscoveryManager, PeerNode};
use deadair::identity::Identity;
use deadair::net_info;
use deadair::outbox::Outbox;
use deadair::relay::{EmbeddedRelay, RelayStore};
use deadair::ticket::StationTicket;
use nostr::event::{Event, EventBuilder, EventId, FinalizeEvent, Kind, Tag};
use nostr::key::PublicKey;
use nostr::nips::nip01::Metadata;
use nostr::nips::nip19::ToBech32;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use tokio::sync::{mpsc, Mutex};

#[derive(Clone, Serialize, Deserialize)]
pub struct NoteDto {
    pub id: String,
    pub pubkey: String,
    pub author_name: String,
    pub handle: String,
    pub content: String,
    pub created_at: u64,
    pub relative_time: String,
    pub color_idx: usize,
    pub is_deadair: bool,
    pub likes: usize,
    pub reposts: usize,
    pub replies: usize,
    pub is_liked_by_me: bool,
    pub reply_to_id: Option<String>,
    pub reply_to_pubkey: Option<String>,
    pub is_image: bool,
    pub image_data: Option<String>,
    pub tab: String,
    pub group_name: Option<String>,
    pub recipient_pubkey: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BookmarkDto {
    pub url: String,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RelayStatusDto {
    pub url: String,
    pub connected: bool,
    pub latency_ms: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PeerDto {
    pub ws_url: String,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StateDto {
    pub name: String,
    pub pubkey: String,
    pub npub: String,
    pub relay_active: bool,
    pub relay_port: u16,
    pub share_address: String,
    pub safe_ticket: String,
    pub detected_ip: String,
    pub outbox_count: usize,
    pub station_notes: Vec<NoteDto>,
    pub direct_notes: Vec<NoteDto>,
    pub group_notes: Vec<NoteDto>,
    pub bookmarks: Vec<BookmarkDto>,
    pub relays: Vec<RelayStatusDto>,
    pub peers: Vec<PeerDto>,
}

pub struct GuiBackend {
    pub identity: Identity,
    pub relay_port: u16,
    pub embedded_relay: Option<EmbeddedRelay>,
    pub relay_store: Option<RelayStore>,
    pub client_pool: ClientPool,
    pub _discovery: Arc<DiscoveryManager>,
    pub outbox: Outbox,
    pub bookmarks: BookmarkManager,
    pub station_notes: Vec<Event>,
    pub direct_notes: Vec<Event>,
    pub group_notes: Vec<Event>,
    pub profiles: HashMap<PublicKey, String>,
    pub profile_details: HashMap<PublicKey, ProfileInfo>,
    pub post_likes: HashMap<EventId, usize>,
    pub post_replies: HashMap<EventId, usize>,
    pub post_reposts: HashMap<EventId, usize>,
    pub my_likes: HashSet<EventId>,
    pub share_address: String,
    pub safe_ticket: String,
    pub detected_ip: String,
    pub discovered_peers: Vec<PeerNode>,
    pub inbound_event_tx: mpsc::UnboundedSender<(Event, String)>,
}

impl GuiBackend {
    pub fn event_to_dto(&self, ev: &Event) -> NoteDto {
        let display_name = self
            .profile_details
            .get(&ev.pubkey)
            .and_then(|p| p.display_name.clone().or_else(|| p.name.clone()))
            .or_else(|| self.profiles.get(&ev.pubkey).cloned())
            .unwrap_or_else(|| {
                let full_hex = ev.pubkey.to_hex();
                format!("{}..{}", &full_hex[..6], &full_hex[full_hex.len() - 4..])
            });

        let profile = self.profile_details.get(&ev.pubkey);
        let handle = profile
            .and_then(|p| p.nip05.clone().or_else(|| p.name.clone()))
            .map(|h| format!("@{}", h))
            .unwrap_or_else(|| {
                let npub = ev.pubkey.to_bech32().unwrap_or_else(|_| ev.pubkey.to_hex());
                format!("@{}..{}", &npub[..8], &npub[npub.len() - 4..])
            });

        let color_idx = {
            let bytes = ev.pubkey.to_bytes();
            ((bytes[0] ^ bytes[1]) % 6) as usize
        };

        let created_at_sec = ev.created_at.as_secs();
        let relative_time = format_relative_time(created_at_sec);

        let likes = self.post_likes.get(&ev.id).copied().unwrap_or(0);
        let reposts = self.post_reposts.get(&ev.id).copied().unwrap_or(0);
        let replies = self.post_replies.get(&ev.id).copied().unwrap_or(0);
        let is_liked_by_me = self.my_likes.contains(&ev.id);

        let reply_to_id = ev.tags.event_ids().next().map(|id| id.to_hex());
        let reply_to_pubkey = ev.tags.public_keys().next().map(|pk| pk.to_hex());

        let is_direct = ev.tags.iter().any(|t| {
            let s = t.as_slice();
            s.len() >= 2 && s[0] == "type" && s[1] == "direct"
        });

        let group_name = ev.tags.iter().find_map(|t| {
            let s = t.as_slice();
            if s.len() >= 2 && (s[0] == "g" || s[0] == "group") {
                Some(s[1].to_string())
            } else {
                None
            }
        });

        let tab = if is_direct {
            "direct".to_string()
        } else if group_name.is_some() {
            "group".to_string()
        } else {
            "main".to_string()
        };

        let (is_image, image_data) = if ev.content.contains("data:image/") || ev.content.starts_with("![") {
            (true, Some(ev.content.clone()))
        } else {
            (false, None)
        };

        NoteDto {
            id: ev.id.to_hex(),
            pubkey: ev.pubkey.to_hex(),
            author_name: display_name,
            handle,
            content: ev.content.clone(),
            created_at: created_at_sec,
            relative_time,
            color_idx,
            is_deadair: true,
            likes,
            reposts,
            replies,
            is_liked_by_me,
            reply_to_id,
            reply_to_pubkey,
            is_image,
            image_data,
            tab,
            group_name,
            recipient_pubkey: None,
        }
    }

    pub fn is_deadair_event(&self, event: &Event) -> bool {
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
}

// ----------------- Tauri Commands -----------------

#[tauri::command]
async fn get_state(state: tauri::State<'_, Arc<Mutex<GuiBackend>>>) -> Result<StateDto, String> {
    let backend = state.lock().await;

    let station_notes: Vec<NoteDto> = backend
        .station_notes
        .iter()
        .rev()
        .take(150)
        .rev()
        .map(|e| backend.event_to_dto(e))
        .collect();

    let direct_notes: Vec<NoteDto> = backend
        .direct_notes
        .iter()
        .rev()
        .take(150)
        .rev()
        .map(|e| backend.event_to_dto(e))
        .collect();

    let group_notes: Vec<NoteDto> = backend
        .group_notes
        .iter()
        .rev()
        .take(150)
        .rev()
        .map(|e| backend.event_to_dto(e))
        .collect();

    let bookmarks = backend
        .bookmarks
        .list()
        .iter()
        .map(|b| BookmarkDto {
            url: b.url.clone(),
            name: b.name.clone(),
        })
        .collect();

    let relays = backend
        .client_pool
        .statuses()
        .await
        .into_iter()
        .map(|r| RelayStatusDto {
            url: r.url,
            connected: r.connected,
            latency_ms: r.latency_ms,
        })
        .collect();

    let peers = backend
        .discovered_peers
        .iter()
        .map(|p| PeerDto {
            ws_url: p.ws_url.clone(),
            name: p.name.clone(),
        })
        .collect();

    let npub = backend
        .identity
        .keys
        .public_key()
        .to_bech32()
        .unwrap_or_else(|_| backend.identity.keys.public_key().to_hex());

    Ok(StateDto {
        name: backend.identity.name.clone(),
        pubkey: backend.identity.keys.public_key().to_hex(),
        npub,
        relay_active: backend.embedded_relay.is_some(),
        relay_port: backend.relay_port,
        share_address: backend.share_address.clone(),
        safe_ticket: backend.safe_ticket.clone(),
        detected_ip: backend.detected_ip.clone(),
        outbox_count: backend.outbox.len(),
        station_notes,
        direct_notes,
        group_notes,
        bookmarks,
        relays,
        peers,
    })
}

#[tauri::command]
async fn send_note(
    text: String,
    reply_to_id: Option<String>,
    reply_to_pubkey: Option<String>,
    tab: Option<String>,
    group: Option<String>,
    recipient: Option<String>,
    state: tauri::State<'_, Arc<Mutex<GuiBackend>>>,
) -> Result<NoteDto, String> {
    let mut backend = state.lock().await;
    let clean_text = text.trim();
    if clean_text.is_empty() {
        return Err("Message content is empty".to_string());
    }

    let deadair_tag = Tag::parse(["client", "deadair"]).unwrap_or_else(|_| Tag::hashtag("deadair"));
    let mut builder = EventBuilder::new(Kind::TextNote, clean_text).tag(deadair_tag);

    let target_tab = tab.unwrap_or_else(|| "main".to_string());
    if target_tab == "direct" {
        builder = builder.tag(Tag::parse(["type", "direct"]).unwrap());
        if let Some(target_pk_hex) = recipient {
            if let Ok(pk) = PublicKey::from_hex(&target_pk_hex) {
                builder = builder.tag(Tag::public_key(pk));
            }
        }
    } else if target_tab == "group" {
        let grp = group.unwrap_or_else(|| "general".to_string());
        builder = builder
            .tag(Tag::parse(["type", "group"]).unwrap())
            .tag(Tag::parse(["g", &grp]).unwrap());
    }

    if let Some(target_hex) = reply_to_id {
        if let Ok(event_id) = EventId::from_hex(&target_hex) {
            builder = builder.tag(Tag::event(event_id));
            *backend.post_replies.entry(event_id).or_insert(0) += 1;
        }
    }

    if let Some(pk_hex) = reply_to_pubkey {
        if let Ok(pubkey) = PublicKey::from_hex(&pk_hex) {
            builder = builder.tag(Tag::public_key(pubkey));
        }
    }

    let event = builder
        .finalize(&backend.identity.keys)
        .map_err(|e| format!("Failed to sign note: {}", e))?;

    if let Some(store) = &backend.relay_store {
        let _ = store.add_event(event.clone()).await;
    }

    backend.client_pool.publish(event.clone());

    let dto = backend.event_to_dto(&event);
    if target_tab == "direct" {
        backend.direct_notes.push(event);
    } else if target_tab == "group" {
        backend.group_notes.push(event);
    } else {
        backend.station_notes.push(event);
    }

    Ok(dto)
}

#[tauri::command]
async fn send_image(
    data_uri: String,
    caption: Option<String>,
    tab: Option<String>,
    group: Option<String>,
    recipient: Option<String>,
    state: tauri::State<'_, Arc<Mutex<GuiBackend>>>,
) -> Result<NoteDto, String> {
    let cap = caption.unwrap_or_else(|| "picture".to_string());
    let content = format!("![{}]({})", cap, data_uri);
    send_note(content, None, None, tab, group, recipient, state).await
}

#[tauri::command]
async fn get_safe_ticket(state: tauri::State<'_, Arc<Mutex<GuiBackend>>>) -> Result<String, String> {
    let backend = state.lock().await;
    Ok(backend.safe_ticket.clone())
}

#[tauri::command]
async fn decode_safe_ticket(code: String) -> Result<StationTicket, String> {
    StationTicket::decode(&code).ok_or_else(|| "Failed to decode ticket".to_string())
}

#[tauri::command]
async fn connect_safe_ticket(
    code: String,
    state: tauri::State<'_, Arc<Mutex<GuiBackend>>>,
) -> Result<StationTicket, String> {
    let ticket = StationTicket::decode(&code).ok_or_else(|| "Invalid ticket or address".to_string())?;
    let mut backend = state.lock().await;
    backend.bookmarks.add(ticket.ws.clone(), ticket.name.clone());
    backend.client_pool.add_relay(ticket.ws.clone());
    Ok(ticket)
}

#[tauri::command]
async fn like_note(
    event_id_hex: String,
    author_pubkey_hex: String,
    state: tauri::State<'_, Arc<Mutex<GuiBackend>>>,
) -> Result<bool, String> {
    let mut backend = state.lock().await;

    let event_id = EventId::from_hex(&event_id_hex).map_err(|e| e.to_string())?;
    let author_pubkey = PublicKey::from_hex(&author_pubkey_hex).map_err(|e| e.to_string())?;

    if backend.my_likes.contains(&event_id) {
        return Ok(false);
    }

    let reaction = EventBuilder::new(Kind::Reaction, "+")
        .tag(Tag::event(event_id))
        .tag(Tag::public_key(author_pubkey))
        .finalize(&backend.identity.keys)
        .map_err(|e| e.to_string())?;

    backend.my_likes.insert(event_id);
    *backend.post_likes.entry(event_id).or_insert(0) += 1;

    if let Some(store) = &backend.relay_store {
        let _ = store.add_event(reaction.clone()).await;
    }
    backend.client_pool.publish(reaction);

    Ok(true)
}

#[tauri::command]
async fn set_station_name(
    new_name: String,
    state: tauri::State<'_, Arc<Mutex<GuiBackend>>>,
) -> Result<String, String> {
    let mut backend = state.lock().await;
    let name = new_name.trim().to_string();
    if name.is_empty() {
        return Err("Name cannot be empty".to_string());
    }

    backend
        .identity
        .set_name(name.clone())
        .map_err(|e| e.to_string())?;
    let my_pk = backend.identity.keys.public_key();
    backend.profiles.insert(my_pk, name.clone());
    backend.safe_ticket = StationTicket::new(
        &backend.share_address,
        &name,
        Some(backend.identity.short_npub.clone()),
    ).encode();

    let meta = Metadata::new().name(&name).display_name(&name);
    if let Ok(event) = meta.finalize(&backend.identity.keys) {
        if let Some(store) = &backend.relay_store {
            let _ = store.add_event(event.clone()).await;
        }
        backend.client_pool.publish(event);
    }

    Ok(name)
}

#[tauri::command]
async fn toggle_embedded_relay(
    state: tauri::State<'_, Arc<Mutex<GuiBackend>>>,
) -> Result<bool, String> {
    let mut backend = state.lock().await;

    if let Some(mut relay) = backend.embedded_relay.take() {
        relay.stop();
        backend.relay_store = None;
        Ok(false)
    } else {
        match EmbeddedRelay::start(backend.relay_port, backend.inbound_event_tx.clone()).await {
            Ok((relay, store)) => {
                backend.client_pool.add_relay(format!("ws://127.0.0.1:{}", backend.relay_port));
                backend.embedded_relay = Some(relay);
                backend.relay_store = Some(store);
                Ok(true)
            }
            Err(e) => Err(format!("Failed to bind relay: {}", e)),
        }
    }
}

#[tauri::command]
async fn add_bookmark(
    target: String,
    name: Option<String>,
    state: tauri::State<'_, Arc<Mutex<GuiBackend>>>,
) -> Result<Vec<BookmarkDto>, String> {
    let mut backend = state.lock().await;

    let (url, bookmark_name) = if let Some(ticket) = StationTicket::decode(&target) {
        (ticket.ws, name.unwrap_or(ticket.name))
    } else {
        let bname = name.unwrap_or_else(|| format!("node-{}", &target[target.len().saturating_sub(6)..]));
        (target, bname)
    };

    backend.bookmarks.add(url.clone(), bookmark_name);
    backend.client_pool.add_relay(url);

    let list = backend
        .bookmarks
        .list()
        .iter()
        .map(|b| BookmarkDto {
            url: b.url.clone(),
            name: b.name.clone(),
        })
        .collect();

    Ok(list)
}

#[tauri::command]
async fn remove_bookmark(
    url: String,
    state: tauri::State<'_, Arc<Mutex<GuiBackend>>>,
) -> Result<Vec<BookmarkDto>, String> {
    let mut backend = state.lock().await;
    backend.bookmarks.remove(&url);

    let list = backend
        .bookmarks
        .list()
        .iter()
        .map(|b| BookmarkDto {
            url: b.url.clone(),
            name: b.name.clone(),
        })
        .collect();

    Ok(list)
}

#[tauri::command]
async fn clear_timeline(
    state: tauri::State<'_, Arc<Mutex<GuiBackend>>>,
) -> Result<(), String> {
    let mut backend = state.lock().await;
    backend.station_notes.clear();
    backend.direct_notes.clear();
    backend.group_notes.clear();
    Ok(())
}

fn format_relative_time(created_at_sec: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if created_at_sec > now {
        return "just now".to_string();
    }

    let diff = now.saturating_sub(created_at_sec);
    if diff < 60 {
        format!("{}s ago", diff)
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

// ----------------- Tauri App Setup -----------------

pub fn run() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();

            tauri::async_runtime::block_on(async move {
                let identity = Identity::load_or_create().expect("Failed to load or generate identity");
                let relay_port = 4869;

                let (inbound_event_tx, mut inbound_event_rx) = mpsc::unbounded_channel::<(Event, String)>();
                let (peer_tx, mut peer_rx) = mpsc::unbounded_channel::<PeerNode>();

                let client_pool = ClientPool::new(inbound_event_tx.clone());
                let discovery = Arc::new(DiscoveryManager::new(peer_tx));
                discovery.clone().start(
                    identity.name.clone(),
                    relay_port,
                    identity.npub.clone(),
                );

                let bookmarks = BookmarkManager::load();
                for url in bookmarks.auto_connect_urls() {
                    client_pool.add_relay(url);
                }

                let outbox = Outbox::new();

                let (embedded_relay, relay_store) = match EmbeddedRelay::start(relay_port, inbound_event_tx.clone()).await {
                    Ok((r, s)) => {
                        client_pool.add_relay(format!("ws://127.0.0.1:{}", relay_port));
                        (Some(r), Some(s))
                    }
                    Err(_) => (None, None),
                };

                let mut profiles = HashMap::new();
                profiles.insert(identity.keys.public_key(), identity.name.clone());

                let share_address = format!("ws://127.0.0.1:{}", relay_port);
                let safe_ticket = StationTicket::new(
                    &share_address,
                    &identity.name,
                    Some(identity.short_npub.clone()),
                ).encode();

                let backend = Arc::new(Mutex::new(GuiBackend {
                    identity,
                    relay_port,
                    embedded_relay,
                    relay_store,
                    client_pool,
                    _discovery: discovery,
                    outbox,
                    bookmarks,
                    station_notes: Vec::new(),
                    direct_notes: Vec::new(),
                    group_notes: Vec::new(),
                    profiles,
                    profile_details: HashMap::new(),
                    post_likes: HashMap::new(),
                    post_replies: HashMap::new(),
                    post_reposts: HashMap::new(),
                    my_likes: HashSet::new(),
                    share_address,
                    safe_ticket,
                    detected_ip: "127.0.0.1".to_string(),
                    discovered_peers: Vec::new(),
                    inbound_event_tx,
                }));

                app_handle.manage(backend.clone());

                // Background task 1: Inbound Nostr events (strictly for DEADAIR users)
                let backend_inbound = backend.clone();
                let handle_inbound = app_handle.clone();
                tokio::spawn(async move {
                    while let Some((event, _source_url)) = inbound_event_rx.recv().await {
                        let mut b = backend_inbound.lock().await;

                        if event.kind == Kind::Metadata {
                            if let Ok(meta) = Metadata::from_json(&event.content) {
                                let display = meta.display_name.clone().or_else(|| meta.name.clone());
                                if let Some(name) = display {
                                    b.profiles.insert(event.pubkey, name);
                                }
                                b.profile_details.insert(event.pubkey, ProfileInfo {
                                    name: meta.name,
                                    display_name: meta.display_name,
                                    nip05: meta.nip05,
                                    about: meta.about,
                                });
                            }
                        } else if event.kind == Kind::Reaction {
                            for target_id in event.tags.event_ids() {
                                *b.post_likes.entry(target_id).or_insert(0) += 1;
                            }
                            let _ = handle_inbound.emit("metrics_updated", ());
                        } else if event.kind == Kind::Repost {
                            for target_id in event.tags.event_ids() {
                                *b.post_reposts.entry(target_id).or_insert(0) += 1;
                            }
                            let _ = handle_inbound.emit("metrics_updated", ());
                        } else if event.kind == Kind::TextNote {
                            b.outbox.remove(&event.id);
                            for target_id in event.tags.event_ids() {
                                *b.post_replies.entry(target_id).or_insert(0) += 1;
                            }

                            // Mesh presence detection
                            let is_presence = event.tags.iter().any(|t| {
                                let s = t.as_slice();
                                s.len() >= 2 && s[0] == "type" && s[1] == "presence"
                            });

                            if is_presence {
                                let mut ws_addr = None;
                                let mut peer_name = None;
                                for t in event.tags.iter() {
                                    let s = t.as_slice();
                                    if s.len() >= 2 {
                                        if s[0] == "address" { ws_addr = Some(s[1].to_string()); }
                                        if s[0] == "name" { peer_name = Some(s[1].to_string()); }
                                        if s[0] == "ticket" {
                                            if let Some(decoded) = StationTicket::decode(&s[1]) {
                                                ws_addr = Some(decoded.ws);
                                                peer_name = Some(decoded.name);
                                            }
                                        }
                                    }
                                }

                                if let Some(addr) = ws_addr {
                                    if event.pubkey != b.identity.keys.public_key()
                                        && !addr.contains("127.0.0.1")
                                        && !b.discovered_peers.iter().any(|p| p.ws_url == addr)
                                    {
                                        let pname = peer_name.unwrap_or_else(|| "mesh-peer".to_string());
                                        let ip = addr.replace("ws://", "").split(':').next().unwrap_or("0.0.0.0").to_string();
                                        b.client_pool.add_relay(addr.clone());
                                        b.discovered_peers.push(PeerNode {
                                            ip,
                                            relay_port: 4869,
                                            ws_url: addr.clone(),
                                            name: pname.clone(),
                                            last_seen: std::time::Instant::now(),
                                        });
                                        let _ = handle_inbound.emit("peer_discovered", PeerDto {
                                            ws_url: addr,
                                            name: pname,
                                        });
                                    }
                                }
                            }

                            // Strict filter: Accept only notes from DEADAIR stations
                            if b.is_deadair_event(&event) {
                                let is_direct = event.tags.iter().any(|t| {
                                    let s = t.as_slice();
                                    s.len() >= 2 && s[0] == "type" && s[1] == "direct"
                                });

                                let is_group = event.tags.iter().any(|t| {
                                    let s = t.as_slice();
                                    s.len() >= 2 && (s[0] == "g" || s[0] == "group")
                                });

                                let note_dto = b.event_to_dto(&event);

                                if is_direct {
                                    if !b.direct_notes.iter().any(|e| e.id == event.id) {
                                        b.direct_notes.push(event);
                                        let _ = handle_inbound.emit("direct_note", note_dto);
                                    }
                                } else if is_group {
                                    if !b.group_notes.iter().any(|e| e.id == event.id) {
                                        b.group_notes.push(event);
                                        let _ = handle_inbound.emit("group_note", note_dto);
                                    }
                                } else {
                                    if !b.station_notes.iter().any(|e| e.id == event.id) {
                                        b.station_notes.push(event);
                                        if b.station_notes.len() > 1000 {
                                            b.station_notes.remove(0);
                                        }
                                        let _ = handle_inbound.emit("station_note", note_dto);
                                    }
                                }
                            }
                        }
                    }
                });

                // Background task 2: LAN Peer discovery listener
                let backend_peers = backend.clone();
                let handle_peers = app_handle.clone();
                tokio::spawn(async move {
                    while let Some(peer) = peer_rx.recv().await {
                        let mut b = backend_peers.lock().await;
                        let exists = b.discovered_peers.iter().any(|p| p.ws_url == peer.ws_url);
                        if !exists {
                            b.client_pool.add_relay(peer.ws_url.clone());
                            b.discovered_peers.push(peer.clone());
                            let _ = handle_peers.emit("peer_discovered", PeerDto {
                                ws_url: peer.ws_url,
                                name: peer.name,
                            });
                        }
                    }
                });

                // Background task 3: IP resolution
                let backend_ip = backend.clone();
                let handle_ip = app_handle.clone();
                tokio::spawn(async move {
                    let (ip, share_url) = net_info::resolve_best_address(relay_port).await;
                    let mut b = backend_ip.lock().await;
                    b.detected_ip = ip.clone();
                    b.share_address = share_url.clone();
                    b.safe_ticket = StationTicket::new(
                        &share_url,
                        &b.identity.name,
                        Some(b.identity.short_npub.clone()),
                    ).encode();
                    let _ = handle_ip.emit("ip_updated", (ip, share_url));
                });
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            send_note,
            send_image,
            get_safe_ticket,
            decode_safe_ticket,
            connect_safe_ticket,
            like_note,
            set_station_name,
            toggle_embedded_relay,
            add_bookmark,
            remove_bookmark,
            clear_timeline,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
