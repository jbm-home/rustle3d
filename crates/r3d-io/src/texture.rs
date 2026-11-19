//! Print images: PNG and JPEG files decoded to [`Texture`] pixels, and
//! textures written back as PNG (built-in prints in the exports).

use std::io::Cursor;

use r3d_core::Texture;
use r3d_core::message::{Localizable, Message};

/// Largest image accepted (pixels a side); bigger ones are refused before
/// decoding, so a hostile file cannot ask for gigabytes.
pub const MAX_SIDE: u32 = 16384;

/// Why an image could not be read.
#[derive(Debug, thiserror::Error)]
pub enum TextureError {
    #[error("not a PNG or JPEG image")]
    Format,
    #[error("cannot read the image: {detail}")]
    Bad { detail: String },
    #[error("the image is too large ({width} x {height} pixels)")]
    TooLarge { width: u32, height: u32 },
}

impl Localizable for TextureError {
    fn message(&self) -> Message {
        let text = self.to_string();
        match self {
            Self::Format => Message::new("texture.format", text),
            Self::Bad { detail } => Message::new("texture.bad", text).arg("detail", detail),
            Self::TooLarge { width, height } => Message::new("texture.too_large", text)
                .arg("width", width)
                .arg("height", height),
        }
    }
}

/// The image's type, from its first bytes: `image/png` or `image/jpeg`.
pub fn mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else {
        None
    }
}

/// Decode a PNG or JPEG file into RGBA pixels (transparency is kept in
/// the alpha channel; prints ignore it).
pub fn decode(bytes: &[u8]) -> Result<Texture, TextureError> {
    match mime(bytes) {
        Some("image/png") => decode_png(bytes),
        Some(_) => decode_jpeg(bytes),
        None => Err(TextureError::Format),
    }
}

fn check(width: u32, height: u32) -> Result<(), TextureError> {
    if width == 0 || height == 0 {
        return Err(TextureError::Bad {
            detail: "empty image".into(),
        });
    }
    if width > MAX_SIDE || height > MAX_SIDE {
        return Err(TextureError::TooLarge { width, height });
    }
    Ok(())
}

fn decode_png(bytes: &[u8]) -> Result<Texture, TextureError> {
    let bad = |e: png::DecodingError| TextureError::Bad {
        detail: e.to_string(),
    };
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(bad)?;
    let (w, h) = reader.info().size();
    check(w, h)?;
    let size = reader.output_buffer_size().ok_or(TextureError::TooLarge {
        width: w,
        height: h,
    })?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(bad)?;
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => {
            return Err(TextureError::Bad {
                detail: "indexed colours were not expanded".into(),
            });
        }
    };
    let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
    for row in buf.chunks(info.line_size.max(1)).take(h as usize) {
        for px in row.chunks(channels).take(w as usize) {
            let get = |i: usize| px.get(i).copied().unwrap_or(0);
            rgba.extend(match channels {
                1 => [get(0), get(0), get(0), 255],
                2 => [get(0), get(0), get(0), get(1)],
                3 => [get(0), get(1), get(2), 255],
                _ => [get(0), get(1), get(2), get(3)],
            });
        }
    }
    finish(w, h, rgba)
}

fn finish(width: u32, height: u32, rgba: Vec<u8>) -> Result<Texture, TextureError> {
    if rgba.len() != width as usize * height as usize * 4 {
        return Err(TextureError::Bad {
            detail: "truncated image data".into(),
        });
    }
    Ok(Texture {
        width,
        height,
        rgba,
    })
}

fn decode_jpeg(bytes: &[u8]) -> Result<Texture, TextureError> {
    use zune_jpeg::zune_core::colorspace::ColorSpace;
    use zune_jpeg::zune_core::options::DecoderOptions;
    let options = DecoderOptions::default()
        .jpeg_set_out_colorspace(ColorSpace::RGBA)
        .set_max_width(MAX_SIDE as usize)
        .set_max_height(MAX_SIDE as usize);
    // The decoder is third-party code fed with a user's file: a panic in
    // it must not take the app down.
    let decoded = std::panic::catch_unwind(|| {
        let mut decoder = zune_jpeg::JpegDecoder::new_with_options(bytes, options);
        let pixels = decoder.decode();
        let info = decoder.info();
        let space = decoder.get_output_colorspace();
        (pixels, info, space)
    })
    .map_err(|_| TextureError::Bad {
        detail: "the JPEG decoder failed".into(),
    })?;
    let (pixels, info, space) = decoded;
    let pixels = pixels.map_err(|e| TextureError::Bad {
        detail: format!("{e:?}"),
    })?;
    let info = info.ok_or(TextureError::Bad {
        detail: "no image header".into(),
    })?;
    let (w, h) = (u32::from(info.width), u32::from(info.height));
    check(w, h)?;
    // Greyscale files may come out with one channel whatever was asked.
    let rgba = match space.map(|s| s.num_components()) {
        Some(4) | None => pixels,
        Some(n) => pixels
            .chunks(n.max(1))
            .flat_map(|p| {
                let get = |i: usize| p.get(i).copied().unwrap_or(0);
                if n >= 3 {
                    [get(0), get(1), get(2), 255]
                } else {
                    [get(0), get(0), get(0), 255]
                }
            })
            .collect(),
    };
    finish(w, h, rgba)
}

/// The texture as an 8-bit RGBA PNG.
pub fn encode_png(texture: &Texture) -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, texture.width, texture.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let written = enc
        .write_header()
        .and_then(|mut w| w.write_image_data(&texture.rgba).and_then(|()| w.finish()));
    if written.is_err() {
        // Only a size mismatch fails, which `Texture` never has when made
        // by this crate or r3d-core; an empty file is then the safe answer.
        return Vec::new();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3d_core::print::{BuiltinPrint, builtin};

    #[test]
    fn png_round_trips() {
        let t = builtin(BuiltinPrint::Gingham, 0x336699);
        let png = encode_png(&t);
        assert_eq!(mime(&png), Some("image/png"));
        assert_eq!(decode(&png).unwrap(), t);
    }

    #[test]
    fn other_files_are_refused() {
        assert!(matches!(decode(b"GIF89a"), Err(TextureError::Format)));
        assert!(decode(&[0xFF, 0xD8, 0xFF, 0x00, 0x01]).is_err());
        assert!(decode(&[0x89, b'P', b'N', b'G', 0, 0]).is_err());
    }

    /// A 2 × 1 baseline JPEG (red, blue), made once with a standard
    /// encoder at quality 100.
    #[test]
    fn jpeg_decodes() {
        let jpeg = include_bytes!("../tests/data/two.jpg");
        let t = decode(jpeg).unwrap();
        assert_eq!((t.width, t.height), (2, 1));
        assert!(t.rgba[0] > 200 && t.rgba[2] < 80, "{:?}", &t.rgba[..4]);
        assert!(t.rgba[6] > 200 && t.rgba[4] < 80, "{:?}", &t.rgba[4..8]);
    }
}
