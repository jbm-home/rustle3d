//! The tech pack: a garment's production file as a PDF
//! (`docs/specs/23-production.md`). The command layer decides what goes in
//! it (sections of tables, pictures, swatches, text and the pattern); this
//! lays it out on landscape pages, breaking tables across pages, and writes
//! it. Every word comes from the caller, so the file can be in any
//! language.

use std::io::Cursor;

use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref, Str};
use r3d_core::{Localizable, Message};
use r3d_geom::DVec2;

use crate::drawing::{Drawing, approx_text_width};
use crate::pdf::{draw, win_ansi};

const PT_PER_MM: f64 = 72.0 / 25.4;
const MARGIN_MM: f64 = 12.0;
/// Band at the top of every page: the title and the subtitle.
const HEADER_MM: f64 = 12.0;
/// Band at the bottom: the page number.
const FOOTER_MM: f64 = 8.0;
const TITLE_MM: f64 = 5.0;
const HEADING_MM: f64 = 4.2;
const SUBHEADING_MM: f64 = 3.2;
const TEXT_MM: f64 = 2.9;
const TABLE_MM: f64 = 2.5;
const ROW_MM: f64 = 5.0;
const CAPTION_MM: f64 = 2.6;
/// Tallest picture (mm).
const IMAGE_MAX_MM: f64 = 125.0;
const SWATCH_MM: f64 = 14.0;
const SWATCH_CELL_MM: f64 = 48.0;
/// Room a heading needs below it, so it never ends a page alone.
const KEEP_WITH_HEADING_MM: f64 = 25.0;

const REGULAR: Name<'static> = Name(b"F1");
const BOLD: Name<'static> = Name(b"F2");

/// Page size, landscape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PageSize {
    #[default]
    A4,
    Letter,
}

impl PageSize {
    fn mm(self) -> DVec2 {
        match self {
            PageSize::A4 => DVec2::new(297.0, 210.0),
            PageSize::Letter => DVec2::new(279.4, 215.9),
        }
    }
}

/// A picture (a render of the garment), as RGB pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// Three bytes per pixel, row after row from the top.
    pub rgb: Vec<u8>,
    pub caption: String,
}

/// Why a picture could not go into the tech pack.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TechPackError {
    #[error("this picture cannot be read as PNG: {detail}")]
    BadImage { detail: String },
}

impl Localizable for TechPackError {
    fn message(&self) -> Message {
        match self {
            Self::BadImage { detail } => {
                Message::new("image_unreadable", self.to_string()).arg("detail", detail)
            }
        }
    }
}

/// Largest picture accepted (pixels): 4096 × 4096.
const MAX_PIXELS: u64 = 4096 * 4096;

impl Image {
    /// Decode a PNG (8 or 16 bits, grey or colour, with or without
    /// alpha; transparent pixels go on white).
    pub fn from_png(bytes: &[u8], caption: impl Into<String>) -> Result<Image, TechPackError> {
        let bad = |detail: String| TechPackError::BadImage { detail };
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(|e| bad(e.to_string()))?;
        let (w, h) = reader.info().size();
        if u64::from(w) * u64::from(h) > MAX_PIXELS {
            return Err(bad(format!("{w} x {h} pixels is too large")));
        }
        let size = reader
            .output_buffer_size()
            .ok_or_else(|| bad("too large".into()))?;
        let mut buf = vec![0; size];
        let info = reader
            .next_frame(&mut buf)
            .map_err(|e| bad(e.to_string()))?;
        let channels = match info.color_type {
            png::ColorType::Grayscale => 1,
            png::ColorType::GrayscaleAlpha => 2,
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            png::ColorType::Indexed => return Err(bad("indexed colours were not expanded".into())),
        };
        let mut rgb = Vec::with_capacity(w as usize * h as usize * 3);
        for row in buf.chunks(info.line_size).take(h as usize) {
            for px in row.chunks(channels).take(w as usize) {
                let get = |i: usize| f32::from(px.get(i).copied().unwrap_or(0));
                let (c, alpha) = match channels {
                    1 => ([get(0); 3], 255.0),
                    2 => ([get(0); 3], get(1)),
                    3 => ([get(0), get(1), get(2)], 255.0),
                    _ => ([get(0), get(1), get(2)], get(3)),
                };
                let a = alpha / 255.0;
                for v in c {
                    rgb.push((v * a + 255.0 * (1.0 - a)).round().clamp(0.0, 255.0) as u8);
                }
            }
        }
        if rgb.len() != w as usize * h as usize * 3 {
            return Err(bad("truncated image data".into()));
        }
        Ok(Image {
            width: w,
            height: h,
            rgb,
            caption: caption.into(),
        })
    }
}

/// A colour sample with its name.
#[derive(Clone, Debug, PartialEq)]
pub struct Swatch {
    /// sRGB `0xRRGGBB`.
    pub rgb: u32,
    /// Lines written under the sample.
    pub lines: Vec<String>,
}

/// A table column.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    pub title: String,
    /// Share of the page's width, relative to the other columns.
    pub weight: f64,
    /// Figures: aligned right.
    pub numeric: bool,
}

impl Column {
    pub fn text(title: impl Into<String>, weight: f64) -> Self {
        Self {
            title: title.into(),
            weight,
            numeric: false,
        }
    }

    pub fn number(title: impl Into<String>, weight: f64) -> Self {
        Self {
            title: title.into(),
            weight,
            numeric: true,
        }
    }
}

/// What a section holds, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    /// A smaller heading inside a section.
    Subheading(String),
    /// Pictures side by side, each with its caption.
    Images(Vec<Image>),
    Swatches(Vec<Swatch>),
    /// A row of line samples, each in its colour (sRGB) with its name:
    /// the sizes of a graded pattern.
    Legend(Vec<(u32, String)>),
    /// A table; it continues on the next page with its header repeated.
    Table {
        columns: Vec<Column>,
        rows: Vec<Vec<String>>,
    },
    /// Text, its lines as given, wrapped to the page.
    Text(String),
    /// A drawing scaled to fit the rest of the page (a fresh one when
    /// less than half is left), never above 1:1; its caption gets the
    /// scale (`caption - 1:N`).
    Drawing {
        drawing: Drawing,
        caption: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub title: String,
    pub blocks: Vec<Block>,
}

/// A tech pack to write.
#[derive(Clone, Debug, PartialEq)]
pub struct TechPack {
    /// On every page, top left (the project).
    pub title: String,
    /// On every page, top right (garments, sizes).
    pub subtitle: String,
    pub sections: Vec<Section>,
    pub page: PageSize,
}

/// Lays blocks out page after page.
struct Layout {
    size: DVec2,
    title: String,
    subtitle: String,
    done: Vec<Content>,
    page: Content,
    /// Cursor: the top of what comes next (mm from the bottom).
    y: f64,
    /// Images, in the order they were drawn (their XObject names).
    images: Vec<Image>,
}

fn text(c: &mut Content, font: Name<'_>, size: f64, x: f64, y: f64, s: &str) {
    c.begin_text();
    c.set_font(font, size as f32);
    c.next_line(x as f32, y as f32);
    c.show(Str(&win_ansi(s)));
    c.end_text();
}

/// `s` cut with "..." so it fits in `width` mm at `size`.
fn fit(s: &str, size: f64, width: f64) -> String {
    if approx_text_width(s, size) <= width {
        return s.to_owned();
    }
    let mut out: String = String::new();
    for ch in s.chars() {
        out.push(ch);
        if approx_text_width(&out, size) + approx_text_width("...", size) > width {
            out.pop();
            break;
        }
    }
    format!("{}...", out.trim_end())
}

/// `s` broken into lines no wider than `width` mm at `size` (its own line
/// breaks kept; a word too long for a line is cut).
fn wrap(s: &str, size: f64, width: f64) -> Vec<String> {
    let mut lines = Vec::new();
    for para in s.lines() {
        let mut line = String::new();
        for word in para.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if approx_text_width(&candidate, size) <= width || line.is_empty() {
                line = candidate;
            } else {
                lines.push(std::mem::take(&mut line));
                line = word.to_owned();
            }
        }
        lines.push(fit(&line, size, width));
    }
    lines
}

impl Layout {
    fn new(tp: &TechPack) -> Self {
        let mut l = Self {
            size: tp.page.mm(),
            title: tp.title.clone(),
            subtitle: tp.subtitle.clone(),
            done: Vec::new(),
            page: Content::new(),
            y: 0.0,
            images: Vec::new(),
        };
        l.start_page();
        l
    }

    fn width(&self) -> f64 {
        self.size.x - 2.0 * MARGIN_MM
    }

    fn top(&self) -> f64 {
        self.size.y - MARGIN_MM - HEADER_MM
    }

    fn bottom(&self) -> f64 {
        MARGIN_MM + FOOTER_MM
    }

    fn fresh(&self) -> bool {
        self.y >= self.top()
    }

    fn start_page(&mut self) {
        let mut c = Content::new();
        let k = PT_PER_MM as f32;
        c.transform([k, 0.0, 0.0, k, 0.0, 0.0]);
        let base = self.size.y - MARGIN_MM - TITLE_MM;
        c.set_fill_gray(0.0);
        let half = self.width() / 2.0 - 4.0;
        text(
            &mut c,
            BOLD,
            TITLE_MM,
            MARGIN_MM,
            base,
            &fit(&self.title, TITLE_MM, half),
        );
        let sub = fit(&self.subtitle, TEXT_MM, half);
        let w = approx_text_width(&sub, TEXT_MM);
        text(
            &mut c,
            REGULAR,
            TEXT_MM,
            self.size.x - MARGIN_MM - w,
            base,
            &sub,
        );
        c.set_line_width(0.3);
        c.set_stroke_gray(0.0);
        let rule = self.size.y - MARGIN_MM - HEADER_MM + 3.0;
        c.move_to(MARGIN_MM as f32, rule as f32);
        c.line_to((self.size.x - MARGIN_MM) as f32, rule as f32);
        c.stroke();
        let old = std::mem::replace(&mut self.page, c);
        if !self.done.is_empty() || self.y != 0.0 {
            self.done.push(old);
        }
        self.y = self.top();
    }

    /// Start a new page unless `h` mm still fit on this one.
    fn ensure(&mut self, h: f64) {
        if self.y - h < self.bottom() && !self.fresh() {
            self.start_page();
        }
    }

    fn heading(&mut self, s: &str, size: f64) {
        self.ensure(size * 1.8 + KEEP_WITH_HEADING_MM);
        if !self.fresh() {
            self.y -= size * 0.8;
        }
        let y = self.y - size;
        let w = self.width();
        text(&mut self.page, BOLD, size, MARGIN_MM, y, &fit(s, size, w));
        self.y = y - size * 0.9;
    }

    fn images(&mut self, images: &[Image]) {
        if images.is_empty() {
            return;
        }
        let gap = 6.0;
        let n = images.len() as f64;
        let cell = (self.width() - gap * (n - 1.0)) / n;
        let room = self.top() - self.bottom() - CAPTION_MM * 2.5;
        let max_h = IMAGE_MAX_MM.min(room);
        let sized: Vec<(f64, f64)> = images
            .iter()
            .map(|im| {
                let aspect = f64::from(im.width.max(1)) / f64::from(im.height.max(1));
                let w = cell.min(max_h * aspect);
                (w, w / aspect)
            })
            .collect();
        let row_h = sized.iter().map(|s| s.1).fold(0.0, f64::max);
        self.ensure(row_h + CAPTION_MM * 2.5);
        for (i, (im, (w, h))) in images.iter().zip(&sized).enumerate() {
            let x0 = MARGIN_MM + i as f64 * (cell + gap);
            let x = x0 + (cell - w) / 2.0;
            let y = self.y - row_h + (row_h - h) / 2.0;
            let name = format!("Im{}", self.images.len() + 1);
            let c = &mut self.page;
            c.save_state();
            c.transform([*w as f32, 0.0, 0.0, *h as f32, x as f32, y as f32]);
            c.x_object(Name(name.as_bytes()));
            c.restore_state();
            c.set_line_width(0.2);
            c.set_stroke_gray(0.7);
            c.rect(x as f32, y as f32, *w as f32, *h as f32);
            c.stroke();
            let cap = fit(&im.caption, CAPTION_MM, cell);
            let cw = approx_text_width(&cap, CAPTION_MM);
            c.set_fill_gray(0.2);
            text(
                c,
                REGULAR,
                CAPTION_MM,
                x0 + (cell - cw) / 2.0,
                self.y - row_h - CAPTION_MM * 1.6,
                &cap,
            );
            self.images.push(im.clone());
        }
        self.y -= row_h + CAPTION_MM * 2.5;
    }

    fn swatches(&mut self, swatches: &[Swatch]) {
        let per_row = ((self.width() / SWATCH_CELL_MM).floor() as usize).max(1);
        for row in swatches.chunks(per_row) {
            let lines = row.iter().map(|s| s.lines.len()).max().unwrap_or(0) as f64;
            let h = SWATCH_MM + 2.0 + lines * TEXT_MM * 1.35;
            self.ensure(h);
            for (i, s) in row.iter().enumerate() {
                let x = MARGIN_MM + i as f64 * SWATCH_CELL_MM;
                let channel = |shift: u32| ((s.rgb >> shift) & 0xFF) as f32 / 255.0;
                let c = &mut self.page;
                c.set_fill_rgb(channel(16), channel(8), channel(0));
                c.set_line_width(0.2);
                c.set_stroke_gray(0.4);
                c.rect(
                    x as f32,
                    (self.y - SWATCH_MM) as f32,
                    SWATCH_MM as f32,
                    SWATCH_MM as f32,
                );
                c.fill_nonzero_and_stroke();
                c.set_fill_gray(0.0);
                let mut y = self.y - SWATCH_MM - 2.0 - TEXT_MM;
                for (k, line) in s.lines.iter().enumerate() {
                    let font = if k == 0 { BOLD } else { REGULAR };
                    let t = fit(line, TEXT_MM, SWATCH_CELL_MM - 3.0);
                    text(c, font, TEXT_MM, x, y, &t);
                    y -= TEXT_MM * 1.35;
                }
            }
            self.y -= h + 3.0;
        }
    }

    fn legend(&mut self, items: &[(u32, String)]) {
        let sample = 8.0;
        let mut x = MARGIN_MM;
        let line_h = TEXT_MM * 1.8;
        self.ensure(line_h);
        let right = MARGIN_MM + self.width();
        for (rgb, name) in items {
            let w = sample + 1.5 + approx_text_width(name, TEXT_MM) + 5.0;
            if x + w > right && x > MARGIN_MM {
                x = MARGIN_MM;
                self.y -= line_h;
                self.ensure(line_h);
            }
            let y = self.y - line_h / 2.0;
            let channel = |shift: u32| ((rgb >> shift) & 0xFF) as f32 / 255.0;
            let c = &mut self.page;
            c.set_line_width(0.6);
            c.set_stroke_rgb(channel(16), channel(8), channel(0));
            c.move_to(x as f32, y as f32);
            c.line_to((x + sample) as f32, y as f32);
            c.stroke();
            c.set_fill_gray(0.0);
            text(
                c,
                REGULAR,
                TEXT_MM,
                x + sample + 1.5,
                y - TEXT_MM * 0.35,
                name,
            );
            x += w;
        }
        self.y -= line_h + 1.0;
    }

    fn table(&mut self, columns: &[Column], rows: &[Vec<String>]) {
        if columns.is_empty() {
            return;
        }
        let total: f64 = columns.iter().map(|c| c.weight.max(0.1)).sum();
        let widths: Vec<f64> = columns
            .iter()
            .map(|c| self.width() * c.weight.max(0.1) / total)
            .collect();
        let pad = 1.2;
        let draw_row = |c: &mut Content, y: f64, cells: &[String], header: bool, shade: f32| {
            if shade < 1.0 {
                c.set_fill_gray(shade);
                c.rect(
                    MARGIN_MM as f32,
                    (y - ROW_MM) as f32,
                    widths.iter().sum::<f64>() as f32,
                    ROW_MM as f32,
                );
                c.fill_nonzero();
            }
            c.set_fill_gray(0.0);
            let mut x = MARGIN_MM;
            for ((col, w), cell) in columns.iter().zip(&widths).zip(cells) {
                let t = fit(cell, TABLE_MM, w - 2.0 * pad);
                let tx = if col.numeric {
                    x + w - pad - approx_text_width(&t, TABLE_MM)
                } else {
                    x + pad
                };
                let font = if header { BOLD } else { REGULAR };
                text(c, font, TABLE_MM, tx, y - ROW_MM + 1.6, &t);
                x += w;
            }
        };
        let titles: Vec<String> = columns.iter().map(|c| c.title.clone()).collect();
        self.ensure(ROW_MM * 3.0);
        draw_row(&mut self.page, self.y, &titles, true, 0.88);
        self.y -= ROW_MM;
        for (i, row) in rows.iter().enumerate() {
            if self.y - ROW_MM < self.bottom() {
                self.start_page();
                draw_row(&mut self.page, self.y, &titles, true, 0.88);
                self.y -= ROW_MM;
            }
            let shade = if i % 2 == 1 { 0.96 } else { 1.0 };
            draw_row(&mut self.page, self.y, row, false, shade);
            self.y -= ROW_MM;
        }
        let right = MARGIN_MM + self.width();
        let c = &mut self.page;
        c.set_line_width(0.2);
        c.set_stroke_gray(0.5);
        c.move_to(MARGIN_MM as f32, self.y as f32);
        c.line_to(right as f32, self.y as f32);
        c.stroke();
        self.y -= 3.0;
    }

    fn text_block(&mut self, s: &str) {
        let line_h = TEXT_MM * 1.4;
        for line in wrap(s, TEXT_MM, self.width()) {
            self.ensure(line_h);
            self.y -= line_h;
            self.page.set_fill_gray(0.0);
            text(
                &mut self.page,
                REGULAR,
                TEXT_MM,
                MARGIN_MM,
                self.y + 0.8,
                &line,
            );
        }
        self.y -= 2.0;
    }

    fn drawing(&mut self, d: &Drawing, caption: &str) {
        let full = self.top() - self.bottom() - CAPTION_MM * 3.0;
        if self.y - self.bottom() < full / 2.0 {
            self.start_page();
        }
        let room = DVec2::new(self.width(), self.y - self.bottom() - CAPTION_MM * 3.0);
        let size = d.bounds.max - d.bounds.min;
        if size.x <= 0.0 || size.y <= 0.0 || room.y <= 0.0 {
            return;
        }
        // An exact 1:N.
        let fit = (room.x / size.x).min(room.y / size.y).min(1.0);
        let n = (1.0 / fit).ceil().max(1.0);
        let s = 1.0 / n;
        let used = size * s;
        let x = MARGIN_MM + (room.x - used.x) / 2.0;
        let y = self.y - used.y;
        let k = s as f32;
        let c = &mut self.page;
        c.save_state();
        c.transform([
            k,
            0.0,
            0.0,
            k,
            (x - d.bounds.min.x * s) as f32,
            (y - d.bounds.min.y * s) as f32,
        ]);
        // Lines 0.6 of their 1:1 width, labels at least 1.6 mm.
        draw(c, d, (0.6 / s) as f32, 1.6);
        c.restore_state();
        let cap = format!("{caption} - 1:{n:.0}");
        let cw = approx_text_width(&cap, CAPTION_MM);
        c.set_fill_gray(0.2);
        text(
            c,
            REGULAR,
            CAPTION_MM,
            MARGIN_MM + (room.x - cw) / 2.0,
            y - CAPTION_MM * 1.8,
            &cap,
        );
        self.y = y - CAPTION_MM * 3.0;
    }

    fn finish(mut self) -> (Vec<Content>, Vec<Image>) {
        let last = std::mem::replace(&mut self.page, Content::new());
        self.done.push(last);
        (self.done, self.images)
    }
}

/// Write the tech pack as a PDF.
pub fn to_pdf(tp: &TechPack) -> Vec<u8> {
    let mut l = Layout::new(tp);
    for section in &tp.sections {
        l.heading(&section.title, HEADING_MM);
        for block in &section.blocks {
            match block {
                Block::Subheading(s) => l.heading(s, SUBHEADING_MM),
                Block::Images(images) => l.images(images),
                Block::Swatches(s) => l.swatches(s),
                Block::Legend(items) => l.legend(items),
                Block::Table { columns, rows } => l.table(columns, rows),
                Block::Text(s) => l.text_block(s),
                Block::Drawing { drawing, caption } => l.drawing(drawing, caption),
            }
        }
    }
    let size = l.size;
    let title = l.title.clone();
    let (mut pages, images) = l.finish();

    let mut pdf = Pdf::new();
    let catalog = Ref::new(1);
    let tree = Ref::new(2);
    let regular = Ref::new(3);
    let bold = Ref::new(4);
    let mut next = 5;
    let mut alloc = || {
        let r = Ref::new(next);
        next += 1;
        r
    };
    let image_ids: Vec<Ref> = images.iter().map(|_| alloc()).collect();
    let page_ids: Vec<(Ref, Ref)> = pages.iter().map(|_| (alloc(), alloc())).collect();
    pdf.catalog(catalog).pages(tree);
    pdf.pages(tree)
        .kids(page_ids.iter().map(|(p, _)| *p))
        .count(page_ids.len() as i32);
    pdf.type1_font(regular)
        .base_font(Name(b"Helvetica"))
        .encoding_predefined(Name(b"WinAnsiEncoding"));
    pdf.type1_font(bold)
        .base_font(Name(b"Helvetica-Bold"))
        .encoding_predefined(Name(b"WinAnsiEncoding"));
    for (im, id) in images.iter().zip(&image_ids) {
        let data = miniz_oxide::deflate::compress_to_vec_zlib(&im.rgb, 6);
        let mut x = pdf.image_xobject(*id, &data);
        x.filter(Filter::FlateDecode);
        x.width(im.width as i32);
        x.height(im.height as i32);
        x.color_space().device_rgb();
        x.bits_per_component(8);
        x.finish();
    }
    let count = pages.len();
    for (i, (c, (page_id, content_id))) in pages.iter_mut().zip(&page_ids).enumerate() {
        // The footer, now that the page count is known.
        let foot = format!("{title} - {} / {count}", i + 1);
        let w = approx_text_width(&foot, CAPTION_MM);
        c.set_fill_gray(0.4);
        text(c, REGULAR, CAPTION_MM, (size.x - w) / 2.0, MARGIN_MM, &foot);
        let mut page = pdf.page(*page_id);
        page.media_box(Rect::new(
            0.0,
            0.0,
            (size.x * PT_PER_MM) as f32,
            (size.y * PT_PER_MM) as f32,
        ));
        page.parent(tree);
        page.contents(*content_id);
        let mut res = page.resources();
        res.fonts().pair(REGULAR, regular).pair(BOLD, bold);
        {
            let mut xs = res.x_objects();
            for (k, id) in image_ids.iter().enumerate() {
                let name = format!("Im{}", k + 1);
                xs.pair(Name(name.as_bytes()), *id);
            }
        }
        res.finish();
        page.finish();
    }
    for ((_, content_id), c) in page_ids.iter().zip(pages) {
        pdf.stream(*content_id, &c.finish());
    }
    pdf.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use r3d_geom::Aabb;

    fn png_of(w: u32, h: u32) -> Vec<u8> {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().unwrap();
        let mut px = Vec::new();
        for i in 0..w * h {
            // Opaque red, then fully transparent.
            px.extend_from_slice(if i == 0 {
                &[255, 0, 0, 255]
            } else {
                &[0, 0, 0, 0]
            });
        }
        writer.write_image_data(&px).unwrap();
        writer.finish().unwrap();
        out
    }

    #[test]
    fn png_decodes_onto_white() {
        let im = Image::from_png(&png_of(2, 2), "front").unwrap();
        assert_eq!((im.width, im.height), (2, 2));
        assert_eq!(&im.rgb[..6], &[255, 0, 0, 255, 255, 255]);
        assert!(Image::from_png(b"not a png", "x").is_err());
    }

    #[test]
    fn wraps_and_fits_text() {
        assert_eq!(fit("short", 2.0, 100.0), "short");
        assert!(fit(&"x".repeat(200), 2.0, 20.0).ends_with("..."));
        let lines = wrap("one two three four\nfive", 2.0, 8.0);
        assert!(lines.len() >= 3);
        assert_eq!(lines.last().map(String::as_str), Some("five"));
    }

    #[test]
    fn long_tables_go_on_several_pages() {
        let rows: Vec<Vec<String>> = (0..120)
            .map(|i| vec![format!("Piece {i}"), format!("{i}.0")])
            .collect();
        let drawing = Drawing {
            lines: vec![crate::drawing::Polyline {
                points: vec![
                    DVec2::ZERO,
                    DVec2::new(500.0, 0.0),
                    DVec2::new(500.0, 700.0),
                ],
                closed: true,
                stroke: crate::drawing::Stroke::Cut,
            }],
            labels: Vec::new(),
            bounds: Aabb {
                min: DVec2::ZERO,
                max: DVec2::new(500.0, 700.0),
            },
        };
        let tp = TechPack {
            title: "Shirt".into(),
            subtitle: "40".into(),
            sections: vec![
                Section {
                    title: "Views".into(),
                    blocks: vec![Block::Images(vec![
                        Image::from_png(&png_of(4, 3), "Front").unwrap(),
                        Image::from_png(&png_of(4, 3), "Back").unwrap(),
                    ])],
                },
                Section {
                    title: "Pieces".into(),
                    blocks: vec![Block::Table {
                        columns: vec![Column::text("Piece", 2.0), Column::number("cm", 1.0)],
                        rows,
                    }],
                },
                Section {
                    title: "Pattern".into(),
                    blocks: vec![Block::Drawing {
                        drawing,
                        caption: "Pattern".into(),
                    }],
                },
            ],
            page: PageSize::A4,
        };
        let pdf = to_pdf(&tp);
        let doc = lopdf::Document::load_mem(&pdf).unwrap();
        // 120 rows of 5 mm need three pages; the views fit on the first
        // and the pattern takes a page of its own.
        assert!(doc.get_pages().len() >= 4, "{}", doc.get_pages().len());
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("/Helvetica-Bold"));
        assert!(text.contains("/Im2"));
    }
}
