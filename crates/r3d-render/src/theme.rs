//! Colours used by the renderers. The UI design system fills this in, so
//! the canvas matches the light or dark theme of the app.

/// sRGB colour, `[r, g, b, a]` in 0..1.
pub type Rgba = [f32; 4];

/// Number of colours used to tell pattern pieces apart.
pub const PIECE_COLORS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub board_background: Rgba,
    pub grid_minor: Rgba,
    pub grid_major: Rgba,
    /// Fill colours of pattern pieces, cycled in project order; a mirror
    /// copy uses its source's colour so pairs read at a glance.
    pub pieces: [Rgba; PIECE_COLORS],
    pub panel_line: Rgba,
    pub selection: Rgba,
    pub sky_top: Rgba,
    pub sky_bottom: Rgba,
    pub ground: Rgba,
    /// The avatar (a neutral dress-form tone).
    pub avatar: Rgba,
}

/// `#RRGGBB` as a colour.
pub(crate) fn parse_hex(s: &str) -> Option<Rgba> {
    let digits = s.strip_prefix('#').filter(|d| d.len() == 6)?;
    u32::from_str_radix(digits, 16).ok().map(hex)
}

pub(crate) const fn hex(rgb: u32) -> Rgba {
    [
        ((rgb >> 16) & 0xff) as f32 / 255.0,
        ((rgb >> 8) & 0xff) as f32 / 255.0,
        (rgb & 0xff) as f32 / 255.0,
        1.0,
    ]
}

impl Theme {
    pub const LIGHT: Theme = Theme {
        board_background: hex(0xF7F6F3),
        grid_minor: hex(0xECEAE5),
        grid_major: hex(0xDCD9D2),
        pieces: [hex(0xFFFFFF); PIECE_COLORS],
        panel_line: hex(0x2B2A28),
        selection: hex(0x5B5BD6),
        sky_top: hex(0xF4F3F0),
        sky_bottom: hex(0xDEDBD4),
        ground: hex(0xE7E4DE),
        avatar: hex(0xD9DCE3),
    };

    pub const DARK: Theme = Theme {
        board_background: hex(0x18181B),
        grid_minor: hex(0x202024),
        grid_major: hex(0x2C2C31),
        pieces: [hex(0x232328); PIECE_COLORS],
        panel_line: hex(0xE6E4E0),
        selection: hex(0x8B8BF5),
        sky_top: hex(0x2A2A30),
        sky_bottom: hex(0x141417),
        ground: hex(0x1E1E22),
        avatar: hex(0x6B7080),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_colours() {
        assert_eq!(parse_hex("#FF0000"), Some([1.0, 0.0, 0.0, 1.0]));
        assert_eq!(parse_hex("FF0000"), None);
        assert_eq!(parse_hex("#GG0000"), None);
    }
}
