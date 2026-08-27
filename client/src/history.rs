use crate::app::App;
use protocol::ServerMsg;
use std::collections::BTreeMap;

pub fn load(nick: &str) -> BTreeMap<String, Vec<protocol::Message>> {
    let dir = history_dir();
    let path = dir.join(format!("{nick}.jsonl"));
    let mut map: BTreeMap<String, Vec<protocol::Message>> = BTreeMap::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        return map;
    };
    for line in content.lines() {
        if let Ok(m) = serde_json::from_str(line) {
            let key = buffer_key_for(&m);
            map.entry(key).or_default().push(m);
        }
    }
    map
}

fn buffer_key_for(m: &protocol::Message) -> String {
    match &m.target {
        protocol::Target::Channel(c) => format!("#{c}"),
        protocol::Target::Dm(a, b) => format!("@{}", if m.from == *a { b } else { a }),
    }
}

pub fn save(nick: &str, buffers: &BTreeMap<String, crate::app::Buffer>) -> anyhow::Result<()> {
    let dir = history_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{nick}.jsonl"));
    let mut f = std::fs::File::create(&path)?;
    for b in buffers.values() {
        if b.title.contains('↳') {
            continue; // thread views are derived from their parent buffer
        }
        for m in &b.messages {
            writeln(&mut f, &serde_json::to_string(m)?)?;
        }
    }
    Ok(())
}

fn writeln(f: &mut std::fs::File, s: &str) -> std::io::Result<()> {
    use std::io::Write;
    f.write_all(s.as_bytes())?;
    f.write_all(b"\n")
}

fn history_dir() -> std::path::PathBuf {
    dirs_or_default().join("chattui/history")
}

fn dirs_or_default() -> std::path::PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                let mut p = std::path::PathBuf::from(h);
                p.push(".local/share");
                p
            })
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

impl App {
    pub fn handle_server(&mut self, msg: ServerMsg) {
        match msg {
            ServerMsg::Msg(m) => self.push_msg(m),
            ServerMsg::Nicks { channel, nicks } => self.set_nicks(&channel, &nicks),
            ServerMsg::Welcome { nick } => {
                self.status = format!("connected as {nick}");
                self.push_notif(
                    crate::app::NotifKind::Info,
                    format!("welcome, {nick}"),
                );
            }
            ServerMsg::Error(e) => {
                self.status = format!("error: {e}");
                self.push_notif(crate::app::NotifKind::Warn, e);
            }
            ServerMsg::Pong => {
                // lag is measured in main and delivered as AppEvent::Lag
            }
        }
    }
}
