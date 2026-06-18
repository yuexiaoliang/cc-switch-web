# Deployment files for cc-switch.yue.center

This directory contains the three files that wire cc-switch-web into
the existing infrastructure on this machine. They are **not** part of
the source build — they are deployment-time templates.

## Layout

| File | Where it goes | Purpose |
| --- | --- | --- |
| `cc-switch-web.service` | `~/.config/systemd/user/cc-switch-web.service` | User-systemd unit that keeps the binary alive |
| `nginx.conf.snippet` | `~/nginx-config/conf.d/cc-switch.conf` | nginx vhost (HTTP→HTTPS, SSE + asset caching) |
| `frpc.snippet.toml` | append to `~/frpc.toml` | FRP proxies that publish the domain on the public edge |

## Install steps

1. **Build + install the binary**
   ```bash
   cd /home/yuexiaoliang/projects/cc-switch-web
   cargo build --release -p cc-switch-web-server
   sudo install -m 0755 target/release/cc-switch-web /usr/local/bin/cc-switch-web
   ```

2. **Install the systemd service**
   ```bash
   cp .ccsm/deploy/cc-switch-web.service ~/.config/systemd/user/
   systemctl --user daemon-reload
   systemctl --user enable --now cc-switch-web.service
   ```
   (If this is the first user service, run `loginctl enable-linger <user>`
   from a privileged shell so it survives logout.)

3. **Wire nginx**
   ```bash
   cp .ccsm/deploy/nginx.conf.snippet ~/nginx-config/conf.d/cc-switch.conf
   sudo nginx -t && sudo systemctl reload nginx
   ```

4. **Expose via FRP**
   Append the two `[[proxies]]` blocks from `frpc.snippet.toml` to
   `~/frpc.toml`, then restart the user FRP service:
   ```bash
   systemctl --user restart frpc.service
   ```

5. **Verify**
   ```bash
   curl -sS https://cc-switch.yue.center/api/health
   ```
   Expected: `{"status":"ok","version":"3.16.2",...}`

## Day-to-day management

A convenience wrapper is installed at `~/.local/bin/cc-switch-web-ctl`:

```bash
cc-switch-web-ctl status    # service + listener + public endpoint
cc-switch-web-ctl logs      # journalctl -f for the service
cc-switch-web-ctl restart   # graceful restart
cc-switch-web-ctl update    # pull + rebuild + reinstall + restart
```

## Why these specific paths

- `CC_SWITCH_MINI_DATA_DIR` is set to `~/.local/share/cc-switch-web/`
  (only used for logs and temp files; the real DB lives at
  `~/.cc-switch/cc-switch.db` so it stays interchangeable with the
  upstream Tauri app).
- The FRP tunnel terminates on the public IP at `39.96.7.171`; the
  matching public DNS A record for `cc-switch.yue.center` already
  points to that IP.
- The Let's Encrypt cert under `live/yue.center/` covers the
  `*.yue.center` wildcard.
