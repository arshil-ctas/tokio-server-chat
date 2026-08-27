use crate::theme::Theme;
use protocol::{Message, ServerMsg, Target};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone)]
pub enum AppEvent {
    Server(ServerMsg),
    /// local tick used for clock / sparkline refresh / ping RTT
    Tick,
    /// measured round-trip from Ping → Pong (milliseconds)
    Lag(u64),
}

#[derive(Debug, Clone)]
pub struct Buffer {
    pub title: String,
    pub messages: Vec<Message>,
    pub unread: usize,
    /// known members of a channel (from server Nicks broadcast)
    pub members: Vec<String>,
    /// optional channel topic (set locally with /topic)
    pub topic: String,
    /// how many messages from the bottom are scrolled away
    pub scroll: usize,
}

impl Buffer {
    fn new(title: String) -> Self {
        Self {
            title,
            messages: Vec::new(),
            unread: 0,
            members: Vec::new(),
            topic: String::new(),
            scroll: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Notif {
    pub kind: NotifKind,
    pub text: String,
    pub ts: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifKind {
    Mention,
    Join,
    Info,
    Warn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Chat,
    Sidebar,
}

pub struct App {
    pub me: String,
    pub server_label: String,
    pub buffers: BTreeMap<String, Buffer>,
    pub order: Vec<String>, // buffer keys in creation order
    /// thread buffers: key -> (parent target, root message id)
    pub threads: BTreeMap<String, (Target, String)>,
    /// root message id -> thread buffer key
    pub thread_by_root: BTreeMap<String, String>,
    pub active: usize,
    pub input: String,
    pub cursor: usize,
    pub status: String,
    pub theme: Theme,
    pub lag_ms: Option<u64>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub notifications: VecDeque<Notif>,
    pub focus: Focus,
    /// rolling message-rate samples for the footer sparkline
    pub activity: VecDeque<u64>,
    pub msgs_this_tick: u64,
    pub total_messages: u64,
    pub show_help: bool,
}

impl App {
    pub fn new(
        me: String,
        server_label: String,
        saved: BTreeMap<String, Vec<Message>>,
        theme: Theme,
    ) -> Self {
        let mut app = App {
            me,
            server_label,
            buffers: BTreeMap::new(),
            order: Vec::new(),
            threads: BTreeMap::new(),
            thread_by_root: BTreeMap::new(),
            active: 0,
            input: String::new(),
            cursor: 0,
            status: "connecting...".into(),
            theme,
            lag_ms: None,
            connected_at: chrono::Utc::now(),
            notifications: VecDeque::new(),
            focus: Focus::Chat,
            activity: VecDeque::from(vec![0; 40]),
            msgs_this_tick: 0,
            total_messages: 0,
            show_help: false,
        };
        for (k, msgs) in saved {
            if k.contains('↳') {
                continue; // thread views are derived, not persisted
            }
            app.order.push(k.clone());
            let mut b = Buffer::new(k);
            app.total_messages += msgs.len() as u64;
            b.messages = msgs;
            app.buffers.insert(b.title.clone(), b);
        }
        if !app.buffers.contains_key("server") {
            app.ensure_buffer("server");
        }
        app.ensure_buffer("#general");
        if let Some(b) = app.buffers.get_mut("#general") {
            if b.topic.is_empty() {
                b.topic = "Welcome to #general — say hi, share code with `backticks`".into();
            }
        }
        if let Some(i) = app.order.iter().position(|k| k == "#general") {
            app.active = i;
        }
        app.push_notif(NotifKind::Info, format!("connected to {}", app.server_label));
        app
    }

    pub fn push_notif(&mut self, kind: NotifKind, text: impl Into<String>) {
        self.notifications.push_front(Notif {
            kind,
            text: text.into(),
            ts: chrono::Utc::now(),
        });
        while self.notifications.len() > 20 {
            self.notifications.pop_back();
        }
    }

    pub fn ensure_buffer(&mut self, title: &str) -> &mut Buffer {
        if !self.buffers.contains_key(title) {
            self.order.push(title.to_string());
            self.buffers
                .insert(title.to_string(), Buffer::new(title.to_string()));
        }
        self.buffers.get_mut(title).unwrap()
    }

    pub fn buffer_key_for(&self, target: &Target) -> String {
        match target {
            Target::Channel(c) => format!("#{c}"),
            Target::Dm(a, b) => {
                let peer = if a == &self.me { b.clone() } else { a.clone() };
                format!("@{peer}")
            }
        }
    }

    pub fn push_msg(&mut self, m: Message) {
        let key = self.buffer_key_for(&m.target);
        let is_active = self.active_key() == key;
        let mentioned = m.from != self.me
            && m.body.split_whitespace().any(|w| {
                w.trim_matches(|c: char| !c.is_alphanumeric()) == self.me
            });

        {
            let b = self.ensure_buffer(&key);
            b.messages.push(m.clone());
            if !is_active {
                b.unread += 1;
            } else if b.scroll > 0 {
                // new message while scrolled up — keep relative position
                b.scroll += 1;
            }
        }
        self.msgs_this_tick += 1;
        self.total_messages += 1;

        if mentioned {
            self.push_notif(
                NotifKind::Mention,
                format!("{} mentioned you in {key}", m.from),
            );
        }

        // mirror threaded replies into their thread view (auto-create if needed)
        if let Some(root_id) = m.reply_to.clone() {
            let thread_key = match self.thread_by_root.get(&root_id) {
                Some(k) => k.clone(),
                None => {
                    let k = format!("{key} ↳{}", &root_id[..4.min(root_id.len())]);
                    let target = m.target.clone();
                    self.threads.insert(k.clone(), (target, root_id.clone()));
                    self.thread_by_root.insert(root_id, k.clone());
                    k
                }
            };
            let t_active = self.active_key() == thread_key;
            let tb = self.ensure_buffer(&thread_key);
            tb.messages.push(m);
            if !t_active {
                tb.unread += 1;
            }
        }
        self.status = format!("connected as {}", self.me);
    }

    pub fn active_key(&self) -> String {
        self.order
            .get(self.active)
            .cloned()
            .unwrap_or_else(|| "server".into())
    }

    pub fn active_buffer(&self) -> Option<&Buffer> {
        self.buffers.get(&self.active_key())
    }

    pub fn active_buffer_mut(&mut self) -> Option<&mut Buffer> {
        let key = self.active_key();
        self.buffers.get_mut(&key)
    }

    /// message #n (1-based, matches the number shown in the chat view)
    fn msg_at(&self, n: usize) -> Option<Message> {
        if n == 0 {
            return None;
        }
        self.buffers
            .get(&self.active_key())
            .and_then(|b| b.messages.get(n - 1).cloned())
    }

    fn buffer_target(&self, key: &str) -> Option<Target> {
        if let Some((target, _)) = self.threads.get(key) {
            return Some(target.clone());
        }
        match key.strip_prefix('#') {
            Some(c) => Some(Target::Channel(c.to_string())),
            None => {
                let peer = key.strip_prefix('@')?;
                let mut pair = [self.me.clone(), peer.to_string()];
                pair.sort();
                Some(Target::Dm(pair[0].clone(), pair[1].clone()))
            }
        }
    }

    fn current_target(&self) -> Option<Target> {
        let key = self.active_key();
        self.buffer_target(&key)
    }

    pub fn set_nicks(&mut self, channel: &str, nicks: &[String]) {
        let key = format!("#{channel}");
        let prev = self
            .buffers
            .get(&key)
            .map(|b| b.members.clone())
            .unwrap_or_default();
        let mut members: Vec<String> = nicks.to_vec();
        members.sort();
        // detect joins for the notifications feed
        for n in &members {
            if !prev.contains(n) && n != &self.me && !prev.is_empty() {
                self.push_notif(NotifKind::Join, format!("{n} joined #{channel}"));
            }
        }
        self.ensure_buffer(&key).members = members;
    }

    fn clear_active_unread(&mut self) {
        let key = self.active_key();
        if let Some(b) = self.buffers.get_mut(&key) {
            b.unread = 0;
        }
    }

    pub fn next_buffer(&mut self) {
        if !self.order.is_empty() {
            self.active = (self.active + 1) % self.order.len();
            self.clear_active_unread();
        }
    }

    pub fn prev_buffer(&mut self) {
        if !self.order.is_empty() {
            self.active = (self.active + self.order.len() - 1) % self.order.len();
            self.clear_active_unread();
        }
    }

    pub fn select_buffer_index(&mut self, i: usize) {
        if i < self.order.len() {
            self.active = i;
            self.clear_active_unread();
        }
    }

    pub fn scroll_up(&mut self, n: usize) {
        if let Some(b) = self.active_buffer_mut() {
            let max = b.messages.len().saturating_sub(1);
            b.scroll = (b.scroll + n).min(max);
        }
    }

    pub fn scroll_down(&mut self, n: usize) {
        if let Some(b) = self.active_buffer_mut() {
            b.scroll = b.scroll.saturating_sub(n);
        }
    }

    pub fn scroll_bottom(&mut self) {
        if let Some(b) = self.active_buffer_mut() {
            b.scroll = 0;
        }
    }

    pub fn on_tick(&mut self) {
        self.activity.push_back(self.msgs_this_tick);
        while self.activity.len() > 40 {
            self.activity.pop_front();
        }
        self.msgs_this_tick = 0;
    }

    pub fn insert_char(&mut self, c: char) {
        let i = self.cursor.min(self.input.len());
        self.input.insert(i, c);
        self.cursor = i + c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut i = self.cursor - 1;
        while i > 0 && !self.input.is_char_boundary(i) {
            i -= 1;
        }
        self.input.remove(i);
        self.cursor = i;
    }

    pub fn delete_forward(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        self.input.remove(self.cursor);
    }

    pub fn cursor_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut i = self.cursor - 1;
        while i > 0 && !self.input.is_char_boundary(i) {
            i -= 1;
        }
        self.cursor = i;
    }

    pub fn cursor_right(&mut self) {
        if self.cursor >= self.input.len() {
            return;
        }
        let mut i = self.cursor + 1;
        while i < self.input.len() && !self.input.is_char_boundary(i) {
            i += 1;
        }
        self.cursor = i;
    }

    /// channel keys in sidebar order
    pub fn channel_keys(&self) -> Vec<(usize, String)> {
        self.order
            .iter()
            .enumerate()
            .filter(|(_, k)| k.starts_with('#') && !k.contains('↳'))
            .map(|(i, k)| (i, k.clone()))
            .collect()
    }

    pub fn query_keys(&self) -> Vec<(usize, String)> {
        self.order
            .iter()
            .enumerate()
            .filter(|(_, k)| k.starts_with('@'))
            .map(|(i, k)| (i, k.clone()))
            .collect()
    }

    pub fn thread_keys(&self) -> Vec<(usize, String)> {
        self.order
            .iter()
            .enumerate()
            .filter(|(_, k)| k.contains('↳'))
            .map(|(i, k)| (i, k.clone()))
            .collect()
    }

    /// Parse user input. Supported commands:
    ///   /join #chan          join/create channel
    ///   /query nick          open DM buffer
    ///   /msg nick text       send DM
    ///   /reply text          threaded reply to last visible msg in buffer
    ///   /topic text          set local channel topic
    ///   /nick new            (local rename; reconnect needed)
    ///   plain text           message to current buffer
    pub fn submit(
        &mut self,
        out: &tokio::sync::mpsc::UnboundedSender<Message>,
    ) -> anyhow::Result<()> {
        let raw = std::mem::take(&mut self.input);
        self.cursor = 0;
        let raw = raw.trim().to_string();
        if raw.is_empty() {
            return Ok(());
        }
        if let Some(rest) = raw.strip_prefix('/') {
            return self.run_command(rest, out);
        }

        // plain text: thread buffer -> threaded reply; else normal message
        let key = self.active_key();
        if let Some((target, root_id)) = self.threads.get(&key).cloned() {
            let m = self.mk_msg(target, &raw, Some(root_id));
            out.send(m)?;
            return Ok(());
        }
        let target = match key.strip_prefix('#') {
            Some(c) => Target::Channel(c.to_string()),
            None => match key.strip_prefix('@') {
                Some(peer) => {
                    let mut pair = [self.me.clone(), peer.to_string()];
                    pair.sort();
                    Target::Dm(pair[0].clone(), pair[1].clone())
                }
                None => {
                    // server buffer: fall back to #general so typing always works
                    self.ensure_buffer("#general");
                    Target::Channel("general".into())
                }
            },
        };
        let m = self.mk_msg(target, &raw, None);
        out.send(m)?;
        Ok(())
    }

    fn run_command(
        &mut self,
        rest: &str,
        out: &tokio::sync::mpsc::UnboundedSender<Message>,
    ) -> anyhow::Result<()> {
        let key = self.active_key();
        let mut parts = rest.splitn(2, ' ');
        let cmd = parts.next().unwrap_or("");
        let args = parts.next().unwrap_or("").trim();
        match cmd {
            "join" | "j" => {
                let c = args.trim_start_matches('#');
                let c = if c.is_empty() { "general" } else { c };
                let new_key = format!("#{c}");
                self.ensure_buffer(&new_key);
                self.active = self
                    .order
                    .iter()
                    .position(|k| k == &new_key)
                    .unwrap_or(self.active);
                self.clear_active_unread();
                self.push_notif(NotifKind::Info, format!("joined {new_key}"));
                Ok(())
            }
            "query" | "q" => {
                let new_key = format!("@{args}");
                self.ensure_buffer(&new_key);
                self.active = self
                    .order
                    .iter()
                    .position(|k| k == &new_key)
                    .unwrap_or(self.active);
                self.clear_active_unread();
                Ok(())
            }
            "msg" => {
                let mut it = args.splitn(2, ' ');
                let peer = it.next().unwrap_or_default();
                let body = it.next().unwrap_or_default();
                if peer.is_empty() || body.is_empty() {
                    self.status = "usage: /msg <nick> <text>".into();
                    return Ok(());
                }
                let t = {
                    let mut pair = [self.me.clone(), peer.to_string()];
                    pair.sort();
                    Target::Dm(pair[0].clone(), pair[1].clone())
                };
                out.send(self.mk_msg(t, body, None))?;
                Ok(())
            }
            "topic" => {
                if args.is_empty() {
                    self.status = "usage: /topic <text>".into();
                    return Ok(());
                }
                let key = self.active_key();
                if let Some(b) = self.buffers.get_mut(&key) {
                    b.topic = args.to_string();
                    self.status = format!("topic set for {key}");
                    self.push_notif(NotifKind::Info, format!("topic updated in {key}"));
                }
                Ok(())
            }
            "part" | "close" => {
                let key = self.active_key();
                if key == "server" || key == "#general" {
                    self.status = "cannot close server/#general".into();
                    return Ok(());
                }
                if let Some(pos) = self.order.iter().position(|k| k == &key) {
                    self.order.remove(pos);
                    self.buffers.remove(&key);
                    self.threads.remove(&key);
                    if self.active >= self.order.len() {
                        self.active = self.order.len().saturating_sub(1);
                    }
                    self.clear_active_unread();
                    self.status = format!("closed {key}");
                }
                Ok(())
            }
            "reply" | "r" => {
                // /reply <text>          reply to last message in buffer
                // /reply <num> <text>    reply to message #<num> (see numbers on the left)
                let mut it = args.splitn(2, ' ');
                let first = it.next().unwrap_or_default().trim();
                let second = it.next().unwrap_or_default().trim();
                let (num, body) = match first.parse::<usize>() {
                    Ok(n) if !second.is_empty() => (Some(n), second),
                    _ => (None, args),
                };
                if body.is_empty() {
                    self.status = "usage: /reply [msg#] <text>".into();
                    return Ok(());
                }
                let target = self.current_target();
                let Some(target) = target else { return Ok(()) };
                let reply_to = match num {
                    Some(n) => self.msg_at(n).map(|m| m.id.clone()),
                    None => self
                        .buffers
                        .get(&key)
                        .and_then(|b| b.messages.iter().rev().find(|m| m.from != "*"))
                        .map(|m| m.id.clone()),
                };
                let Some(reply_to) = reply_to else {
                    self.status = format!("no message #{first} here");
                    return Ok(());
                };
                out.send(self.mk_msg(target, body, Some(reply_to)))?;
                Ok(())
            }
            "thread" | "t" => {
                // /thread <num>: open a dedicated view for the thread rooted at msg #num
                let Some(m) = args
                    .parse::<usize>()
                    .ok()
                    .and_then(|n| self.msg_at(n))
                else {
                    self.status = "usage: /thread <msg#>".into();
                    return Ok(());
                };
                let root_id = m.id.clone();
                let parent_key = key.clone();
                let thread_key = format!("{parent_key} ↳{}", &root_id[..4]);
                let existing = self.thread_by_root.get(&root_id).cloned();
                let thread_key = existing.unwrap_or(thread_key);
                if !self.buffers.contains_key(&thread_key) {
                    let target = self.buffer_target(&parent_key);
                    if let Some(target) = target {
                        self.threads
                            .insert(thread_key.clone(), (target, root_id.clone()));
                        self.thread_by_root
                            .insert(root_id.clone(), thread_key.clone());
                        // seed view with root + any replies already in the buffer
                        let replies: Vec<Message> = self
                            .buffers
                            .get(&parent_key)
                            .map(|b| {
                                b.messages
                                    .iter()
                                    .filter(|x| x.reply_to.as_deref() == Some(root_id.as_str()))
                                    .cloned()
                                    .collect()
                            })
                            .unwrap_or_default();
                        let root = self
                            .buffers
                            .get(&parent_key)
                            .and_then(|b| b.messages.iter().find(|x| x.id == root_id).cloned());
                        let tb = self.ensure_buffer(&thread_key);
                        if let Some(r) = root {
                            tb.messages.push(r);
                        }
                        for r in replies {
                            tb.messages.push(r);
                        }
                    }
                }
                self.active = self
                    .order
                    .iter()
                    .position(|k| k == &thread_key)
                    .unwrap_or(self.active);
                self.clear_active_unread();
                Ok(())
            }
            "theme" => {
                if args.is_empty() {
                    self.status = format!(
                        "themes: {} (current: {})",
                        Theme::names().join(", "),
                        self.theme.name
                    );
                    return Ok(());
                }
                self.theme = Theme::by_name(args);
                self.status = format!("theme -> {}", self.theme.name);
                if let Err(e) = crate::theme::save_theme(&self.theme.name) {
                    self.status = format!("theme -> {} (not saved: {e})", self.theme.name);
                }
                Ok(())
            }
            "help" | "?" => {
                self.show_help = !self.show_help;
                Ok(())
            }
            other => {
                self.status = format!("unknown command /{other}");
                Ok(())
            }
        }
    }

    fn mk_msg(&self, target: Target, body: &str, reply_to: Option<String>) -> Message {
        Message {
            id: uuid::Uuid::new_v4().to_string(),
            from: self.me.clone(),
            target,
            body: body.to_string(),
            ts: chrono::Utc::now(),
            reply_to,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new(
            "alice".into(),
            "localhost".into(),
            Default::default(),
            Theme::midnight(),
        )
    }

    fn type_and_enter(a: &mut App, line: &str) -> Vec<Message> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        a.input = line.into();
        a.cursor = a.input.len();
        a.submit(&tx).unwrap();
        let mut out = Vec::new();
        while let Ok(m) = rx.try_recv() {
            out.push(m);
        }
        out
    }

    #[tokio::test]
    async fn commands_work_from_server_buffer() {
        let mut a = app();
        assert_eq!(a.active_key(), "#general"); // auto-joined at startup
        type_and_enter(&mut a, "/join #dev");
        assert_eq!(a.active_key(), "#dev");
        let sent = type_and_enter(&mut a, "hello world");
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].target, Target::Channel("dev".into()));
        // typing on server buffer falls back to #general
        a.ensure_buffer("server");
        a.active = a.order.iter().position(|k| k == "server").unwrap();
        let sent = type_and_enter(&mut a, "fallback msg");
        assert_eq!(sent[0].target, Target::Channel("general".into()));
    }

    #[tokio::test]
    async fn dm_commands() {
        let mut a = app();
        let sent = type_and_enter(&mut a, "/msg bob hi bob");
        assert_eq!(sent[0].target, Target::Dm("alice".into(), "bob".into()));
        type_and_enter(&mut a, "/query bob");
        assert_eq!(a.active_key(), "@bob");
        let sent = type_and_enter(&mut a, "direct hello");
        assert_eq!(sent[0].target, Target::Dm("alice".into(), "bob".into()));
    }

    #[tokio::test]
    async fn unread_clears_on_switch() {
        let mut a = app();
        a.ensure_buffer("#dev");
        if let Some(b) = a.buffers.get_mut("#dev") {
            b.unread = 3;
        }
        a.active = a.order.iter().position(|k| k == "#dev").unwrap();
        a.next_buffer();
        a.prev_buffer();
        assert_eq!(a.buffers.get("#dev").unwrap().unread, 0);
    }
}
