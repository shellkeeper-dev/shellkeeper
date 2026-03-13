use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};

use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use uuid::Uuid;

use crate::models::SshConnection;

// ──────────────────────────────────────────────────────────────────────────────
// PtySession
// ──────────────────────────────────────────────────────────────────────────────

/// One live terminal session (either local shell or SSH).
pub struct PtySession {
    #[allow(dead_code)]
    pub id:         String,
    /// `Some` for SSH sessions, `None` for local shell.
    pub connection: Option<SshConnection>,
    /// Shared terminal state parsed by vt100.
    pub parser:     Arc<Mutex<vt100::Parser>>,
    /// Write side of the PTY (keyboard input goes here).
    writer:         Box<dyn Write + Send>,
    /// Keep master alive so we can resize.
    master:         Box<dyn portable_pty::MasterPty + Send>,
    _child:         Box<dyn portable_pty::Child + Send + Sync>,
    /// `false` once the read thread exits (process died / connection dropped).
    alive:          Arc<AtomicBool>,
    /// `true` once the first byte of output has been received.
    has_output:     Arc<AtomicBool>,
    /// When the session was created — used to detect instant failures.
    pub started_at: Instant,
    /// The SSH command string shown in the connecting overlay.
    pub ssh_command: String,
    /// Path to the active log file, if logging is enabled.
    #[allow(dead_code)]
    pub log_path: Option<PathBuf>,
    /// Local mountpoints that were auto-mounted for this session.
    pub active_mounts: Vec<String>,
    /// SSHFS mount errors to surface in the UI.
    pub mount_errors: Vec<String>,
}

impl PtySession {
    // ── Constructors ──────────────────────────────────────────────────────────

    pub fn new_ssh(
        conn: SshConnection,
        cols: u16, rows: u16,
        ctx: egui::Context,
        log_dir: Option<PathBuf>,
    ) -> Result<Self> {
        // If a password is saved in the keyring and auth = Password,
        // wrap the ssh command with sshpass so the user isn't prompted.
        let saved_pass = if conn.save_password && matches!(conn.auth, crate::models::AuthMethod::Password) {
            crate::vault::get_password(&conn.id)
        } else {
            None
        };

        let mut cmd = if let Some(ref pass) = saved_pass {
            // sshpass -p <pass> ssh <args>
            let mut c = CommandBuilder::new("sshpass");
            c.args(["-p", pass.as_str()]);
            c.arg("ssh");
            c.args(conn.ssh_args());
            c
        } else {
            let mut c = CommandBuilder::new("ssh");
            c.args(conn.ssh_args());
            c
        };
        cmd.env("TERM", "xterm-256color");
        Self::spawn(cmd, Some(conn), cols, rows, ctx, log_dir)
    }

    pub fn new_local(cols: u16, rows: u16, ctx: egui::Context, shell_override: &str) -> Result<Self> {
        let shell = resolve_shell(shell_override);
        let mut cmd = CommandBuilder::new(&shell);
        cmd.env("TERM", "xterm-256color");
        Self::spawn(cmd, None, cols, rows, ctx, None)
    }

    fn spawn(
        cmd: CommandBuilder,
        conn: Option<SshConnection>,
        cols: u16,
        rows: u16,
        ctx: egui::Context,
        log_dir: Option<PathBuf>,
    ) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width:  0,
            pixel_height: 0,
        })?;

        let child  = pair.slave.spawn_command(cmd)?;
        let master = pair.master;
        let reader = master.try_clone_reader()?;
        let writer = master.take_writer()?;

        let parser     = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 5000)));
        let alive      = Arc::new(AtomicBool::new(true));
        let has_output = Arc::new(AtomicBool::new(false));

        let parser_bg     = parser.clone();
        let alive_bg      = alive.clone();
        let has_output_bg = has_output.clone();

        // Open log file if logging is enabled
        let log_file_path: Option<PathBuf> = log_dir.as_ref().map(|dir| {
            let conn_name = conn.as_ref()
                .map(|c| sanitize_name(&c.name))
                .unwrap_or_else(|| "local".into());
            let ts = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
            dir.join(&conn_name).join(format!("{}.log", ts))
        });
        let log_file: Option<std::fs::File> = log_file_path.as_ref().and_then(|p| {
            fs::create_dir_all(p.parent().unwrap()).ok();
            fs::File::create(p).ok()
        });
        let log_writer: Option<Arc<Mutex<fs::File>>> = log_file.map(|f| Arc::new(Mutex::new(f)));
        let log_writer_bg = log_writer.clone();

        std::thread::spawn(move || {
            let mut buf    = [0u8; 4096];
            let mut reader = reader;
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        has_output_bg.store(true, Ordering::Relaxed);
                        parser_bg.lock().unwrap().process(&buf[..n]);
                        if let Some(ref lw) = log_writer_bg {
                            let _ = lw.lock().unwrap().write_all(&buf[..n]);
                        }
                        ctx.request_repaint();
                    }
                }
            }
            alive_bg.store(false, Ordering::Relaxed);
            ctx.request_repaint();
        });

        // Build a human-readable ssh command string for the overlay
        let ssh_command = match &conn {
            Some(c) => format!("ssh {}", c.ssh_args().join(" ")),
            None    => std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into()),
        };

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            connection: conn,
            parser,
            writer,
            master,
            _child: child,
            alive,
            has_output,
            started_at:    Instant::now(),
            ssh_command,
            log_path:      log_file_path,
            active_mounts: Vec::new(),
            mount_errors:  Vec::new(),
        })
    }

    // ── I/O ──────────────────────────────────────────────────────────────────

    pub fn write_input(&mut self, data: &[u8]) {
        let _ = self.writer.write_all(data);
        let _ = self.writer.flush();
    }

    // ── Resize ────────────────────────────────────────────────────────────────

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width:  0,
            pixel_height: 0,
        });
        self.parser.lock().unwrap().set_size(rows, cols);
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    pub fn is_alive(&self) -> bool { self.alive.load(Ordering::Relaxed) }
    pub fn has_output(&self) -> bool { self.has_output.load(Ordering::Relaxed) }

    pub fn tab_label(&self) -> String {
        match &self.connection {
            Some(c) => c.name.clone(),
            None    => "Local".into(),
        }
    }
}
/// Resolve which shell binary to use for local tabs.
/// Priority: config override (if valid path) → $SHELL → /bin/bash
fn resolve_shell(override_path: &str) -> String {
    if !override_path.trim().is_empty() && std::path::Path::new(override_path.trim()).exists() {
        return override_path.trim().to_string();
    }
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into())
}

fn sanitize_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}
