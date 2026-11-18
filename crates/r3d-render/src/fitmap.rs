//! The colour scale of fit maps (spec 24): viridis, a sequential map
//! whose lightness rises steadily from dark blue-violet to yellow, so it
//! reads in grey and for the common colour vision deficiencies (van der
//! Walt & Smith, matplotlib's default colour map, 2015, released under
//! CC0; Nuñez, Anderton & Renslow, "Optimizing colormaps with
//! consideration for color vision deficiency", PLoS ONE 2018). Nine
//! stops sampled from it, linear in between.
//!
//! A level runs from 0 (loose, dark) to 1 (tight, yellow); what a level
//! means is the map's business (`r3d_commands::fit::FitMap`).

use crate::theme::Rgba;

/// Viridis at 0, 1/8, …, 1 (sRGB).
pub const STOPS: [u32; 9] = [
    0x440154, 0x472D7B, 0x3B528B, 0x2C728E, 0x21918C, 0x28AE80, 0x5EC962, 0xADDC30, 0xFDE725,
];

fn rgb(hex: u32) -> [f32; 3] {
    [16, 8, 0].map(|shift| ((hex >> shift) & 0xFF) as f32 / 255.0)
}

/// The colour of `level` (0 loose … 1 tight), opaque sRGB.
pub fn color(level: f32) -> Rgba {
    let t = if level.is_finite() {
        level.clamp(0.0, 1.0)
    } else {
        0.0
    } * (STOPS.len() - 1) as f32;
    let i = (t.floor() as usize).min(STOPS.len() - 2);
    let f = t - i as f32;
    let (a, b) = (
        rgb(STOPS.get(i).copied().unwrap_or_default()),
        rgb(STOPS.get(i + 1).copied().unwrap_or_default()),
    );
    [
        a[0] + (b[0] - a[0]) * f,
        a[1] + (b[1] - a[1]) * f,
        a[2] + (b[2] - a[2]) * f,
        1.0,
    ]
}

/// Paint the scale as a bar on RGBA pixels (row after row): at the right
/// edge, tight at the top, with a light frame. For headless renders,
/// whose values are given beside the image.
pub fn paint_legend(pixels: &mut [u8], width: u32, height: u32) {
    let (w, h) = (width as usize, height as usize);
    let bar_w = (w / 40).clamp(6, 24);
    let bar_h = h * 2 / 5;
    let margin = (w / 40).clamp(6, 32);
    let (x0, y0) = (w.saturating_sub(margin + bar_w), (h - bar_h) / 2);
    let frame = 2;
    for y in y0.saturating_sub(frame)..(y0 + bar_h + frame).min(h) {
        for x in x0.saturating_sub(frame)..(x0 + bar_w + frame).min(w) {
            let inside = (y0..y0 + bar_h).contains(&y) && (x0..x0 + bar_w).contains(&x);
            let c = if inside {
                let level = 1.0 - (y - y0) as f32 / (bar_h.max(2) - 1) as f32;
                color(level).map(|v| (v * 255.0).round() as u8)
            } else {
                [255, 255, 255, 255]
            };
            if let Some(px) = pixels.get_mut((y * w + x) * 4..(y * w + x) * 4 + 4) {
                px.copy_from_slice(&c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(c: Rgba) -> f32 {
        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    }

    #[test]
    fn the_scale_gets_lighter_from_loose_to_tight() {
        let mut last = -1.0;
        for k in 0..=64 {
            let l = luminance(color(k as f32 / 64.0));
            assert!(l > last, "level {k}");
            last = l;
        }
        assert_eq!(color(-1.0), color(0.0));
        assert_eq!(color(f32::NAN), color(0.0));
        assert_eq!(color(2.0), color(1.0));
        let yellow = color(1.0);
        assert!(yellow[0] > 0.9 && yellow[1] > 0.85 && yellow[2] < 0.2);
    }

    #[test]
    fn the_legend_is_drawn_at_the_right_edge() {
        let (w, h) = (200u32, 100u32);
        let mut px = vec![0u8; (w * h * 4) as usize];
        paint_legend(&mut px, w, h);
        let at = |x: usize, y: usize| &px[(y * w as usize + x) * 4..][..4];
        assert_eq!(at(10, 50), [0, 0, 0, 0]);
        // Tight (yellow) above loose (dark).
        let (top, bottom) = (at(w as usize - 10, 32), at(w as usize - 10, 68));
        assert!(top[1] > bottom[1], "{top:?} {bottom:?}");
    }
}
