//! Sidebar panel: connection list, search, accordion sections, footer.

use chrono::Utc;
use egui::{Align, Color32, FontId, RichText, Sense, Stroke, Vec2};

use crate::{
    config::AppConfig,
    models::SshConnection,
    pty::PtySession,
    theme::ThemePalette,
};

use super::{
    c,
    widgets::{accordion_header, ConnectionItem},
};

const SIDEBAR_W: f32 = 270.0;

// ── State ─────────────────────────────────────────────────────────────────────

/// Persistent UI state owned by the sidebar.
pub struct SidebarState {
    pub search:        String,
    pub sec_favorites: bool,
    pub sec_recent:    bool,
    pub sec_all:       bool,
    /// Currently selected namespace filter. Empty string = show all.
    pub namespace: String,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            search:        String::new(),
            sec_favorites: true,
            sec_recent:    false,
            sec_all:       true,
            namespace:     String::new(),
        }
    }
}

// ── Events ─────────────────────────────────────────────────────────────────────

/// Actions requested by the sidebar, processed by the orchestrator.
#[derive(Default)]
pub struct SidebarEvents {
    /// Open (or switch to) an existing connection.
    pub open:         Option<usize>,
    /// Always open a fresh tab for this connection.
    pub open_new_tab: Option<usize>,
    /// Open the add-connection dialog.
    pub add_new:      bool,
    /// Open the edit dialog for this connection index.
    pub edit:         Option<usize>,
    /// Delete this connection.
    pub delete:       Option<usize>,
    /// Toggle favourite status.
    pub toggle_fav:   Option<usize>,
    /// Navigate to the settings view.
    pub open_settings: bool,
    /// Theme change requested (theme name).
    pub theme_change: Option<String>,
}

// ── Main render ───────────────────────────────────────────────────────────────

/// Render the sidebar and return all user actions for this frame.
pub fn show(
    state:      &mut SidebarState,
    config:     &mut AppConfig,
    sessions:   &[PtySession],
    active_tab: usize,
    _palette:   &ThemePalette,
    icon:       &egui::TextureHandle,
    ui:         &mut egui::Ui,
) -> SidebarEvents {
    let mut events = SidebarEvents::default();

    ui.visuals_mut().override_text_color = Some(c::TEXT());

    // Collect unique groups for namespace selector
    let groups: Vec<String> = {
        let gs: Vec<String> = config.connections.iter()
            .map(|c| c.group.trim().to_string())
            .filter(|g| !g.is_empty())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        gs
    };

    render_header(state, &groups, icon, ui, &mut events);
    render_search(state, ui);

    // Compute live-session sets once, reused in every conn_list call
    let live_ids: std::collections::HashSet<String> = sessions.iter()
        .filter(|s| s.is_alive())
        .filter_map(|s| s.connection.as_ref().map(|c| c.id.clone()))
        .collect();
    let focused_id: Option<String> = sessions
        .get(active_tab)
        .and_then(|s| s.connection.as_ref())
        .map(|c| c.id.clone());

    let indices = filtered_indices(config, &state.search, &state.namespace);
    let q       = state.search.to_lowercase();
    let searching = !q.is_empty();
    let filtering = !state.namespace.is_empty();

    egui::ScrollArea::vertical()
        .id_salt("sidebar_scroll")
        .show(ui, |ui| {
            if filtering {
                // Namespace active → flat filtered list (no accordion sections)
                render_ns_filtered(state, config, &indices, &live_ids, &focused_id, ui, &mut events);
            } else {
                render_favourites(state, config, &indices, &live_ids, &focused_id, searching, ui, &mut events);
                render_recent(state, config, &indices, &live_ids, &focused_id, searching, ui, &mut events);
                if groups.is_empty() {
                    render_all(state, config, &indices, &live_ids, &focused_id, searching, ui, &mut events);
                } else {
                    render_by_group(state, config, &indices, &live_ids, &focused_id, &groups, searching, ui, &mut events);
                }
            }
        });

    render_footer(config, _palette, ui, &mut events);

    events
}

// ── Private helpers ────────────────────────────────────────────────────────────

fn render_header(
    state:  &mut SidebarState,
    groups: &[String],
    icon:   &egui::TextureHandle,
    ui:     &mut egui::Ui,
    events: &mut SidebarEvents,
) {
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.add(egui::Image::new(egui::load::SizedTexture::from_handle(icon))
            .fit_to_exact_size(egui::Vec2::splat(22.0)));
        ui.add_space(6.0);
        ui.label(RichText::new("ShellKeeper").size(17.0).color(c::TEXT()).strong());
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(10.0);
            // ⚙ Settings
            let gear = ui.add(
                egui::Button::new(RichText::new("⚙").size(14.0).color(c::MUTED()))
                    .frame(false),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("Settings");
            if gear.clicked() { events.open_settings = true; }

            // Namespace selector — only shown when groups exist
            if !groups.is_empty() {
                ui.add_space(4.0);
                let ns_active  = !state.namespace.is_empty();
                let ns_display = if state.namespace.is_empty() { "All".to_string() } else { state.namespace.clone() };
                let label_col  = if ns_active { c::CYAN() } else { c::MUTED() };
                let fill_col   = if ns_active { c::CYAN().linear_multiply(0.08) } else { Color32::TRANSPARENT };
                let stroke_col = if ns_active { c::CYAN().linear_multiply(0.45) } else { c::BORDER() };

                let popup_id = egui::Id::new("ns_popup");
                let btn_resp = ui.add(
                    egui::Button::new(
                        RichText::new(format!("▾ {}", ns_display))
                            .size(10.5).color(label_col).monospace()
                    )
                    .fill(fill_col)
                    .stroke(Stroke::new(0.8, stroke_col))
                    .rounding(egui::Rounding::same(3.0))
                ).on_hover_cursor(egui::CursorIcon::PointingHand);

                if btn_resp.clicked() {
                    ui.memory_mut(|m| m.toggle_popup(popup_id));
                }

                egui::popup_below_widget(
                    ui, popup_id, &btn_resp,
                    egui::PopupCloseBehavior::CloseOnClickOutside,
                    |ui| {
                        ui.set_min_width(130.0);
                        ui.visuals_mut().override_text_color = Some(c::TEXT());
                        ui.visuals_mut().widgets.inactive.bg_fill  = c::PANEL();
                        ui.visuals_mut().widgets.hovered.bg_fill   = c::HOVER();

                        let all_sel = state.namespace.is_empty();
                        if ui.add(
                            egui::Button::new(RichText::new("All").size(11.0)
                                .color(if all_sel { c::CYAN() } else { c::TEXT() }))
                                .fill(if all_sel { c::CYAN().linear_multiply(0.1) } else { Color32::TRANSPARENT })
                                .frame(false)
                        ).clicked() {
                            state.namespace = String::new();
                            ui.memory_mut(|m| m.close_popup());
                        }
                        ui.separator();
                        for g in groups {
                            let active = &state.namespace == g;
                            if ui.add(
                                egui::Button::new(RichText::new(g).size(11.0)
                                    .color(if active { c::CYAN() } else { c::TEXT() }))
                                    .fill(if active { c::CYAN().linear_multiply(0.1) } else { Color32::TRANSPARENT })
                                    .frame(false)
                            ).clicked() {
                                state.namespace = g.clone();
                                ui.memory_mut(|m| m.close_popup());
                            }
                        }
                    }
                );
            }
        });
    });
    ui.add_space(4.0);
    let div = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [div.left_top(), egui::Pos2::new(div.left_top().x + SIDEBAR_W, div.left_top().y)],
        Stroke::new(1.0, c::CYAN().linear_multiply(0.3)),
    );
    ui.add_space(10.0);
}

fn render_search(state: &mut SidebarState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        ui.label(RichText::new("⌕").size(15.0).color(c::MUTED()));
        ui.add_space(2.0);
        ui.add(
            egui::TextEdit::singleline(&mut state.search)
                .hint_text(
                    RichText::new("filter connections…")
                        .color(c::MUTED2().linear_multiply(1.5))
                        .italics(),
                )
                .desired_width(SIDEBAR_W - 52.0)
                .font(FontId::monospace(12.5))
                .text_color(c::TEXT()),
        );
    });
    ui.add_space(8.0);
}

fn render_favourites(
    state:      &mut SidebarState,
    config:     &AppConfig,
    indices:    &[usize],
    live_ids:   &std::collections::HashSet<String>,
    focused_id: &Option<String>,
    searching:  bool,
    ui:         &mut egui::Ui,
    events:     &mut SidebarEvents,
) {
    let favs: Vec<usize> = indices.iter().copied()
        .filter(|&i| config.connections[i].favorite)
        .collect();
    if favs.is_empty() { return; }

    let expanded = searching || state.sec_favorites;
    if accordion_header(ui, "⭐  FAVOURITES", favs.len(), expanded) {
        state.sec_favorites = !state.sec_favorites;
    }
    if expanded {
        conn_list(ui, config, &favs, live_ids, focused_id, events);
    }
}

fn render_recent(
    state:      &mut SidebarState,
    config:     &AppConfig,
    indices:    &[usize],
    live_ids:   &std::collections::HashSet<String>,
    focused_id: &Option<String>,
    searching:  bool,
    ui:         &mut egui::Ui,
    events:     &mut SidebarEvents,
) {
    if searching { return; }

    let mut recents: Vec<usize> = indices.iter().copied()
        .filter(|&i| !config.connections[i].favorite && config.connections[i].last_used.is_some())
        .collect();
    recents.sort_by(|&a, &b| config.connections[b].last_used.cmp(&config.connections[a].last_used));
    recents.truncate(5);
    if recents.is_empty() { return; }

    let expanded = state.sec_recent;
    if accordion_header(ui, "🕐  RECENT", recents.len(), expanded) {
        state.sec_recent = !state.sec_recent;
    }
    if expanded {
        conn_list(ui, config, &recents, live_ids, focused_id, events);
    }
}

fn render_all(
    state:      &mut SidebarState,
    config:     &AppConfig,
    indices:    &[usize],
    live_ids:   &std::collections::HashSet<String>,
    focused_id: &Option<String>,
    searching:  bool,
    ui:         &mut egui::Ui,
    events:     &mut SidebarEvents,
) {
    let expanded = searching || state.sec_all;
    if accordion_header(ui, "🖥  ALL CONNECTIONS", indices.len(), expanded) {
        state.sec_all = !state.sec_all;
    }
    if expanded {
        conn_list(ui, config, indices, live_ids, focused_id, events);
    }
}

fn render_footer(
    _config:  &AppConfig,
    _palette: &ThemePalette,
    ui:       &mut egui::Ui,
    events:   &mut SidebarEvents,
) {
    ui.with_layout(egui::Layout::bottom_up(Align::Center), |ui| {
        ui.add_space(10.0);

        // ── NEW CONNECTION button ─────────────────────────────────────────────
        let btn_w = SIDEBAR_W - 24.0;
        let (btn_r, btn_resp) = ui.allocate_exact_size(Vec2::new(btn_w, 34.0), Sense::click());
        ui.advance_cursor_after_rect(btn_r);
        let hot = btn_resp.hovered();
        ui.painter().rect_filled(btn_r, egui::Rounding::same(3.0),
            if hot { c::GREEN().linear_multiply(0.12) } else { Color32::TRANSPARENT });
        ui.painter().rect_stroke(btn_r, egui::Rounding::same(3.0),
            Stroke::new(1.0, if hot { c::GREEN() } else { c::GREEN().linear_multiply(0.35) }));
        ui.painter().text(
            btn_r.center(), egui::Align2::CENTER_CENTER,
            "+ NEW CONNECTION", FontId::monospace(11.5),
            if hot { c::GREEN() } else { c::GREEN().linear_multiply(0.7) },
        );
        if btn_resp.clicked() { events.add_new = true; }
        ui.add_space(10.0);
    });
}

/// Render a flat list of connections (by config index). Fills in `events`.
fn conn_list(
    ui:         &mut egui::Ui,
    config:     &AppConfig,
    indices:    &[usize],
    live_ids:   &std::collections::HashSet<String>,
    focused_id: &Option<String>,
    events:     &mut SidebarEvents,
) {
    for &i in indices {
        let conn = &config.connections[i];
        let live    = live_ids.contains(&conn.id);
        let focused = focused_id.as_deref() == Some(conn.id.as_str());

        let resp = ui.add(ConnectionItem {
            name: &conn.name, subtitle: &conn.subtitle(),
            favorite: conn.favorite, live, focused,
        });

        if resp.clicked()        { events.open = Some(i); }
        if resp.middle_clicked() { events.edit = Some(i); }

        resp.context_menu(|ui| {
            if ui.button("Connect").clicked() {
                events.open = Some(i); ui.close_menu();
            }
            if ui.button("Open in new tab").clicked() {
                events.open_new_tab = Some(i); ui.close_menu();
            }
            if ui.button("Edit").clicked() {
                events.edit = Some(i); ui.close_menu();
            }
            let fav_label = if conn.favorite { "Remove favourite" } else { "Add favourite" };
            if ui.button(fav_label).clicked() {
                events.toggle_fav = Some(i); ui.close_menu();
            }
            ui.separator();
            if ui.button(RichText::new("Delete").color(c::DANGER())).clicked() {
                events.delete = Some(i); ui.close_menu();
            }
        });
    }
}

/// Indices of connections matching the current search query and namespace filter.
pub fn filtered_indices(config: &AppConfig, search: &str, namespace: &str) -> Vec<usize> {
    let q = search.to_lowercase();
    config.connections.iter().enumerate()
        .filter(|(_, c)| {
            // Namespace filter
            (namespace.is_empty() || c.group.trim() == namespace)
            // Search filter
            && (q.is_empty()
                || c.name.to_lowercase().contains(&q)
                || c.host.to_lowercase().contains(&q)
                || c.username.to_lowercase().contains(&q)
                || c.group.to_lowercase().contains(&q))
        })
        .map(|(i, _)| i)
        .collect()
}

/// Flat list of connections for when a namespace filter is active.
fn render_ns_filtered(
    _state:     &mut SidebarState,
    config:     &AppConfig,
    indices:    &[usize],
    live_ids:   &std::collections::HashSet<String>,
    focused_id: &Option<String>,
    ui:         &mut egui::Ui,
    events:     &mut SidebarEvents,
) {
    if indices.is_empty() {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("No connections").size(12.0).color(c::MUTED()));
        });
        return;
    }
    ui.add_space(4.0);
    conn_list(ui, config, indices, live_ids, focused_id, events);
}

/// Grouped accordion view — one section per group + ungrouped at the bottom.
fn render_by_group(
    _state:     &mut SidebarState,
    config:     &AppConfig,
    indices:    &[usize],
    live_ids:   &std::collections::HashSet<String>,
    focused_id: &Option<String>,
    groups:     &[String],
    searching:  bool,
    ui:         &mut egui::Ui,
    events:     &mut SidebarEvents,
) {
    // Per-group sections
    for group in groups {
        let group_indices: Vec<usize> = indices.iter().copied()
            .filter(|&i| config.connections[i].group.trim() == group.as_str())
            .collect();
        if group_indices.is_empty() && !searching { continue; }

        let open_key = egui::Id::new(("ns_section", group.as_str()));
        let mut open = ui.ctx().data(|d| d.get_temp::<bool>(open_key).unwrap_or(true));
        let changed  = accordion_header(ui, group, group_indices.len(), open);
        if changed { open = !open; ui.ctx().data_mut(|d| d.insert_temp(open_key, open)); }
        if open {
            conn_list(ui, config, &group_indices, live_ids, focused_id, events);
        }
    }

    // Ungrouped section
    let ungrouped: Vec<usize> = indices.iter().copied()
        .filter(|&i| config.connections[i].group.trim().is_empty())
        .collect();
    if !ungrouped.is_empty() {
        let open_key = egui::Id::new("ns_section_ungrouped");
        let mut open = ui.ctx().data(|d| d.get_temp::<bool>(open_key).unwrap_or(true));
        let changed  = accordion_header(ui, "General", ungrouped.len(), open);
        if changed { open = !open; ui.ctx().data_mut(|d| d.insert_temp(open_key, open)); }
        if open {
            conn_list(ui, config, &ungrouped, live_ids, focused_id, events);
        }
    }
}

/// Update config.last_used and return a cloned connection ready to open.
pub fn stamp_last_used(config: &mut AppConfig, i: usize) -> SshConnection {
    config.connections[i].last_used = Some(Utc::now());
    config.connections[i].clone()
}
