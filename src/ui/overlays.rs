//! Full-panel overlays: connecting, dead session, empty state.

use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use crate::{models::SshConnection};
use super::{c, default_bg};

// ── Connecting overlay ─────────────────────────────────────────────────────────

/// Shown while `alive=true` and no PTY output has arrived yet.
pub fn show_connecting(
    area:    Rect,
    ssh_cmd: &str,
    conn:    Option<&SshConnection>,
    ui:      &mut egui::Ui,
) {
    let painter = ui.painter_at(area);
    painter.rect_filled(area, 0.0, default_bg());

    // HUD corner brackets
    let m = 24.0;
    let l = 20.0;
    let s = Stroke::new(1.5, c::CYAN().linear_multiply(0.35));
    for (origin, dx, dy) in [
        (area.left_top(),     Vec2::new(l, 0.0),  Vec2::new(0.0, l)),
        (area.right_top(),    Vec2::new(-l, 0.0), Vec2::new(0.0, l)),
        (area.left_bottom(),  Vec2::new(l, 0.0),  Vec2::new(0.0, -l)),
        (area.right_bottom(), Vec2::new(-l, 0.0), Vec2::new(0.0, -l)),
    ] {
        let o = origin + Vec2::new(
            if dx.x > 0.0 { m } else { -m },
            if dy.y > 0.0 { m } else { -m },
        );
        painter.line_segment([o, o + dx], s);
        painter.line_segment([o, o + dy], s);
    }

    let cx = area.center().x;
    let cy = area.center().y - 20.0;

    // Spinner
    let frame = (ui.ctx().input(|i| i.time) * 10.0) as usize % 8;
    let spinner_chars = ['⣾','⣽','⣻','⢿','⡿','⣟','⣯','⣷'];
    painter.text(
        Pos2::new(cx, cy - 36.0), egui::Align2::CENTER_CENTER,
        spinner_chars[frame].to_string(), FontId::proportional(28.0), c::CYAN(),
    );

    // Title + animated dots
    painter.text(
        Pos2::new(cx, cy + 4.0), egui::Align2::CENTER_CENTER,
        "CONNECTING", FontId::monospace(14.0), c::CYAN(),
    );
    let dots = ["   ", ".  ", ".. ", "..."][(ui.ctx().input(|i| i.time) * 2.0) as usize % 4];
    painter.text(
        Pos2::new(cx + 56.0, cy + 4.0), egui::Align2::LEFT_CENTER,
        dots, FontId::monospace(14.0), c::CYAN().linear_multiply(0.7),
    );

    // Connection name
    if let Some(c) = conn {
        painter.text(
            Pos2::new(cx, cy + 28.0), egui::Align2::CENTER_CENTER,
            &format!("{}  ·  {}", c.name, c.subtitle()),
            FontId::proportional(13.0), c::TEXT().linear_multiply(0.8),
        );
    }

    // SSH command
    let cmd_display = if ssh_cmd.len() > 60 {
        format!("{}…", &ssh_cmd[..60])
    } else {
        ssh_cmd.to_string()
    };
    painter.text(
        Pos2::new(cx, cy + 56.0), egui::Align2::CENTER_CENTER,
        &format!("$ {cmd_display}"), FontId::monospace(11.0), c::MUTED(),
    );
}

// ── Dead session overlay ───────────────────────────────────────────────────────

/// Possible actions from the dead-session overlay.
pub enum DeadAction {
    Retry,
    Close,
}

/// Shown when the PTY process has exited. Returns a user action if clicked.
pub fn show_dead(
    area: Rect,
    conn: Option<&SshConnection>,
    age:  std::time::Duration,
    ui:   &mut egui::Ui,
) -> Option<DeadAction> {
    let painter = ui.painter_at(area);
    painter.rect_filled(area, 0.0, Color32::from_rgba_premultiplied(6, 9, 15, 220));
    painter.rect_stroke(area, 0.0, Stroke::new(1.5, c::DANGER().linear_multiply(0.7)));

    let cx = area.center().x;
    let cy = area.center().y - 40.0;

    painter.text(
        Pos2::new(cx, cy - 28.0), egui::Align2::CENTER_CENTER,
        "⊗", FontId::proportional(36.0), c::DANGER().linear_multiply(0.8),
    );

    let title = if age.as_secs() < 8 { "CONNECTION FAILED" } else { "SESSION ENDED" };
    painter.text(
        Pos2::new(cx, cy + 10.0), egui::Align2::CENTER_CENTER,
        title, FontId::monospace(14.0), c::DANGER(),
    );

    let subtitle = match conn {
        Some(c) => format!("{}  ·  {}", c.name, c.subtitle()),
        None    => "local shell".into(),
    };
    painter.text(
        Pos2::new(cx, cy + 32.0), egui::Align2::CENTER_CENTER,
        &subtitle, FontId::monospace(11.0), c::MUTED(),
    );

    if age.as_secs() < 8 {
        painter.text(
            Pos2::new(cx, cy + 52.0), egui::Align2::CENTER_CENTER,
            "scroll up to see the error message",
            FontId::monospace(10.0), c::MUTED2().linear_multiply(1.4),
        );
    }

    // ── Buttons ─────────────────────────────────────────────────────────────
    let btn_w   = 110.0;
    let gap     = 12.0;
    let btn_y   = cy + 82.0;

    if conn.is_some() {
        let start_x = cx - btn_w - gap / 2.0;

        let retry_r = Rect::from_min_size(Pos2::new(start_x, btn_y), Vec2::new(btn_w, 30.0));
        let retry   = ui.interact(retry_r, ui.id().with("dead_retry"), Sense::click());
        draw_btn(&painter, retry_r, retry.hovered(), c::GREEN());
        painter.text(retry_r.center(), egui::Align2::CENTER_CENTER,
            "↺  RETRY", FontId::monospace(11.5),
            if retry.hovered() { c::GREEN() } else { c::GREEN().linear_multiply(0.5) });

        let close_r = Rect::from_min_size(Pos2::new(start_x + btn_w + gap, btn_y), Vec2::new(btn_w, 30.0));
        let close   = ui.interact(close_r, ui.id().with("dead_close"), Sense::click());
        draw_btn(&painter, close_r, close.hovered(), c::DANGER());
        painter.text(close_r.center(), egui::Align2::CENTER_CENTER,
            "✕  CLOSE TAB", FontId::monospace(11.5),
            if close.hovered() { c::DANGER() } else { c::DANGER().linear_multiply(0.5) });

        if retry.clicked() { return Some(DeadAction::Retry); }
        if close.clicked() { return Some(DeadAction::Close); }
    } else {
        let close_r = Rect::from_min_size(Pos2::new(cx - 60.0, btn_y), Vec2::new(120.0, 30.0));
        let close   = ui.interact(close_r, ui.id().with("dead_close_local"), Sense::click());
        draw_btn(&painter, close_r, close.hovered(), c::DANGER());
        painter.text(close_r.center(), egui::Align2::CENTER_CENTER,
            "✕  CLOSE TAB", FontId::monospace(11.5),
            if close.hovered() { c::DANGER() } else { c::DANGER().linear_multiply(0.5) });

        if close.clicked() { return Some(DeadAction::Close); }
    }

    None
}

fn draw_btn(painter: &egui::Painter, r: Rect, hovered: bool, color: Color32) {
    painter.rect_filled(r, egui::Rounding::same(3.0),
        if hovered { color.linear_multiply(0.15) } else { Color32::TRANSPARENT });
    painter.rect_stroke(r, egui::Rounding::same(3.0),
        Stroke::new(1.0, if hovered { color } else { color.linear_multiply(0.5) }));
}

// ── Empty state ────────────────────────────────────────────────────────────────

/// Shown when there are no open sessions.
pub fn show_empty(icon: &egui::TextureHandle, ui: &mut egui::Ui) {
    let rect    = ui.available_rect_before_wrap();
    let painter = ui.painter_at(rect);

    // Subtle grid
    let step = 40.0;
    let grid_color = Color32::from_rgba_premultiplied(0, 60, 80, 12);
    let mut gx = rect.left() + (rect.width() % step) / 2.0;
    while gx < rect.right() {
        painter.line_segment([Pos2::new(gx, rect.top()), Pos2::new(gx, rect.bottom())],
            Stroke::new(0.4, grid_color));
        gx += step;
    }
    let mut gy = rect.top() + (rect.height() % step) / 2.0;
    while gy < rect.bottom() {
        painter.line_segment([Pos2::new(rect.left(), gy), Pos2::new(rect.right(), gy)],
            Stroke::new(0.4, grid_color));
        gy += step;
    }

    // Corner brackets
    let m = 30.0; let l = 24.0;
    let s = Stroke::new(1.5, c::CYAN().linear_multiply(0.25));
    let corners = [
        (rect.left_top()     + Vec2::splat(m),  Vec2::new(l, 0.0),  Vec2::new(0.0, l)),
        (Pos2::new(rect.right()-m, rect.top()+m), Vec2::new(-l,0.0), Vec2::new(0.0, l)),
        (Pos2::new(rect.left()+m, rect.bottom()-m), Vec2::new(l,0.0), Vec2::new(0.0,-l)),
        (rect.right_bottom() - Vec2::splat(m),  Vec2::new(-l,0.0), Vec2::new(0.0,-l)),
    ];
    for (o, dx, dy) in corners {
        painter.line_segment([o, o + dx], s);
        painter.line_segment([o, o + dy], s);
    }

    let cx = rect.center().x;
    let cy = rect.center().y - 30.0;

    // App icon centered — 80×80
    let icon_size = 80.0;
    let icon_rect = egui::Rect::from_center_size(
        Pos2::new(cx, cy - 24.0),
        Vec2::splat(icon_size),
    );
    painter.image(
        icon.id(),
        icon_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::from_rgba_premultiplied(255, 255, 255, 200),
    );

    painter.text(Pos2::new(cx, cy + 52.0), egui::Align2::CENTER_CENTER,
        "NO ACTIVE SESSIONS", FontId::monospace(13.0), c::MUTED());
    painter.text(Pos2::new(cx, cy + 74.0), egui::Align2::CENTER_CENTER,
        "select a connection  ·  press + for local shell",
        FontId::monospace(10.5), c::MUTED2().linear_multiply(1.6));

    ui.allocate_rect(rect, Sense::hover());
}
