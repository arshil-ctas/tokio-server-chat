use anyhow::Result;
use axum::{
    extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    extract::State as AxState,
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use protocol::{ClientMsg, ServerMsg, Target};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

type Tx = mpsc::UnboundedSender<String>;

#[derive(Default)]
struct State {
    clients: HashMap<String, Tx>,
}

impl State {
    fn route(&self, msg: &protocol::Message) {
        let line = serde_json::to_string(&ServerMsg::Msg(msg.clone())).unwrap();
        let recipients: Vec<String> = match &msg.target {
            Target::Channel(_) => {
                self.clients.keys().filter(|n| **n != msg.from).cloned().collect()
            }
            Target::Dm(a, b) => {
                if msg.from == *a { vec![b.clone()] } else { vec![a.clone()] }
            }
        };
        for r in recipients {
            if let Some(tx) = self.clients.get(&r) {
                let _ = tx.send(line.clone());
            }
        }
        if let Some(tx) = self.clients.get(&msg.from) {
            let _ = tx.send(line);
        }
    }

    fn nicks_of(&self) -> Vec<String> {
        let mut v: Vec<String> = self.clients.keys().cloned().collect();
        v.sort();
        v
    }
}

async fn health(AxState(state): AxState<Arc<Mutex<State>>>) -> impl IntoResponse {
    let n = state.lock().await.clients.len();
    axum::Json(serde_json::json!({"ok": true, "clients": n}))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    AxState(state): AxState<Arc<Mutex<State>>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<Mutex<State>>) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let mut nick: Option<String> = None;

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        let Ok(cmsg) = serde_json::from_str::<ClientMsg>(&text) else {
                            let e = serde_json::to_string(&ServerMsg::Error("bad message".into())).unwrap();
                            let _ = tx.send(e);
                            continue;
                        };
                        match cmsg {
                            ClientMsg::Hello { nick: n } => {
                                let mut st = state.lock().await;
                                if st.clients.contains_key(&n) {
                                    let e = serde_json::to_string(&ServerMsg::Error(format!("nick {n} taken"))).unwrap();
                                    let _ = tx.send(e);
                                    continue;
                                }
                                nick = Some(n.clone());
                                st.clients.insert(n.clone(), tx.clone());
                                let w = serde_json::to_string(&ServerMsg::Welcome { nick: n.clone() }).unwrap();
                                let _ = tx.send(w);
                                tracing::info!("{n} connected");
                            }
                            ClientMsg::Send(m) => {
                                let st = state.lock().await;
                                st.route(&m);
                                if let Target::Channel(c) = &m.target {
                                    let nicks = st.nicks_of();
                                    let line = serde_json::to_string(&ServerMsg::Nicks { channel: c.clone(), nicks }).unwrap();
                                    for t in st.clients.values() {
                                        let _ = t.send(line.clone());
                                    }
                                }
                            }
                            ClientMsg::Ping => {
                                let p = serde_json::to_string(&ServerMsg::Pong).unwrap();
                                let _ = tx.send(p);
                            }
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    _ => {}
                }
            }
            out = rx.recv() => {
                match out {
                    Some(text) => {
                        if sender.send(WsMessage::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    if let Some(n) = nick {
        state.lock().await.clients.remove(&n);
        tracing::info!("{n} disconnected");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let port = std::env::var("PORT").unwrap_or_else(|_| "7777".into());
    let addr = format!("0.0.0.0:{port}");
    let state = Arc::new(Mutex::new(State::default()));
    let app = Router::new()
        .route("/health", get(health))
        .route("/", get(ws_handler))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
