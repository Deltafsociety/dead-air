use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, InputMode};

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

    let block = Block::default()
        .title(format!(" Local Timeline ({} events) ", app.timeline.len()))
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    if app.timeline.is_empty() {
        let empty_msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("No transmissions yet.", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("Type a note below and hit [Enter] to broadcast.", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("Type /share to see your domestic address for friends.", Style::default().fg(Color::DarkGray))),
        ])
        .alignment(Alignment::Center)
        .block(block);
        f.render_widget(empty_msg, area);
        return;
    }

    let mut list_items = Vec::new();
    for event in app.timeline.iter() {
        let is_self = event.pubkey == app.identity.keys.public_key();
        let display_name = app.profiles.get(&event.pubkey).cloned().unwrap_or_else(|| {
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

    let list_len = list_items.len();
    let visible_height = area.height.saturating_sub(2) as usize;
    let scroll_offset = if list_len > visible_height {
        list_len - visible_height
    } else {
        0
    };

    let items_slice: Vec<ListItem> = list_items.into_iter().skip(scroll_offset).collect();
    let list = List::new(items_slice).block(block);
    f.render_widget(list, area);
}

fn render_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let side_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
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

    let info_block = Block::default()
        .title(" Domestic Station Specs ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let public_relays_status = if app.public_relays_active {
        Span::styled("Connected (Damus, nos.lol)", Style::default().fg(Color::Green))
    } else {
        Span::styled("Disabled ([p] to connect)", Style::default().fg(Color::DarkGray))
    };

    let bookmarks_count = app.bookmarks.list().len();

    let info_lines = vec![
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
        Line::from(Span::styled("Commands: /share | /bookmark add <url> [name]", Style::default().fg(Color::Cyan))),
    ];

    let info_widget = Paragraph::new(info_lines).block(info_block);
    f.render_widget(info_widget, side_chunks[1]);
}

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let (border_color, prompt_prefix) = if app.input_mode == InputMode::Editing {
        (Color::Yellow, " >> ")
    } else {
        (Color::DarkGray, " >  ")
    };

    let title = if app.input_mode == InputMode::Editing {
        " Broadcasting (Enter to transmit, /share for address, Esc for nav) "
    } else {
        " Note Buffer (Press [i] or [Enter] to write) "
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
        Line::from(vec![
            Span::styled(" [i/Enter]", Style::default().fg(Color::Cyan)),
            Span::styled(" Write  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[s]", Style::default().fg(Color::Cyan)),
            Span::styled(" Share Address  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[b]", Style::default().fg(Color::Cyan)),
            Span::styled(" Connect Bookmarks  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[r]", Style::default().fg(Color::Cyan)),
            Span::styled(" Toggle Relay  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[p]", Style::default().fg(Color::Cyan)),
            Span::styled(" Public Relays  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[q]", Style::default().fg(Color::Cyan)),
            Span::styled(" Quit", Style::default().fg(Color::DarkGray)),
        ])
    };

    let paragraph = Paragraph::new(footer_text);
    f.render_widget(paragraph, area);
}

fn format_timestamp(ts: u64) -> String {
    let secs = ts % 86400;
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, s)
}
