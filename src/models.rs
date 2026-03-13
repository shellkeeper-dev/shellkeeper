use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ──────────────────────────────────────────────────────────────────────────────
// Auth
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthMethod {
    /// Use the running SSH agent (default)
    Agent,
    /// Will prompt for password inside the terminal
    Password,
    /// Path to a private-key file (.pem / id_rsa / id_ed25519 …)
    Key(String),
}

impl Default for AuthMethod {
    fn default() -> Self {
        AuthMethod::Agent
    }
}

impl AuthMethod {
    pub fn variants() -> &'static [&'static str] {
        &["SSH Agent", "Password", "Key File"]
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Port forwarding
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ForwardKind {
    /// -L local_port:remote_host:remote_port
    Local,
    /// -R remote_port:local_host:local_port
    Remote,
    /// -D local_port  (SOCKS5 proxy)
    Dynamic,
}

impl ForwardKind {
    pub fn label(&self) -> &'static str {
        match self {
            ForwardKind::Local   => "Local  -L",
            ForwardKind::Remote  => "Remote -R",
            ForwardKind::Dynamic => "Dynamic -D (SOCKS5)",
        }
    }
    pub fn variants() -> &'static [&'static str] {
        &["Local  -L", "Remote -R", "Dynamic -D (SOCKS5)"]
    }
    pub fn from_label(s: &str) -> Self {
        match s {
            "Remote -R"           => ForwardKind::Remote,
            "Dynamic -D (SOCKS5)" => ForwardKind::Dynamic,
            _                     => ForwardKind::Local,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForward {
    pub kind:        ForwardKind,
    /// Port on the local machine (always required)
    pub local_port:  u16,
    /// Destination host (not used for Dynamic)
    pub remote_host: String,
    /// Destination port  (not used for Dynamic)
    pub remote_port: u16,
}

impl PortForward {
    pub fn new_local() -> Self {
        Self {
            kind:        ForwardKind::Local,
            local_port:  8080,
            remote_host: String::new(),
            remote_port: 80,
        }
    }

    /// Returns the ssh flag(s) for this rule, e.g. ["-L", "8080:db:5432"]
    pub fn ssh_args(&self) -> Vec<String> {
        match self.kind {
            ForwardKind::Local => vec![
                "-L".into(),
                format!("{}:{}:{}", self.local_port, self.remote_host, self.remote_port),
            ],
            ForwardKind::Remote => vec![
                "-R".into(),
                format!("{}:localhost:{}", self.remote_port, self.local_port),
            ],
            ForwardKind::Dynamic => vec![
                "-D".into(),
                self.local_port.to_string(),
            ],
        }
    }

    /// Human-readable summary shown in the UI
    pub fn summary(&self) -> String {
        match self.kind {
            ForwardKind::Local =>
                format!("localhost:{} → {}:{}", self.local_port, self.remote_host, self.remote_port),
            ForwardKind::Remote =>
                format!("{}:{} ← localhost:{}", self.remote_host, self.remote_port, self.local_port),
            ForwardKind::Dynamic =>
                format!("SOCKS5 proxy on localhost:{}", self.local_port),
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// SSHFS mounts
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshMount {
    /// Remote path on the server (e.g. /var/www)
    pub remote_path: String,
    /// Local mountpoint (e.g. ~/mnt/prod-www)
    pub local_path:  String,
    /// Mount automatically when the connection opens
    #[serde(default)]
    pub auto_mount:  bool,
}

impl SshMount {
    pub fn new() -> Self {
        Self {
            remote_path: String::new(),
            local_path:  String::new(),
            auto_mount:  true,
        }
    }

    /// Human-readable summary for tooltips
    pub fn summary(&self) -> String {
        let local = shellexpand::tilde(&self.local_path);
        format!("{} → {}", self.remote_path, local)
    }

    /// The sshfs command args for this mount
    pub fn sshfs_args(&self, conn: &SshConnection) -> Vec<String> {
        let local = shellexpand::tilde(&self.local_path).into_owned();
        let mut args = vec![
            format!("{}@{}:{}", conn.username, conn.host, self.remote_path),
            local,
        ];
        if conn.port != 22 {
            args.extend(["-p".into(), conn.port.to_string()]);
        }
        if let AuthMethod::Key(path) = &conn.auth {
            args.extend(["-o".into(), format!("IdentityFile={}", path)]);
        }
        args.extend(["-o".into(), "StrictHostKeyChecking=accept-new".into()]);
        args
    }

    /// fusermount3 -u <local_path>
    #[allow(dead_code)]
    pub fn unmount_args(&self) -> Vec<String> {
        let local = shellexpand::tilde(&self.local_path).into_owned();
        vec!["-u".into(), local]
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Connection
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConnection {
    pub id:            String,
    pub name:          String,
    pub host:          String,
    pub port:          u16,
    pub username:      String,
    pub auth:          AuthMethod,
    pub favorite:      bool,
    pub last_used:     Option<DateTime<Utc>>,
    pub description:   String,
    #[serde(default)]
    pub port_forwards: Vec<PortForward>,
    /// Whether the user opted to save the password in the OS keyring.
    #[serde(default)]
    pub save_password: bool,
    /// SSHFS mounts for this connection.
    #[serde(default)]
    pub mounts:        Vec<SshMount>,
    /// Namespace / group label. Empty string = ungrouped.
    #[serde(default)]
    pub group:         String,
    /// Override global log_sessions. None = follow global setting.
    #[serde(default)]
    pub log_session:   Option<bool>,
    /// When true, SSH wraps in `tmux new-session -A -s <tmux_session>`
    /// so the remote shell survives shellkeeper being closed.
    #[serde(default)]
    pub persistent:    bool,
    /// Tmux session name (auto-generated from id if empty).
    #[serde(default)]
    pub tmux_session:  String,
}

impl Default for SshConnection {
    fn default() -> Self {
        Self {
            id:            Uuid::new_v4().to_string(),
            name:          String::new(),
            host:          String::new(),
            port:          22,
            username:      std::env::var("USER").unwrap_or_else(|_| "root".into()),
            auth:          AuthMethod::default(),
            favorite:      false,
            last_used:     None,
            description:   String::new(),
            port_forwards: Vec::new(),
            save_password: false,
            mounts:        Vec::new(),
            group:         String::new(),
            persistent:    false,
            tmux_session:  String::new(),
            log_session:   None,
        }
    }
}

impl SshConnection {
    /// Tmux session name — uses explicit name or falls back to id prefix.
    pub fn tmux_name(&self) -> String {
        if self.tmux_session.trim().is_empty() {
            format!("shellkeeper-{}", &self.id[..8])
        } else {
            self.tmux_session.clone()
        }
    }

    /// Build the argument list for the `ssh` binary.
    pub fn ssh_args(&self) -> Vec<String> {
        let mut args: Vec<String> = Vec::new();

        if self.port != 22 {
            args.push("-p".into());
            args.push(self.port.to_string());
        }

        if let AuthMethod::Key(path) = &self.auth {
            args.push("-i".into());
            args.push(path.clone());
        }

        // Port forwarding rules
        for fwd in &self.port_forwards {
            args.extend(fwd.ssh_args());
        }

        args.push("-o".into());
        args.push("StrictHostKeyChecking=accept-new".into());

        if self.persistent {
            // -t forces PTY allocation; tmux -A attaches if session exists, creates if not
            args.push("-t".into());
            args.push(format!("{}@{}", self.username, self.host));
            args.push(format!(
                "tmux new-session -A -s {}",
                self.tmux_name()
            ));
        } else {
            args.push(format!("{}@{}", self.username, self.host));
        }

        args
    }

    /// Local ports this connection will bind. Used to detect conflicts.
    pub fn local_ports(&self) -> Vec<u16> {
        self.port_forwards.iter().map(|f| f.local_port).collect()
    }

    pub fn display_host(&self) -> String {
        if self.port != 22 {
            format!("{}:{}", self.host, self.port)
        } else {
            self.host.clone()
        }
    }

    /// One-liner label shown below the friendly name in the sidebar.
    pub fn subtitle(&self) -> String {
        format!("{}@{}", self.username, self.display_host())
    }
}
