//! Connection add / edit dialog.

use egui::{Align, Color32, FontId, RichText, Vec2};

use crate::{
    models::{AuthMethod, ForwardKind, PortForward, SshConnection},
    ssh_parse::ParsedSsh,
};
use super::{c, widgets::{form_row, styled_button}};

// ── Form state ─────────────────────────────────────────────────────────────────

/// In-progress form data for adding or editing a connection.
#[derive(Default)]
pub struct ConnForm {
    pub id:            String,
    pub name:          String,
    pub host:          String,
    pub port:          String,
    pub username:      String,
    pub auth_label:    String,
    pub key_path:      String,
    pub description:   String,
    pub favorite:      bool,
    pub ssh_cmd:       String,
    pub ssh_cmd_err:   String,
    pub port_forwards: Vec<PortForward>,
    pub save_password: bool,
    pub password_input: String,  // ephemeral — never serialized
    pub group:         String,
    pub persistent:    bool,
    pub tmux_session:  String,
    /// None = follow global setting, Some(true/false) = override
    pub log_session:   Option<bool>,
    pub mounts:        Vec<crate::models::SshMount>,
}

impl ConnForm {
    /// Initialise form from an existing connection (edit mode).
    pub fn from_conn(c: &SshConnection) -> Self {
        let (auth_label, key_path) = match &c.auth {
            AuthMethod::Agent    => ("SSH Agent".into(), String::new()),
            AuthMethod::Password => ("Password".into(),  String::new()),
            AuthMethod::Key(p)   => ("Key File".into(),  p.clone()),
        };
        Self {
            id: c.id.clone(), name: c.name.clone(), host: c.host.clone(),
            port: c.port.to_string(), username: c.username.clone(),
            auth_label, key_path, description: c.description.clone(),
            favorite: c.favorite, ssh_cmd: String::new(), ssh_cmd_err: String::new(),
            port_forwards: c.port_forwards.clone(),
            save_password: c.save_password,
            password_input: String::new(),
            group: c.group.clone(), persistent: c.persistent,
            tmux_session: c.tmux_session.clone(), log_session: c.log_session,
            mounts: c.mounts.clone(),
        }
    }

    /// Convert form into a `SshConnection`. Returns `None` if required fields are empty.
    pub fn to_connection(&self) -> Option<SshConnection> {
        if self.name.trim().is_empty() || self.host.trim().is_empty() { return None; }
        let port = self.port.parse::<u16>().unwrap_or(22);
        let auth = match self.auth_label.as_str() {
            "Password" => AuthMethod::Password,
            "Key File" => AuthMethod::Key(self.key_path.trim().to_string()),
            _          => AuthMethod::Agent,
        };
        Some(SshConnection {
            id:            if self.id.is_empty() { uuid::Uuid::new_v4().to_string() } else { self.id.clone() },
            name:          self.name.trim().to_string(),
            host:          self.host.trim().to_string(),
            port,
            username:      self.username.trim().to_string(),
            auth,
            favorite:      self.favorite,
            last_used:     None,
            description:   self.description.trim().to_string(),
            port_forwards: self.port_forwards.clone(),
            save_password: self.save_password,
            group:         self.group.trim().to_string(),
            persistent:    self.persistent,
            tmux_session:  self.tmux_session.trim().to_string(),
            log_session:   self.log_session,
            mounts:        self.mounts.clone(),
        })
    }

    pub fn is_new(&self) -> bool { self.id.is_empty() }
}

// ── Dialog state ──────────────────────────────────────────────────────────────

pub struct DialogState {
    pub open:  bool,
    pub form:  ConnForm,
    pub error: String,
}

impl Default for DialogState {
    fn default() -> Self {
        Self { open: false, form: ConnForm::default(), error: String::new() }
    }
}

// ── Events ─────────────────────────────────────────────────────────────────────

pub enum DialogEvent {
    Save(SshConnection),
    /// Open ssh-copy-id for this connection, then close dialog.
    SshCopyId(SshConnection),
    Cancel,
}

// ── Render ────────────────────────────────────────────────────────────────────

/// Render the modal dialog. Returns an event if the user acted.
pub fn show(state: &mut DialogState, ctx: &egui::Context) -> Option<DialogEvent> {
    if !state.open { return None; }
    let mut result: Option<DialogEvent> = None;
    let mut open = true;
    let title = if state.form.is_new() { "Add Connection" } else { "Edit Connection" };

    egui::Window::new(title)
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .min_width(640.0)
        .min_height(520.0)
        .default_size(Vec2::new(700.0, 640.0))
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.visuals_mut().override_text_color = Some(c::TEXT());
            ui.spacing_mut().item_spacing.y = 10.0;
            ui.spacing_mut().window_margin = egui::Margin::same(24.0);

            // Scroll the entire form so the window never grows beyond the screen
            egui::ScrollArea::vertical()
                .id_salt("dialog_scroll")
                .max_height(ui.ctx().screen_rect().height() * 0.82)
                .show(ui, |ui| {

            ui.add_space(4.0);
            render_ssh_import(state, ui);
            ui.add_space(6.0);
            ui.add(egui::Separator::default());
            ui.add_space(6.0);
            render_basic_fields(state, ui);
            ui.add_space(8.0);
            ui.add(egui::Separator::default());
            ui.add_space(6.0);
            render_port_forwarding(state, ui);
            ui.add_space(8.0);
            ui.add(egui::Separator::default());
            ui.add_space(6.0);
            render_sshfs_mounts(state, ui);
            ui.add_space(8.0);
            ui.add(egui::Separator::default());
            ui.horizontal(|ui| {
                ui.checkbox(&mut state.form.favorite, "Favourite");
                ui.add_space(12.0);
                ui.checkbox(&mut state.form.persistent, "Persistent (tmux)");
            });
            ui.horizontal(|ui| {
                // Log session: tri-state — None (global), Some(true), Some(false)
                let log_label = match state.form.log_session {
                    None        => "Log: global",
                    Some(true)  => "Log: on",
                    Some(false) => "Log: off",
                };
                if ui.add(
                    egui::Button::new(RichText::new(log_label).size(11.0).color(c::MUTED()).monospace())
                        .fill(Color32::TRANSPARENT)
                        .stroke(egui::Stroke::new(0.7, c::BORDER()))
                        .rounding(egui::Rounding::same(3.0))
                ).on_hover_text("Click to cycle: global → always on → always off")
                 .on_hover_cursor(egui::CursorIcon::PointingHand)
                 .clicked()
                {
                    state.form.log_session = match state.form.log_session {
                        None        => Some(true),
                        Some(true)  => Some(false),
                        Some(false) => None,
                    };
                }
            });
            // Group / namespace field
            form_row(ui, "Group", &mut state.form.group, "e.g. Backend, Prod…");

            if !state.error.is_empty() {
                ui.label(RichText::new(&state.error).color(c::DANGER()).size(12.0));
            }

            // ── SSH command preview ───────────────────────────────────────────
            let preview = build_ssh_preview(&state.form);
            if !preview.is_empty() {
                ui.add_space(6.0);
                ui.add(egui::Separator::default());
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("COMMAND PREVIEW").size(9.0).color(c::MUTED()).monospace());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let copy_id = egui::Id::new("cmd_preview_copied");
                        let copied_at: Option<f64> = ui.ctx().data(|d| d.get_temp(copy_id));
                        let now = ui.ctx().input(|i| i.time);
                        if let Some(t) = copied_at {
                            if now - t < 2.0 {
                                ui.label(RichText::new("✓ copied!").size(10.0).color(c::GREEN()).monospace());
                                ui.ctx().request_repaint();
                            } else {
                                ui.ctx().data_mut(|d| d.remove::<f64>(copy_id));
                            }
                        } else if ui.add(
                            egui::Button::new(RichText::new("copy").size(10.0).color(c::CYAN()).monospace())
                                .fill(Color32::TRANSPARENT).frame(false)
                        ).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            if let Ok(mut cb) = arboard::Clipboard::new() {  // arboard used inline
                                let _ = cb.set_text(preview.clone());
                                ui.ctx().data_mut(|d| d.insert_temp(copy_id, now));
                            }
                        }
                    });
                });
                ui.add_space(2.0);
                // Multi-line formatted preview with shell-style line continuations
                let formatted = format_ssh_preview(&preview);
                let rows = formatted.lines().count().max(1);
                let mut display = formatted.clone();
                ui.add(
                    egui::TextEdit::multiline(&mut display)
                        .desired_width(f32::INFINITY)
                        .desired_rows(rows)
                        .font(egui::FontId::monospace(10.5))
                        .text_color(c::CYAN().linear_multiply(0.9))
                        .interactive(false)
                        .frame(true)
                );
                ui.add_space(2.0);
            }

            }); // end ScrollArea

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                // ssh-copy-id on the left (only for existing connections)
                if !state.form.is_new() {
                    if styled_button(ui, "ssh-copy-id →", c::GREEN(), true).clicked() {
                        if let Some(conn) = state.form.to_connection() {
                            result = Some(DialogEvent::SshCopyId(conn));
                        }
                    }
                }
                // Save + Cancel pinned to the right
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if styled_button(ui, "Cancel", c::MUTED(), true).clicked() {
                        result = Some(DialogEvent::Cancel);
                    }
                    ui.add_space(4.0);
                    if styled_button(ui, "Save", c::CYAN(), false).clicked() {
                        match state.form.to_connection() {
                            None => state.error = "Name and Host are required.".into(),
                            Some(conn) => {
                                if conn.save_password && !state.form.password_input.is_empty() {
                                    crate::vault::set_password(&conn.id, &state.form.password_input);
                                } else if !conn.save_password {
                                    crate::vault::delete_password(&conn.id);
                                }
                                result = Some(DialogEvent::Save(conn));
                            }
                        }
                    }
                });
            });
        });

    if !open { result = Some(DialogEvent::Cancel); }
    result
}

// ── Sections ──────────────────────────────────────────────────────────────────

fn render_ssh_import(state: &mut DialogState, ui: &mut egui::Ui) {
    ui.label(RichText::new("IMPORT FROM SSH COMMAND").size(10.0).color(c::MUTED()).monospace());
    ui.horizontal(|ui| {
        let cmd_field = ui.add(
            egui::TextEdit::singleline(&mut state.form.ssh_cmd)
                .hint_text(
                    RichText::new("ssh -p 2222 -i ~/.ssh/key.pem user@host")
                        .color(c::MUTED2().linear_multiply(1.5)).italics().monospace(),
                )
                .desired_width(300.0)
                .font(FontId::monospace(12.0))
                .text_color(c::CYAN()),
        );
        let parse = styled_button(ui, "↵ Parse", c::CYAN(), true).clicked();
        let enter = cmd_field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

        if parse || enter {
            match ParsedSsh::parse(&state.form.ssh_cmd) {
                None => state.form.ssh_cmd_err = "Could not parse — use: ssh user@host [-p port] [-i key]".into(),
                Some(p) => {
                    state.form.ssh_cmd_err = String::new();
                    state.form.host     = p.host.clone();
                    state.form.username = p.username.clone();
                    state.form.port     = p.port.to_string();
                    if let Some(key) = p.key_path {
                        state.form.key_path   = key;
                        state.form.auth_label = "Key File".into();
                    }
                    if state.form.name.is_empty() {
                        state.form.name = format!("{}@{}", p.username, p.host);
                    }
                }
            }
        }
    });
    if !state.form.ssh_cmd_err.is_empty() {
        ui.label(RichText::new(&state.form.ssh_cmd_err).color(c::DANGER()).size(11.0).monospace());
    }
}

fn render_basic_fields(state: &mut DialogState, ui: &mut egui::Ui) {
    form_row(ui, "Name *",      &mut state.form.name,     "My Server");
    form_row(ui, "Host *",      &mut state.form.host,     "192.168.1.10");
    form_row(ui, "Port",        &mut state.form.port,     "22");
    form_row(ui, "Username",    &mut state.form.username, "root");

    // Auth method
    ui.horizontal(|ui| {
        ui.label(RichText::new("Auth      ").color(c::MUTED()).size(12.0).strong());
        egui::ComboBox::from_id_salt("auth_cb")
            .selected_text(&state.form.auth_label)
            .width(140.0)
            .show_ui(ui, |ui| {
                for v in AuthMethod::variants() {
                    ui.selectable_value(&mut state.form.auth_label, v.to_string(), *v);
                }
            });
    });

    // Save password option (only shown when auth = Password)
    if state.form.auth_label == "Password" {
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            let prev = state.form.save_password;
            ui.checkbox(&mut state.form.save_password,
                RichText::new("Save password in OS keyring").size(11.0).color(c::MUTED()));
            if prev && !state.form.save_password {
                if !state.form.id.trim().is_empty() {
                    crate::vault::delete_password(&state.form.id);
                }
                state.form.password_input.clear();
            }
        });
        if state.form.save_password {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                ui.label(RichText::new("Password  ").color(c::MUTED()).size(12.0).strong());
                ui.add(
                    egui::TextEdit::singleline(&mut state.form.password_input)
                        .password(true)
                        .desired_width(200.0)
                        .hint_text(RichText::new(
                            if !state.form.id.is_empty() && crate::vault::get_password(&state.form.id).is_some() {
                                "●●●● (saved)"
                            } else {
                                "enter password"
                            }
                        ).color(c::MUTED2().linear_multiply(1.5)).italics())
                        .text_color(c::TEXT())
                        .font(FontId::monospace(12.0)),
                );
            });
        }
    }

    // Key file picker
    if state.form.auth_label == "Key File" {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Key File  ").color(c::MUTED()).size(12.0).strong());
            ui.add(
                egui::TextEdit::singleline(&mut state.form.key_path)
                    .hint_text(RichText::new("~/.ssh/id_rsa").color(c::MUTED2().linear_multiply(1.5)).italics().monospace())
                    .desired_width(200.0)
                    .font(FontId::monospace(12.0))
                    .text_color(c::TEXT()),
            );
            if styled_button(ui, "📂 Browse", c::MUTED(), true).clicked() {
                let ssh_dir = dirs::home_dir().map(|h| h.join(".ssh")).unwrap_or_default();
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Select SSH private key")
                    .set_directory(&ssh_dir)
                    .add_filter("Key files", &["pem", "key", "ppk", ""])
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    state.form.key_path = path.to_string_lossy().to_string();
                }
            }
        });
    }

    form_row(ui, "Description", &mut state.form.description, "(optional)");
}

fn render_port_forwarding(state: &mut DialogState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("PORT FORWARDING").size(10.0).color(c::MUTED()).monospace().strong());
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            if styled_button(ui, "+ Add Rule", c::CYAN(), true).clicked() {
                state.form.port_forwards.push(PortForward::new_local());
            }
        });
    });

    let mut to_remove: Option<usize> = None;
    for (idx, fwd) in state.form.port_forwards.iter_mut().enumerate() {
        ui.add_space(2.0);
        let card_h = 58.0;
        let card_r = ui.available_rect_before_wrap();
        let card_r = egui::Rect::from_min_size(card_r.min, Vec2::new(card_r.width(), card_h));
        ui.painter().rect_filled(card_r, egui::Rounding::same(3.0), c::SURFACE());
        ui.painter().rect_stroke(card_r,  egui::Rounding::same(3.0), egui::Stroke::new(0.5, c::BORDER()));

        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(card_r.shrink(6.0)), |ui| {
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt(("fwd_kind", idx))
                    .selected_text(fwd.kind.label()).width(150.0)
                    .show_ui(ui, |ui| {
                        for v in ForwardKind::variants() {
                            ui.selectable_value(&mut fwd.kind, ForwardKind::from_label(v), *v);
                        }
                    });
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button(RichText::new("✕").color(c::DANGER()))
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked() {
                        to_remove = Some(idx);
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Local port").color(c::MUTED()).size(11.0));
                let mut lp = fwd.local_port.to_string();
                if ui.add(egui::TextEdit::singleline(&mut lp).desired_width(50.0).font(FontId::monospace(12.0))).changed() {
                    fwd.local_port = lp.parse().unwrap_or(fwd.local_port);
                }
                if !matches!(fwd.kind, ForwardKind::Dynamic) {
                    ui.label(RichText::new("→  remote").color(c::MUTED()).size(11.0));
                    ui.add(egui::TextEdit::singleline(&mut fwd.remote_host)
                        .hint_text(RichText::new("host or IP").color(c::MUTED2().linear_multiply(1.5)).italics())
                        .desired_width(100.0).font(FontId::monospace(12.0)));
                    ui.label(RichText::new(":").color(c::MUTED()).size(11.0));
                    let mut rp = fwd.remote_port.to_string();
                    if ui.add(egui::TextEdit::singleline(&mut rp).desired_width(45.0).font(FontId::monospace(12.0))).changed() {
                        fwd.remote_port = rp.parse().unwrap_or(fwd.remote_port);
                    }
                }
            });
        });
        ui.label(RichText::new(format!("  {}", fwd.summary())).color(c::CYAN().linear_multiply(0.6)).monospace().size(10.0));
        ui.add_space(2.0);
    }
    if let Some(i) = to_remove { state.form.port_forwards.remove(i); }
}

/// Build a preview of the final SSH command from the current form state.
fn build_ssh_preview(form: &ConnForm) -> String {
    if form.host.trim().is_empty() { return String::new(); }
    let mut parts = vec!["ssh".to_string()];
    let port = form.port.parse::<u16>().unwrap_or(22);
    if port != 22 { parts.extend(["-p".into(), port.to_string()]); }
    if form.auth_label == "Key File" && !form.key_path.trim().is_empty() {
        parts.extend(["-i".into(), form.key_path.trim().to_string()]);
    }
    for fwd in &form.port_forwards {
        parts.extend(fwd.ssh_args());
    }
    let user = if form.username.trim().is_empty() { "user" } else { form.username.trim() };
    if form.persistent {
        let tmux = if form.tmux_session.trim().is_empty() { "shellkeeper-xxxx".to_string() } else { form.tmux_session.trim().to_string() };
        parts.extend(["-t".into(), format!("{}@{}", user, form.host.trim())]);
        parts.push(format!("\"tmux new-session -A -s {}\"", tmux));
    } else {
        parts.push(format!("{}@{}", user, form.host.trim()));
    }
    parts.join(" ")
}

/// Format the SSH command with shell-style `\` line continuations.
/// Each flag group goes on its own line for readability.
fn format_ssh_preview(cmd: &str) -> String {
    // Tokenize naively (splits on spaces, but respects quoted strings)
    let mut tokens = Vec::<String>::new();
    let mut cur = String::new();
    let mut in_q = false;
    for ch in cmd.chars() {
        match ch {
            '"' => { in_q = !in_q; cur.push(ch); }
            ' ' if !in_q => { if !cur.is_empty() { tokens.push(cur.clone()); cur.clear(); } }
            _ => cur.push(ch),
        }
    }
    if !cur.is_empty() { tokens.push(cur); }

    // Group tokens: binary first, then flag+value pairs, then destination
    let mut lines: Vec<String> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let t = &tokens[i];
        if i == 0 {
            lines.push(t.clone()); // "ssh"
            i += 1;
        } else if t.starts_with('-') {
            // Flags that take a value: -p, -i, -L, -R, -D, -t (only if next isn't a flag)
            let takes_val = matches!(t.as_str(), "-p" | "-i" | "-L" | "-R" | "-D");
            if takes_val {
                if let Some(val) = tokens.get(i + 1) {
                    lines.push(format!("  {} {}", t, val));
                    i += 2;
                    continue;
                }
            }
            // -t alone
            lines.push(format!("  {}", t));
            i += 1;
        } else {
            // destination / tmux command
            lines.push(format!("  {}", t));
            i += 1;
        }
    }

    // Join with " \\\n"
    lines.join(" \\\n")
}

fn render_sshfs_mounts(state: &mut DialogState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("SSHFS MOUNTS").size(9.0).color(c::CYAN().linear_multiply(0.8)).monospace().strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(
                egui::Button::new(RichText::new("+ add mount").size(10.0).color(c::GREEN()).monospace())
                    .fill(Color32::TRANSPARENT).frame(false)
            ).on_hover_cursor(egui::CursorIcon::PointingHand)
             .clicked() {
                state.form.mounts.push(crate::models::SshMount::new());
            }
        });
    });

    let mut to_remove: Option<usize> = None;
    for (idx, mount) in state.form.mounts.iter_mut().enumerate() {
        ui.add_space(4.0);
        let card_bg = c::SURFACE();
        let card_r  = ui.available_rect_before_wrap();
        let card_h  = 58.0;
        let card_r  = egui::Rect::from_min_size(card_r.min, egui::Vec2::new(card_r.width(), card_h));
        ui.painter().rect_filled(card_r, egui::Rounding::same(4.0), card_bg);
        ui.painter().rect_stroke(card_r, egui::Rounding::same(4.0), egui::Stroke::new(0.5, c::BORDER()));
        ui.allocate_space(egui::Vec2::new(card_r.width(), 4.0));

        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("Remote").size(10.0).color(c::MUTED()));
            ui.add(egui::TextEdit::singleline(&mut mount.remote_path)
                .desired_width(130.0)
                .hint_text(RichText::new("/var/www").italics().color(c::MUTED2()))
                .text_color(c::TEXT()).font(egui::FontId::monospace(11.0)));
            ui.label(RichText::new("→").size(11.0).color(c::MUTED()));
            ui.label(RichText::new("Local").size(10.0).color(c::MUTED()));
            ui.add(egui::TextEdit::singleline(&mut mount.local_path)
                .desired_width(130.0)
                .hint_text(RichText::new("~/mnt/prod").italics().color(c::MUTED2()))
                .text_color(c::TEXT()).font(egui::FontId::monospace(11.0)));
        });
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.checkbox(&mut mount.auto_mount, RichText::new("auto-mount on connect").size(10.0).color(c::MUTED()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                if ui.add(
                    egui::Button::new(RichText::new("remove").size(10.0).color(c::DANGER()).monospace())
                        .fill(Color32::TRANSPARENT).frame(false)
                ).on_hover_cursor(egui::CursorIcon::PointingHand)
                 .clicked() { to_remove = Some(idx); }
            });
        });
        ui.allocate_space(egui::Vec2::new(card_r.width(), 4.0));
    }
    if let Some(i) = to_remove { state.form.mounts.remove(i); }
}
