# tokio-server-chat (chattui)

A weechat-style terminal chat client + lightweight relay server, written in Rust.

## Features
- WebSocket transport (works through Render's HTTPS proxy)
- Nickname-based identity
- Channels (`/join #dev`) — everyone sees messages
- DMs (`/query nick`, `/msg nick text`)
- Threads (`/reply 3 text`, `/thread 3`)
- Numbered messages with @mention highlighting
- WhatsApp-style top bar (who you are + active chat)
- Sidebar with command cheatsheet (no memorizing needed)
- Per-nick color palette for usernames
- Members panel for channels
- Chat history persisted locally (`~/.local/share/chattui/history/<nick>.jsonl`)

## Quick start (local)
```bash
# terminal 1: server
cargo run -p chatserver     # ws://127.0.0.1:7777

# terminal 2: client
cargo run -p chattui -- alice ws://127.0.0.1:7777
```

## Remote (Render)
```bash
curl -fsSL https://raw.githubusercontent.com/arshil-ctas/tokio-server-chat/main/try.sh | bash -s -- alice
```

## Commands
| Command | Action |
|---|---|
| `<text>` + Enter | Send to current buffer |
| `/join #chan` | Join/create channel |
| `/query nick` | Open DM buffer |
| `/msg nick text` | Send DM without switching |
| `/reply N text` | Threaded reply to message #N |
| `/thread N` | Open dedicated thread view |
| Tab / Shift+Tab | Switch buffers |
| Ctrl+Q / Ctrl+C | Quit |
