/// Convert a `vt100::Color` to an egui `Color32`.
///
/// `bold` is used to brighten indexed colours 0-7 (the classic terminal behaviour).
pub fn vt100_to_egui(color: vt100::Color, is_fg: bool, bold: bool) -> egui::Color32 {
    use egui::Color32;
    use vt100::Color::*;
    match color {
        Default => {
            if is_fg {
                Color32::from_rgb(204, 204, 204)
            } else {
                Color32::from_rgb(18, 18, 18)
            }
        }
        Idx(i)       => indexed(i, bold),
        Rgb(r, g, b) => Color32::from_rgb(r, g, b),
    }
}


// Standard 16-colour palette (same as most modern terminals)
const NORMAL: [(u8, u8, u8); 8] = [
    (0,   0,   0),
    (170, 0,   0),
    (0,   170, 0),
    (170, 85,  0),
    (0,   0,   170),
    (170, 0,   170),
    (0,   170, 170),
    (170, 170, 170),
];
const BRIGHT: [(u8, u8, u8); 8] = [
    (85,  85,  85),
    (255, 85,  85),
    (85,  255, 85),
    (255, 255, 85),
    (85,  85,  255),
    (255, 85,  255),
    (85,  255, 255),
    (255, 255, 255),
];

fn indexed(idx: u8, bold: bool) -> egui::Color32 {
    let (r, g, b) = if idx < 8 {
        if bold { BRIGHT[idx as usize] } else { NORMAL[idx as usize] }
    } else if idx < 16 {
        BRIGHT[(idx - 8) as usize]
    } else if idx < 232 {
        // 6×6×6 colour cube
        let i = idx - 16;
        let bi = i % 6;
        let gi = (i / 6) % 6;
        let ri = i / 36;
        let c  = |v: u8| if v == 0 { 0u8 } else { 55 + v * 40 };
        (c(ri), c(gi), c(bi))
    } else {
        // 24-step greyscale
        let v = 8 + (idx - 232) * 10;
        (v, v, v)
    };
    egui::Color32::from_rgb(r, g, b)
}
