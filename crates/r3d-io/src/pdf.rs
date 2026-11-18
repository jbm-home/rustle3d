//! PDF export at 1:1 scale: one large page (plotter) or tiled pages
//! (home printing) with overlap, page labels and assembly guides.

use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, Str};
use r3d_geom::DVec2;

use crate::drawing::{Drawing, Stroke, approx_text_width};

const PT_PER_MM: f64 = 72.0 / 25.4;
/// Unprintable border kept on every tiled page (mm).
pub const PAGE_MARGIN_MM: f64 = 10.0;
/// Overlap between neighbouring tiles, used to align the pages (mm).
pub const TILE_OVERLAP_MM: f64 = 10.0;
const FONT: Name<'static> = Name(b"F1");

/// Output page layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paper {
    A4,
    Letter,
    /// One page as large as the pattern (plotters, print shops).
    Single,
}

impl Paper {
    /// Portrait page size in mm (`None` for `Single`).
    fn size_mm(self) -> Option<DVec2> {
        match self {
            Paper::A4 => Some(DVec2::new(210.0, 297.0)),
            Paper::Letter => Some(DVec2::new(215.9, 279.4)),
            Paper::Single => None,
        }
    }
}

/// One page: the board window it shows and its label.
#[derive(Clone, Debug, PartialEq)]
pub struct Tile {
    /// Bottom-left corner of the window, in board mm.
    pub origin: DVec2,
    pub label: String,
}

/// Page size and windows for a drawing. Tiles go left to right, top to
/// bottom; rows are lettered and columns numbered (`A1`, `A2`, … `B1`).
pub fn layout(d: &Drawing, paper: Paper) -> (DVec2, DVec2, Vec<Tile>) {
    let size = d.bounds.max - d.bounds.min;
    let Some(page) = paper.size_mm() else {
        let page = size + DVec2::splat(2.0 * PAGE_MARGIN_MM);
        let tile = Tile {
            origin: d.bounds.min,
            label: "A1".into(),
        };
        return (page, size, vec![tile]);
    };
    let window = page - DVec2::splat(2.0 * PAGE_MARGIN_MM);
    let step = window - DVec2::splat(TILE_OVERLAP_MM);
    let count = |total: f64, w: f64, s: f64| -> usize {
        if total <= w {
            1
        } else {
            ((total - w) / s).ceil() as usize + 1
        }
    };
    let cols = count(size.x, window.x, step.x);
    let rows = count(size.y, window.y, step.y);
    let mut tiles = Vec::with_capacity(rows * cols);
    for r in 0..rows {
        for c in 0..cols {
            let top = d.bounds.max.y - r as f64 * step.y;
            tiles.push(Tile {
                origin: DVec2::new(d.bounds.min.x + c as f64 * step.x, top - window.y),
                label: format!("{}{}", row_letter(r), c + 1),
            });
        }
    }
    (page, window, tiles)
}

fn row_letter(r: usize) -> String {
    // A..Z, then AA, AB… (more than enough rows for any garment).
    let letters = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut n = r;
    let mut s = Vec::new();
    loop {
        s.push(letters[n % 26]);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s.reverse();
    String::from_utf8_lossy(&s).into_owned()
}

/// Text for the built-in Helvetica (WinAnsi): Latin-1 maps directly, other
/// characters become `?`.
fn win_ansi(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| match u32::from(c) {
            0x20..=0x7E | 0xA0..=0xFF => c as u8,
            _ => b'?',
        })
        .collect()
}

fn line_style(c: &mut Content, stroke: Stroke) {
    let (width, gray, dash): (f32, f32, &[f32]) = match stroke {
        Stroke::Cut | Stroke::Notch => (0.5, 0.0, &[]),
        Stroke::Sew => (0.3, 0.0, &[4.0, 2.0]),
        Stroke::Internal => (0.3, 0.25, &[6.0, 2.0, 1.0, 2.0]),
        Stroke::Grain => (0.3, 0.25, &[]),
        Stroke::Mark => (0.3, 0.0, &[]),
        Stroke::Size(_) => (0.4, 0.0, &[]),
    };
    c.set_line_width(width);
    if let Stroke::Size(k) = stroke {
        let rgb = Stroke::size_color(k);
        let channel = |shift: u32| ((rgb >> shift) & 0xFF) as f32 / 255.0;
        c.set_stroke_rgb(channel(16), channel(8), channel(0));
    } else {
        c.set_stroke_gray(gray);
    }
    c.set_dash_pattern(dash.iter().copied(), 0.0);
}

/// Draw the drawing in board millimetres (the caller sets the transform).
fn draw(c: &mut Content, d: &Drawing) {
    for line in &d.lines {
        let Some((first, rest)) = line.points.split_first() else {
            continue;
        };
        line_style(c, line.stroke);
        c.move_to(first.x as f32, first.y as f32);
        for p in rest {
            c.line_to(p.x as f32, p.y as f32);
        }
        if line.closed {
            c.close_path();
        }
        c.stroke();
    }
    c.set_fill_gray(0.0);
    for label in &d.labels {
        let bytes = win_ansi(&label.text);
        let width = approx_text_width(&label.text, label.size_mm);
        c.begin_text();
        c.set_font(FONT, label.size_mm as f32);
        c.next_line(
            (label.pos.x - width / 2.0) as f32,
            (label.pos.y - label.size_mm * 0.35) as f32,
        );
        c.show(Str(&bytes));
        c.end_text();
    }
}

/// Render the drawing as a PDF document.
pub fn to_pdf(d: &Drawing, paper: Paper) -> Vec<u8> {
    let (page_mm, window, tiles) = layout(d, paper);
    let k = PT_PER_MM as f32;
    let m = PAGE_MARGIN_MM;
    let mut pdf = Pdf::new();
    let catalog = Ref::new(1);
    let tree = Ref::new(2);
    let font = Ref::new(3);
    let page_ids: Vec<Ref> = (0..tiles.len())
        .map(|i| Ref::new(4 + 2 * i as i32))
        .collect();
    pdf.catalog(catalog).pages(tree);
    pdf.pages(tree)
        .kids(page_ids.iter().copied())
        .count(tiles.len() as i32);
    pdf.type1_font(font)
        .base_font(Name(b"Helvetica"))
        .encoding_predefined(Name(b"WinAnsiEncoding"));

    for (i, tile) in tiles.iter().enumerate() {
        let page_id = page_ids[i];
        let content_id = Ref::new(page_id.get() + 1);
        let mut page = pdf.page(page_id);
        page.media_box(Rect::new(
            0.0,
            0.0,
            (page_mm.x * PT_PER_MM) as f32,
            (page_mm.y * PT_PER_MM) as f32,
        ));
        page.parent(tree);
        page.contents(content_id);
        page.resources().fonts().pair(FONT, font);
        page.finish();

        let mut c = Content::new();
        // Page decorations in page millimetres.
        c.save_state();
        c.transform([k, 0.0, 0.0, k, 0.0, 0.0]);
        if paper != Paper::Single {
            c.set_line_width(0.2);
            c.set_stroke_gray(0.6);
            c.set_dash_pattern([2.0, 2.0], 0.0);
            // Overlap guides: align these with the neighbouring page.
            let (x0, y0, x1, y1) = (m, m, m + window.x, m + window.y);
            let o = TILE_OVERLAP_MM;
            for (ax, ay, bx, by) in [
                (x0 + o, y0, x0 + o, y1),
                (x1 - o, y0, x1 - o, y1),
                (x0, y0 + o, x1, y0 + o),
                (x0, y1 - o, x1, y1 - o),
            ] {
                c.move_to(ax as f32, ay as f32);
                c.line_to(bx as f32, by as f32);
            }
            c.stroke();
            c.set_dash_pattern([], 0.0);
            c.rect(x0 as f32, y0 as f32, window.x as f32, window.y as f32);
            c.stroke();
            let label = format!("{}  ({}/{})", tile.label, i + 1, tiles.len());
            c.set_fill_gray(0.3);
            c.begin_text();
            c.set_font(FONT, 4.0);
            c.next_line(m as f32, (m + window.y + 3.0) as f32);
            c.show(Str(&win_ansi(&label)));
            c.end_text();
        }
        c.restore_state();

        // Pattern, clipped to the window, in board millimetres.
        c.save_state();
        c.transform([
            k,
            0.0,
            0.0,
            k,
            (k as f64 * (m - tile.origin.x)) as f32,
            (k as f64 * (m - tile.origin.y)) as f32,
        ]);
        c.rect(
            tile.origin.x as f32,
            tile.origin.y as f32,
            window.x as f32,
            window.y as f32,
        );
        c.clip_nonzero();
        c.end_path();
        draw(&mut c, d);
        c.restore_state();
        pdf.stream(content_id, &c.finish());
    }
    pdf.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_letters() {
        assert_eq!(row_letter(0), "A");
        assert_eq!(row_letter(25), "Z");
        assert_eq!(row_letter(26), "AA");
    }

    #[test]
    fn win_ansi_keeps_latin1() {
        assert_eq!(win_ansi("Dos é"), b"Dos \xe9".to_vec());
        assert_eq!(win_ansi("袖"), b"?".to_vec());
    }
}
