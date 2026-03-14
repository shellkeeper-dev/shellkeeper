//! Shared UI primitives: buttons, form rows, accordion headers, sidebar items.

use egui::{Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use super::c;

// ── Generic helpers ────────────────────────────────────────────────────────────

/// A styled button with an optional ghost (transparent-fill) variant.
pub fn styled_button(
    ui: &mut egui::Ui,
    label: &str,
    color: Color32,
    ghost: bool,
) -> egui::Response {
    let text = RichText::new(label).size(13.0).color(color);
    let resp = if ghost {
        ui.add(egui::Button::new(text)
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, color)))
    } else {
        ui.add(egui::Button::new(text)
            .fill(color.linear_multiply(0.15))
            .stroke(Stroke::new(1.0, color)))
    };
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A labelled single-line text field with dim-italic placeholder.
pub fn form_row(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(c::MUTED()).size(12.0).strong());
        ui.add(
            egui::TextEdit::singleline(value)
                .hint_text(
                    RichText::new(hint)
                        .color(c::MUTED2().linear_multiply(1.6))
                        .italics(),
                )
                .desired_width(260.0)
                .text_color(c::TEXT()),
        );
    });
}

/// Clickable section header with chevron and count badge.
/// Returns `true` if the user clicked (i.e. wants to toggle expansion).
pub fn accordion_header(
    ui: &mut egui::Ui,
    label: &str,
    count: usize,
    expanded: bool,
) -> bool {
    let desired = Vec2::new(ui.available_width(), 28.0);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());

    let hovered = resp.hovered();
    let bg      = if hovered { c::HOVER() } else { Color32::TRANSPARENT };
    ui.painter().rect_filled(rect, 0.0, bg);

    ui.painter().line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(0.5, c::BORDER()),
    );

    let chevron  = if expanded { "▾" } else { "▸" };
    let chev_col = if hovered { c::CYAN() } else { c::MUTED() };
    ui.painter().text(
        Pos2::new(rect.left() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        chevron,
        FontId::monospace(11.0),
        chev_col,
    );

    let label_col = if hovered { c::TEXT() } else { c::MUTED() };
    ui.painter().text(
        Pos2::new(rect.left() + 26.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::monospace(10.0),
        label_col,
    );

    let badge_col = if expanded {
        c::CYAN().linear_multiply(0.7)
    } else {
        c::MUTED2().linear_multiply(1.5)
    };
    ui.painter().text(
        Pos2::new(rect.right() - 12.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        &count.to_string(),
        FontId::monospace(10.0),
        badge_col,
    );

    resp.clicked()
}

/// Pixel-accurate text truncation. Appends "…" if text exceeds `max_w` pixels.
pub fn truncate_text(ui: &egui::Ui, text: &str, max_w: f32, font_id: &FontId) -> String {
    let full_w = ui.fonts(|f| {
        f.layout_no_wrap(text.to_string(), font_id.clone(), Color32::WHITE)
            .size()
            .x
    });
    if full_w <= max_w {
        return text.to_string();
    }
    let ellipsis_w = ui.fonts(|f| {
        f.layout_no_wrap("…".into(), font_id.clone(), Color32::WHITE)
            .size()
            .x
    });
    let target_w   = max_w - ellipsis_w;
    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        let s: String = chars[..mid].iter().collect();
        let w = ui.fonts(|f| {
            f.layout_no_wrap(s, font_id.clone(), Color32::WHITE).size().x
        });
        if w <= target_w { lo = mid; } else { hi = mid - 1; }
    }
    format!("{}…", chars[..lo].iter().collect::<String>())
}

// ── ConnectionItem widget ──────────────────────────────────────────────────────

/// A single connection row in the sidebar list.
pub struct ConnectionItem<'a> {
    pub name:     &'a str,
    pub subtitle: &'a str,
    pub favorite: bool,
    /// Has **any** alive session (not necessarily the focused tab).
    pub live:     bool,
    /// Is this connection open in the currently **focused** tab.
    pub focused:  bool,
}

impl<'a> egui::Widget for ConnectionItem<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired = Vec2::new(ui.available_width(), 54.0);
        let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());

        let hovered = resp.hovered();
        let is_lit  = hovered || self.focused;

        // Background
        let bg = if self.focused && !hovered {
            c::HOVER().linear_multiply(0.6) // subtle active bg
        } else if hovered {
            c::HOVER()
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 0.0, bg);

        // Hover: left accent bar + glow
        if hovered {
            ui.painter().rect_filled(
                Rect::from_min_size(rect.left_top(), Vec2::new(2.0, rect.height())),
                0.0,
                c::CYAN().linear_multiply(0.7),
            );
            ui.painter().rect_filled(
                Rect::from_min_size(rect.left_top(), Vec2::new(14.0, rect.height())),
                0.0,
                c::CYAN_DIM(),
            );
        }

        // Right-side live indicator / arrow
        let right_offset = if self.favorite { 32.0 } else { 18.0 };
        let badge_pos    = Pos2::new(rect.right() - right_offset, rect.top() + 10.0);

        if self.focused && self.live {
            ui.painter().text(
                badge_pos, egui::Align2::RIGHT_CENTER,
                "●", FontId::monospace(8.5), c::GREEN(),
            );
        } else if self.live {
            ui.painter().text(
                badge_pos, egui::Align2::RIGHT_CENTER,
                "●", FontId::monospace(9.0), c::GREEN().linear_multiply(0.55),
            );
        } else if hovered {
            ui.painter().text(
                Pos2::new(rect.right() - 14.0, rect.center().y),
                egui::Align2::CENTER_CENTER,
                "›", FontId::monospace(18.0), c::CYAN().linear_multiply(0.7),
            );
        }

        // Favourite star
        if self.favorite {
            ui.painter().text(
                Pos2::new(rect.right() - 14.0, rect.top() + 10.0),
                egui::Align2::CENTER_CENTER,
                "★", FontId::proportional(11.0), c::YELLOW(),
            );
        }

        // Bottom separator
        ui.painter().line_segment(
            [Pos2::new(rect.left() + 14.0, rect.bottom()), Pos2::new(rect.right(), rect.bottom())],
            Stroke::new(0.5, c::BORDER()),
        );

        // Name
        let name_col = if self.focused {
            c::CYAN()
        } else if hovered {
            c::CYAN().linear_multiply(0.85)
        } else {
            c::TEXT()
        };
        ui.painter().text(
            Pos2::new(rect.left() + 16.0, rect.top() + 10.0),
            egui::Align2::LEFT_TOP,
            self.name,
            FontId::proportional(13.5),
            name_col,
        );

        // Subtitle
        ui.painter().text(
            Pos2::new(rect.left() + 16.0, rect.top() + 29.0),
            egui::Align2::LEFT_TOP,
            self.subtitle,
            FontId::monospace(11.0),
            if is_lit {
                c::MUTED().linear_multiply(1.5)
            } else {
                c::MUTED()
            },
        );

        resp
    }
}
