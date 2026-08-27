use crate::app::{App, AppEvent, Focus, NotifKind};
use crate::highlight::highlight_body;
use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures_util::StreamExt;
use protocol::Message;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Sparkline};
use ratatui::Frame;
use tokio::sync::mpsc;

pub async fn run_app(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    mut ev_rx: mpsc::UnboundedReceiver<AppEvent>,
    out_tx: mpsc::UnboundedSender<Message>,
) -> Result<()> {
    let mut events = EventStream::new();
    loop {
        terminal.draw(|f| draw(f, app))?;
        tokio::select! {
            Some(ev) = ev_rx.recv() => {
                match ev {
                    AppEvent::Server(msg) => app.handle_server(msg),
                    AppEvent::Tick => app.on_tick(),
                    AppEvent::Lag(ms) => app.lag_ms = Some(ms),
                }
            }
            maybe = events.next() => {
                let Some(Ok(ev)) = maybe else { break };
                if let Event::Key(key) = ev {
                    if key.kind != KeyEventKind::Press { continue; }
                    if handle_key(app, key.code, key.modifiers, &out_tx)? {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

fn handle_key(
    app: &mut App,
    code: KeyCode,
    mods: KeyModifiers,
    out_tx: &mpsc::UnboundedSender<Message>,
) -> Result<bool> {
    // global quit
    if matches!(code, KeyCode::Char('c') | KeyCode::Char('q'))
        && mods.contains(KeyModifiers::CONTROL)
    {
        return Ok(true);
    }

    // help overlay: any key dismisses except F1 toggle
    if app.show_help {
        match code {
            KeyCode::F(1) | KeyCode::Esc | KeyCode::Char('?') => app.show_help = false,
            _ => {}
        }
        return Ok(false);
    }

    match code {
        KeyCode::F(1) => app.show_help = true,
        KeyCode::F(2) => {
            app.focus = match app.focus {
                Focus::Sidebar => Focus::Chat,
                _ => Focus::Sidebar,
            };
        }
        KeyCode::F(3) => {
            // jump to #general
            if let Some(i) = app.order.iter().position(|k| k == "#general") {
                app.select_buffer_index(i);
            }
        }
        KeyCode::F(4) => {
            // cycle themes
            let names = Theme::names();
            let i = names
                .iter()
                .position(|n| *n == app.theme.name)
                .unwrap_or(0);
            let next = names[(i + 1) % names.len()];
            app.theme = Theme::by_name(next);
            let _ = crate::theme::save_theme(next);
            app.status = format!("theme -> {next}");
        }
        KeyCode::F(5) => app.scroll_bottom(),
        KeyCode::F(10) => return Ok(true),
        KeyCode::Tab => {
            if app.focus == Focus::Sidebar {
                app.next_buffer();
            } else {
                app.next_buffer();
            }
        }
        KeyCode::BackTab => app.prev_buffer(),
        KeyCode::PageUp => app.scroll_up(10),
        KeyCode::PageDown => app.scroll_down(10),
        KeyCode::Up if mods.contains(KeyModifiers::CONTROL) => app.scroll_up(1),
        KeyCode::Down if mods.contains(KeyModifiers::CONTROL) => app.scroll_down(1),
        KeyCode::Home if mods.contains(KeyModifiers::CONTROL) => {
            if let Some(b) = app.active_buffer_mut() {
                b.scroll = b.messages.len().saturating_sub(1);
            }
        }
        KeyCode::End if mods.contains(KeyModifiers::CONTROL) => app.scroll_bottom(),
        KeyCode::Enter => {
            if app.submit(out_tx).is_err() {
                app.status = "send failed".into();
            }
        }
        KeyCode::Backspace => app.backspace(),
        KeyCode::Delete => app.delete_forward(),
        KeyCode::Left => app.cursor_left(),
        KeyCode::Right => app.cursor_right(),
        KeyCode::Home => app.cursor = 0,
        KeyCode::End => app.cursor = app.input.len(),
        KeyCode::Esc => {
            app.input.clear();
            app.cursor = 0;
            app.focus = Focus::Chat;
        }
        KeyCode::Char(c) if !mods.contains(KeyModifiers::CONTROL) => app.insert_char(c),
        _ => {}
    }
    Ok(false)
}

fn msg_line(num: usize, m: &Message, me: &str, theme: &Theme) -> Line<'static> {
    let t = m.ts.format("%H:%M:%S").to_string();
    let thread_mark = if m.reply_to.is_some() { " ↳" } else { "" };

    // system / status lines from the server use from="*"
    if m.from == "*" {
        return Line::from(vec![
            Span::styled(format!("{num:>3} "), Style::default().fg(theme.msg_num)),
            Span::styled(format!("[{t}] "), Style::default().fg(theme.timestamp)),
            Span::styled(
                format!("* {}", m.body),
                Style::default()
                    .fg(theme.mention_fg)
                    .bg(theme.mention_bg)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]);
    }

    let (from_style, base_body) = if m.from == me {
        (
            Style::default()
                .fg(theme.self_nick)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(theme.self_body),
        )
    } else {
        (
            Style::default()
                .fg(theme.nick_color(&m.from))
                .add_modifier(Modifier::BOLD),
            Style::default().fg(theme.text),
        )
    };
    let mentioned = m.from != me
        && m.body
            .split_whitespace()
            .any(|w| w.trim_matches(|c: char| !c.is_alphanumeric()) == me);
    let body_style = if mentioned {
        Style::default()
            .fg(theme.mention_fg)
            .bg(theme.mention_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        base_body
    };

    let mut spans = vec![
        Span::styled(format!("{num:>3} "), Style::default().fg(theme.msg_num)),
        Span::styled(format!("[{t}] "), Style::default().fg(theme.timestamp)),
        Span::styled(format!("<{}>", m.from), from_style),
        Span::styled(thread_mark.to_string(), Style::default().fg(theme.thread)),
        Span::raw(" "),
    ];
    if mentioned {
        spans.push(Span::styled(m.body.clone(), body_style));
    } else {
        spans.extend(highlight_body(&m.body, theme, body_style));
    }
    Line::from(spans)
}

fn draw(f: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = f.area();

    // paint full background
    f.render_widget(Block::new().style(Style::default().bg(theme.bg)), area);

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(3),
    ])
    .areas(area);

    draw_header(f, header, app);
    draw_body(f, body, app);
    draw_footer(f, footer, app);

    if app.show_help {
        draw_help_modal(f, area, app);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let key = app.active_key();
    let buffer = app.active_buffer();
    let online = buffer.map(|b| b.members.len()).unwrap_or(0);
    let context = match (key.strip_prefix('#'), key.strip_prefix('@')) {
        (Some(c), _) => {
            let kind = if app.threads.contains_key(&key) {
                "thread"
            } else {
                "channel"
            };
            format!("{kind} #{c} · {online} online")
        }
        (None, Some(peer)) => format!("DM with @{peer}"),
        _ => "server console".to_string(),
    };
    let lag = match app.lag_ms {
        Some(ms) if ms < 80 => Span::styled(
            format!(" Lag:{ms}ms "),
            Style::default().fg(theme.ok).add_modifier(Modifier::BOLD),
        ),
        Some(ms) if ms < 200 => Span::styled(
            format!(" Lag:{ms}ms "),
            Style::default().fg(theme.warn).add_modifier(Modifier::BOLD),
        ),
        Some(ms) => Span::styled(
            format!(" Lag:{ms}ms "),
            Style::default().fg(theme.err).add_modifier(Modifier::BOLD),
        ),
        None => Span::styled(" Lag:… ", Style::default().fg(theme.text_dim)),
    };
    let now = chrono::Local::now().format("%H:%M:%S").to_string();

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " ✦ chattui ",
                Style::default()
                    .fg(theme.brand_fg)
                    .bg(theme.brand_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" you are {} ", app.me),
                Style::default()
                    .fg(theme.me_fg)
                    .bg(theme.me_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  talking in ", Style::default().fg(theme.text_dim)),
            Span::styled(context, Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            lag,
            Span::styled(
                format!("  {now}  "),
                Style::default().fg(theme.accent_alt),
            ),
            Span::styled(
                format!("theme:{} ", theme.name),
                Style::default().fg(theme.text_dim),
            ),
        ]))
        .style(Style::default().bg(theme.bg)),
        area,
    );
}

fn draw_body(f: &mut Frame, area: Rect, app: &App) {
    let [sidebar, center, right] = Layout::horizontal([
        Constraint::Length(28),
        Constraint::Min(30),
        Constraint::Length(24),
    ])
    .areas(area);

    draw_sidebar(f, sidebar, app);
    draw_center(f, center, app);
    draw_right(f, right, app);
}

fn section_title<'a>(label: &'a str, count: Option<usize>, theme: &Theme) -> Line<'a> {
    let text = match count {
        Some(n) => format!(" {label} ({n}) "),
        None => format!(" {label} "),
    };
    Line::from(Span::styled(
        text,
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    ))
}

fn draw_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let channels = app.channel_keys();
    let queries = app.query_keys();
    let threads = app.thread_keys();

    let chan_h = (channels.len() as u16 + 2).clamp(4, 10);
    let query_h = (queries.len() as u16 + 2).clamp(3, 8);
    let thread_h = if threads.is_empty() {
        0
    } else {
        (threads.len() as u16 + 2).min(5)
    };

    let [servers, chans, qs, ths, cmds] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(chan_h),
        Constraint::Length(query_h),
        Constraint::Length(thread_h),
        Constraint::Min(6),
    ])
    .areas(area);

    // ---- SERVERS
    let online = app
        .buffers
        .values()
        .map(|b| b.members.len())
        .max()
        .unwrap_or(1);
    let server_items = vec![ListItem::new(Line::from(vec![
        Span::styled(" ● ", Style::default().fg(theme.presence)),
        Span::styled(
            format!("{}  ", app.server_label),
            Style::default()
                .fg(theme.active_fg)
                .bg(theme.active_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{online} online"),
            Style::default().fg(theme.text_dim),
        ),
    ]))];
    f.render_widget(
        List::new(server_items).block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(section_title("servers", None, theme)),
        ),
        servers,
    );

    // ---- CHANNELS
    let chan_items: Vec<ListItem> = channels
        .iter()
        .map(|(i, k)| {
            let unread = app.buffers.get(k).map(|b| b.unread).unwrap_or(0);
            let name = k.trim_start_matches('#');
            let mut spans = vec![
                Span::styled(" # ", Style::default().fg(theme.channel)),
                Span::styled(name.to_string(), Style::default().fg(theme.text)),
            ];
            if unread > 0 {
                spans.push(Span::styled(
                    format!("  {unread}"),
                    Style::default()
                        .fg(theme.unread)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            let style = if *i == app.active {
                Style::default().fg(theme.active_fg).bg(theme.active_bg)
            } else if unread > 0 {
                Style::default().fg(theme.unread)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(spans).style(style))
        })
        .collect();
    f.render_widget(
        List::new(chan_items).block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(
                    if app.focus == Focus::Sidebar {
                        theme.border_focus
                    } else {
                        theme.border
                    },
                ))
                .title(section_title("channels", Some(channels.len()), theme)),
        ),
        chans,
    );

    // ---- QUERIES (DMs)
    let q_items: Vec<ListItem> = if queries.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  /query nick",
            Style::default().fg(theme.text_dim).add_modifier(Modifier::ITALIC),
        )))]
    } else {
        queries
            .iter()
            .map(|(i, k)| {
                let unread = app.buffers.get(k).map(|b| b.unread).unwrap_or(0);
                let name = k.trim_start_matches('@');
                // presence: online if appears in any channel member list
                let online = app
                    .buffers
                    .values()
                    .any(|b| b.members.iter().any(|m| m == name));
                let dot = if online {
                    Span::styled(" ●", Style::default().fg(theme.presence))
                } else {
                    Span::styled(" ○", Style::default().fg(theme.text_dim))
                };
                let mut spans = vec![
                    dot,
                    Span::styled(format!(" @{name}"), Style::default().fg(theme.dm)),
                ];
                if unread > 0 {
                    spans.push(Span::styled(
                        format!("  {unread}"),
                        Style::default()
                            .fg(theme.unread)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                let style = if *i == app.active {
                    Style::default().fg(theme.active_fg).bg(theme.active_bg)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(spans).style(style))
            })
            .collect()
    };
    f.render_widget(
        List::new(q_items).block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(section_title("queries", Some(queries.len()), theme)),
        ),
        qs,
    );

    // ---- THREADS (optional)
    if thread_h > 0 {
        let t_items: Vec<ListItem> = threads
            .iter()
            .map(|(i, k)| {
                let style = if *i == app.active {
                    Style::default().fg(theme.active_fg).bg(theme.active_bg)
                } else {
                    Style::default().fg(theme.thread)
                };
                ListItem::new(Line::from(format!(" ↳ {k}")).style(style))
            })
            .collect();
        f.render_widget(
            List::new(t_items).block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .title(section_title("threads", Some(threads.len()), theme)),
            ),
            ths,
        );
    }

    // ---- COMMANDS cheatsheet
    let dim = Style::default().fg(theme.text_dim);
    let cmd = Style::default().fg(theme.unread);
    let help_lines = [
        Line::from(vec![
            Span::styled("/join #c", cmd),
            Span::styled("  channel", dim),
        ]),
        Line::from(vec![
            Span::styled("/query n", cmd),
            Span::styled("  private", dim),
        ]),
        Line::from(vec![
            Span::styled("/reply N", cmd),
            Span::styled("  thread", dim),
        ]),
        Line::from(vec![
            Span::styled("/topic  ", cmd),
            Span::styled("  set topic", dim),
        ]),
        Line::from(vec![
            Span::styled("/theme  ", cmd),
            Span::styled("  colors", dim),
        ]),
        Line::from(vec![
            Span::styled("/part   ", cmd),
            Span::styled("  close buf", dim),
        ]),
    ];
    f.render_widget(
        List::new(help_lines.into_iter().map(ListItem::new)).block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(Span::styled(
                    " commands ",
                    Style::default().fg(theme.accent_alt).add_modifier(Modifier::BOLD),
                )),
        ),
        cmds,
    );
}

fn draw_center(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let key = app.active_key();
    let buffer = app.active_buffer();

    let [topic_area, msgs_area, input_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(3),
    ])
    .areas(area);

    // topic / channel header
    let topic = buffer
        .map(|b| {
            if b.topic.is_empty() {
                if key.starts_with('@') {
                    format!("private conversation · Tab to switch chats")
                } else {
                    format!("no topic — /topic <text> to set one")
                }
            } else {
                b.topic.clone()
            }
        })
        .unwrap_or_else(|| "—".into());
    let members = buffer.map(|b| b.members.len()).unwrap_or(0);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {key} "),
                Style::default()
                    .fg(theme.active_fg)
                    .bg(theme.active_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Topic: ", Style::default().fg(theme.text_dim)),
            Span::styled(topic, Style::default().fg(theme.text)),
        ]))
        .block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(Span::styled(
                    format!(" messages · {members} here "),
                    Style::default().fg(theme.accent),
                )),
        ),
        topic_area,
    );

    // messages with scroll
    let scroll = buffer.map(|b| b.scroll).unwrap_or(0);
    let visible_h = msgs_area.height.saturating_sub(2) as usize;
    let msgs: Vec<Line> = buffer
        .map(|b| {
            let end = b.messages.len().saturating_sub(b.scroll);
            let start = end.saturating_sub(visible_h.max(1));
            b.messages[start..end]
                .iter()
                .enumerate()
                .map(|(i, m)| msg_line(start + i + 1, m, &app.me, theme))
                .collect()
        })
        .unwrap_or_default();

    let mut title_spans = vec![Span::styled(
        format!(" {key} "),
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )];
    if scroll > 0 {
        title_spans.push(Span::styled(
            format!(" ↓ {scroll} unread below ↓ "),
            Style::default()
                .fg(theme.mention_fg)
                .bg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
    }

    f.render_widget(
        Paragraph::new(msgs).block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(Line::from(title_spans)),
        ),
        msgs_area,
    );

    // input with cursor
    let input_line = if app.input.is_empty() {
        Line::from(vec![
            Span::styled(
                "█",
                Style::default()
                    .fg(theme.border_focus)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Type a message…  (/ for commands, F1 help)",
                Style::default()
                    .fg(theme.text_dim)
                    .add_modifier(Modifier::ITALIC),
            ),
        ])
    } else {
        let before = &app.input[..app.cursor.min(app.input.len())];
        let after = &app.input[app.cursor.min(app.input.len())..];
        let cursor_ch = after.chars().next().unwrap_or(' ');
        let rest: String = after.chars().skip(1).collect();
        Line::from(vec![
            Span::styled(before.to_string(), Style::default().fg(theme.text)),
            Span::styled(
                cursor_ch.to_string(),
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.border_focus)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(rest, Style::default().fg(theme.text)),
        ])
    };
    f.render_widget(
        Paragraph::new(input_line).block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_focus))
                .title(Span::styled(
                    format!(" {key} · Enter send · PgUp/PgDn scroll "),
                    Style::default().fg(theme.text_dim),
                )),
        ),
        input_area,
    );
}

fn draw_right(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let key = app.active_key();
    let buffer = app.active_buffer();
    let members = buffer.map(|b| b.members.as_slice()).unwrap_or(&[]);

    let [users, info, notifs] = Layout::vertical([
        Constraint::Min(6),
        Constraint::Length(7),
        Constraint::Length(8),
    ])
    .areas(area);

    // ---- USERS with role glyphs
    let user_items: Vec<ListItem> = if members.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  (no roster yet)",
            Style::default().fg(theme.text_dim).add_modifier(Modifier::ITALIC),
        )))]
    } else {
        members
            .iter()
            .enumerate()
            .map(|(idx, n)| {
                let role = if idx == 0 {
                    Span::styled(" ♛ ", Style::default().fg(theme.warn))
                } else if n == &app.me {
                    Span::styled(" ◆ ", Style::default().fg(theme.self_nick))
                } else {
                    Span::styled(" ◇ ", Style::default().fg(theme.channel))
                };
                let nick_style = if n == &app.me {
                    Style::default()
                        .fg(theme.self_nick)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.nick_color(n))
                };
                ListItem::new(Line::from(vec![
                    role,
                    Span::styled(n.clone(), nick_style),
                ]))
            })
            .collect()
    };
    f.render_widget(
        List::new(user_items).block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(Span::styled(
                    format!(" users in {key} ({}) ", members.len()),
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                )),
        ),
        users,
    );

    // ---- CHANNEL INFO
    let uptime = chrono::Utc::now()
        .signed_duration_since(app.connected_at)
        .num_seconds()
        .max(0);
    let up_h = uptime / 3600;
    let up_m = (uptime % 3600) / 60;
    let info_lines = vec![
        Line::from(vec![
            Span::styled(" mode  ", Style::default().fg(theme.text_dim)),
            Span::styled("+ntr", Style::default().fg(theme.channel)),
        ]),
        Line::from(vec![
            Span::styled(" topic ", Style::default().fg(theme.text_dim)),
            Span::styled(
                if buffer.map(|b| !b.topic.is_empty()).unwrap_or(false) {
                    "locked"
                } else {
                    "open"
                },
                Style::default().fg(theme.accent_alt),
            ),
        ]),
        Line::from(vec![
            Span::styled(" msgs  ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{}", app.total_messages),
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled(" up    ", Style::default().fg(theme.text_dim)),
            Span::styled(format!("{up_h}h{up_m:02}m"), Style::default().fg(theme.ok)),
        ]),
        Line::from(vec![
            Span::styled(" bufs  ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{}", app.order.len()),
                Style::default().fg(theme.text),
            ),
        ]),
    ];
    f.render_widget(
        List::new(info_lines.into_iter().map(ListItem::new)).block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(Span::styled(
                    " channel info ",
                    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
                )),
        ),
        info,
    );

    // ---- NOTIFICATIONS
    let n_items: Vec<ListItem> = if app.notifications.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  quiet for now",
            Style::default().fg(theme.text_dim).add_modifier(Modifier::ITALIC),
        )))]
    } else {
        app.notifications
            .iter()
            .take(6)
            .map(|n| {
                let (icon, color) = match n.kind {
                    NotifKind::Mention => ("@", theme.unread),
                    NotifKind::Join => ("+", theme.ok),
                    NotifKind::Warn => ("!", theme.err),
                    NotifKind::Info => ("·", theme.accent),
                };
                let t = n.ts.format("%H:%M").to_string();
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {icon} "), Style::default().fg(color)),
                    Span::styled(format!("{t} "), Style::default().fg(theme.text_dim)),
                    Span::styled(n.text.clone(), Style::default().fg(theme.text)),
                ]))
            })
            .collect()
    };
    f.render_widget(
        List::new(n_items).block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(Span::styled(
                    " notifications ",
                    Style::default()
                        .fg(theme.accent_alt)
                        .add_modifier(Modifier::BOLD),
                )),
        ),
        notifs,
    );
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let [status_row, spark_row, keys_row] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
            .areas(area);

    // status + activity gauge
    let unread_total: usize = app.buffers.values().map(|b| b.unread).sum();
    let status_text = if unread_total > 0 {
        format!(" {}  ·  {} unread elsewhere ", app.status, unread_total)
    } else {
        format!(" {} ", app.status)
    };
    let [status_area, gauge_area] =
        Layout::horizontal([Constraint::Min(20), Constraint::Length(22)]).areas(status_row);

    f.render_widget(
        Paragraph::new(Span::styled(
            status_text,
            Style::default().fg(theme.bg).bg(theme.text_dim),
        )),
        status_area,
    );

    let rate = app.activity.back().copied().unwrap_or(0);
    let ratio = (rate as f64 / 5.0).clamp(0.0, 1.0);
    let g_color = if ratio > 0.7 {
        theme.err
    } else if ratio > 0.3 {
        theme.warn
    } else {
        theme.ok
    };
    f.render_widget(
        Gauge::default()
            .gauge_style(Style::default().fg(g_color).bg(theme.bg))
            .ratio(ratio)
            .label(format!("activity {rate}/s")),
        gauge_area,
    );

    // sparkline of recent message rates
    let data: Vec<u64> = app.activity.iter().copied().collect();
    let [spark_label, spark_area] =
        Layout::horizontal([Constraint::Length(12), Constraint::Min(1)]).areas(spark_row);
    f.render_widget(
        Paragraph::new(Span::styled(
            " msg/sec ",
            Style::default().fg(theme.text_dim),
        )),
        spark_label,
    );
    f.render_widget(
        Sparkline::default()
            .data(&data)
            .style(Style::default().fg(theme.accent)),
        spark_area,
    );

    // F-key bar
    let key = |k: &str, label: &str| {
        vec![
            Span::styled(
                format!(" {k} "),
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" {label} "), Style::default().fg(theme.text_dim)),
        ]
    };
    let mut spans = Vec::new();
    for (k, label) in [
        ("F1", "Help"),
        ("F2", "Sidebar"),
        ("F3", "#general"),
        ("F4", "Theme"),
        ("F5", "Bottom"),
        ("Tab", "Next"),
        ("F10", "Quit"),
    ] {
        spans.extend(key(k, label));
    }
    if unread_total > 0 {
        spans.push(Span::styled(
            format!("  NEW ↓ {unread_total} "),
            Style::default()
                .fg(theme.mention_fg)
                .bg(theme.unread)
                .add_modifier(Modifier::BOLD),
        ));
    }
    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Left),
        keys_row,
    );
}

fn draw_help_modal(f: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let w = area.width.min(64).max(40);
    let h = area.height.min(20).max(12);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect::new(x, y, w, h);
    f.render_widget(Clear, popup);

    let lines = vec![
        Line::from(Span::styled(
            " chattui — keyboard & commands ",
            Style::default()
                .fg(theme.brand_fg)
                .bg(theme.brand_bg)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("Navigation", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
        Line::from("  Tab / Shift+Tab   next / prev buffer"),
        Line::from("  PgUp / PgDn       scroll history"),
        Line::from("  Ctrl+↑ / Ctrl+↓   scroll one line"),
        Line::from("  F3                jump to #general"),
        Line::from("  F5                jump to latest"),
        Line::from(""),
        Line::from(Span::styled("Commands", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD))),
        Line::from("  /join #chan   /query nick   /msg nick text"),
        Line::from("  /reply [N] text   /thread N   /topic text"),
        Line::from("  /theme name   /part   /help"),
        Line::from(""),
        Line::from(Span::styled(
            " Esc clears input · F1 / Esc closes this help ",
            Style::default().fg(theme.text_dim),
        )),
    ];
    f.render_widget(
        Paragraph::new(lines).block(
            Block::new()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_focus))
                .style(Style::default().bg(theme.bg).fg(theme.text))
                .title(Span::styled(
                    " help ",
                    Style::default().fg(theme.accent_alt).add_modifier(Modifier::BOLD),
                )),
        ),
        popup,
    );
}
