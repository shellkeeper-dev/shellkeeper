//! Terminal renderer and keyboard input handler.

use arboard::Clipboard;

use egui::{Color32, FontId, Key, Modifiers, Pos2, Rect, Sense, Stroke, Vec2};

use crate::{colors::vt100_to_egui, pty::PtySession};
use super::{c, default_bg};


// ── State ─────────────────────────────────────────────────────────────────────

pub struct TerminalState {
    pub focused:    bool,
    pub auto_focus: bool,
    pub last_cols:  u16,
    pub last_rows:  u16,
    /// Selection anchor (row, col) set on mouse-press
    pub sel_start:   Option<(u16, u16)>,
    /// Selection end (row, col) updated while dragging
    pub sel_end:     Option<(u16, u16)>,
    /// Set to true by context-menu "Copy" click; consumed next frame to actually copy
    pub pending_copy: bool,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            focused: false, auto_focus: true,
            last_cols: 80, last_rows: 24,
            sel_start: None, sel_end: None,
            pending_copy: false,
        }
    }
}

// ── Events ─────────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct TerminalEvents {
    pub focused: bool,
    pub cols:    u16,
    pub rows:    u16,
}

// ── Render ────────────────────────────────────────────────────────────────────

/// Render the active terminal session.
///
/// Returns the cell dimensions for PTY resize tracking.
/// Does **not** handle overlays — caller checks `session.is_alive()` / `has_output()`.
pub fn show(
    state:      &mut TerminalState,
    sessions:   &mut Vec<PtySession>,
    active_tab: usize,
    font_size:  f32,
    ui:         &mut egui::Ui,
) -> TerminalEvents {
    let mut events = TerminalEvents::default();

    let avail = ui.available_rect_before_wrap();

    // ── Pending copy from context-menu (executed at frame start, before any clear) ──
    if state.pending_copy {
        state.pending_copy = false;
        if let (Some(start), Some(end)) = (state.sel_start, state.sel_end) {
            if let Some(session) = sessions.get(active_tab) {
                let parser = session.parser.lock().unwrap();
                let screen = parser.screen();
                let text   = extract_selection(&screen, start, end, state.last_cols);
                drop(parser);
                if !text.is_empty() {
                    if let Ok(mut cb) = Clipboard::new() { let _ = cb.set_text(text); }
                }
            }
        }
        state.sel_start = None;
        state.sel_end   = None;
    }

    // Auto-focus when tab opens or switches — no click required.
    // Must be applied BEFORE the click check so the same-frame sidebar click
    // doesn't immediately steal it back.
    if state.auto_focus {
        state.focused    = true;
        state.auto_focus = false;
    }

    // Click/drag inside terminal — focus + text selection
    let term_resp = ui.interact(avail, ui.id().with("term_area"), Sense::click_and_drag());
    if term_resp.clicked() {
        state.focused = true;
        // term_resp.clicked() fires for both buttons in egui 0.29.
        // Only clear selection when it was the LEFT (primary) button.
        if ui.input(|i| i.pointer.primary_clicked()) {
            state.sel_start = None;
            state.sel_end   = None;
        }
    }

    // Border
    let border_col = if state.focused { c::BORDER_LIT() } else { c::BORDER() };
    ui.painter().rect_stroke(
        avail, 0.0,
        Stroke::new(if state.focused { 1.5 } else { 1.0 }, border_col),
    );

    // Measure monospace cell
    let font_id   = FontId::monospace(font_size);
    let cell_size = ui.fonts(|f| {
        f.layout_no_wrap("W".into(), font_id.clone(), Color32::WHITE).size()
    });
    let cw = cell_size.x;
    let ch = cell_size.y;
    let inner = avail.shrink(2.0);
    let cols  = ((inner.width()  / cw).floor() as u16).max(10);
    let rows  = ((inner.height() / ch).floor() as u16).max(4);

    // Resize PTY if needed
    if cols != state.last_cols || rows != state.last_rows {
        state.last_cols = cols;
        state.last_rows = rows;
        for s in sessions.iter_mut() { s.resize(cols, rows); }
    }
    events.cols = cols;
    events.rows = rows;

    // ── Render cells ─────────────────────────────────────────────────────────
    let session = &sessions[active_tab];
    let parser  = session.parser.lock().unwrap();
    let screen  = parser.screen();
    let painter = ui.painter_at(inner);
    painter.rect_filled(inner, 0.0, default_bg());

    // Pass 1 — non-default backgrounds
    for row in 0..rows {
        for col in 0..cols {
            if let Some(cell) = screen.cell(row, col) {
                let (fg_raw, bg_raw) = cell_colors(cell);
                let (_, bg) = if cell.inverse() { (bg_raw, fg_raw) } else { (fg_raw, bg_raw) };
                if bg != default_bg() {
                    let px = inner.left() + col as f32 * cw;
                    let py = inner.top()  + row as f32 * ch;
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(px, py), Vec2::new(cw, ch)),
                        0.0, bg,
                    );
                }
            }
        }
    }

    // Pass 2 — text via LayoutJob (one galley per row, runs merged by colour)
    for row in 0..rows {
        let py = inner.top() + row as f32 * ch;
        let mut job         = egui::text::LayoutJob::default();
        let mut run_text    = String::with_capacity(cols as usize);
        let mut run_fg      = Color32::TRANSPARENT;
        let mut run_ul      = false;
        let mut has_content = false;

        let flush = |job: &mut egui::text::LayoutJob,
                     text: &mut String,
                     fg: Color32,
                     ul: bool,
                     fid: &FontId| {
            if text.is_empty() { return; }
            let mut fmt = egui::text::TextFormat {
                font_id: fid.clone(), color: fg, ..Default::default()
            };
            if ul { fmt.underline = Stroke::new(1.0, fg); }
            job.append(text, 0.0, fmt);
            text.clear();
        };

        for col in 0..cols {
            let (glyph, fg) = if let Some(cell) = screen.cell(row, col) {
                let (fg_raw, bg_raw) = cell_colors(cell);
                let (fg, _) = if cell.inverse() { (bg_raw, fg_raw) } else { (fg_raw, bg_raw) };
                let g = cell.contents();
                let g: &str = if g.is_empty() { " " } else { &g };
                (g.to_string(), fg)
            } else {
                (" ".into(), Color32::TRANSPARENT)
            };

            if fg != run_fg || run_ul {
                flush(&mut job, &mut run_text, run_fg, run_ul, &font_id);
                run_fg = fg; run_ul = false;
            }
            run_text.push_str(&glyph);
            if !glyph.trim().is_empty() { has_content = true; }
        }
        flush(&mut job, &mut run_text, run_fg, run_ul, &font_id);

        if has_content || !job.sections.is_empty() {
            let galley = ui.fonts(|f| f.layout_job(job));
            painter.galley(Pos2::new(inner.left(), py), galley, Color32::WHITE);
        }
    }

    // Cursor
    let (cur_row, cur_col) = screen.cursor_position();
    if (cur_row as u16) < rows && (cur_col as u16) < cols {
        let px = inner.left() + cur_col as f32 * cw;
        let py = inner.top()  + cur_row as f32 * ch;
        let cr = Rect::from_min_size(Pos2::new(px, py), Vec2::new(cw, ch));
        if state.focused {
            painter.rect_filled(cr, 0.0, Color32::from_rgba_unmultiplied(255, 255, 255, 180));
            let ch_str = screen.cell(cur_row, cur_col)
                .map(|c| c.contents().to_string())
                .unwrap_or_default();
            if !ch_str.is_empty() && ch_str != " " {
                painter.text(Pos2::new(px, py), egui::Align2::LEFT_TOP, &ch_str, font_id, default_bg());
            }
        } else {
            painter.rect_stroke(cr, 0.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 100)));
        }
    }
    // ── Selection: track mouse drag → (row,col) coords ───────────────────
    let pixel_to_cell = |pos: egui::Pos2| -> (u16, u16) {
        let col = ((pos.x - inner.left()) / cw).floor().clamp(0.0, (cols - 1) as f32) as u16;
        let row = ((pos.y - inner.top())  / ch).floor().clamp(0.0, (rows - 1) as f32) as u16;
        (row, col)
    };

    // Only track selection drags initiated with the PRIMARY (left) button.
    let primary_down = ui.input(|i| i.pointer.primary_down());
    if term_resp.drag_started() && primary_down {
        if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
            // New drag = new selection: clear the old one first
            state.sel_start = Some(pixel_to_cell(pos));
            state.sel_end   = None;
        }
    }
    if term_resp.dragged() && primary_down {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            state.sel_end = Some(pixel_to_cell(pos));
        }
    }

    // Render selection highlight
    if let (Some(start), Some(end)) = (state.sel_start, state.sel_end) {
        let (r1, c1, r2, c2) = sel_ordered(start, end);
        let sel_col = c::CYAN().linear_multiply(0.25);
        for row in r1..=r2 {
            let col_from = if row == r1 { c1 } else { 0 };
            let col_to   = if row == r2 { c2 } else { cols - 1 };
            let x1 = inner.left() + col_from as f32 * cw;
            let x2 = inner.left() + (col_to + 1) as f32 * cw;
            let y  = inner.top()  + row as f32 * ch;
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(x1, y), Pos2::new(x2, y + ch)),
                0.0, sel_col,
            );
        }
    }

    // Ctrl+Shift+C → copy selected text to clipboard
    let ctrl_shift_c = ui.ctx().input_mut(|i| {
        if let Some(pos) = i.events.iter().position(|e| matches!(
            e,
            egui::Event::Key { key: egui::Key::C, pressed: true, modifiers, .. }
            if modifiers.ctrl && modifiers.shift
        )) { i.events.remove(pos); true } else { false }
    });
    if ctrl_shift_c || (state.sel_start.is_some() && state.sel_end.is_some() && {
        // Also copy on right-click "Copy" — handled below via context menu flag
        false
    }) {
        if let (Some(start), Some(end)) = (state.sel_start, state.sel_end) {
            let text = extract_selection(&screen, start, end, cols);
            if !text.is_empty() {
                if let Ok(mut cb) = Clipboard::new() { let _ = cb.set_text(text); }
            }
        }
    }

    drop(parser);

    // ── Input ──────────────────────────────────────────────────────────────
    if state.focused {
        // Ctrl+V → paste from clipboard via arboard (reliable on Wayland/X11
        // even before the window has full compositor focus)
        let ctrl_v = ui.ctx().input_mut(|i| {
            if let Some(pos) = i.events.iter().position(|e| matches!(
                e,
                egui::Event::Key { key: egui::Key::V, pressed: true, modifiers, .. }
                if modifiers.ctrl && !modifiers.alt
            )) {
                i.events.remove(pos);
                true
            } else {
                false
            }
        });
        if ctrl_v {
            if let Ok(mut cb) = Clipboard::new() {
                if let Ok(text) = cb.get_text() {
                    if let Some(s) = sessions.get_mut(active_tab) { s.write_input(text.as_bytes()); }
                }
            }
        }

        // Physically remove Ctrl+C from the event queue so egui never sees it
        // as a "copy to clipboard" shortcut. We send 0x03 (SIGINT) ourselves.
        let ctrl_c = ui.ctx().input_mut(|i| {
            if let Some(pos) = i.events.iter().position(|e| matches!(
                e,
                egui::Event::Key { key: egui::Key::C, pressed: true, modifiers, .. }
                if modifiers.ctrl && !modifiers.alt
            )) {
                i.events.remove(pos);
                true
            } else {
                // egui 0.29 on Linux converts Ctrl+C → Event::Copy before we see Key
                let copy_pos = i.events.iter().position(|e| matches!(e, egui::Event::Copy));
                if let Some(pos) = copy_pos {
                    i.events.remove(pos);
                    true
                } else {
                    false
                }
            }
        });
        if ctrl_c {
            if let Some(s) = sessions.get_mut(active_tab) { s.write_input(&[0x03]); }
        }

        // Remaining keyboard + Paste events (Ctrl+V, middle-mouse on X11)
        let bytes = collect_input(ui.ctx());
        if !bytes.is_empty() {
            if let Some(s) = sessions.get_mut(active_tab) { s.write_input(&bytes); }
        }
    }

    // Right-click context menu — common terminal actions + paste
    let ctx_resp = ui.interact(avail, ui.id().with("term_ctx"), egui::Sense::hover());
    ctx_resp.context_menu(|ui| {
        let has_sel = state.sel_start.is_some() && state.sel_end.is_some();
        if ui.add_enabled(has_sel, egui::Button::new("Copy  (Ctrl+Shift+C)"))
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            // Set flag — actual copy runs at the TOP of the NEXT frame
            // so sel_start/sel_end are still valid (not cleared by click handling)
            state.pending_copy = true;
            ui.close_menu();
        }
        if ui.button("Paste  (Ctrl+V)").clicked() {
            // Read clipboard directly via arboard — works even from context menu
            if let Ok(mut cb) = Clipboard::new() {
                if let Ok(text) = cb.get_text() {
                    if let Some(s) = sessions.get_mut(active_tab) { s.write_input(text.as_bytes()); }
                }
            }
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Ctrl+C  — interrupt").clicked() {
            if let Some(s) = sessions.get_mut(active_tab) { s.write_input(&[0x03]); }
            ui.close_menu();
        }
        if ui.button("Ctrl+L  — clear screen").clicked() {
            if let Some(s) = sessions.get_mut(active_tab) { s.write_input(&[0x0c]); }
            ui.close_menu();
        }
        if ui.button("Ctrl+D  — EOF / logout").clicked() {
            if let Some(s) = sessions.get_mut(active_tab) { s.write_input(&[0x04]); }
            ui.close_menu();
        }
    });

    events.focused = state.focused;
    events
}

// ── Input ─────────────────────────────────────────────────────────────────────

pub fn collect_input(ctx: &egui::Context) -> Vec<u8> {
    let mut out = Vec::new();
    ctx.input(|i| {
        for event in &i.events {
            match event {
                egui::Event::Text(text) => {
                    for ch in text.chars() {
                        if ch != '\n' && ch != '\r' && ch != '\t' {
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                    }
                }
                // Paste event: fired by Ctrl+V AND middle-mouse button on X11/Wayland
                egui::Event::Paste(text) => {
                    out.extend_from_slice(text.as_bytes());
                }
                // egui converts Ctrl+C → Event::Copy on some Linux setups.
                // When the terminal is focused there is no selection to copy,
                // so this is always SIGINT.
                egui::Event::Copy => {
                    out.push(0x03);
                }
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    if let Some(seq) = key_sequence(key, modifiers) {
                        out.extend_from_slice(&seq);
                    }
                }
                _ => {}
            }
        }
    });
    out
}

fn key_sequence(key: &Key, m: &Modifiers) -> Option<Vec<u8>> {
    if m.ctrl && !m.alt {
        let b: Option<u8> = match key {
            Key::A => Some(0x01), Key::B => Some(0x02), Key::C => Some(0x03),
            Key::D => Some(0x04), Key::E => Some(0x05), Key::F => Some(0x06),
            Key::G => Some(0x07), Key::H => Some(0x08), Key::K => Some(0x0b),
            Key::L => Some(0x0c), Key::N => Some(0x0e), Key::O => Some(0x0f),
            Key::P => Some(0x10), Key::Q => Some(0x11), Key::R => Some(0x12),
            Key::S => Some(0x13), Key::T => Some(0x14), Key::U => Some(0x15),
            // Key::V omitted — Ctrl+V is clipboard paste, handled via Event::Paste
            Key::W => Some(0x17), Key::X => Some(0x18),
            Key::Y => Some(0x19), Key::Z => Some(0x1a),
            _ => None,
        };
        if let Some(b) = b { return Some(vec![b]); }
    }

    Some(match key {
        Key::Enter     => vec![b'\r'],
        Key::Backspace => vec![0x7f],
        Key::Tab       => vec![b'\t'],
        Key::Escape    => vec![0x1b],
        Key::Delete    => b"\x1b[3~".to_vec(),
        Key::Home      => b"\x1b[H".to_vec(),
        Key::End       => b"\x1b[F".to_vec(),
        Key::PageUp    => b"\x1b[5~".to_vec(),
        Key::PageDown  => b"\x1b[6~".to_vec(),
        Key::Insert    => b"\x1b[2~".to_vec(),
        Key::ArrowUp    => if m.shift { b"\x1b[1;2A".to_vec() } else { b"\x1b[A".to_vec() },
        Key::ArrowDown  => if m.shift { b"\x1b[1;2B".to_vec() } else { b"\x1b[B".to_vec() },
        Key::ArrowRight => if m.shift { b"\x1b[1;2C".to_vec() } else { b"\x1b[C".to_vec() },
        Key::ArrowLeft  => if m.shift { b"\x1b[1;2D".to_vec() } else { b"\x1b[D".to_vec() },
        Key::F1  => b"\x1bOP".to_vec(),  Key::F2  => b"\x1bOQ".to_vec(),
        Key::F3  => b"\x1bOR".to_vec(),  Key::F4  => b"\x1bOS".to_vec(),
        Key::F5  => b"\x1b[15~".to_vec(), Key::F6 => b"\x1b[17~".to_vec(),
        Key::F7  => b"\x1b[18~".to_vec(), Key::F8 => b"\x1b[19~".to_vec(),
        Key::F9  => b"\x1b[20~".to_vec(), Key::F10 => b"\x1b[21~".to_vec(),
        Key::F11 => b"\x1b[23~".to_vec(), Key::F12 => b"\x1b[24~".to_vec(),
        _ => return None,
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

#[inline]
pub fn cell_colors(cell: &vt100::Cell) -> (Color32, Color32) {
    let fg = vt100_to_egui(cell.fgcolor(), true,  cell.bold());
    let bg = vt100_to_egui(cell.bgcolor(), false, cell.bold());
    (fg, bg)
}

// ── Selection helpers ─────────────────────────────────────────────────────────

/// Normalize selection so start ≤ end (top-left to bottom-right).
fn sel_ordered(a: (u16, u16), b: (u16, u16)) -> (u16, u16, u16, u16) {
    let (r1, c1, r2, c2) = (a.0, a.1, b.0, b.1);
    if r1 < r2 || (r1 == r2 && c1 <= c2) {
        (r1, c1, r2, c2)
    } else {
        (r2, c2, r1, c1)
    }
}

/// Extract selected text from the vt100 screen.
fn extract_selection(screen: &vt100::Screen, start: (u16, u16), end: (u16, u16), _cols: u16) -> String {
    let (r1, c1, r2, c2) = sel_ordered(start, end);
    let mut out = String::new();
    for row in r1..=r2 {
        let col_from = if row == r1 { c1 } else { 0 };
        let col_to   = if row == r2 { c2 } else { screen.size().1 - 1 };
        let mut line = String::new();
        for col in col_from..=col_to {
            if let Some(cell) = screen.cell(row, col) {
                let c = cell.contents();
                line.push_str(if c.is_empty() { " " } else { &c });
            }
        }
        // Trim trailing spaces from each line
        let trimmed = line.trim_end().to_string();
        if !out.is_empty() { out.push('\n'); }
        out.push_str(&trimmed);
    }
    out.trim_end().to_string()
}
