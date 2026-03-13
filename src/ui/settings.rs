//! Settings panel — full-panel view for app configuration.

use egui::{FontId, Pos2, Rect, RichText, Stroke, Vec2};

use crate::{config::AppConfig, theme::ThemePalette};
use super::c;

// ── Events ─────────────────────────────────────────────────────────────────────

pub enum SettingsEvent {
    Close,
    ThemeChanged(String),
    /// Config was mutated in-place; caller should persist it.
    ConfigChanged,
}

// ── Render ────────────────────────────────────────────────────────────────────

/// Render the settings panel, filling the central panel.
pub fn show(config: &mut AppConfig, ui: &mut egui::Ui) -> Option<SettingsEvent> {
    let mut event: Option<SettingsEvent> = None;

    ui.visuals_mut().override_text_color = Some(c::TEXT());

    // ── Header ───────────────────────────────────────────────────────────────
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        if ui.add(
            egui::Button::new(RichText::new("< back").size(12.0).color(c::MUTED()).monospace())
                .frame(false),
        ).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            event = Some(SettingsEvent::Close);
        }
        ui.add_space(8.0);
        ui.label(RichText::new("SETTINGS").size(16.0).color(c::TEXT()).strong());
    });
    ui.add_space(6.0);
    let div = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [div.left_top(), egui::Pos2::new(div.right(), div.top())],
        Stroke::new(1.0, c::BORDER()),
    );
    ui.add_space(16.0);

    egui::ScrollArea::vertical()
        .id_salt("settings_scroll")
        .show(ui, |ui| {
        ui.add_space(4.0);
        let avail_w  = ui.available_width();
        let content_w = (avail_w - 80.0).min(640.0);
        let side_pad  = ((avail_w - content_w) / 2.0).max(20.0);

        // Use spacing instead of a nested horizontal layout so egui
        // knows the full width and puts the scrollbar at the panel edge.
        ui.add_space(0.0); // force full-width layout claim
        let mut child = ui.new_child(
            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                egui::pos2(ui.min_rect().left() + side_pad, ui.cursor().top()),
                egui::vec2(content_w, f32::INFINITY),
            ))
        );
        {
            let ui = &mut child;

                if let Some(e) = section_appearance(config, ui) { event = Some(e); }
                ui.add_space(20.0);
                if section_terminal(config, ui) { event = Some(SettingsEvent::ConfigChanged); }
                ui.add_space(20.0);
                if section_ssh_defaults(config, ui) { event = Some(SettingsEvent::ConfigChanged); }
                ui.add_space(20.0);
                if section_logs(config, ui) { event = Some(SettingsEvent::ConfigChanged); }
                ui.add_space(20.0);
                section_ssh_keys(ui);
                ui.add_space(20.0);
                section_about(ui);
                ui.add_space(32.0);
        }
        // Advance the parent ui cursor past the child content
        ui.advance_cursor_after_rect(egui::Rect::from_min_size(
            egui::pos2(ui.min_rect().left(), child.min_rect().top()),
            egui::vec2(avail_w, child.min_rect().height()),
        ));
    });

    event
}

// ── Appearance ────────────────────────────────────────────────────────────────

fn section_appearance(config: &mut AppConfig, ui: &mut egui::Ui) -> Option<SettingsEvent> {
    section_title(ui, "APPEARANCE");
    let mut result = None;

    ui.add_space(8.0);
    ui.label(RichText::new("Theme").color(c::MUTED()).size(12.0));
    ui.add_space(6.0);

    // Theme cards in a horizontal row
    ui.horizontal(|ui| {
        for tp in ThemePalette::all() {
            let is_active = config.theme == tp.name;
            if let Some(e) = theme_card(ui, &tp, is_active) {
                config.theme = tp.name.to_string();
                result = Some(e);
            }
        }
    });

    result
}

/// A visual theme card showing a colour preview strip and name.
fn theme_card(ui: &mut egui::Ui, tp: &ThemePalette, active: bool) -> Option<SettingsEvent> {
    let card_w = 160.0;
    let card_h = 90.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(card_w, card_h), egui::Sense::click());

    let border_col = if active {
        tp.cyan
    } else if resp.hovered() {
        tp.cyan.linear_multiply(0.5)
    } else {
        tp.border
    };

    ui.painter().rect_filled(rect, egui::Rounding::same(6.0), tp.bg);
    ui.painter().rect_stroke(rect, egui::Rounding::same(6.0),
        Stroke::new(if active { 2.0 } else { 1.0 }, border_col));

    // Colour swatches — mini palette preview
    let swatch_y  = rect.top() + 14.0;
    let swatch_h  = 14.0;
    let swatches  = [tp.bg, tp.surface, tp.cyan, tp.green, tp.pink];
    let swatch_w  = (card_w - 20.0) / swatches.len() as f32;
    for (i, &col) in swatches.iter().enumerate() {
        let sx = rect.left() + 10.0 + i as f32 * swatch_w;
        ui.painter().rect_filled(
            Rect::from_min_size(Pos2::new(sx, swatch_y), Vec2::new(swatch_w - 2.0, swatch_h)),
            egui::Rounding::same(2.0), col,
        );
    }

    // Theme name
    let name_label = match tp.name {
        "dark"    => "◑  Dark",
        "dracula" => "◈  Dracula",
        "light"   => "☀  Light",
        _         => "⚡  Cyberpunk",
    };
    ui.painter().text(
        Pos2::new(rect.center().x, rect.top() + 44.0),
        egui::Align2::CENTER_CENTER,
        name_label,
        FontId::proportional(13.0),
        if active { tp.cyan } else { tp.text },
    );

    // Description
    let desc = match tp.name {
        "dark"    => "GitHub-style dark",
        "dracula" => "Purple & soft tones",
        "light"   => "Clean light mode",
        _         => "Electric neon on void",
    };
    ui.painter().text(
        Pos2::new(rect.center().x, rect.top() + 62.0),
        egui::Align2::CENTER_CENTER,
        desc,
        FontId::proportional(10.5),
        tp.muted,
    );

    // Active tick
    if active {
        ui.painter().text(
            Pos2::new(rect.right() - 10.0, rect.top() + 10.0),
            egui::Align2::RIGHT_TOP,
            "✓",
            FontId::proportional(12.0),
            tp.cyan,
        );
    }

    if resp.clicked() && !active {
        Some(SettingsEvent::ThemeChanged(tp.name.to_string()))
    } else {
        None
    }
}

// ── Terminal ──────────────────────────────────────────────────────────────────

/// Returns `true` if config was mutated.
fn section_terminal(config: &mut AppConfig, ui: &mut egui::Ui) -> bool {
    section_title(ui, "TERMINAL");
    let mut changed = false;
    ui.add_space(8.0);

    // Font size slider
    ui.horizontal(|ui| {
        ui.add_sized([120.0, 14.0], egui::Label::new(RichText::new("Font size").color(c::MUTED()).size(12.0)));
        let prev = config.font_size;
        ui.add(egui::Slider::new(&mut config.font_size, 10.0..=22.0).step_by(0.5));
        ui.label(RichText::new(format!("{:.0}px", config.font_size)).color(c::TEXT()).monospace().size(11.0));
        if (config.font_size - prev).abs() > f32::EPSILON { changed = true; }
    });

    ui.add_space(4.0);

    // Scrollback lines
    ui.horizontal(|ui| {
        ui.add_sized([120.0, 14.0], egui::Label::new(RichText::new("Scrollback").color(c::MUTED()).size(12.0)));
        let prev = config.scrollback_lines;
        ui.add(egui::Slider::new(&mut config.scrollback_lines, 500..=50_000).logarithmic(true));
        ui.label(RichText::new(format!("{} lines", config.scrollback_lines)).color(c::TEXT()).monospace().size(11.0));
        if config.scrollback_lines != prev { changed = true; }
    });

    changed
}

// ── SSH Defaults ──────────────────────────────────────────────────────────────

fn section_ssh_defaults(config: &mut AppConfig, ui: &mut egui::Ui) -> bool {
    section_title(ui, "SSH DEFAULTS");
    let mut changed = false;
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.add_sized([120.0, 14.0], egui::Label::new(RichText::new("Default user").color(c::MUTED()).size(12.0)));
        let prev = config.default_username.clone();
        ui.add(egui::TextEdit::singleline(&mut config.default_username)
            .desired_width(180.0)
            .text_color(c::TEXT())
            .hint_text(RichText::new("root").color(c::MUTED2().linear_multiply(1.5)).italics()));
        if config.default_username != prev { changed = true; }
    });

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.add_sized([120.0, 14.0], egui::Label::new(RichText::new("Local shell").color(c::MUTED()).size(12.0)));
        let prev = config.local_shell.clone();
        ui.add(egui::TextEdit::singleline(&mut config.local_shell)
            .desired_width(200.0)
            .hint_text(RichText::new("default: $SHELL").color(c::MUTED2().linear_multiply(1.5)).italics())
            .text_color(c::TEXT())
            .font(egui::FontId::monospace(12.0)));
        if config.local_shell != prev { changed = true; }
    });

    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.add_sized([120.0, 14.0], egui::Label::new(RichText::new("Default port").color(c::MUTED()).size(12.0)));
        let prev = config.default_port;
        let mut port_str = config.default_port.to_string();
        if ui.add(egui::TextEdit::singleline(&mut port_str)
            .desired_width(70.0)
            .text_color(c::TEXT())
            .font(FontId::monospace(13.0))).changed()
        {
            if let Ok(p) = port_str.parse::<u16>() {
                config.default_port = p;
            }
        }
        if config.default_port != prev { changed = true; }
    });

    changed
}

// ── About ─────────────────────────────────────────────────────────────────────

fn section_about(ui: &mut egui::Ui) {
    section_title(ui, "ABOUT");
    ui.add_space(8.0);

    let items = [
        ("Version",    env!("CARGO_PKG_VERSION")),
        ("License",    "Apache-2.0"),
        ("Built with", "Rust · egui 0.29 · portable-pty"),
    ];

    for (label, value) in items {
        ui.horizontal(|ui| {
            ui.add_sized([120.0, 14.0], egui::Label::new(RichText::new(label).color(c::MUTED()).size(12.0)));
            ui.label(RichText::new(value).color(c::TEXT()).monospace().size(12.0));
        });
        ui.add_space(2.0);
    }
}

// ── Layout helper ─────────────────────────────────────────────────────────────

fn section_title(ui: &mut egui::Ui, title: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(10.0).color(c::CYAN().linear_multiply(0.8)).monospace().strong());
        let r = ui.available_rect_before_wrap();
        ui.painter().line_segment(
            [Pos2::new(r.left() + 4.0, r.center().y), Pos2::new(r.right(), r.center().y)],
            Stroke::new(0.5, c::BORDER()),
        );
        ui.allocate_space(Vec2::new(ui.available_width(), 0.0));
    });
}

// ── Session Logs ──────────────────────────────────────────────────────────────

fn section_logs(config: &mut AppConfig, ui: &mut egui::Ui) -> bool {
    section_title(ui, "SESSION LOGS");
    let mut changed = false;
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        let prev = config.log_sessions;
        ui.checkbox(&mut config.log_sessions, "");
        ui.label(RichText::new("Save terminal output to disk").color(c::TEXT()).size(12.0));
        if config.log_sessions != prev { changed = true; }
    });

    if config.log_sessions {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_sized([100.0, 14.0], egui::Label::new(RichText::new("Log directory").color(c::MUTED()).size(12.0)));
            let prev = config.log_dir.clone();
            ui.add(egui::TextEdit::singleline(&mut config.log_dir)
                .desired_width(260.0)
                .text_color(super::c::TEXT())
                .font(egui::FontId::monospace(11.0)));
            if config.log_dir != prev { changed = true; }
            if ui.button(RichText::new("open").size(10.0).color(super::c::MUTED()).monospace())
               .on_hover_text("Open log directory in file manager")
               .clicked()
            {
                let _ = std::process::Command::new("xdg-open").arg(&config.log_dir).spawn();
            }
        });
        ui.add_space(2.0);
        ui.label(RichText::new("Logs are saved as raw text files: <dir>/<conn_name>/<timestamp>.log")
            .size(10.0).color(super::c::MUTED2()).italics());
    }

    changed
}

// ── SSH Keys ──────────────────────────────────────────────────────────────────

fn section_ssh_keys(ui: &mut egui::Ui) {
    section_title(ui, "SSH KEYS");
    ui.add_space(8.0);

    let ssh_dir = dirs::home_dir().unwrap_or_default().join(".ssh");
    let keys    = list_ssh_keys(&ssh_dir);

    if keys.is_empty() {
        ui.label(RichText::new("No keys found in ~/.ssh/").size(12.0).color(super::c::MUTED()));
    } else {
        for key in &keys {
            ui.horizontal(|ui| {
                ui.add_sized([180.0, 14.0], egui::Label::new(
                    RichText::new(&key.name).size(12.0).color(super::c::TEXT()).monospace()
                ));
                ui.label(RichText::new(&key.key_type).size(10.0).color(super::c::MUTED()).monospace());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let copied_id = ui.id().with(("copied", &key.name));
                    let copied_at: Option<f64> = ui.ctx().data(|d| d.get_temp(copied_id));
                    let just_copied = copied_at
                        .map(|t| ui.ctx().input(|i| i.time) - t < 2.0)
                        .unwrap_or(false);

                    if just_copied {
                        ui.label(RichText::new("✓ copied!").size(10.0).color(super::c::GREEN()).monospace());
                        ui.ctx().request_repaint();
                    } else if ui.add(
                        egui::Button::new(RichText::new("copy pub").size(10.0).color(super::c::CYAN()).monospace())
                            .frame(false)
                    ).on_hover_text("Copy public key to clipboard")
                     .on_hover_cursor(egui::CursorIcon::PointingHand)
                     .clicked()
                    {
                        let pub_path = ssh_dir.join(format!("{}.pub", key.name));
                        if let Ok(pub_key) = std::fs::read_to_string(&pub_path) {
                            ui.ctx().copy_text(pub_key.trim().to_string());
                            let now = ui.ctx().input(|i| i.time);
                            ui.ctx().data_mut(|d| d.insert_temp(copied_id, now));
                        }
                    }
                });
            });
            ui.add_space(2.0);
        }
    }

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(6.0);
    ui.label(RichText::new("Generate new key").size(12.0).color(super::c::TEXT()).strong());
    ui.add_space(4.0);

    // Keygen form stored in egui temp storage
    let form_id = ui.id().with("keygen_form");
    let mut form: KeygenForm = ui.ctx().data(|d| d.get_temp(form_id).unwrap_or_default());

    ui.horizontal(|ui| {
        ui.label(RichText::new("Type").size(11.0).color(super::c::MUTED()));
        for t in &["ed25519", "rsa", "ecdsa"] {
            let active = form.key_type == *t;
            if ui.add(
                egui::Button::new(RichText::new(*t).size(11.0).monospace()
                    .color(if active { super::c::CYAN() } else { super::c::TEXT() }))
                    .fill(if active { super::c::CYAN().linear_multiply(0.1) } else { egui::Color32::TRANSPARENT })
                    .stroke(egui::Stroke::new(0.7, if active { super::c::CYAN().linear_multiply(0.5) } else { super::c::BORDER() }))
                    .rounding(egui::Rounding::same(3.0))
            ).clicked() {
                form.key_type = t.to_string();
            }
        }
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Name").size(11.0).color(super::c::MUTED()));
        ui.add(egui::TextEdit::singleline(&mut form.name)
            .desired_width(140.0)
            .hint_text(RichText::new("id_ed25519").italics().color(super::c::MUTED2()))
            .text_color(super::c::TEXT()));
        ui.label(RichText::new("Comment").size(11.0).color(super::c::MUTED()));
        ui.add(egui::TextEdit::singleline(&mut form.comment)
            .desired_width(130.0)
            .hint_text(RichText::new("user@host").italics().color(super::c::MUTED2()))
            .text_color(super::c::TEXT()));
    });

    ui.add_space(4.0);
    if !form.status.is_empty() {
        let col = if form.status.starts_with("Error") { super::c::DANGER() } else { super::c::GREEN() };
        ui.label(RichText::new(&form.status).size(11.0).color(col).monospace());
    }

    ui.add_space(4.0);
    if ui.add(
        egui::Button::new(RichText::new("Generate").size(12.0).color(super::c::GREEN()))
            .fill(super::c::GREEN().linear_multiply(0.1))
            .stroke(egui::Stroke::new(0.8, super::c::GREEN().linear_multiply(0.5)))
            .rounding(egui::Rounding::same(3.0))
    ).clicked() && !form.name.trim().is_empty() {
        let key_name  = form.name.trim().to_string();
        let key_type  = form.key_type.clone();
        let comment   = form.comment.trim().to_string();
        let key_path  = ssh_dir.join(&key_name);
        let mut args  = vec!["-t".to_string(), key_type, "-f".to_string(),
                              key_path.to_string_lossy().into_owned(), "-N".to_string(), "".to_string()];
        if !comment.is_empty() { args.extend(["-C".to_string(), comment]); }

        match std::process::Command::new("ssh-keygen").args(&args).output() {
            Ok(out) if out.status.success() => form.status = format!("Created: {}", key_name),
            Ok(out) => form.status = format!("Error: {}", String::from_utf8_lossy(&out.stderr).trim()),
            Err(e)  => form.status = format!("Error: {e}"),
        }
    }

    ui.ctx().data_mut(|d| d.insert_temp(form_id, form));
}

// ── SSH Keys helpers ──────────────────────────────────────────────────────────

struct SshKeyInfo { name: String, key_type: String }

fn list_ssh_keys(ssh_dir: &std::path::Path) -> Vec<SshKeyInfo> {
    let Ok(entries) = std::fs::read_dir(ssh_dir) else { return vec![]; };
    let mut keys = vec![];
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        // Skip .pub files, known_hosts, config, etc — only private keys
        if name.ends_with(".pub") || name == "known_hosts" || name == "config" || name == "authorized_keys" { continue; }
        // Check if matching .pub exists (confirms it's a keypair)
        if !ssh_dir.join(format!("{}.pub", name)).exists() { continue; }
        let key_type = detect_key_type(&path);
        keys.push(SshKeyInfo { name, key_type });
    }
    keys.sort_by(|a, b| a.name.cmp(&b.name));
    keys
}

fn detect_key_type(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).ok()
        .and_then(|s| {
            let first = s.lines().next().unwrap_or("").to_lowercase();
            if first.contains("ed25519") { Some("ed25519") }
            else if first.contains("ecdsa") { Some("ecdsa") }
            else if first.contains("rsa") { Some("rsa") }
            else { None }
        })
        .unwrap_or("unknown")
        .to_string()
}

#[derive(Clone)]
struct KeygenForm {
    key_type: String,
    name:     String,
    comment:  String,
    status:   String,
}

impl Default for KeygenForm {
    fn default() -> Self {
        Self { key_type: "ed25519".into(), name: String::new(), comment: String::new(), status: String::new() }
    }
}
