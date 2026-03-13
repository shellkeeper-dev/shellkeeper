/// Visual theme palette.
/// Stored in `egui::Context` data each frame so it can be retrieved
/// from any widget or free function via `ui.ctx().data(|d| d.get_temp(...))`.
use egui::Color32;

#[derive(Clone, Debug)]
pub struct ThemePalette {
    pub name: &'static str,

    // Backgrounds
    pub bg:         Color32,
    pub sidebar:    Color32,
    pub panel:      Color32, // terminal bg
    pub surface:    Color32,
    pub hover:      Color32,
    pub tab_bg:     Color32,
    pub tab_active: Color32,

    // Accents
    pub cyan:       Color32,
    pub cyan_dim:   Color32, // very translucent cyan (rgba)
    pub green:      Color32,
    #[allow(dead_code)]
    pub pink:       Color32,
    pub danger:     Color32,
    pub yellow:     Color32,

    // Text
    pub text:       Color32,
    pub muted:      Color32,
    pub muted2:     Color32,

    // Borders
    pub border:     Color32,
    pub border_lit: Color32,
}

impl ThemePalette {
    pub fn cyberpunk() -> Self {
        Self {
            name:       "cyberpunk",
            bg:         Color32::from_rgb(8,   12,  18),
            sidebar:    Color32::from_rgb(10,  14,  22),
            panel:      Color32::from_rgb(6,   9,   15),
            surface:    Color32::from_rgb(14,  20,  32),
            hover:      Color32::from_rgb(14,  28,  46),
            tab_bg:     Color32::from_rgb(8,   13,  22),
            tab_active: Color32::from_rgb(12,  30,  50),
            cyan:       Color32::from_rgb(0,   255, 220),
            cyan_dim:   Color32::from_rgba_premultiplied(0, 38, 33, 38),
            green:      Color32::from_rgb(0,   255, 128),
            pink:       Color32::from_rgb(255, 60,  180),
            danger:     Color32::from_rgb(255, 60,  60),
            yellow:     Color32::from_rgb(255, 220, 0),
            text:       Color32::from_rgb(210, 230, 240),
            muted:      Color32::from_rgb(70,  100, 130),
            muted2:     Color32::from_rgb(40,  60,  85),
            border:     Color32::from_rgb(20,  35,  55),
            border_lit: Color32::from_rgb(0,   180, 160),
        }
    }

    pub fn dark() -> Self {
        Self {
            name:       "dark",
            bg:         Color32::from_rgb(13,  17,  23),
            sidebar:    Color32::from_rgb(22,  27,  34),
            panel:      Color32::from_rgb(9,   12,  17),
            surface:    Color32::from_rgb(33,  38,  45),
            hover:      Color32::from_rgb(30,  37,  48),
            tab_bg:     Color32::from_rgb(13,  17,  23),
            tab_active: Color32::from_rgb(33,  38,  45),
            cyan:       Color32::from_rgb(88,  166, 255),
            cyan_dim:   Color32::from_rgba_premultiplied(10, 25, 50, 25),
            green:      Color32::from_rgb(63,  185, 80),
            pink:       Color32::from_rgb(210, 106, 175),
            danger:     Color32::from_rgb(248, 81,  73),
            yellow:     Color32::from_rgb(230, 167, 0),
            text:       Color32::from_rgb(230, 237, 243),
            muted:      Color32::from_rgb(110, 130, 155),
            muted2:     Color32::from_rgb(60,  75,  95),
            border:     Color32::from_rgb(48,  54,  61),
            border_lit: Color32::from_rgb(88,  166, 255),
        }
    }

    pub fn dracula() -> Self {
        Self {
            name:       "dracula",
            bg:         Color32::from_rgb(40,  42,  54),
            sidebar:    Color32::from_rgb(33,  34,  44),
            panel:      Color32::from_rgb(30,  31,  41),
            surface:    Color32::from_rgb(68,  71,  90),
            hover:      Color32::from_rgb(55,  58,  72),
            tab_bg:     Color32::from_rgb(40,  42,  54),
            tab_active: Color32::from_rgb(68,  71,  90),
            cyan:       Color32::from_rgb(139, 233, 253),
            cyan_dim:   Color32::from_rgba_premultiplied(20, 55, 65, 40),
            green:      Color32::from_rgb(80,  250, 123),
            pink:       Color32::from_rgb(255, 121, 198),
            danger:     Color32::from_rgb(255, 85,  85),
            yellow:     Color32::from_rgb(241, 250, 140),
            text:       Color32::from_rgb(248, 248, 242),
            muted:      Color32::from_rgb(180, 180, 200),
            muted2:     Color32::from_rgb(100, 100, 130),
            border:     Color32::from_rgb(98,  101, 120),
            border_lit: Color32::from_rgb(139, 233, 253),
        }
    }

    pub fn light() -> Self {
        Self {
            name:       "light",
            bg:         Color32::from_rgb(248, 249, 250),
            sidebar:    Color32::from_rgb(241, 243, 245),
            panel:      Color32::from_rgb(255, 255, 255),
            surface:    Color32::from_rgb(233, 236, 239),
            hover:      Color32::from_rgb(222, 226, 230),
            tab_bg:     Color32::from_rgb(241, 243, 245),
            tab_active: Color32::from_rgb(255, 255, 255),
            cyan:       Color32::from_rgb(12,  133, 153),  // teal oscuro — legible en blanco
            cyan_dim:   Color32::from_rgba_premultiplied(12, 133, 153, 20),
            green:      Color32::from_rgb(25,  135, 84),
            pink:       Color32::from_rgb(214, 51,  132),
            danger:     Color32::from_rgb(220, 53,  69),
            yellow:     Color32::from_rgb(255, 153, 0),
            text:       Color32::from_rgb(33,  37,  41),
            muted:      Color32::from_rgb(108, 117, 125),
            muted2:     Color32::from_rgb(173, 181, 189),
            border:     Color32::from_rgb(206, 212, 218),
            border_lit: Color32::from_rgb(12,  133, 153),
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "dark"    => Self::dark(),
            "dracula" => Self::dracula(),
            "light"   => Self::light(),
            _         => Self::cyberpunk(),
        }
    }

    pub fn all() -> [ThemePalette; 4] {
        [Self::cyberpunk(), Self::dark(), Self::dracula(), Self::light()]
    }
}
