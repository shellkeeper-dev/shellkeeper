//! Tab bar: one tab per PTY session.

use egui::{FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::pty::PtySession;
use super::{c, widgets::truncate_text};

const TAB_W:    f32 = 164.0;
const TAB_H:    f32 = 36.0;
const DOT_W:    f32 = 18.0;
const CLOSE_W:  f32 = 26.0;
const NAME_PAD: f32 = 6.0;
const NAME_MAX_W: f32 = TAB_W - DOT_W - CLOSE_W - NAME_PAD * 2.0;

// ── Events ─────────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct TabEvents {
    pub close:       Option<usize>,
    pub switch:      Option<usize>,
    pub new_local:   bool,
}

// ── Render ────────────────────────────────────────────────────────────────────

/// Render the tab bar. Returns events for the orchestrator to handle.
pub fn show(sessions: &[PtySession], active_tab: usize, ui: &mut egui::Ui) -> TabEvents {
    let mut events = TabEvents::default();

    let (bar_rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), TAB_H),
        Sense::hover(),
    );
    let painter = ui.painter();
    painter.rect_filled(bar_rect, 0.0, c::TAB_BG());
    painter.line_segment(
        [bar_rect.left_bottom(), bar_rect.right_bottom()],
        Stroke::new(1.0, c::BORDER()),
    );

    let label_font = FontId::monospace(12.0);
    let mut x = bar_rect.left() + 4.0;

    for (idx, session) in sessions.iter().enumerate() {
        let is_active = idx == active_tab;
        let tab_r = Rect::from_min_size(Pos2::new(x, bar_rect.top()), Vec2::new(TAB_W, TAB_H));

        let close_zone = Rect::from_min_size(
            Pos2::new(tab_r.right() - CLOSE_W, tab_r.top()),
            Vec2::new(CLOSE_W, TAB_H),
        );
        let body_r = Rect::from_min_size(tab_r.min, Vec2::new(TAB_W - CLOSE_W, TAB_H));

        let tab_resp   = ui.interact(body_r,    ui.id().with(("tab_body",  idx)), Sense::click());
        let close_resp = ui.interact(close_zone, ui.id().with(("tab_close", idx)), Sense::click());

        // Background
        let bg = if is_active {
            c::TAB_ACTIVE()
        } else if tab_resp.hovered() || close_resp.hovered() {
            c::HOVER()
        } else {
            c::TAB_BG()
        };
        painter.rect_filled(tab_r, egui::Rounding::same(3.0), bg);

        // Active indicator: neon bottom bar
        if is_active {
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(tab_r.left(), tab_r.bottom() - 2.0),
                    Vec2::new(TAB_W, 2.0),
                ),
                0.0,
                c::CYAN(),
            );
        }

        // Dot icon
        let (dot, dot_col) = if session.connection.is_some() {
            ("⬡", c::GREEN())
        } else {
            ("○", c::MUTED())
        };
        painter.text(
            Pos2::new(tab_r.left() + 10.0, tab_r.center().y),
            egui::Align2::LEFT_CENTER,
            dot, FontId::monospace(9.0), dot_col,
        );

        // Label (truncated)
        let full_label  = session.tab_label();
        let display_txt = truncate_text(ui, &full_label, NAME_MAX_W, &label_font);
        let name_col    = if is_active { c::TEXT() } else { c::MUTED() };
        painter.text(
            Pos2::new(tab_r.left() + DOT_W + NAME_PAD, tab_r.center().y),
            egui::Align2::LEFT_CENTER,
            &display_txt, label_font.clone(), name_col,
        );

        // Tooltip
        if tab_resp.hovered() {
            egui::show_tooltip_at_pointer(
                ui.ctx(), ui.layer_id(), egui::Id::new(("tab_tt", idx)),
                |ui| {
                    match &session.connection {
                        None => {
                            ui.label(RichText::new("Local shell").monospace().size(12.0).color(c::TEXT()));
                        }
                        Some(conn) => {
                            ui.label(RichText::new(&conn.name).size(13.0).color(c::TEXT()).strong());
                            ui.label(RichText::new(&conn.subtitle()).monospace().size(11.0).color(c::MUTED()));
                            if !conn.port_forwards.is_empty() {
                                ui.add_space(4.0);
                                ui.label(RichText::new("PORT FORWARDING").size(9.0).color(c::CYAN().linear_multiply(0.8)).monospace());
                                for fwd in &conn.port_forwards {
                                    ui.label(RichText::new(fwd.summary()).monospace().size(11.0).color(c::MUTED()));
                                }
                            }
                            if !conn.mounts.is_empty() {
                                ui.add_space(4.0);
                                ui.label(RichText::new("SSHFS MOUNTS").size(9.0).color(c::CYAN().linear_multiply(0.8)).monospace());
                                for m in &conn.mounts {
                                    let mounted = session.active_mounts.contains(
                                        &shellexpand::tilde(&m.local_path).into_owned()
                                    );
                                    let status = if mounted { "⊞ " } else { "○ " };
                                    let col = if mounted { c::GREEN() } else { c::MUTED() };
                                    ui.label(RichText::new(format!("{status}{}", m.summary())).monospace().size(11.0).color(col));
                                }
                            }
                            if !session.mount_errors.is_empty() {
                                ui.add_space(4.0);
                                ui.label(RichText::new("MOUNT ERRORS").size(9.0).color(c::DANGER()).monospace());
                                for err in &session.mount_errors {
                                    ui.label(RichText::new(err).monospace().size(10.0).color(c::DANGER().linear_multiply(0.8)));
                                }
                            }
                        }
                    }
                },
            );
        }

        // Close button
        let close_center = Pos2::new(tab_r.right() - CLOSE_W / 2.0, tab_r.center().y);
        if close_resp.hovered() {
            painter.circle_filled(close_center, 8.0, c::DANGER().linear_multiply(0.2));
        }
        let close_col = if close_resp.hovered() { c::DANGER() } else { c::MUTED2().linear_multiply(1.8) };
        painter.text(
            close_center, egui::Align2::CENTER_CENTER,
            "×", FontId::proportional(15.0), close_col,
        );
        // Thin separator before close zone
        painter.line_segment(
            [Pos2::new(close_zone.left(), tab_r.top() + 8.0), Pos2::new(close_zone.left(), tab_r.bottom() - 8.0)],
            Stroke::new(0.5, c::BORDER()),
        );

        if close_resp.clicked() {
            events.close = Some(idx);
        } else if tab_resp.clicked() {
            events.switch = Some(idx);
        }

        x += TAB_W + 2.0;
    }

    // "+" button — new local shell
    let plus_r = Rect::from_min_size(
        Pos2::new(x + 4.0, bar_rect.top() + 6.0),
        Vec2::new(26.0, TAB_H - 12.0),
    );
    let plus_resp = ui.interact(plus_r, ui.id().with("tab_plus"), Sense::click());
    if plus_resp.hovered() {
        ui.painter().rect_filled(plus_r, egui::Rounding::same(3.0), c::HOVER());
        ui.painter().rect_stroke(plus_r, egui::Rounding::same(3.0),
            Stroke::new(1.0, c::CYAN().linear_multiply(0.4)));
        egui::show_tooltip_at_pointer(
            ui.ctx(), ui.layer_id(), egui::Id::new("plus_tt"),
            |ui| { ui.label(RichText::new("Open local shell").size(12.0).color(c::TEXT())); },
        );
    }
    ui.painter().text(
        plus_r.center(), egui::Align2::CENTER_CENTER,
        "+", FontId::proportional(17.0),
        if plus_resp.hovered() { c::CYAN() } else { c::MUTED() },
    );
    if plus_resp.clicked() { events.new_local = true; }

    events
}
