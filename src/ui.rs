use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use nostr::event::Event;
use nostr::key::PublicKey;
use nostr::nips::nip19::ToBech32;

use crate::app::{ActiveTab, App, InputMode};

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(f.area());

    render_header(f, app, chunks[0]);
    render_body(f, app, chunks[1]);
    render_input(f, app, chunks[2]);
    render_footer(f, app, chunks[3]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let relay_status_span = if app.embedded_relay.is_some() {
        Span::styled(
            format!(" [RELAY: ACTIVE :{:?}] ", app.relay_port),
            Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [RELAY: OFF] ",
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        )
    };

    let net_mode_span = if app.public_relays_active {
        Span::styled(" [NET: ONLINE] ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" [NET: INTRANET] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    };

    let peers_count = app.discovered_peers.len();
    let peers_span = Span::styled(
        format!(" [PEERS: {}] ", peers_count),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    );

    let outbox_count = app.outbox.len();
    let outbox_span = if outbox_count > 0 {
        Span::styled(
            format!(" [OUTBOX: {}] ", outbox_count),
            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(" [OUTBOX: 0] ", Style::default().fg(Color::DarkGray))
    };

    let header_line = Line::from(vec![
        Span::styled(" DEADAIR ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" [{}] ", app.identity.name), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(format!("(ID: {}) ", app.identity.short_npub), Style::default().fg(Color::Magenta)),
        relay_status_span,
        Span::raw(" "),
        net_mode_span,
        Span::raw(" "),
        peers_span,
        Span::raw(" "),
        outbox_span,
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(header_line).block(block);
    f.render_widget(paragraph, area);
}

fn render_body(f: &mut Frame, app: &App, area: Rect) {
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(area);

    render_timeline(f, app, body_chunks[0]);
    render_sidebar(f, app, body_chunks[1]);
}

fn render_timeline(f: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.input_mode == InputMode::Normal {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let (active_events, is_station_tab) = match app.active_tab {
        ActiveTab::Station => (&app.timeline, true),
        ActiveTab::Public => (&app.public_timeline, false),
    };

    let (station_style, public_style) = match app.active_tab {
        ActiveTab::Station => (
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
            Style::default().fg(Color::DarkGray),
        ),
        ActiveTab::Public => (
            Style::default().fg(Color::DarkGray),
            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
    };

    let station_badge = Span::styled(
        format!(" [1] DEADAIR Station ({}) ", app.timeline.len()),
        station_style,
    );

    let public_badge = Span::styled(
        format!(" [2] Public Social Mesh ({}) ", app.public_timeline.len()),
        public_style,
    );

    let nav_hint = match (app.active_tab, app.selected_public_index) {
        (ActiveTab::Public, Some(idx)) => Span::styled(
            format!(" [Post {}/{} • 'g' live] ", idx + 1, app.public_timeline.len()),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        (ActiveTab::Public, None) => Span::styled(
            " [LIVE • Up/k to browse] ",
            Style::default().fg(Color::Green),
        ),
        _ => Span::styled(
            " [Tab / 1-2 to switch] ",
            Style::default().fg(Color::DarkGray),
        ),
    };

    let block = Block::default()
        .title(Line::from(vec![
            Span::raw(" "),
            station_badge,
            Span::raw(" "),
            public_badge,
            Span::raw(" "),
            nav_hint,
        ]))
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    if active_events.is_empty() {
        let empty_lines = if is_station_tab {
            vec![
                Line::from(""),
                Line::from(Span::styled("No DEADAIR station transmissions yet.", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled("This feed only displays notes from DEADAIR peers and domestic relays.", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled("Type a note below and hit [Enter] to broadcast to your network.", Style::default().fg(Color::DarkGray))),
                Line::from(Span::styled("Type /share to see your domestic station address for peers.", Style::default().fg(Color::DarkGray))),
            ]
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled("No public social posts received yet.", Style::default().fg(Color::DarkGray))),
                if !app.public_relays_active {
                    Line::from(Span::styled("Public internet mesh is currently OFF. Press [p] or type /relay public to connect.", Style::default().fg(Color::Yellow)))
                } else {
                    Line::from(Span::styled("Streaming posts from Damus, nos.lol, and Primal...", Style::default().fg(Color::Cyan)))
                },
                Line::from(""),
                Line::from(Span::styled("Press [Tab] or [1] to return to your domestic DEADAIR Station tab.", Style::default().fg(Color::DarkGray))),
            ]
        };

        let empty_msg = Paragraph::new(empty_lines)
            .alignment(Alignment::Center)
            .block(block);
        f.render_widget(empty_msg, area);
        return;
    }

    let mut list_items = Vec::new();

    if is_station_tab {
        // Classic domestic station chat layout
        for event in active_events.iter() {
            let is_self = event.pubkey == app.identity.keys.public_key();
            let display_name = app
                .profile_details
                .get(&event.pubkey)
                .and_then(|p| p.display_name.clone().or_else(|| p.name.clone()))
                .or_else(|| app.profiles.get(&event.pubkey).cloned())
                .unwrap_or_else(|| {
                    let full_hex = event.pubkey.to_hex();
                    format!("{}..{}", &full_hex[..6], &full_hex[full_hex.len() - 4..])
                });

            let (author_str, author_color) = if is_self {
                (format!("YOU [{}]", display_name), Color::Green)
            } else {
                (format!("* {}", display_name), Color::Yellow)
            };

            let time_str = format_timestamp(event.created_at.as_secs());

            let header = Line::from(vec![
                Span::styled(format!("[{}] ", time_str), Style::default().fg(Color::DarkGray)),
                Span::styled(author_str, Style::default().fg(author_color).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" (id: {}..)", &event.id.to_hex()[..8]), Style::default().fg(Color::DarkGray)),
                Span::styled(" [STATION] ", Style::default().fg(Color::Green)),
            ]);

            let mut lines = vec![header];
            for line in event.content.lines() {
                lines.push(Line::from(vec![
                    Span::styled("  | ", Style::default().fg(Color::DarkGray)),
                    Span::styled(line.to_string(), Style::default().fg(Color::White)),
                ]));
            }
            lines.push(Line::from(""));

            list_items.push(ListItem::new(lines));
        }
    } else {
        // Social Media Post Cards layout
        for (i, event) in active_events.iter().enumerate() {
            let is_selected = app.selected_public_index == Some(i);
            let display_name = app
                .profile_details
                .get(&event.pubkey)
                .and_then(|p| p.display_name.clone().or_else(|| p.name.clone()))
                .or_else(|| app.profiles.get(&event.pubkey).cloned())
                .unwrap_or_else(|| {
                    let full_hex = event.pubkey.to_hex();
                    format!("{}..{}", &full_hex[..6], &full_hex[full_hex.len() - 4..])
                });
            let profile = app.profile_details.get(&event.pubkey);
            let handle = profile
                .and_then(|p| p.nip05.clone().or_else(|| p.name.clone()))
                .map(|h| format!("@{}", h))
                .unwrap_or_else(|| {
                    let npub = event.pubkey.to_bech32().unwrap_or_else(|_| event.pubkey.to_hex());
                    format!("@{}..{}", &npub[..8], &npub[npub.len() - 4..])
                });

            let relative_time = format_relative_time(event.created_at.as_secs());
            let author_color = author_color_from_pubkey(&event.pubkey);
            let likes = app.post_likes.get(&event.id).copied().unwrap_or(0);
            let replies = app.post_replies.get(&event.id).copied().unwrap_or(0);
            let reposts = app.post_reposts.get(&event.id).copied().unwrap_or(0);
            let is_liked_by_me = app.my_likes.contains(&event.id);

            let item = render_social_card(
                event,
                is_selected,
                &display_name,
                &handle,
                author_color,
                &relative_time,
                likes,
                replies,
                reposts,
                is_liked_by_me,
            );
            list_items.push(item);
        }
    }

    let list_len = list_items.len();
    let visible_height = area.height.saturating_sub(2) as usize;
    let scroll_offset = if is_station_tab || app.selected_public_index.is_none() {
        list_len.saturating_sub(visible_height)
    } else {
        let sel = app.selected_public_index.unwrap_or(0);
        if sel >= visible_height {
            (sel + 1).saturating_sub(visible_height)
        } else {
            0
        }
    };

    let items_slice: Vec<ListItem> = list_items.into_iter().skip(scroll_offset).collect();
    let list = List::new(items_slice).block(block);
    f.render_widget(list, area);
}

#[allow(clippy::too_many_arguments)]
fn render_social_card(
    event: &Event,
    is_selected: bool,
    display_name: &str,
    handle: &str,
    author_color: Color,
    relative_time: &str,
    likes: usize,
    replies: usize,
    reposts: usize,
    is_liked_by_me: bool,
) -> ListItem<'static> {
    let top_border = if is_selected {
        Line::from(vec![
            Span::styled("┏━ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled("● ", Style::default().fg(author_color)),
            Span::styled(display_name.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" {} ", handle), Style::default().fg(Color::Cyan)),
            Span::styled("• ", Style::default().fg(Color::DarkGray)),
            Span::styled(relative_time.to_string(), Style::default().fg(Color::Yellow)),
            Span::styled(" ━━[SELECTED]━", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ])
    } else {
        Line::from(vec![
            Span::styled("╭─ ", Style::default().fg(Color::DarkGray)),
            Span::styled("● ", Style::default().fg(author_color)),
            Span::styled(display_name.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" {} ", handle), Style::default().fg(Color::DarkGray)),
            Span::styled("• ", Style::default().fg(Color::DarkGray)),
            Span::styled(relative_time.to_string(), Style::default().fg(Color::DarkGray)),
            Span::styled(" ─", Style::default().fg(Color::DarkGray)),
        ])
    };

    let side_style = if is_selected {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let side_prefix = if is_selected { "┃ " } else { "│ " };

    let mut card_lines = vec![top_border];

    for line in event.content.lines() {
        let mut spans = vec![Span::styled(side_prefix, side_style)];
        for (i, word) in line.split(' ').enumerate() {
            if i > 0 {
                spans.push(Span::raw(" "));
            }
            if word.starts_with('#') && word.len() > 1 {
                spans.push(Span::styled(word.to_string(), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)));
            } else if word.starts_with('@') || word.starts_with("nostr:npub") || word.starts_with("npub1") {
                spans.push(Span::styled(word.to_string(), Style::default().fg(Color::Cyan)));
            } else if word.starts_with("http://") || word.starts_with("https://") {
                spans.push(Span::styled(word.to_string(), Style::default().fg(Color::LightBlue).add_modifier(Modifier::UNDERLINED)));
            } else {
                spans.push(Span::styled(word.to_string(), Style::default().fg(Color::White)));
            }
        }
        card_lines.push(Line::from(spans));
    }

    let reply_badge = if replies > 0 {
        Span::styled(format!(" 💬 {} ", replies), Style::default().fg(Color::Cyan))
    } else {
        Span::styled(" 💬 0 ", Style::default().fg(Color::DarkGray))
    };

    let repost_badge = if reposts > 0 {
        Span::styled(format!(" 🔁 {} ", reposts), Style::default().fg(Color::Green))
    } else {
        Span::styled(" 🔁 0 ", Style::default().fg(Color::DarkGray))
    };

    let like_badge = if is_liked_by_me {
        Span::styled(format!(" ❤️ {} LIKED ", likes), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
    } else if likes > 0 {
        Span::styled(format!(" ❤️ {} ", likes), Style::default().fg(Color::Red))
    } else {
        Span::styled(" 🤍 0 ", Style::default().fg(Color::DarkGray))
    };

    let bottom_border = if is_selected {
        Line::from(vec![
            Span::styled("┗━", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            reply_badge,
            repost_badge,
            like_badge,
            Span::styled(" ━━ [L] Like  [R] Reply  [g] Stream ━", Style::default().fg(Color::Yellow)),
        ])
    } else {
        Line::from(vec![
            Span::styled("╰─", Style::default().fg(Color::DarkGray)),
            reply_badge,
            repost_badge,
            like_badge,
            Span::styled(format!(" ─ id: {} ─", &event.id.to_hex()[..8]), Style::default().fg(Color::DarkGray)),
        ])
    };

    card_lines.push(bottom_border);
    card_lines.push(Line::from(""));

    ListItem::new(card_lines)
}

fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let side_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(area);

    let relays_block = Block::default()
        .title(" Relays and Intranet Nodes ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue));

    let mut relay_lines = Vec::new();

    if app.relay_statuses.is_empty() {
        relay_lines.push(Line::from(Span::styled("No relays connected", Style::default().fg(Color::DarkGray))));
    } else {
        for st in &app.relay_statuses {
            let (status_icon, color) = if st.connected {
                ("[+]", Color::Green)
            } else {
                ("[-]", Color::Red)
            };
            let latency_text = match st.latency_ms {
                Some(ms) => format!(" ({}ms)", ms),
                None => String::new(),
            };
            relay_lines.push(Line::from(vec![
                Span::styled(format!("{} ", status_icon), Style::default().fg(color)),
                Span::styled(st.url.clone(), Style::default().fg(Color::White)),
                Span::styled(latency_text, Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    relay_lines.push(Line::from(""));
    relay_lines.push(Line::from(Span::styled("Discovered LAN Peers:", Style::default().fg(Color::Cyan))));

    if app.discovered_peers.is_empty() {
        relay_lines.push(Line::from(Span::styled(" Searching UDP :48690...", Style::default().fg(Color::DarkGray))));
    } else {
        for peer in &app.discovered_peers {
            relay_lines.push(Line::from(vec![
                Span::styled(" LAN: ", Style::default().fg(Color::Yellow)),
                Span::styled(format!("{}:{} ", peer.ip, peer.relay_port), Style::default().fg(Color::White)),
                Span::styled(format!("[{}]", peer.name), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    let relays_widget = Paragraph::new(relay_lines).block(relays_block).wrap(Wrap { trim: true });
    f.render_widget(relays_widget, side_chunks[0]);

    let (info_title, info_lines) = match app.active_tab {
        ActiveTab::Station => {
            let public_relays_status = if app.public_relays_active {
                Span::styled("Connected (Damus, nos.lol)", Style::default().fg(Color::Green))
            } else {
                Span::styled("Disabled ([p] to connect)", Style::default().fg(Color::DarkGray))
            };

            let bookmarks_count = app.bookmarks.list().len();

            let lines = vec![
                Line::from(vec![
                    Span::styled("Active Tab: ", Style::default().fg(Color::DarkGray)),
                    Span::styled("DEADAIR Station ([Tab] switch)", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("Handle: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(app.identity.name.clone(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("Share Address: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(app.share_address.clone(), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("Detected IP: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(app.detected_ip.clone(), Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("Bookmarks: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{} saved ([b] connect)", bookmarks_count), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Internet Mesh: ", Style::default().fg(Color::DarkGray)),
                    public_relays_status,
                ]),
                Line::from(vec![
                    Span::styled("Offline Outbox: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{} pending", app.outbox.len()), Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from(Span::styled("Commands: /tab [station|public] | /share | /clear", Style::default().fg(Color::Cyan))),
            ];
            (" Domestic Station Specs ", lines)
        }
        ActiveTab::Public => {
            let selected_event = match app.selected_public_index {
                Some(idx) => app.public_timeline.get(idx).cloned(),
                None => app.public_timeline.last().cloned(),
            };

            let mut lines = Vec::new();
            if let Some(event) = selected_event {
                let display_name = app.profiles.get(&event.pubkey).cloned().unwrap_or_else(|| {
                    let hex = event.pubkey.to_hex();
                    format!("{}..{}", &hex[..6], &hex[hex.len() - 4..])
                });
                let profile = app.profile_details.get(&event.pubkey);
                let handle = profile
                    .and_then(|p| p.nip05.clone().or_else(|| p.name.clone()))
                    .unwrap_or_else(|| "anon".to_string());

                lines.push(Line::from(vec![
                    Span::styled("Author: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(display_name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled(format!(" (@{})", handle), Style::default().fg(Color::Cyan)),
                ]));

                let npub = event.pubkey.to_bech32().unwrap_or_else(|_| event.pubkey.to_hex());
                let short_npub = format!("{}..{}", &npub[..10], &npub[npub.len() - 6..]);
                lines.push(Line::from(vec![
                    Span::styled("Npub: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(short_npub, Style::default().fg(Color::DarkGray)),
                ]));

                if let Some(bio) = profile.and_then(|p| p.about.as_ref()) {
                    let clean_bio = bio.replace('\n', " ");
                    let short_bio = if clean_bio.len() > 50 {
                        format!("{}...", &clean_bio[..47])
                    } else {
                        clean_bio
                    };
                    lines.push(Line::from(vec![
                        Span::styled("Bio: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(short_bio, Style::default().fg(Color::Yellow)),
                    ]));
                }

                let likes = app.post_likes.get(&event.id).copied().unwrap_or(0);
                let replies = app.post_replies.get(&event.id).copied().unwrap_or(0);
                let reposts = app.post_reposts.get(&event.id).copied().unwrap_or(0);

                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Engagement: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("❤️ {} ", likes), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("💬 {} ", replies), Style::default().fg(Color::Cyan)),
                    Span::styled(format!("🔁 {}", reposts), Style::default().fg(Color::Green)),
                ]));

                lines.push(Line::from(vec![
                    Span::styled("Stream: ", Style::default().fg(Color::DarkGray)),
                    if app.selected_public_index.is_some() {
                        Span::styled("[PAUSED - BROWSING]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    } else {
                        Span::styled("[LIVE STREAMING]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                    },
                ]));

                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled("Social Hotkeys:", Style::default().fg(Color::Cyan))));
                lines.push(Line::from(vec![
                    Span::styled(" [L] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::styled("Like note   ", Style::default().fg(Color::White)),
                    Span::styled("[R] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled("Reply to post", Style::default().fg(Color::White)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled(" [j/k] ", Style::default().fg(Color::Yellow)),
                    Span::styled("Browse       ", Style::default().fg(Color::White)),
                    Span::styled("[g] ", Style::default().fg(Color::Green)),
                    Span::styled("Resume live", Style::default().fg(Color::White)),
                ]));
            } else {
                lines.push(Line::from(Span::styled("No public notes received yet.", Style::default().fg(Color::DarkGray))));
                lines.push(Line::from(Span::styled("Press [p] to connect to global relays.", Style::default().fg(Color::Yellow))));
            }
            (" Nostr Social Inspector ", lines)
        }
    };

    let info_block = Block::default()
        .title(info_title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let info_widget = Paragraph::new(info_lines).block(info_block);
    f.render_widget(info_widget, side_chunks[1]);
}

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let (border_color, prompt_prefix) = if app.input_mode == InputMode::Editing {
        (Color::Yellow, " >> ")
    } else {
        (Color::DarkGray, " >  ")
    };

    let title = match (app.input_mode, app.active_tab, &app.replying_to) {
        (InputMode::Editing, _, Some(target)) => {
            let author = app.profiles.get(&target.pubkey).cloned().unwrap_or_else(|| "peer".to_string());
            format!(" Replying to @{} (Enter to send reply, Esc to cancel) ", author)
        }
        (InputMode::Editing, ActiveTab::Station, None) => {
            " Broadcasting to DEADAIR Station (Enter to transmit, Esc for nav) ".to_string()
        }
        (InputMode::Editing, ActiveTab::Public, None) => {
            " Composing Public Nostr Note (DEADAIR signed, Enter to broadcast, Esc for nav) ".to_string()
        }
        (InputMode::Normal, ActiveTab::Station, _) => {
            " Station Chat (Press [i] or [Enter] to write, [Tab] for public feed) ".to_string()
        }
        (InputMode::Normal, ActiveTab::Public, _) => {
            " Public Social Feed (Press [i] write, [L] like, [R] reply, [j/k] browse) ".to_string()
        }
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let cursor = if app.input_mode == InputMode::Editing { "_" } else { "" };
    let input_text = format!("{}{}{}", prompt_prefix, app.input_buffer, cursor);
    let paragraph = Paragraph::new(input_text).block(block);
    f.render_widget(paragraph, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let footer_text = if let Some((msg, _)) = &app.status_toast {
        Line::from(vec![
            Span::styled(" * ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(msg.clone(), Style::default().fg(Color::White)),
        ])
    } else {
        let spans = match app.active_tab {
            ActiveTab::Public => vec![
                Span::styled(" [Tab]", Style::default().fg(Color::Cyan)),
                Span::styled(" Station Tab  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[j/k]", Style::default().fg(Color::Yellow)),
                Span::styled(" Browse  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[L]", Style::default().fg(Color::Red)),
                Span::styled(" Like  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[R]", Style::default().fg(Color::Cyan)),
                Span::styled(" Reply  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[g]", Style::default().fg(Color::Green)),
                Span::styled(" Live  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[i]", Style::default().fg(Color::Cyan)),
                Span::styled(" Compose  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[p]", Style::default().fg(Color::Cyan)),
                Span::styled(" Relays  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[c]", Style::default().fg(Color::Cyan)),
                Span::styled(" Clear  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[q]", Style::default().fg(Color::Cyan)),
                Span::styled(" Quit", Style::default().fg(Color::DarkGray)),
            ],
            ActiveTab::Station => vec![
                Span::styled(" [Tab]", Style::default().fg(Color::Cyan)),
                Span::styled(" Public Mesh  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[i/Enter]", Style::default().fg(Color::Cyan)),
                Span::styled(" Write  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[s]", Style::default().fg(Color::Cyan)),
                Span::styled(" Share  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[b]", Style::default().fg(Color::Cyan)),
                Span::styled(" Bookmarks  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[r]", Style::default().fg(Color::Cyan)),
                Span::styled(" Relay  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[p]", Style::default().fg(Color::Cyan)),
                Span::styled(" Public  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[c]", Style::default().fg(Color::Cyan)),
                Span::styled(" Clear  ", Style::default().fg(Color::DarkGray)),
                Span::styled("[q]", Style::default().fg(Color::Cyan)),
                Span::styled(" Quit", Style::default().fg(Color::DarkGray)),
            ],
        };
        Line::from(spans)
    };

    let paragraph = Paragraph::new(footer_text);
    f.render_widget(paragraph, area);
}

fn format_relative_time(created_at: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(created_at);

    if created_at > now {
        return "just now".to_string();
    }

    let diff = now - created_at;
    if diff < 10 {
        "just now".to_string()
    } else if diff < 60 {
        format!("{}s ago", diff)
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else if diff < 604800 {
        format!("{}d ago", diff / 86400)
    } else {
        format_timestamp(created_at)
    }
}

fn author_color_from_pubkey(pk: &PublicKey) -> Color {
    let bytes = pk.to_bytes();
    match (bytes[0] ^ bytes[1]) % 6 {
        0 => Color::Cyan,
        1 => Color::Magenta,
        2 => Color::Yellow,
        3 => Color::Green,
        4 => Color::LightBlue,
        _ => Color::LightRed,
    }
}

fn format_timestamp(ts: u64) -> String {
    let secs = ts % 86400;
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_relative_time() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        assert_eq!(format_relative_time(now), "just now");
        assert_eq!(format_relative_time(now - 5), "just now");
        assert_eq!(format_relative_time(now - 30), "30s ago");
        assert_eq!(format_relative_time(now - 120), "2m ago");
        assert_eq!(format_relative_time(now - 7200), "2h ago");
        assert_eq!(format_relative_time(now - 172800), "2d ago");
    }

    #[test]
    fn test_author_color_consistency() {
        let pk = PublicKey::from_hex("0000000000000000000000000000000000000000000000000000000000000001").unwrap();
        let color1 = author_color_from_pubkey(&pk);
        let color2 = author_color_from_pubkey(&pk);
        assert_eq!(color1, color2);
    }
}
