# ◈ ShellKeeper

> SSH connection manager with embedded terminal — built in Rust + egui.

```
┌──────────────────────────────────────────────────────────────────────┐
│  ◈ ShellKeeper   ▾ Backend  ⚙        │  [prod-web ×][db-01 ×][ + ]  │
│ ──────────────────────────────  ─────────────────────────────────── │
│  ⌕ filter connections…          │  user@prod-server:~$ ▌           │
│ ──────────────────────────────  │                                   │
│  ── FAVOURITES ───────────────  │  full xterm-256color              │
│    Prod DB      ●  root@db      │  terminal here                    │
│    Dev Server      admin@dev    │                                   │
│  ── BACKEND ─────────────────  │  ← real PTY, your shell,          │
│    prod-web     ●  root@10.0…   │    real SSH                       │
│    prod-db         root@10.0…   │                                   │
│    staging-01      deploy@stg   │                                   │
│                                 │                                   │
│  [ + NEW CONNECTION ]           │                                   │
└──────────────────────────────────────────────────────────────────────┘
```

---

## ⚡ Quick Start

```bash
git clone https://gitlab.com/jdzl/sshed.git
cd sshed
make dev
```

`make dev` automatically detects whether Rust and system dependencies are installed,
installs anything missing, and launches the app.

> If Rust was just installed, open a new terminal and run `make dev` again — the PATH
> updates take effect in new shells.

### Individual commands

```bash
make setup   # install Rust + system libraries (run once)
make run     # compile and run in development mode
make release # optimised binary → ./target/release/shellkeeper
make deb     # build a .deb package → target/debian/shellkeeper_*.deb  (Linux only)
make install # install to /usr/local/bin (requires sudo)
```

### Installing the .deb package

```bash
make deb
sudo apt install ./target/debian/shellkeeper_0.1.3_amd64.deb
```

The package automatically pulls in all runtime dependencies:
`openssh-client`, `tmux`, `sshfs`, `sshpass`, `libsecret-1-0`.

After installation, `shellkeeper` is available system-wide and appears in
the application launcher (GNOME Activities, KDE, etc.).

To uninstall:
```bash
sudo apt remove shellkeeper
```

---

## 🍎 macOS Setup

ShellKeeper on macOS is distributed as a **pre-built binary**, not a package manager formula. Two additional dependencies must be installed manually before SSHFS mounts and password auth work correctly.

### Running the binary

Because the binary is not notarised, macOS Gatekeeper will block it on first launch. To allow it:

1. Double-click (or run) `shellkeeper` — macOS will show a security warning.
2. Open **System Settings → Privacy & Security**, scroll down to the blocked app notice, and click **Allow Anyway**.
3. Re-run `shellkeeper`. Confirm the prompt that appears.

Alternatively, remove the quarantine attribute from the terminal:
```bash
xattr -d com.apple.quarantine ./shellkeeper
```

### sshfs (required for directory mounts)

sshfs on macOS requires [macFUSE](https://osxfuse.github.io/) as a kernel extension plus the `sshfs-mac` userspace tool:

```bash
brew install --cask macfuse
brew install gromgit/fuse/sshfs-mac
```

After installing macFUSE you may need to **reboot** and allow the kernel extension in **System Settings → Privacy & Security → Security** before sshfs will work.

### sshpass (required for saved-password auth)

`sshpass` is not in the main Homebrew tap. Install it from a community tap:

```bash
brew install hudochenkov/sshpass/sshpass
```

> If you prefer not to add a third-party tap, you can build from source:
> ```bash
> brew install wget
> wget https://sourceforge.net/projects/sshpass/files/sshpass/1.10/sshpass-1.10.tar.gz
> tar xf sshpass-1.10.tar.gz && cd sshpass-1.10
> ./configure && make && sudo make install
> ```

---

## 📋 Commands

| Command | Description |
|---------|-------------|
| `make dev` | **Setup + run in one command** (auto-detects missing deps) |
| `make setup` | Install Rust + system deps. Run manually if needed. |
| `make run` | Build and launch (dev mode, fast recompile) |
| `make release` | Optimised binary → `./target/release/shellkeeper` |
| `make install` | Install to `/usr/local/bin/shellkeeper` (requires sudo) |
| `make deb` | Build `.deb` package → `target/debian/shellkeeper_*.deb` |
| `make check` | Type-check only, no binary (fast feedback) |
| `make clean` | Remove build artifacts |

---

## 🖥 System requirements

| OS | Status |
|----|--------|
| **Ubuntu / Debian** | ✅ |
| **Fedora / RHEL** | ✅ |
| **Arch Linux** | ✅ |
| **macOS** | ⚠️ Compiles; sshfs requires macFUSE |
| **Windows** | ❌ Not supported |

---

## ✨ Features

### Terminal
- **Embedded xterm-256color terminal** — real PTY, no external window
- **Multiple tabs** — open as many sessions as needed; `+` opens a local shell
- **Auto-resize** — terminal tracks window size in real time
- **Text selection** — click and drag to select; **Ctrl+Shift+C** or right-click → Copy
- **Clipboard paste** — **Ctrl+V** or right-click → Paste (works immediately, even before OS focus)
- **Right-click context menu** — Copy, Paste, Ctrl+C, Ctrl+L, Ctrl+D
- **Configurable local shell** — set a custom shell in Settings (fallback: `$SHELL` → `/bin/bash`)
- **Session logging** — save raw terminal output to disk per session

### Connections
- **Connection manager** — add, edit, delete, favourite connections
- **Namespaces / groups** — organise connections into groups (Backend, Prod, etc.)
- **Search / filter** — type to filter by name, host, user, or group instantly
- **Favourites** — pin frequently used servers to the top
- **Recents** — last used connections float up automatically
- **SSH command import** — paste `ssh -p 2222 -i key.pem user@host` and fields auto-fill
- **SSH command preview** — live multi-line preview of the final SSH command in the edit dialog

### SSH Authentication

| Method | How it works |
|--------|-------------|
| **SSH Agent** | Uses your running `ssh-agent` — no config needed |
| **Key File** | Browse for `.pem` / `id_rsa` / `id_ed25519` with native file picker |
| **Password** | Type it in the terminal, or save it securely in the OS keyring |

#### Password storage (OS keyring)
When **Password** auth is selected in the edit dialog, you can enable
**"Save password in OS keyring"**. ShellKeeper stores the password using the native
OS credential store:

- **Linux** — GNOME Keyring (libsecret). Visible in *Passwords and Keys* (Seahorse).
- **macOS** — macOS Keychain. Visible in *Keychain Access*.

The password is retrieved automatically at connection time and passed via `sshpass`.
It is never written to disk in plain text. Unchecking the option deletes it from the keyring.

### Port Forwarding
- **Local (`-L`)** — expose a remote service on a local port
- **Remote (`-R`)** — expose a local service on the remote server
- **Dynamic (`-D`)** — SOCKS5 proxy through the connection
- Multiple rules per connection; conflicting ports are auto-skipped

### SSHFS Mounts
Mount remote directories directly from ShellKeeper — configured per connection in the edit dialog.

```
SSHFS MOUNTS                          [+ add mount]
┌─────────────────────────────────────────────────┐
│ Remote /var/www  →  Local ~/mnt/prod-www        │
│ ☑ auto-mount on connect          [remove]       │
└─────────────────────────────────────────────────┘
```

- **Auto-mount on connect** — mounts when the SSH tab opens; unmounts when the tab closes
- **Status in tab tooltip** — hover a tab to see mount status (⊞ mounted / ○ not mounted)
- **Error reporting** — if sshfs is missing or the mount fails, the tooltip shows the reason
- **Linux:** `sshfs` is included in the `.deb` dependencies; for other distros: `sudo apt install sshfs` / `sudo dnf install fuse-sshfs` / `sudo pacman -S sshfs`
- **macOS:** see [macOS Setup → sshfs](#-macos-setup) above — macFUSE kernel extension required

### Persistent Sessions (tmux)
Enable **Persistent session** on any connection and ShellKeeper wraps the SSH call in:

```
ssh user@host -t "tmux new-session -A -s shellkeeper-<id>"
```

The `-A` flag attaches to an existing tmux session or creates a new one.
Your remote shell **survives ShellKeeper being closed** — reconnect and pick up where you left off.

> Requires `tmux` on the **remote** server.

### Session Logging
Enable in **Settings → Session Logs** (globally or per connection).
Every session is saved as a raw text file:

```
~/.local/share/shellkeeper/logs/<conn_name>/<yyyy-mm-dd_HH-MM-SS>.log
```

Per-connection override in the edit dialog cycles: `Log: global` → `Log: on` → `Log: off`.

### SSH Key Manager (Settings → SSH Keys)
- Lists all key pairs in `~/.ssh/` with detected type (ed25519 / rsa / ecdsa)
- **Copy public key** to clipboard with one click (shows ✓ feedback for 2 seconds)
- **Generate new key pair** inline — choose type, name, comment → runs `ssh-keygen`

### Visual Themes
Four built-in themes, switchable from **Settings → Appearance**:

| Theme | Style |
|-------|-------|
| ⚡ Cyberpunk | Void black + electric cyan + matrix green |
| ◑ Dark | GitHub-style dark + blue accent |
| ◈ Dracula | Purple-tinted dark + soft cyan |
| ☀ Light | Clean light mode with teal accents |

### App Icon
ShellKeeper ships with a full icon set (16 → 512 px) embedded in the binary and installed
alongside the `.deb`. After installation, the app appears with its icon in:

- Window title bar and taskbar
- Alt+Tab switcher
- GNOME Activities / KDE application launcher

### Settings Panel
Open with **⚙** in the sidebar header. Covers:

- Visual theme selector (card view)
- Font size (10–22px, live preview)
- Scrollback buffer (500–50 000 lines)
- Default SSH username and port
- **Local shell** override (fallback chain: config → `$SHELL` → `/bin/bash`)
- Session logging toggle + log directory
- SSH key manager

---

## ⌨️ Terminal keyboard shortcuts

| Keys | Action |
|------|--------|
| `Ctrl+C` | Interrupt (SIGINT) |
| `Ctrl+D` | EOF / logout |
| `Ctrl+L` | Clear screen |
| `Ctrl+V` | Paste from clipboard |
| `Ctrl+Shift+C` | Copy selected text |
| `Ctrl+R` | Reverse history search |
| `Ctrl+Z` | Suspend process |
| Arrow keys | Move cursor / navigate history |
| `F1`–`F12` | Forwarded to the remote server |

---

## 🗂 Project structure

```
sshed/
├── Makefile
├── Cargo.toml
├── LICENSE                  ← Apache 2.0
├── scripts/
│   └── setup.sh             ← system dependency installer (apt / dnf / pacman / brew)
├── assets/
│   ├── shellkeeper.desktop  ← XDG desktop entry for system launcher
│   └── icons/
│       ├── shellkeeper.svg  ← source icon (cyberpunk hexagon)
│       └── shellkeeper_*.png ← generated sizes: 16, 24, 32, 48, 64, 128, 256, 512 px
└── src/
    ├── main.rs              ← entry point, window config, embedded icon
    ├── models.rs            ← SshConnection, AuthMethod, PortForward, SshMount
    ├── pty.rs               ← PTY session management + session log writer
    ├── vault.rs             ← OS keyring wrapper (save/get/delete passwords)
    ├── colors.rs            ← ANSI 256-colour → egui Color32
    ├── config.rs            ← JSON persistence (~/.config/shellkeeper/config.json)
    ├── ssh_parse.rs         ← SSH command string parser / importer
    ├── theme.rs             ← ThemePalette (Cyberpunk / Dark / Dracula / Light)
    ├── app.rs               ← thin orchestrator — event routing, mount lifecycle
    └── ui/
        ├── mod.rs           ← thread-local palette, apply_theme(), c::* colour accessors
        ├── widgets.rs       ← reusable widgets (ConnectionItem, accordion_header, …)
        ├── sidebar.rs       ← sidebar, search, namespace selector, accordions
        ├── tabs.rs          ← tab bar + tooltip (port forwards, SSHFS mounts, status)
        ├── terminal.rs      ← terminal renderer, keyboard input, text selection
        ├── overlays.rs      ← connecting / dead session / empty state overlays
        ├── dialog.rs        ← add/edit connection form (port forwards + SSHFS mounts)
        └── settings.rs      ← full settings panel
```

---

## 💾 Config file

`~/.config/shellkeeper/config.json` — plain JSON, human-readable, safe to edit by hand.

Stores: connections, theme, font size, scrollback, SSH defaults, local shell, logging settings.

---

## 🔐 License

Apache License 2.0 — see [LICENSE](LICENSE).

Copyright 2026 David Zambrano Lizarazo (jdzl), Juan Fajardo.

---

## 🐛 Troubleshooting

**`cargo: command not found` after setup**
→ Open a new terminal. Rust installs to `~/.cargo/bin` — PATH updates take effect in new shells.

**App opens but terminal is blank**
→ Check `ssh` is installed: `which ssh`. On Ubuntu: `sudo apt install openssh-client`.

**`libxcb` errors on Linux**
→ Run `make setup` again — it will install the missing library.

**SSH connection hangs**
→ Test with `ssh user@host` in a regular terminal first.

**Persistent session not reconnecting**
→ Make sure `tmux` is installed on the **remote** server: `apt install tmux`.

**SSHFS mount fails**
→ Check `sshfs` is installed: `which sshfs`. On Ubuntu: `sudo apt install sshfs`.
→ On macOS: see [macOS Setup → sshfs](#-macos-setup) — macFUSE kernel extension and a reboot may be required.
→ Hover the tab tooltip to see the exact error message.

**Paste not working**
→ Click on the terminal area first to give it focus (border should glow cyan), then paste.

**Password not being saved**
→ On Linux, make sure `libsecret-1-0` is installed and GNOME Keyring is running.
→ Check: `secret-tool lookup service shellkeeper account <conn-id>`
