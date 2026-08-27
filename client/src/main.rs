mod app;
mod highlight;
mod history;
mod theme;
mod ui;

use anyhow::{Context, Result};
use app::{App, AppEvent};
use futures_util::{SinkExt, StreamExt};
use protocol::{ClientMsg, Message, ServerMsg};
use std::sync::Arc;
use std::time::{Duration, Instant};
use theme::Theme;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

#[tokio::main]
async fn main() -> Result<()> {
    // install rustls crypto provider (ring) before any TLS connections
    let _ = rustls::crypto::ring::default_provider().install_default();

    // positional args: nick, addr. `--theme <name>` / `-t <name>` can appear
    // anywhere; falls back to the last saved theme, then "midnight".
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut positional = Vec::new();
    let mut theme_name: Option<String> = None;
    let mut it = raw.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--theme" | "-t" => theme_name = it.next(),
            _ => positional.push(a),
        }
    }
    let theme_name = theme_name
        .or_else(theme::load_saved_theme)
        .unwrap_or_else(|| "midnight".into());
    let theme = Theme::by_name(&theme_name);

    let mut pos = positional.into_iter();
    let nick = pos.next().unwrap_or_else(whoami);
    let addr = pos.next().unwrap_or_else(|| "ws://127.0.0.1:7777".into());
    run(nick, addr, theme).await
}

fn whoami() -> String {
    std::env::var("USER").unwrap_or_else(|_| "anon".into())
}

pub async fn run(nick: String, addr: String, theme: Theme) -> Result<()> {
    let url = normalize_url(&addr);
    let server_label = display_host(&url);
    let (ws_stream, _) = connect_async(&url)
        .await
        .with_context(|| format!("cannot connect to {url}"))?;
    let (mut ws_sink, mut ws_src) = ws_stream.split();

    let hello = serde_json::to_string(&ClientMsg::Hello { nick: nick.clone() })?;
    ws_sink.send(WsMessage::Text(hello.into())).await?;

    let (ev_tx, ev_rx) = mpsc::unbounded_channel::<AppEvent>();
    let mut app = App::new(nick.clone(), server_label, history::load(&nick), theme);

    // shared sink for writer + ping tasks
    let sink = Arc::new(Mutex::new(ws_sink));
    let ping_sent = Arc::new(Mutex::new(None::<Instant>));

    // reader task: WebSocket text frames -> AppEvent
    let tx = ev_tx.clone();
    let ping_sent_r = ping_sent.clone();
    tokio::spawn(async move {
        while let Some(Ok(WsMessage::Text(text))) = ws_src.next().await {
            if let Ok(msg) = serde_json::from_str::<ServerMsg>(&text) {
                if matches!(msg, ServerMsg::Pong) {
                    if let Some(t0) = ping_sent_r.lock().await.take() {
                        let ms = t0.elapsed().as_millis() as u64;
                        let _ = tx.send(AppEvent::Lag(ms));
                    }
                }
                if tx.send(AppEvent::Server(msg)).is_err() {
                    break;
                }
            }
        }
    });

    // writer task: outgoing messages -> WebSocket text frames
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();
    let sink_w = sink.clone();
    tokio::spawn(async move {
        while let Some(m) = out_rx.recv().await {
            if let Ok(line) = serde_json::to_string(&ClientMsg::Send(m)) {
                let mut s = sink_w.lock().await;
                if s.send(WsMessage::Text(line.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // ping + UI tick every second
    let sink_p = sink.clone();
    let ping_sent_w = ping_sent.clone();
    let tx_tick = ev_tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            let _ = tx_tick.send(AppEvent::Tick);
            if let Ok(line) = serde_json::to_string(&ClientMsg::Ping) {
                *ping_sent_w.lock().await = Some(Instant::now());
                let mut s = sink_p.lock().await;
                if s.send(WsMessage::Text(line.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut terminal = ratatui::init();
    let res = ui::run_app(&mut terminal, &mut app, ev_rx, out_tx.clone()).await;
    ratatui::restore();
    history::save(&nick, &app.buffers)?;
    res
}

fn display_host(url: &str) -> String {
    url.trim_start_matches("ws://")
        .trim_start_matches("wss://")
        .split('/')
        .next()
        .unwrap_or(url)
        .to_string()
}

/// Normalize user-provided address into a WebSocket URL.
///   "ws://host"        -> "ws://host"
///   "wss://host"       -> "wss://host"
///   "host:port"        -> "wss://host:port"    (remote = TLS by default)
///   "localhost:7777"   -> "ws://localhost:7777" (local = plain)
fn normalize_url(addr: &str) -> String {
    if addr.starts_with("ws://") || addr.starts_with("wss://") {
        return addr.to_string();
    }
    let lower = addr.to_ascii_lowercase();
    if lower.contains("localhost") || lower.starts_with("127.") || lower.starts_with("0.") {
        format!("ws://{addr}")
    } else {
        format!("wss://{addr}")
    }
}
