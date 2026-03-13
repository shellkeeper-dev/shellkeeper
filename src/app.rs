//! Application orchestrator.
//!
//! `SshedApp` is a thin shell that:
//!   1. Owns all app-level state.
//!   2. Delegates every UI concern to the `ui::*` modules.
//!   3. Processes the events those modules return.


use crate::{
    config::AppConfig,
    models::SshConnection,
    pty::PtySession,
    theme::ThemePalette,
    ui::{
        self,
        dialog::{ConnForm, DialogEvent, DialogState},
        overlays::{self, DeadAction},
        settings::SettingsEvent,
        sidebar::{self, SidebarState},
        tabs,
        terminal::{self, TerminalState},
    },
};

// ── View ──────────────────────────────────────────────────────────────────────

#[derive(Default, PartialEq)]
enum AppView {
    #[default]
    Terminal,
    Settings,
}

// ── App ───────────────────────────────────────────────────────────────────────

pub struct SshedApp {
    config:    AppConfig,
    palette:   ThemePalette,
    sessions:  Vec<PtySession>,
    active_tab: usize,
    view:      AppView,

    // UI sub-state (each owned by its respective ui module)
    sidebar:  SidebarState,
    terminal: TerminalState,
    dialog:   DialogState,

    // egui context — cloned into PTY threads for `request_repaint`.
    ctx: egui::Context,

    /// App icon texture — loaded once, reused everywhere.
    icon_texture: egui::TextureHandle,
}

impl SshedApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config  = AppConfig::load();
        let palette = ThemePalette::from_name(&config.theme);
        ui::set_palette(palette.clone());
        ui::apply_theme(&cc.egui_ctx);

        // Load app icon as GPU texture (used in sidebar header + empty state)
        let icon_texture = {
            let bytes = include_bytes!("../assets/icons/shellkeeper_256.png");
            let img   = image::load_from_memory(bytes).unwrap().to_rgba8();
            let size  = [img.width() as usize, img.height() as usize];
            let pixels = img.into_raw();
            cc.egui_ctx.load_texture(
                "app_icon",
                egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                egui::TextureOptions::LINEAR,
            )
        };

        Self {
            palette,
            icon_texture,
            config,
            sessions:   Vec::new(),
            active_tab: 0,
            view:       AppView::default(),
            sidebar:    SidebarState::default(),
            terminal:   TerminalState::default(),
            dialog:     DialogState::default(),
            ctx:        cc.egui_ctx.clone(),
        }
    }

    // ── Session management ────────────────────────────────────────────────────

    fn open_ssh(&mut self, conn: SshConnection) {
        // Strip any port-forward rules whose local port is already bound by
        // an active session. This lets duplicate tabs open cleanly without
        // needing the user to manually disable forwarding.
        let bound_ports: std::collections::HashSet<u16> = self.sessions.iter()
            .filter(|s| s.is_alive())
            .flat_map(|s| s.connection.iter().flat_map(|c| c.local_ports()))
            .collect();

        let mut conn = conn;
        if !bound_ports.is_empty() {
            conn.port_forwards.retain(|fwd| !bound_ports.contains(&fwd.local_port));
        }

        let (cols, rows) = (self.terminal.last_cols, self.terminal.last_rows);
        let should_log = conn.log_session.unwrap_or(self.config.log_sessions);
        let log_dir = should_log.then(|| std::path::PathBuf::from(&self.config.log_dir));

        // Trigger auto-mounts before opening the SSH session
        let sshfs_available = std::process::Command::new("which")
            .arg("sshfs").output().map(|o| o.status.success()).unwrap_or(false);

        let mut auto_mounts: Vec<String> = Vec::new();
        let mut mount_errors: Vec<String> = Vec::new();

        for m in conn.mounts.iter().filter(|m| {
            m.auto_mount && !m.remote_path.trim().is_empty() && !m.local_path.trim().is_empty()
        }) {
            if !sshfs_available {
                #[cfg(target_os = "macos")]
                mount_errors.push(format!(
                    "sshfs not installed — cannot mount {}. Install: brew install --cask macfuse && brew install gromgit/fuse/sshfs-mac", m.remote_path
                ));
                #[cfg(not(target_os = "macos"))]
                mount_errors.push(format!(
                    "sshfs not installed — cannot mount {}. Install: sudo apt install sshfs", m.remote_path
                ));
                continue;
            }
            let local = shellexpand::tilde(&m.local_path).into_owned();
            if let Err(e) = std::fs::create_dir_all(&local) {
                mount_errors.push(format!("Cannot create mountpoint {}: {e}", local));
                continue;
            }
            match std::process::Command::new("sshfs")
                .args(m.sshfs_args(&conn))
                .spawn()
            {
                Ok(_)  => auto_mounts.push(local),
                Err(e) => mount_errors.push(format!("sshfs {}: {e}", m.remote_path)),
            }
        }

        match PtySession::new_ssh(conn, cols, rows, self.ctx.clone(), log_dir) {
            Ok(mut s) => {
                s.active_mounts = auto_mounts;
                s.mount_errors  = mount_errors;
                self.sessions.push(s);
                self.focus_last();
            }
            Err(e) => eprintln!("[shellkeeper] SSH error: {e}"),
        }
    }

    fn open_local(&mut self) {
        let (cols, rows) = (self.terminal.last_cols, self.terminal.last_rows);
        match PtySession::new_local(cols, rows, self.ctx.clone(), &self.config.local_shell) {
            Ok(s) => { self.sessions.push(s); self.focus_last(); }
            Err(e) => eprintln!("[shellkeeper] local shell error: {e}"),
        }
    }

    fn close_tab(&mut self, idx: usize) {
        if idx < self.sessions.len() {
            // Unmount any sshfs mounts that were auto-mounted for this session.
            // macOS uses `umount` directly; Linux prefers fusermount3/fusermount.
            for mountpoint in &self.sessions[idx].active_mounts {
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("umount")
                    .arg(mountpoint).spawn();

                #[cfg(not(target_os = "macos"))]
                let _ = std::process::Command::new("fusermount3")
                    .args(["-u", mountpoint])
                    .spawn()
                    .or_else(|_| std::process::Command::new("fusermount")
                        .args(["-u", mountpoint]).spawn())
                    .or_else(|_| std::process::Command::new("umount")
                        .arg(mountpoint).spawn());
            }
            self.sessions.remove(idx);
            if self.active_tab >= self.sessions.len() && !self.sessions.is_empty() {
                self.active_tab = self.sessions.len() - 1;
            }
        }
    }

    fn focus_last(&mut self) {
        self.active_tab        = self.sessions.len() - 1;
        self.terminal.auto_focus = true;
        self.terminal.focused = true;
    }



    // ── Sidebar event handling ─────────────────────────────────────────────────

    fn handle_sidebar(&mut self, ev: sidebar::SidebarEvents) {
        if let Some(i) = ev.toggle_fav {
            self.config.connections[i].favorite ^= true;
            let _ = self.config.save();
        }
        if ev.add_new {
            self.dialog = DialogState {
                open: true,
                form: ConnForm {
                    port:     self.config.default_port.to_string(),
                    username: self.config.default_username.clone(),
                    auth_label: "SSH Agent".into(),
                    ..Default::default()
                },
                error: String::new(),
            };
        }
        if let Some(i) = ev.edit {
            self.dialog = DialogState {
                open:  true,
                form:  ConnForm::from_conn(&self.config.connections[i]),
                error: String::new(),
            };
        }
        if let Some(i) = ev.delete {
            // Clean up stored password if any
            crate::vault::delete_password(&self.config.connections[i].id);
            self.config.connections.remove(i);
            let _ = self.config.save();
        }
        if let Some(i) = ev.open {
            self.open_conn(i, false);
            self.view = AppView::Terminal; // always leave settings when connecting
        }
        if let Some(i) = ev.open_new_tab {
            self.open_conn(i, true);
            self.view = AppView::Terminal;
        }
        if ev.open_settings { self.view = AppView::Settings; }
        if let Some(name) = ev.theme_change {
            self.apply_theme_by_name(&name);
        }
    }

    fn open_conn(&mut self, i: usize, force_new_tab: bool) {
        let conn_id = self.config.connections[i].id.clone();

        if !force_new_tab {
            if let Some(tab) = self.sessions.iter().position(|s| {
                s.connection.as_ref().map(|c| c.id.as_str()) == Some(conn_id.as_str())
            }) {
                self.active_tab     = tab;
                self.terminal.focused = true;
                return;
            }
        }
        let conn = sidebar::stamp_last_used(&mut self.config, i);
        let _ = self.config.save();
        self.open_ssh(conn);
    }

    // ── Dialog event handling ─────────────────────────────────────────────────

    fn handle_dialog(&mut self, ev: DialogEvent) {
        match ev {
            DialogEvent::Cancel => { self.dialog.open = false; }
            DialogEvent::Save(conn) => {
                let pos = self.config.connections.iter().position(|c| c.id == conn.id);
                if let Some(p) = pos { self.config.connections[p] = conn; }
                else                 { self.config.connections.push(conn); }
                let _ = self.config.save();
                self.dialog.open = false;
            }
            DialogEvent::SshCopyId(conn) => {
                self.dialog.open = false;
                self.run_ssh_copy_id(&conn);
            }
        }
    }

    fn run_ssh_copy_id(&mut self, conn: &SshConnection) {
        let cmd = format!("ssh-copy-id -p {} {}@{}\r", conn.port, conn.username, conn.host);
        let (cols, rows) = (self.terminal.last_cols, self.terminal.last_rows);
        if let Ok(mut s) = PtySession::new_local(cols, rows, self.ctx.clone(), &self.config.local_shell) {
            s.write_input(cmd.as_bytes());
            self.sessions.push(s);
            self.focus_last();
        }
    }

    // ── Settings event handling ────────────────────────────────────────────────

    fn handle_settings(&mut self, ev: SettingsEvent) {
        match ev {
            SettingsEvent::Close => { self.view = AppView::Terminal; }
            SettingsEvent::ThemeChanged(name) => { self.apply_theme_by_name(&name); }
            SettingsEvent::ConfigChanged => { let _ = self.config.save(); }
        }
    }

    fn apply_theme_by_name(&mut self, name: &str) {
        self.palette     = ThemePalette::from_name(name);
        self.config.theme = name.to_string();
        let _ = self.config.save();
    }
}

// ── eframe::App ───────────────────────────────────────────────────────────────

impl eframe::App for SshedApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Push palette into thread-local so all UI helpers read the right colours.
        ui::set_palette(self.palette.clone());
        ui::apply_theme(ctx);

        // ── Sidebar ───────────────────────────────────────────────────────────
        let sidebar_events = egui::SidePanel::left("sidebar")
            .exact_width(270.0)
            .resizable(false)
            .frame(egui::Frame::none().fill(ui::c::SIDEBAR()).inner_margin(0.0))
            .show(ctx, |ui| {
                sidebar::show(
                    &mut self.sidebar,
                    &mut self.config,
                    &self.sessions,
                    self.active_tab,
                    &self.palette,
                    &self.icon_texture,
                    ui,
                )
            })
            .inner;
        self.handle_sidebar(sidebar_events);

        // ── Central panel ─────────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(ui::c::PANEL()).inner_margin(0.0))
            .show(ctx, |ui| match self.view {
                AppView::Settings => {
                    if let Some(ev) = ui::settings::show(&mut self.config, ui) {
                        self.handle_settings(ev);
                    }
                }
                AppView::Terminal => {
                    self.render_terminal_panel(ui);
                }
            });

        // ── Dialog ────────────────────────────────────────────────────────────
        if let Some(ev) = ui::dialog::show(&mut self.dialog, ctx) {
            self.handle_dialog(ev);
        }

        // Periodic repaint for cursor blink. PTY data triggers immediate repaints
        // from the background reader thread, so this only fires when idle.
        if !self.sessions.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }
}

// ── Terminal panel (private) ───────────────────────────────────────────────────

impl SshedApp {
    fn render_terminal_panel(&mut self, ui: &mut egui::Ui) {
        // Unfocus terminal when dialog or settings is open — prevents keystrokes
        // from leaking into the PTY while the user types in a text field.
        if self.dialog.open || self.view == AppView::Settings {
            self.terminal.focused = false;
        }

        if self.sessions.is_empty() {
            overlays::show_empty(&self.icon_texture, ui);
            return;
        }

        // Safety clamp
        if self.active_tab >= self.sessions.len() {
            self.active_tab = self.sessions.len() - 1;
        }

        // Tab bar
        let tab_ev = tabs::show(&self.sessions, self.active_tab, ui);
        if let Some(i) = tab_ev.close  { self.close_tab(i); }
        if let Some(i) = tab_ev.switch {
            self.active_tab        = i;
            self.terminal.auto_focus = true;
        }
        if tab_ev.new_local            { self.open_local(); }

        if self.sessions.is_empty() { return; }

        // Clamp again after a possible close
        if self.active_tab >= self.sessions.len() {
            self.active_tab = self.sessions.len() - 1;
        }

        // Snapshot session state before borrowing sessions mutably
        let (alive, has_output, conn, age, ssh_cmd) = {
            let s = &self.sessions[self.active_tab];
            (s.is_alive(), s.has_output(), s.connection.clone(), s.started_at.elapsed(), s.ssh_command.clone())
        };

        let avail = ui.available_rect_before_wrap();

        if !alive {
            // Render terminal bg (stale output visible behind overlay)
            terminal::show(&mut self.terminal, &mut self.sessions, self.active_tab, self.config.font_size, ui);
            if let Some(action) = overlays::show_dead(avail, conn.as_ref(), age, ui) {
                match action {
                    DeadAction::Retry => {
                        let tab = self.active_tab;
                        self.close_tab(tab);
                        if let Some(c) = conn { self.open_ssh(c); }
                    }
                    DeadAction::Close => {
                        let tab = self.active_tab;
                        self.close_tab(tab);
                    }
                }
            }
            return;
        }

        if !has_output {
            overlays::show_connecting(avail, &ssh_cmd, conn.as_ref(), ui);
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(80));
            return;
        }

        terminal::show(&mut self.terminal, &mut self.sessions, self.active_tab, self.config.font_size, ui);
    }
}
