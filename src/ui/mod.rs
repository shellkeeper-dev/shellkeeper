//! UI subsystem: palette management, theme application, and all UI modules.

pub mod dialog;
pub mod overlays;
pub mod settings;
pub mod sidebar;
pub mod tabs;
pub mod terminal;
pub mod widgets;

use egui::{Color32, Stroke, Vec2};

use crate::theme::ThemePalette;

// ── Active palette ─────────────────────────────────────────────────────────────
// Set once per frame from SshedApp::update(). Thread-local is safe because
// egui calls update() on a single thread.
thread_local! {
    static ACTIVE: std::cell::RefCell<ThemePalette> =
        std::cell::RefCell::new(ThemePalette::cyberpunk());
}

/// Push the palette that all UI helpers will read this frame.
pub fn set_palette(p: ThemePalette) {
    ACTIVE.with(|c| *c.borrow_mut() = p);
}

/// Clone of the currently active palette.
pub fn pal() -> ThemePalette {
    ACTIVE.with(|c| c.borrow().clone())
}

/// Terminal default background — matches palette panel colour.
pub fn default_bg() -> Color32 {
    pal().panel
}

// ── Palette colour accessors ───────────────────────────────────────────────────
// Grouped in a sub-module so the `#[allow]` only affects this block.
#[allow(non_snake_case)]
pub mod c {
    use egui::Color32;
    use super::pal;

    #[allow(dead_code)]
    pub fn BG()         -> Color32 { pal().bg         }
    pub fn SIDEBAR()    -> Color32 { pal().sidebar     }
    pub fn PANEL()      -> Color32 { pal().panel       }
    pub fn SURFACE()    -> Color32 { pal().surface     }
    pub fn CYAN()       -> Color32 { pal().cyan        }
    pub fn CYAN_DIM()   -> Color32 { pal().cyan_dim    }
    pub fn GREEN()      -> Color32 { pal().green       }
    pub fn DANGER()     -> Color32 { pal().danger      }
    pub fn YELLOW()     -> Color32 { pal().yellow      }
    pub fn TEXT()       -> Color32 { pal().text        }
    pub fn MUTED()      -> Color32 { pal().muted       }
    pub fn MUTED2()     -> Color32 { pal().muted2      }
    pub fn TAB_BG()     -> Color32 { pal().tab_bg      }
    pub fn TAB_ACTIVE() -> Color32 { pal().tab_active  }
    pub fn BORDER()     -> Color32 { pal().border      }
    pub fn BORDER_LIT() -> Color32 { pal().border_lit  }
    pub fn HOVER()      -> Color32 { pal().hover       }
}

// ── egui theme application ────────────────────────────────────────────────────

/// Apply the active palette to egui's visual style.
/// Call at the top of every frame (after `set_palette`).
pub fn apply_theme(ctx: &egui::Context) {
    let p = pal();
    // Use egui's light base for the light theme so text/widget defaults are correct.
    let mut v = if p.name == "light" { egui::Visuals::light() } else { egui::Visuals::dark() };

    // Surfaces
    v.window_fill      = p.surface;
    v.panel_fill       = p.bg;
    v.extreme_bg_color = p.panel;
    v.faint_bg_color   = p.bg.linear_multiply(1.2);

    // Window chrome
    v.window_stroke  = Stroke::new(1.0, p.cyan.linear_multiply(0.3));
    v.window_rounding = egui::Rounding::same(4.0);
    v.window_shadow  = egui::epaint::Shadow {
        blur:   20.0,
        spread: 4.0,
        offset: Vec2::new(0.0, 4.0),
        color:  p.cyan_dim,
    };

    // Widgets — use palette for all widget surface colours
    let bg_idle   = p.surface.linear_multiply(0.8);
    let bg_hover  = p.hover;
    let bg_active = p.cyan.linear_multiply(0.15);

    v.widgets.noninteractive.bg_fill   = bg_idle;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);

    v.widgets.inactive.bg_fill   = bg_idle;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);

    v.widgets.hovered.bg_fill   = bg_hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.cyan.linear_multiply(0.5));
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, p.cyan);

    v.widgets.active.bg_fill   = bg_active;
    v.widgets.active.bg_stroke = Stroke::new(1.0, p.cyan);
    v.widgets.active.fg_stroke = Stroke::new(2.0, p.cyan);

    // Selection
    v.selection.bg_fill = p.cyan.linear_multiply(0.25);
    v.selection.stroke  = Stroke::new(1.0, p.cyan);

    // Misc
    v.popup_shadow  = v.window_shadow;
    v.menu_rounding = egui::Rounding::same(4.0);

    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        s.spacing.item_spacing   = Vec2::new(6.0, 4.0);
        s.spacing.window_margin  = egui::Margin::same(12.0);
        s.spacing.button_padding = Vec2::new(10.0, 5.0);
    });
}
