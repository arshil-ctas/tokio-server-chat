use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub from: String,
    pub target: Target,
    pub body: String,
    pub ts: DateTime<Utc>,
    /// Some(parent_id) if this is a threaded reply.
    pub reply_to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Target {
    Channel(String),
    Dm(String, String), // (nick_a, nick_b) canonical sorted pair
}

impl Target {
    pub fn key(&self) -> String {
        match self {
            Target::Channel(c) => format!("#{c}"),
            Target::Dm(a, b) => format!("@{a}:{b}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    Hello { nick: String },
    Send(Message),
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    Welcome { nick: String },
    Msg(Message),
    Nicks { channel: String, nicks: Vec<String> },
    Error(String),
    Pong,
}
