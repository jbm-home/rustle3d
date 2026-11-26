//! Environment maps in the Radiance picture format (`.hdr`, RGBE): a
//! latitude-longitude image of the light around a scene, read for the
//! photo renderer (spec 35).
//!
//! The format is Greg Ward's (Ward 1991, "Real Pixels", Graphics Gems
//! II; Radiance's `color.c` documents the encoding): a text header ending
//! with a blank line, a resolution line, then scanlines of four bytes per
//! pixel (red, green and blue mantissas sharing the exponent in the
//! fourth byte), flat, in the old run-length encoding, or in the newer
//! one where each of the four components is run-length encoded apart.

use r3d_core::message::{Localizable, Message};

/// Widest picture accepted (pixels); bigger ones are refused before
/// decoding, so a hostile file cannot ask for gigabytes.
pub const MAX_WIDTH: u32 = 8192;
/// Tallest picture accepted (pixels).
pub const MAX_HEIGHT: u32 = 8192;

/// A decoded picture: `width × height` RGBE texels, top row first.
#[derive(Clone, Debug, PartialEq)]
pub struct Hdr {
    pub width: u32,
    pub height: u32,
    pub rgbe: Vec<[u8; 4]>,
}

/// Why a picture could not be read.
#[derive(Debug, thiserror::Error)]
pub enum HdrError {
    #[error("not a Radiance .hdr picture")]
    Format,
    #[error("cannot read the picture: {detail}")]
    Bad { detail: String },
    #[error("the picture is too large ({width} x {height} pixels)")]
    TooLarge { width: u32, height: u32 },
}

impl Localizable for HdrError {
    fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::Format => Message::new("hdr.format", text),
            Self::Bad { detail } => Message::new("hdr.bad", text).arg("detail", detail),
            Self::TooLarge { width, height } => Message::new("hdr.too_large", text)
                .arg("width", width)
                .arg("height", height),
        }
    }
}

fn bad(detail: &str) -> HdrError {
    HdrError::Bad {
        detail: detail.into(),
    }
}

/// The linear RGB a texel stands for (Ward's decoding: the mantissa's
/// middle, scaled by 2^(exponent − 136)).
pub fn texel_rgb(t: [u8; 4]) -> [f32; 3] {
    if t[3] == 0 {
        return [0.0; 3];
    }
    let scale = 2f32.powi(i32::from(t[3]) - 136);
    [0, 1, 2].map(|i| (f32::from(t[i]) + 0.5) * scale)
}

/// Decode a `.hdr` file.
pub fn decode(bytes: &[u8]) -> Result<Hdr, HdrError> {
    if !(bytes.starts_with(b"#?RADIANCE") || bytes.starts_with(b"#?RGBE")) {
        return Err(HdrError::Format);
    }
    let mut at = 0;
    let mut line = || -> Result<&[u8], HdrError> {
        let rest = bytes.get(at..).unwrap_or_default();
        let end = rest
            .iter()
            .position(|&b| b == b'\n')
            .ok_or_else(|| bad("the header does not end"))?;
        at += end + 1;
        Ok(rest.get(..end).unwrap_or_default())
    };
    // Header lines up to a blank one.
    loop {
        let l = line()?;
        if l.is_empty() {
            break;
        }
        if let Some(format) = l.strip_prefix(b"FORMAT=")
            && format.trim_ascii() != b"32-bit_rle_rgbe"
        {
            return Err(bad("only RGB pictures are read (not XYZ)"));
        }
    }
    let resolution = String::from_utf8_lossy(line()?).into_owned();
    let words: Vec<&str> = resolution.split_whitespace().collect();
    let (flip, height, width) = match words.as_slice() {
        ["-Y", h, "+X", w] => (false, *h, *w),
        ["+Y", h, "+X", w] => (true, *h, *w),
        _ => return Err(bad("only pictures stored by rows from the left are read")),
    };
    let size = |s: &str| s.parse::<u32>().map_err(|_| bad("bad picture size"));
    let (width, height) = (size(width)?, size(height)?);
    if width == 0 || height == 0 {
        return Err(bad("empty picture"));
    }
    if width > MAX_WIDTH || height > MAX_HEIGHT {
        return Err(HdrError::TooLarge { width, height });
    }
    let data = bytes.get(at..).unwrap_or_default();
    let w = width as usize;
    let mut rgbe = Vec::with_capacity(w * height as usize);
    let mut pos = 0;
    let mut row = vec![[0u8; 4]; w];
    for _ in 0..height {
        pos = scanline(data, pos, &mut row)?;
        rgbe.extend_from_slice(&row);
    }
    if flip {
        let rows: Vec<&[[u8; 4]]> = rgbe.chunks(w).rev().collect();
        rgbe = rows.concat();
    }
    Ok(Hdr {
        width,
        height,
        rgbe,
    })
}

/// Read one scanline into `row` from `data` at `pos`; returns where the
/// next one starts.
fn scanline(data: &[u8], mut pos: usize, row: &mut [[u8; 4]]) -> Result<usize, HdrError> {
    let short = || bad("the picture is cut short");
    let w = row.len();
    let head = data.get(pos..pos + 4).ok_or_else(short)?;
    let new_rle = (8..0x8000).contains(&w) && head[0] == 2 && head[1] == 2 && head[2] & 0x80 == 0;
    if new_rle {
        if (usize::from(head[2]) << 8 | usize::from(head[3])) != w {
            return Err(bad("a scanline has the wrong length"));
        }
        pos += 4;
        // Each component in turn, as runs and literal spans.
        for c in 0..4 {
            let mut x = 0;
            while x < w {
                let count = usize::from(*data.get(pos).ok_or_else(short)?);
                pos += 1;
                if count > 128 {
                    let n = count - 128;
                    let value = *data.get(pos).ok_or_else(short)?;
                    pos += 1;
                    let span = row
                        .get_mut(x..x + n)
                        .ok_or_else(|| bad("a run overflows"))?;
                    span.iter_mut().for_each(|t| t[c] = value);
                    x += n;
                } else {
                    if count == 0 {
                        return Err(bad("an empty run"));
                    }
                    let values = data.get(pos..pos + count).ok_or_else(short)?;
                    let span = row
                        .get_mut(x..x + count)
                        .ok_or_else(|| bad("a run overflows"))?;
                    span.iter_mut().zip(values).for_each(|(t, v)| t[c] = *v);
                    pos += count;
                    x += count;
                }
            }
        }
        return Ok(pos);
    }
    // Flat texels, where (1, 1, 1, n) repeats the previous texel (old
    // run-length encoding; consecutive repeats shift the count by 8 bits).
    let mut x = 0;
    let mut shift = 0;
    while x < w {
        let t = data.get(pos..pos + 4).ok_or_else(short)?;
        pos += 4;
        if t[0] == 1 && t[1] == 1 && t[2] == 1 {
            let previous = *x
                .checked_sub(1)
                .and_then(|p| row.get(p))
                .ok_or_else(|| bad("a repeat without a texel before it"))?;
            let n = usize::from(t[3])
                .checked_shl(shift)
                .ok_or_else(|| bad("a repeat overflows"))?;
            let span = row
                .get_mut(x..x + n)
                .ok_or_else(|| bad("a repeat overflows"))?;
            span.fill(previous);
            x += n;
            shift += 8;
        } else {
            if let Some(slot) = row.get_mut(x) {
                *slot = [t[0], t[1], t[2], t[3]];
            }
            x += 1;
            shift = 0;
        }
    }
    Ok(pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture written flat (no run-length encoding).
    fn flat(width: u32, height: u32, texels: &[[u8; 4]]) -> Vec<u8> {
        let mut out =
            format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {height} +X {width}\n").into_bytes();
        texels.iter().for_each(|t| out.extend_from_slice(t));
        out
    }

    #[test]
    fn reads_flat_texels() {
        let texels: Vec<[u8; 4]> = (0..6).map(|i| [i, 2 * i, 3 * i, 128]).collect();
        let hdr = decode(&flat(3, 2, &texels)).unwrap();
        assert_eq!((hdr.width, hdr.height), (3, 2));
        assert_eq!(hdr.rgbe, texels);
    }

    #[test]
    fn decodes_ward_texels() {
        // 2^(128 - 136) · (128 + 0.5) = 0.5019…
        let [r, g, b] = texel_rgb([128, 64, 0, 128]);
        assert!((r - 128.5 / 256.0).abs() < 1e-6);
        assert!((g - 64.5 / 256.0).abs() < 1e-6);
        assert!((b - 0.5 / 256.0).abs() < 1e-6);
        assert_eq!(texel_rgb([200, 1, 1, 0]), [0.0; 3]);
    }

    #[test]
    fn reads_new_run_length_encoding() {
        let w = 10usize;
        let mut out = b"#?RGBE\n\n-Y 1 +X 10\n".to_vec();
        out.extend([2, 2, 0, w as u8]);
        // Red: a run of 10; green: 10 literals; blue: runs of 4 and 6;
        // exponent: a run.
        out.extend([128 + 10, 7]);
        out.push(10);
        out.extend(0..10u8);
        out.extend([128 + 4, 1, 128 + 6, 2]);
        out.extend([128 + 10, 129]);
        let hdr = decode(&out).unwrap();
        for (x, t) in hdr.rgbe.iter().enumerate() {
            assert_eq!(*t, [7, x as u8, if x < 4 { 1 } else { 2 }, 129]);
        }
    }

    #[test]
    fn reads_old_repeats_and_bottom_up_pictures() {
        let mut out = b"#?RADIANCE\n\n+Y 2 +X 4\n".to_vec();
        // Bottom row first: one texel repeated 3 times, then a top row.
        out.extend([9, 9, 9, 130, 1, 1, 1, 3]);
        out.extend([5, 5, 5, 130].repeat(4));
        let hdr = decode(&out).unwrap();
        assert_eq!(hdr.rgbe[..4], [[5, 5, 5, 130]; 4]);
        assert_eq!(hdr.rgbe[4..], [[9, 9, 9, 130]; 4]);
    }

    #[test]
    fn refuses_bad_files() {
        assert!(matches!(decode(b"P6 1 1"), Err(HdrError::Format)));
        assert!(decode(b"#?RADIANCE\n\n-Y 2 +X 2\n\x01").is_err());
        assert!(decode(b"#?RADIANCE\nFORMAT=32-bit_rle_xyze\n\n-Y 1 +X 1\n1234").is_err());
        assert!(decode(b"#?RADIANCE\n\n+X 1 -Y 1\n1234").is_err());
        assert!(matches!(
            decode(b"#?RADIANCE\n\n-Y 99999 +X 99999\n"),
            Err(HdrError::TooLarge { .. })
        ));
        // A repeat with nothing before it, and a run past the row's end.
        assert!(decode(&flat(2, 1, &[[1, 1, 1, 2], [0, 0, 0, 0]])).is_err());
        let mut over = b"#?RGBE\n\n-Y 1 +X 8\n".to_vec();
        over.extend([2, 2, 0, 8, 128 + 9, 1]);
        assert!(decode(&over).is_err());
    }
}
