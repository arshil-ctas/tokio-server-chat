# Deploy & Publish Guide

## 1. Backend on Render (free)

The relay server uses **axum**: `GET /health` returns JSON, `GET /` upgrades to WebSocket.

### Deploy
1. Push repo to GitHub (`arshil-ctas/tokio-server-chat`)
2. On [render.com](https://render.com) → **New → Blueprint** → select repo
3. It reads `render.yaml`:
   - Docker build (`Dockerfile` with libssl for TLS)
   - **free** tier
   - health check: `GET /health` → `{"ok":true,"clients":0}`
4. Render assigns a dynamic `$PORT` — server binds to `0.0.0.0:$PORT` automatically
5. External URL: `https://tokio-server-chat.onrender.com`

No extra ports, no TCP — all traffic flows through Render's HTTPS proxy, which forwards WebSocket upgrades to your app.

### `/health` endpoint
```bash
curl https://tokio-server-chat.onrender.com/health
# {"ok":true,"clients":0}
```

## 2. UptimeRobot keep-alive (free)

1. Sign up at [uptimerobot.com](https://uptimerobot.com)
2. **Add Monitor** → HTTP(s) → `https://tokio-server-chat.onrender.com/health`
3. Interval: **3 min**
4. Free tier stays warm, never sleeps

## 3. Client — one line, straight into TUI

After deploy, edit `try.sh` if needed (it already has your repo + host hardcoded).
Users join with:

```bash
curl -fsSL https://raw.githubusercontent.com/arshil-ctas/tokio-server-chat/main/try.sh | bash -s -- alice
```

- Auto-detects platform (Linux x86_64/arm64, macOS)
- Downloads prebuilt binary from GitHub Releases (falls back to source build via cargo)
- Connects to `wss://tokio-server-chat.onrender.com` (TLS by default for remote hosts)
- Nick defaults to OS username if not passed

**Server stores nothing** — messages are relayed in memory only. Each user's
history is saved locally (`~/.local/share/chattui/history/<nick>.jsonl`).

## 4. Publishing releases

Tag a release → GitHub Actions builds binaries → attaches to GitHub release:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

`.github/workflows/release.yml` builds for:
- x86_64-linux, aarch64-linux, x86_64-macOS

## 5. Checklist

- [ ] Push repo to GitHub
- [ ] Render Blueprint deploy green → `/health` returns 200
- [ ] UptimeRobot monitor every 3 min
- [ ] Tag `v0.1.0` → release binaries built
- [ ] Test: `curl -fsSL <raw-url>/try.sh | bash -s -- alice` → TUI opens, chat works
