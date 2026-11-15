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
    pub cloth: Rgba,
    pub cloth_sheen: Rgba,
}

const fn hex(rgb: u32) -> Rgba {
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
        cloth: hex(0x8A9BB8),
        cloth_sheen: hex(0xE8ECF5),
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
        cloth: hex(0x7C8DAD),
        cloth_sheen: hex(0xC9D2E6),
    };
}
