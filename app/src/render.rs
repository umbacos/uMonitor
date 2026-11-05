use crate::{model::*, original};
use ab_glyph::{Font, FontArc, ScaleFont};
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{imageops::FilterType, Rgba, RgbaImage};
use imageproc::{
    drawing::{draw_filled_circle_mut, draw_filled_rect_mut, draw_line_segment_mut},
    rect::Rect,
};
fn rect(img: &mut RgbaImage, x: f32, y: f32, w: f32, h: f32, c: [u8; 4]) {
    let (width, height) = (w as u32, h as u32);
    if width > 0 && height > 0 {
        draw_filled_rect_mut(
            img,
            Rect::at(x as i32, y as i32).of_size(width, height),
            Rgba(c),
        );
    }
}
fn image_data(b64: &str) -> Option<RgbaImage> {
    STANDARD
        .decode(b64)
        .ok()
        .and_then(|b| image::load_from_memory(&b).ok())
        .map(|i| i.to_rgba8())
}
fn paste(img: &mut RgbaImage, data: &str, x: f32, y: f32, w: f32, h: f32) {
    if let Some(i) = image_data(data) {
        let i =
            image::imageops::resize(&i, w.max(1.) as u32, h.max(1.) as u32, FilterType::Lanczos3);
        image::imageops::overlay(img, &i, x as i64, y as i64);
    }
}
pub fn frame(p: &Project, d: &DataState) -> RgbaImage {
    frame_at(p, d, animation_seconds())
}
fn animation_seconds() -> f64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_secs_f64()
}
pub fn has_animation(p: &Project) -> bool {
    p.widgets
        .iter()
        .any(|w| w.visible && w.kind == Kind::Ticker && w.scroll_speed > 0.)
}
fn frame_at(p: &Project, d: &DataState, seconds: f64) -> RgbaImage {
    let (width, height) = p.dimensions();
    let mut img = RgbaImage::from_pixel(width, height, Rgba(p.background));
    if let Some(b) = &p.wallpaper {
        if let Some(mut wallpaper) = image_data(b) {
            wallpaper = match p.wallpaper_rotation % 4 {
                1 => image::imageops::rotate90(&wallpaper),
                2 => image::imageops::rotate180(&wallpaper),
                3 => image::imageops::rotate270(&wallpaper),
                _ => wallpaper,
            };
            if p.wallpaper_mirror {
                wallpaper = image::imageops::flip_horizontal(&wallpaper);
            }
            let wallpaper =
                image::imageops::resize(&wallpaper, width, height, FilterType::Lanczos3);
            image::imageops::overlay(&mut img, &wallpaper, 0, 0);
        }
    }
    for w in &p.widgets {
        if !w.visible {
            continue;
        }
        let (x, y, _, _) = widget_bounds(w);
        let tile = widget_tile_at(w, d, seconds);
        image::imageops::overlay(&mut img, &tile, x as i64, y as i64);
    }
    img
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}
/// Dirty 16-pixel tiles, coalesced into rectangles. Small changes stay local,
/// while adjacent glyph and gauge pixels avoid hundreds of serial commands.
pub fn changed_regions(
    previous: &[u8],
    current: &[u8],
    width: u32,
    height: u32,
    channels: usize,
) -> Vec<Region> {
    if previous.len() != current.len() || previous.is_empty() {
        return vec![Region {
            x: 0,
            y: 0,
            w: width,
            h: height,
        }];
    }
    let mut regions: Vec<Region> = vec![];
    for y in (0..height).step_by(16) {
        let h = (height - y).min(16);
        let mut runs: Vec<Region> = vec![];
        for x in (0..width).step_by(16) {
            let w = (width - x).min(16);
            let changed = (y..y + h).any(|yy| {
                let start = ((yy * width + x) as usize) * channels;
                let end = start + w as usize * channels;
                previous[start..end] != current[start..end]
            });
            if changed {
                if let Some(run) = runs.last_mut().filter(|r| r.x + r.w == x) {
                    run.w += w;
                } else {
                    runs.push(Region { x, y, w, h });
                }
            }
        }
        for run in runs {
            if let Some(prior) = regions
                .iter_mut()
                .rev()
                .find(|r| r.x == run.x && r.w == run.w && r.y + r.h == run.y)
            {
                prior.h += run.h;
            } else {
                regions.push(run);
            }
        }
    }
    regions
}

/// Rasterize only widgets whose appearance or bound data changed. Static images
/// and fonts are decoded once; composition still respects transparent layers.
#[derive(Default)]
pub struct FrameCache {
    base_key: Vec<u8>,
    base: Option<RgbaImage>,
    layers: std::collections::BTreeMap<u64, (Vec<u8>, RgbaImage)>,
    ticker_lines: std::collections::BTreeMap<u64, (Vec<u8>, RgbaImage)>,
}
impl FrameCache {
    pub fn frame(&mut self, p: &Project, d: &DataState) -> RgbaImage {
        self.frame_at(p, d, animation_seconds())
    }
    fn frame_at(&mut self, p: &Project, d: &DataState, seconds: f64) -> RgbaImage {
        let mut background = p.clone();
        background.widgets.clear();
        background.sources.clear();
        let key = serde_json::to_vec(&background).unwrap_or_default();
        if self.base_key != key || self.base.is_none() {
            self.base = Some(frame(&background, d));
            self.base_key = key;
        }
        let mut img = self.base.as_ref().unwrap().clone();
        self.layers
            .retain(|id, _| p.widgets.iter().any(|w| w.id == *id));
        self.ticker_lines.retain(|id, _| {
            p.widgets
                .iter()
                .any(|w| w.id == *id && w.kind == Kind::Ticker)
        });
        for w in p.widgets.iter().filter(|w| w.visible) {
            let mut key = serde_json::to_vec(w).unwrap_or_default();
            if !w.binding.is_empty() {
                key.extend(d.value(w).bytes());
            }
            if matches!(w.kind, Kind::Line | Kind::Arrow) {
                key.extend(serde_json::to_vec(&d.histories.get(&w.binding)).unwrap_or_default());
            }
            if w.kind == Kind::Clock {
                key.extend(clock_value(w).bytes());
            }
            if w.kind == Kind::Ticker {
                key.extend(ticker_text(w, d).bytes());
                let (previous, line) = self
                    .ticker_lines
                    .entry(w.id)
                    .or_insert_with(|| (vec![], RgbaImage::new(1, 1)));
                if *previous != key {
                    *line = ticker_line(w, d);
                    *previous = key;
                }
                let tile = ticker_tile(w, line, seconds);
                image::imageops::overlay(&mut img, &tile, w.x as i64, w.y as i64);
                continue;
            }
            let (previous, tile) = self
                .layers
                .entry(w.id)
                .or_insert_with(|| (vec![], RgbaImage::new(1, 1)));
            if *previous != key {
                *tile = widget_tile_at(w, d, seconds);
                *previous = key;
            }
            let (x, y, _, _) = widget_bounds(w);
            image::imageops::overlay(&mut img, tile, x as i64, y as i64);
        }
        img
    }
}
fn clock_value(w: &Widget) -> String {
    chrono::Local::now()
        .format(if !w.clock_format.is_empty() {
            &w.clock_format
        } else if w.text == "date" {
            "%d %b %Y"
        } else {
            "%H:%M:%S"
        })
        .to_string()
}
fn segment_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy).max(0.001)).clamp(0., 1.);
    ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt()
}
/// Rotation preserves the center and expands the raster so corners are never clipped.
pub fn widget_bounds(w: &Widget) -> (f32, f32, f32, f32) {
    if !w.kind.is_shape() || w.rotation_degrees.rem_euclid(360.).abs() < 0.001 {
        return (w.x, w.y, w.w.max(1.).ceil(), w.h.max(1.).ceil());
    }
    let angle = w.rotation_degrees.to_radians();
    let width = angle.cos().abs() * w.w + angle.sin().abs() * w.h;
    let height = angle.sin().abs() * w.w + angle.cos().abs() * w.h;
    let left = (w.x + (w.w - width) / 2.).floor();
    let top = (w.y + (w.h - height) / 2.).floor();
    (
        left,
        top,
        (w.x + (w.w + width) / 2.).ceil() - left,
        (w.y + (w.h + height) / 2.).ceil() - top,
    )
}
#[cfg(test)]
fn widget_tile(w: &Widget, d: &DataState) -> RgbaImage {
    widget_tile_at(w, d, 0.)
}
fn widget_tile_at(w: &Widget, d: &DataState, seconds: f64) -> RgbaImage {
    if w.kind == Kind::Ticker {
        return ticker_tile(w, &ticker_line(w, d), seconds);
    }
    let (_, _, width, height) = widget_bounds(w);
    let mut tile = RgbaImage::new(width.max(1.) as u32, height.max(1.) as u32);
    draw_widget(&mut tile, w, d, seconds);
    tile
}
fn draw_shape(img: &mut RgbaImage, w: &Widget) {
    let width = w.w.max(1.);
    let height = w.h.max(1.);
    let stroke = w.stroke_width.max(1.);
    let (left, top, _, _) = widget_bounds(w);
    let (sin, cos) = w.rotation_degrees.to_radians().sin_cos();
    let points: Vec<_> = (0..w.sides.clamp(3, 12))
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / w.sides.clamp(3, 12) as f32
                - std::f32::consts::FRAC_PI_2;
            (
                width / 2. + width / 2. * a.cos(),
                height / 2. + height / 2. * a.sin(),
            )
        })
        .collect();
    let sample = |p: (f32, f32)| -> Option<[u8; 4]> {
        if w.kind == Kind::StraightLine {
            let inset = (stroke / 2.).min(width / 2.).min(height / 2.);
            let (a, b) = match w.line_direction {
                LineDirection::Horizontal => ((inset, height / 2.), (width - inset, height / 2.)),
                LineDirection::Vertical => ((width / 2., inset), (width / 2., height - inset)),
                LineDirection::Diagonal if w.line_reverse => {
                    ((inset, height - inset), (width - inset, inset))
                }
                LineDirection::Diagonal => ((inset, inset), (width - inset, height - inset)),
            };
            return (w.outline_enabled && segment_distance(p, a, b) <= stroke / 2.)
                .then_some(w.outline_color);
        }
        let distance = match w.kind {
            Kind::Circle | Kind::Oval => {
                let rx = if w.kind == Kind::Circle {
                    width.min(height) / 2.
                } else {
                    width / 2.
                };
                let ry = if w.kind == Kind::Circle {
                    rx
                } else {
                    height / 2.
                };
                let norm =
                    (((p.0 - width / 2.) / rx).powi(2) + ((p.1 - height / 2.) / ry).powi(2)).sqrt();
                (1. - norm) * rx.min(ry)
            }
            Kind::Polygon => {
                let mut inside = false;
                let mut distance = f32::MAX;
                for i in 0..points.len() {
                    let a = points[i];
                    let b = points[(i + 1) % points.len()];
                    distance = distance.min(segment_distance(p, a, b));
                    if (a.1 > p.1) != (b.1 > p.1)
                        && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0
                    {
                        inside = !inside;
                    }
                }
                if inside {
                    distance
                } else {
                    -distance
                }
            }
            _ => {
                let r = if w.kind == Kind::RoundedRectangle {
                    w.corner_radius.clamp(0., width.min(height) / 2.)
                } else {
                    0.
                };
                let dx = (p.0 - width / 2.).abs() - (width / 2. - r);
                let dy = (p.1 - height / 2.).abs() - (height / 2. - r);
                r - (dx.max(0.).powi(2) + dy.max(0.).powi(2)).sqrt() - dx.max(dy).min(0.)
            }
        };
        if distance < 0. {
            None
        } else if w.outline_enabled && distance < stroke {
            Some(w.outline_color)
        } else if w.fill_enabled {
            Some(w.color)
        } else {
            None
        }
    };
    // 4?4 subpixel coverage for every shape, including strokes and rotated edges.
    // Accumulate premultiplied channels so transparent edge pixels keep their color.
    for y in 0..img.height() {
        for x in 0..img.width() {
            let mut channels = [0u32; 3];
            let mut alpha = 0u32;
            for sy in 0..4 {
                for sx in 0..4 {
                    let dx = left - w.x + x as f32 + (sx as f32 + 0.5) / 4. - width / 2.;
                    let dy = top - w.y + y as f32 + (sy as f32 + 0.5) / 4. - height / 2.;
                    if let Some(color) = sample((
                        cos * dx + sin * dy + width / 2.,
                        -sin * dx + cos * dy + height / 2.,
                    )) {
                        alpha += color[3] as u32;
                        for i in 0..3 {
                            channels[i] += color[i] as u32 * color[3] as u32;
                        }
                    }
                }
            }
            if alpha > 0 {
                img.put_pixel(
                    x,
                    y,
                    Rgba([
                        (channels[0] / alpha) as u8,
                        (channels[1] / alpha) as u8,
                        (channels[2] / alpha) as u8,
                        ((alpha + 8) / 16) as u8,
                    ]),
                );
            }
        }
    }
}
/// Shared baseline, kerning for letters, tabular digit advances and fractional pen
/// positions. Imported presets scale from their saved size instead of cancelling it.
fn text_line(
    s: &str,
    size: f32,
    color: [u8; 4],
    font: &FontArc,
    metrics: Option<&LegacyText>,
    spacing: f32,
) -> RgbaImage {
    let scaled = font.as_scaled(if metrics.is_some() { 64. } else { size.max(6.) });
    let zero = scaled.scaled_glyph('0');
    let Some(reference) = font.outline_glyph(zero.clone()).map(|g| g.px_bounds()) else {
        return RgbaImage::new(1, 1);
    };
    let ratio = metrics.map_or(1., |m| {
        if m.base_size > 0. {
            size / m.base_size
        } else {
            1.
        }
    });
    let sx = metrics.map_or(1., |m| m.cell_width * ratio / reference.width().max(1.));
    let sy = metrics.map_or(1., |m| m.cell_height * ratio / reference.height().max(1.));
    let digit_advance = metrics.map_or_else(|| scaled.h_advance(zero.id), |m| m.advance * ratio);
    let mut pen = 0.;
    let mut previous = None;
    let mut glyphs = vec![];
    let mut left = f32::MAX;
    let mut right = f32::MIN;
    let mut bottom = metrics.map_or(scaled.height(), |m| m.cell_height * ratio);
    for ch in s.chars().take(1024) {
        let glyph = scaled.scaled_glyph(ch);
        let digit = ch.is_ascii_digit();
        if let Some((id, was_digit)) = previous.filter(|_| metrics.is_none()) {
            if !(digit && was_digit) {
                pen += scaled.kern(id, glyph.id);
            }
        }
        if let Some(outline) = font.outline_glyph(glyph.clone()) {
            let b = outline.px_bounds();
            let x = pen
                + if digit {
                    // Center all digits in identical cells; a 1 no longer shifts the value.
                    let cell = metrics.map_or(digit_advance, |m| m.cell_width * ratio);
                    (cell - b.width() * sx) / 2.
                } else {
                    b.min.x * sx
                };
            let y = if metrics.is_some() {
                (b.min.y - reference.min.y) * sy
            } else {
                scaled.ascent() + b.min.y
            };
            left = left.min(x);
            right = right.max(x + b.width() * sx);
            bottom = bottom.max(y + b.height() * sy);
            glyphs.push((outline, x, y));
        }
        // TItem RVA 0x4f80c IL_0744..0751 / 0x500ec IL_01ea..01f7:
        // every character advances by FontConfig.interval, with no kerning.
        // Preserve the editor's explicit size/spacing adjustments on top.
        pen += (if digit || metrics.is_some() {
            digit_advance
        } else {
            scaled.h_advance(glyph.id)
        }) + spacing;
        previous = Some((glyph.id, digit));
    }
    if let Some(metrics) = metrics.filter(|_| !s.is_empty()) {
        // TItem 0x500ec draws at posX, advancing even across blank 'x' cells.
        // Keep cell margins and leading/trailing blanks instead of cropping
        // them to the visible ink. Original glyph pixels remain approximate.
        left = left.min(0.);
        right = right.max(pen - digit_advance - spacing + metrics.cell_width * ratio);
    } else if glyphs.is_empty() {
        return RgbaImage::new(1, 1);
    }
    let mut line = RgbaImage::new(
        (right - left.floor()).ceil().clamp(1., 8192.) as u32,
        bottom.ceil().clamp(1., 1024.) as u32,
    );
    for (glyph, x, y) in glyphs {
        let b = glyph.px_bounds();
        let mut raster = RgbaImage::new(b.width() as u32, b.height() as u32);
        glyph.draw(|gx, gy, coverage| {
            raster.put_pixel(
                gx,
                gy,
                Rgba([
                    color[0],
                    color[1],
                    color[2],
                    (coverage * color[3] as f32).round() as u8,
                ]),
            );
        });
        let raster = image::imageops::resize(
            &raster,
            (b.width() * sx).round().max(1.) as u32,
            (b.height() * sy).round().max(1.) as u32,
            FilterType::Lanczos3,
        );
        image::imageops::overlay(
            &mut line,
            &raster,
            (x - left.floor()).round() as i64,
            y.round() as i64,
        );
    }
    if metrics.is_some() {
        return line;
    }
    // Ordinary editor text aligns its visible edge, including unit suffixes.
    let mut bounds = (line.width(), 0);
    for (x, _, p) in line.enumerate_pixels() {
        if p[3] > 0 {
            bounds.0 = bounds.0.min(x);
            bounds.1 = bounds.1.max(x);
        }
    }
    if bounds.0 <= bounds.1 {
        image::imageops::crop_imm(&line, bounds.0, 0, bounds.1 - bounds.0 + 1, line.height())
            .to_image()
    } else {
        RgbaImage::new(1, 1)
    }
}
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn legacy_text(
    img: &mut RgbaImage,
    s: &str,
    x: i32,
    y: i32,
    size: f32,
    color: [u8; 4],
    font: &Option<FontArc>,
    metrics: &LegacyText,
) {
    if let Some(font) = font {
        image::imageops::overlay(
            img,
            &text_line(s, size, color, font, Some(metrics), 0.),
            x as i64,
            y as i64,
        );
    }
}
fn ticker_text(w: &Widget, d: &DataState) -> String {
    if !w.ticker_bound {
        return w.text.clone();
    }
    let separator = "   •   ";
    let mut text = w
        .ticker_sources
        .iter()
        .map(|source| source.display(d))
        .collect::<Vec<_>>()
        .join(separator);
    if w.ticker_circular && w.scroll_speed > 0. && !w.ticker_sources.is_empty() {
        // The last-to-first wrap is also a data field boundary.
        text.push_str(separator);
    }
    text
}
fn ticker_line(w: &Widget, d: &DataState) -> RgbaImage {
    crate::fonts::resolve(&w.font, w.font_data.as_deref()).map_or_else(
        || RgbaImage::new(1, 1),
        |font| {
            let line = text_line(
                &ticker_text(w, d),
                w.size,
                w.color,
                &font,
                w.legacy_text.as_ref(),
                w.letter_spacing,
            );
            if w.ticker_circular && w.scroll_speed > 0. {
                // Keep adjacent repetitions legible without a field-wide blank gap.
                let gap = (w.size.max(6.) * 0.75).ceil() as u32;
                let mut strip = RgbaImage::new(line.width() + gap, line.height());
                image::imageops::overlay(&mut strip, &line, 0, 0);
                strip
            } else {
                line
            }
        },
    )
}
/// Translate a cached text strip with premultiplied-alpha linear sampling.
/// Fractional positions keep slow scrolls smooth without rerasterizing glyphs.
fn ticker_tile(w: &Widget, line: &RgbaImage, seconds: f64) -> RgbaImage {
    let mut tile = RgbaImage::new(w.w.max(1.).ceil() as u32, w.h.max(1.).ceil() as u32);
    let circular = w.ticker_circular && w.scroll_speed > 0.;
    let x = if circular {
        // Retain the usual pixels/second, wrapping by the strip rather than the field.
        let distance = w.w.max(1.) as f64 + line.width() as f64;
        let offset = (seconds * w.scroll_speed.clamp(0., 10.) as f64 * distance / 10.)
            .rem_euclid(line.width() as f64) as f32;
        match w.scroll_direction {
            ScrollDirection::RightToLeft => -offset,
            ScrollDirection::LeftToRight => offset,
        }
    } else {
        ticker_x(
            w.w.max(1.),
            line.width() as f32,
            w.scroll_speed,
            w.scroll_direction,
            seconds,
        )
    };
    let left = x.floor() as i64;
    let fraction = (x - x.floor()) as f64;
    for y in 0..tile.height().min(line.height()) {
        let columns = if circular {
            0..tile.width() as i64
        } else {
            left.max(0)..(left + line.width() as i64 + 1).min(tile.width() as i64)
        };
        for dest in columns {
            let source = dest - left;
            let sample = |column: i64| {
                if circular {
                    line.get_pixel(column.rem_euclid(line.width() as i64) as u32, y)
                        .0
                } else if (0..line.width() as i64).contains(&column) {
                    line.get_pixel(column as u32, y).0
                } else {
                    [0; 4]
                }
            };
            let a = sample(source);
            let b = sample(source - 1);
            let wa = (1. - fraction) * a[3] as f64;
            let wb = fraction * b[3] as f64;
            let alpha = wa + wb;
            if alpha > 0. {
                tile.put_pixel(
                    dest as u32,
                    y,
                    Rgba([
                        ((a[0] as f64 * wa + b[0] as f64 * wb) / alpha).round() as u8,
                        ((a[1] as f64 * wa + b[1] as f64 * wb) / alpha).round() as u8,
                        ((a[2] as f64 * wa + b[2] as f64 * wb) / alpha).round() as u8,
                        alpha.round() as u8,
                    ]),
                );
            }
        }
    }
    tile
}
// A complete passage across the field takes 10 / speed seconds.
fn ticker_x(
    width: f32,
    text_width: f32,
    speed: f32,
    direction: ScrollDirection,
    seconds: f64,
) -> f32 {
    if speed <= 0. {
        return 0.;
    }
    let phase = (seconds * speed.clamp(0., 10.) as f64 / 10.).rem_euclid(1.) as f32;
    let distance = width + text_width;
    match direction {
        ScrollDirection::RightToLeft => width - phase * distance,
        ScrollDirection::LeftToRight => -text_width + phase * distance,
    }
}
fn draw_widget(img: &mut RgbaImage, w: &Widget, d: &DataState, _seconds: f64) {
    if w.kind.is_shape() {
        draw_shape(img, w);
        return;
    }
    let font = crate::fonts::resolve(&w.font, w.font_data.as_deref());
    let paint_text = |img: &mut RgbaImage, s: &str, x: i32, y: i32, size: f32, color: [u8; 4]| {
        if let Some(font) = &font {
            let line = text_line(
                s,
                size,
                color,
                font,
                w.legacy_text.as_ref(),
                w.letter_spacing,
            );
            image::imageops::overlay(img, &line, x as i64, y as i64);
        }
    };
    let paint_value =
        |img: &mut RgbaImage, s: &str, y: i32, size: f32, color: [u8; 4], right: bool| {
            if let Some(font) = &font {
                let line = text_line(
                    s,
                    size,
                    color,
                    font,
                    w.legacy_text.as_ref(),
                    w.letter_spacing,
                );
                let x = if right && w.legacy_text.is_none() {
                    img.width() as i64 - line.width() as i64
                } else {
                    0
                };
                image::imageops::overlay(img, &line, x, y as i64);
            }
        };
    let paint_centered =
        |img: &mut RgbaImage, s: &str, x: f32, y: f32, size: f32, color: [u8; 4]| {
            if let Some(font) = &font {
                let line = text_line(
                    s,
                    size,
                    color,
                    font,
                    w.legacy_text.as_ref(),
                    w.letter_spacing,
                );
                image::imageops::overlay(
                    img,
                    &line,
                    (x - line.width() as f32 / 2.).round() as i64,
                    y.round() as i64,
                );
            }
        };
    let width = img.width() as f32;
    let height = img.height() as f32;
    let value = d.value(w);
    let num = d.number(w);
    let frac = num
        .map(|n| ((n - w.min) / (w.max - w.min).max(0.001)).clamp(0., 1.) as f32)
        .unwrap_or(0.);
    let label = [132, 159, 165, 255];
    let white = [231, 241, 240, 255];
    let formatted = if let Some(n) = num.filter(|n| {
        w.legacy_text.is_some()
            && w.binding == "cpu.frequency"
            && w.decimals == Some(1)
            && n.is_finite()
            && *n >= 0.
            && *n < i32::MAX as f64 / 10.
    }) {
        // Controller RVA 0x12418 IL_1632..167b, getter RVA 0xdc54:
        // float32 MHz -> Round(MHz/10000, 2) -> *100 -> conv.i4 ->
        // PadLeft(2,'0') -> Insert(1,"."). Keep every arithmetic step,
        // including ties-to-even and binary rounding before truncation.
        original::cpu_frequency_text(n)
    } else if let Some(mut n) = num {
        // Controller RVA 0x12418: InternetDown/InternetUp use conv.i4(rate*10),
        // replace x00 with x01 only above .01 MiB/s, replace values above 99
        // with 999, then insert the decimal point before the last digit.
        // This changes presentation only; retain the precise shared reading.
        if w.legacy_text.is_some()
            && w.decimals == Some(1)
            && n.is_finite()
            && n >= 0.
            && matches!(w.binding.as_str(), "network.download" | "network.upload")
        {
            n = original::network_tenths(n) as f64 / 10.;
        }
        // Controller RVA 0x12418 uses conv.i4 before formatting these TItem
        // readings (IL_0c62,0fc6,14e7,15c1,15fd,1775). The bar paths use the
        // original fractional value. An explicit decimal choice stays editable.
        let original_integer = w.legacy_text.is_some()
            && w.decimals == Some(0)
            && n.is_finite()
            && matches!(
                w.binding.as_str(),
                "cpu.usage"
                    | "cpu.temperature"
                    | "gpu.usage"
                    | "gpu.temperature"
                    | "memory.usage"
                    | "disk.usage"
                    | "cpu.power"
                    | "gpu.memory"
            );
        if original_integer {
            n = (n as f32 as i32) as f64;
        } else if w.legacy_text.is_some()
            && w.decimals == Some(0)
            && n.is_finite()
            && n >= i32::MIN as f64
            && n <= i32::MAX as f64
            && matches!(
                w.binding.as_str(),
                "disk.temperature" | "cpu.fan" | "gpu.fan" | "volume.level"
            )
        {
            // Controller RVA 0x12418 draws these from int getters:
            // HDDTemp 0xece8, CpuFan 0xd2b0, GpuFan 0xd574, Volume 0xeb84.
            // Temperature helper 0x41960 converts its double using conv.i4.
            // Keep fractional replacement-provider samples for charts and
            // explicit decimals, but truncate default imported integer text.
            n = (n as i32) as f64;
        }
        if original_integer && matches!(w.binding.as_str(), "cpu.temperature" | "gpu.temperature") {
            // IL_14f1..14f4 and IL_15cb..15ce: PadLeft(2, '0').
            format!("{:02}", n as i32)
        } else if let Some(decimals) = w.decimals {
            format!("{:.*}", decimals.min(6) as usize, n)
        } else if n.abs() >= 100. || n.fract().abs() < 0.01 {
            format!("{:.0}", n)
        } else {
            format!("{:.1}", n)
        }
    } else {
        value.clone()
    };
    let formatted = if w.legacy_text.is_some() && num.is_some_and(f64::is_finite) {
        original::padded_sensor_text(&w.binding, w.decimals, formatted)
    } else {
        formatted
    };
    let display = if !w.binding.is_empty() {
        format!("{}{}{}", w.unit_prefix, formatted, w.unit)
    } else {
        formatted
    };
    match w.kind {
        Kind::Ticker => {
            unreachable!("ticker tiles are composed from their cached text strip");
        }
        Kind::Rectangle
        | Kind::RoundedRectangle
        | Kind::Circle
        | Kind::Oval
        | Kind::Polygon
        | Kind::StraightLine => unreachable!(),
        Kind::Arrow => {
            let trend = trend(w, d);
            let top = if w.show_label { 17. } else { 0. };
            if w.show_label {
                paint_text(img, &w.label, 0, 0, 10., label);
            }
            if let Some(up) = trend {
                let cy = top + (height - top) / 2.;
                let y = |v: f32| if up { top + v } else { 2. * cy - top - v };
                let h = (height - top - 2.).max(4.);
                let aw = width.min(h).min(45.);
                let pts: Vec<_> = [
                    (aw / 2., 0.),
                    (aw, h * 0.4),
                    (aw * 0.7, h * 0.4),
                    (aw * 0.7, h),
                    (aw * 0.3, h),
                    (aw * 0.3, h * 0.4),
                    (0., h * 0.4),
                ]
                .into_iter()
                .map(|(x, v)| imageproc::point::Point::new(x as i32, y(v) as i32))
                .collect();
                imageproc::drawing::draw_polygon_mut(img, &pts, Rgba(w.color));
                if width > aw + 30. {
                    paint_text(
                        img,
                        &display,
                        aw as i32 + 8,
                        cy as i32 - 10,
                        w.size.min(23.),
                        white,
                    );
                }
            } else {
                paint_text(img, "—", 0, top as i32, w.size, w.color);
            }
        }
        Kind::Text => paint_value(img, &display, 0, w.size, w.color, num.is_some()),
        Kind::Clock => {
            paint_value(
                img,
                &clock_value(w),
                if w.show_label { 18 } else { 0 },
                w.size,
                w.color,
                true,
            );
            if w.show_label {
                paint_text(img, &w.label, 0, 0, 10., label);
            }
        }
        Kind::Value => {
            if w.show_label {
                paint_text(img, &w.label, 0, 0, 10., label);
            }
            paint_value(
                img,
                &display,
                if w.show_label { 20 } else { 0 },
                w.size,
                w.color,
                true,
            );
        }
        Kind::Image => {
            if let Some(b) = &w.image {
                paste(img, b, 0., 0., width, height);
            }
        }
        Kind::Bar => {
            if w.show_label {
                paint_text(img, &w.label, 0, 0, 10., label);
                paint_value(img, &display, 17, w.size.min(23.), w.color, true);
            }
            let y = if w.show_label { height - 10. } else { 0. };
            let h = if w.show_label { 8. } else { height };
            // Original StatuBar0x60590 uses the composed screen as its track
            // in ThemeConfig.barType0. Transparent pixels retain that artwork
            // in both the preview and the shared image-upload framebuffer.
            if !w.legacy_bar_image_background {
                rect(img, 0., y, width, h, w.background);
            }
            // Controller RVA 0x12418 IL_0809..0822 uses float MHz / 8000
            // for CpuFreq. Other supported bars promote the float to double,
            // multiply by .01 and length in that order, then truncate.
            let pixels = if w.legacy_text.is_some() {
                num.filter(|n| n.is_finite())
                    .map(|n| original::bar_pixels(&w.binding, n, width))
                    .unwrap_or(0.)
            } else {
                width * frac
            };
            rect(img, 0., y, pixels, h, w.color);
        }
        Kind::Ring | Kind::Pie => {
            let cx = width / 2.;
            let gauge_height = if w.ring_label_inside || w.legacy_arc {
                height
            } else {
                height - 20.
            };
            let cy = gauge_height / 2.;
            let radius =
                (width.min(gauge_height) / 2. - if w.legacy_arc { 0. } else { 4. }).max(2.);
            // Original ArchBar is assembled from 100 segments of 3.6 degrees.
            let frac = if w.legacy_arc {
                num.map(|n| {
                    (n.clamp(0., original::ARC_SEGMENTS).trunc() / original::ARC_SEGMENTS) as f32
                })
                .unwrap_or(0.)
            } else {
                frac
            };
            for y in 0..img.height() {
                for x in 0..img.width() {
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let ring = dist <= radius
                        && (w.kind == Kind::Pie || dist >= radius - w.ring_thickness.max(1.));
                    if ring {
                        let a = (dy.atan2(dx) - w.ring_start.to_radians())
                            .rem_euclid(std::f32::consts::TAU)
                            / std::f32::consts::TAU;
                        img.put_pixel(
                            x,
                            y,
                            Rgba(
                                if frac > 0.
                                    && a < frac
                                    && !(w.legacy_arc && w.legacy_arc_gaps && original::arc_gap(a))
                                {
                                    w.color
                                } else {
                                    w.background
                                },
                            ),
                        );
                    }
                }
            }
            if w.kind == Kind::Ring && !w.legacy_arc {
                paint_centered(img, &display, cx, cy - w.size / 2. - 3., w.size, white);
            }
            if w.show_label {
                paint_centered(
                    img,
                    &w.label,
                    cx,
                    if w.ring_label_inside {
                        cy + 12.
                    } else {
                        height - 15.
                    },
                    if w.ring_label_inside { 13. } else { 10. },
                    if w.ring_label_inside { white } else { label },
                );
            }
        }
        Kind::Line => {
            if w.show_label {
                paint_text(img, &w.label, 0, 0, 10., label);
            }
            let top = if w.show_label { 22. } else { 1. };
            for n in 0..3 {
                let y = top + (height - top - 2.) * n as f32 / 2.;
                draw_line_segment_mut(img, (0., y), (width, y), Rgba(w.background));
            }
            if let Some(h) = d.histories.get(&w.binding) {
                for i in 1..h.len() {
                    let x1 = (i - 1) as f32 / (h.len() - 1).max(1) as f32 * (width - 2.);
                    let x2 = i as f32 / (h.len() - 1).max(1) as f32 * (width - 2.);
                    let yy = |n: f64| {
                        height
                            - 2.
                            - ((n - w.min) / (w.max - w.min).max(0.001)).clamp(0., 1.) as f32
                                * (height - top - 3.)
                    };
                    draw_line_segment_mut(img, (x1, yy(h[i - 1])), (x2, yy(h[i])), Rgba(w.color));
                }
            }
        }
        Kind::Status => {
            let on = num.map(|n| n > w.min).unwrap_or_else(|| {
                ["on", "true", "home", "open", "online"].contains(&value.to_lowercase().as_str())
            });
            draw_filled_circle_mut(
                img,
                (10, 15),
                6,
                Rgba(if on { w.color } else { [104, 118, 126, 255] }),
            );
            paint_text(img, &display, 25, 0, w.size.min(23.), w.color);
            if w.show_label {
                paint_text(img, &w.label, 25, 25, 10., label);
            }
        }
    }
}
pub fn trend(w: &Widget, d: &DataState) -> Option<bool> {
    if w.binding.is_empty() {
        return d.number(w).filter(|n| *n != 0.).map(|n| n > 0.);
    }
    let history = d.histories.get(&w.binding)?;
    let values = history.get(history.len().checked_sub(2)?..)?;
    if values[0] == values[1] {
        None
    } else {
        Some(values[1] > values[0])
    }
}
pub fn png(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut c = std::io::Cursor::new(Vec::new());
    img.write_to(&mut c, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(c.into_inner())
}
pub fn rgb565(img: &RgbaImage) -> Vec<u8> {
    img.pixels()
        .flat_map(|p| original::rgb565_pixel(p[0], p[1], p[2]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imported_blank_columns_keep_the_saved_origin_when_resized() {
        let font = crate::fonts::resolve("Microsoft YaHei Bold", None).unwrap();
        let metrics = LegacyText {
            cell_width: 20.,
            cell_height: 26.,
            advance: 24.,
            base_size: 20.,
        };
        for size in [20., 40.] {
            let single = text_line("8", size, [255; 4], &font, Some(&metrics), 0.);
            let padded = text_line("  8 ", size, [255; 4], &font, Some(&metrics), 0.);
            let step = (24. * size / 20.) as u32;
            assert_eq!(ink_bounds(&padded).0, ink_bounds(&single).0 + 2 * step);
            assert_eq!(padded.width(), single.width() + 3 * step);
        }

        let mut widget = Widget::new(1, Kind::Value, 0., 0.);
        widget.show_label = false;
        widget.binding = "cpu.power".into();
        widget.decimals = Some(0);
        widget.size = 20.;
        widget.w = 120.;
        widget.h = 28.;
        widget.legacy_text = Some(metrics);
        let mut data = DataState::default();
        data.values.insert("cpu.power".into(), "8".into());
        let small = widget_tile(&widget, &data);
        widget.w = 180.;
        let large = widget_tile(&widget, &data);
        assert_eq!(ink_bounds(&small), ink_bounds(&large));
        assert_eq!(ink_bounds(&small).0, 48);
        let restored: Widget =
            serde_json::from_str(&serde_json::to_string(&widget).unwrap()).unwrap();
        assert!(large == widget_tile(&restored, &data));
    }

    #[test]
    fn bound_prefix_and_suffix_work_for_numbers_text_and_unavailable_values() {
        let mut w = Widget::new(1, Kind::Value, 0., 0.);
        w.w = 280.;
        w.show_label = false;
        w.binding = "source:1:/price".into();
        w.unit_prefix = "$".into();
        w.unit = " USD".into();
        let mut d = DataState::default();
        for raw in ["42", "Ready", "N/A"] {
            d.values.insert(w.binding.clone(), raw.into());
            let mut expected = w.clone();
            expected.binding.clear();
            expected.text = format!("${raw} USD");
            assert!(widget_tile(&w, &d) == widget_tile(&expected, &d));
        }
        w.binding.clear();
        w.text = "42".into();
        let mut without_units = w.clone();
        without_units.unit.clear();
        without_units.unit_prefix.clear();
        assert!(widget_tile(&w, &d) == widget_tile(&without_units, &d));
    }
    #[test]
    fn circular_ticker_joins_tail_to_head_in_both_directions_and_any_field_width() {
        let colors = [
            [240, 0, 0, 255],
            [0, 240, 0, 255],
            [0, 0, 240, 255],
            [240, 240, 0, 255],
        ];
        let line = RgbaImage::from_fn(4, 1, |x, _| Rgba(colors[x as usize]));
        let mut w = Widget::new(1, Kind::Ticker, 0., 0.);
        w.h = 2.;
        w.scroll_speed = 10.;
        w.ticker_circular = true;
        for width in [2, 4, 10] {
            w.w = width as f32;
            for direction in [ScrollDirection::RightToLeft, ScrollDirection::LeftToRight] {
                w.scroll_direction = direction;
                for step in 0..=8 {
                    // One pixel per step, including either side of two complete wraps.
                    let time = step as f64 / (width + 4) as f64;
                    let tile = ticker_tile(&w, &line, time);
                    for x in 0..width {
                        let index = match direction {
                            ScrollDirection::RightToLeft => x + step,
                            ScrollDirection::LeftToRight => x + 8 - step,
                        } % 4;
                        assert_eq!(tile.get_pixel(x, 0).0, colors[index as usize]);
                        assert_eq!(tile.get_pixel(x, 1).0, [0; 4]);
                    }
                }
            }
        }
        // With circular off, the original passage still starts outside the field.
        w.ticker_circular = false;
        assert!(ticker_tile(&w, &line, 0.).pixels().all(|p| p[3] == 0));
    }

    #[test]
    fn circular_ticker_interpolates_transparency_across_the_wrap() {
        let line = RgbaImage::from_fn(2, 1, |x, _| {
            Rgba(if x == 0 {
                [200, 0, 0, 128]
            } else {
                [0, 0, 200, 255]
            })
        });
        let mut w = Widget::new(1, Kind::Ticker, 0., 0.);
        w.w = 2.;
        w.h = 1.;
        w.scroll_speed = 10.;
        w.ticker_circular = true;
        for direction in [ScrollDirection::RightToLeft, ScrollDirection::LeftToRight] {
            w.scroll_direction = direction;
            for time in [0.125, 0.375, 0.625] {
                let tile = ticker_tile(&w, &line, time);
                assert!(tile.pixels().all(|p| p.0 == [67, 0, 133, 192]));
            }
        }
    }

    #[test]
    fn circular_ticker_cache_handles_toggling_data_updates_and_stationary_text() {
        let mut w = Widget::new(1, Kind::Ticker, 0., 0.);
        w.w = 280.;
        w.h = 36.;
        w.ticker_bound = true;
        w.ticker_sources = vec![TickerSource {
            binding: "weather".into(),
            ..Default::default()
        }];
        let mut p = Project::blank();
        p.widgets.push(w);
        let mut d = DataState::default();
        let mut cache = FrameCache::default();
        for circular in [true, false, true] {
            p.widgets[0].ticker_circular = circular;
            for text in ["Sunny", "Rain", ""] {
                d.values.insert("weather".into(), text.into());
                for direction in [ScrollDirection::RightToLeft, ScrollDirection::LeftToRight] {
                    p.widgets[0].scroll_direction = direction;
                    for speed in [0., 0.02, 10.] {
                        p.widgets[0].scroll_speed = speed;
                        for time in [0., 0.4375, 1., 250.001] {
                            assert_eq!(cache.frame_at(&p, &d, time), frame_at(&p, &d, time));
                        }
                    }
                }
            }
        }
        // Zero speed keeps one ordinary static copy, even with Circular selected.
        p.widgets[0].ticker_bound = false;
        p.widgets[0].text = "Static text".into();
        p.widgets[0].scroll_speed = 0.;
        let still = cache.frame_at(&p, &d, 0.);
        assert!(!has_animation(&p));
        assert_eq!(still, cache.frame_at(&p, &d, 10.));
        p.widgets[0].ticker_circular = false;
        assert_eq!(still, cache.frame_at(&p, &d, 10.));
        p.widgets[0].ticker_circular = true;
        p.widgets[0].scroll_speed = 10.;
        assert_eq!(cache.frame_at(&p, &d, 0.4375), frame_at(&p, &d, 0.4375));
    }

    #[test]
    fn ticker_moves_between_old_30hz_steps_and_preserves_fractional_alpha() {
        let mut w = Widget::new(1, Kind::Ticker, 0., 0.);
        w.w = 280.;
        w.h = 36.;
        w.text = "Smooth ticker 123".into();
        w.scroll_speed = 0.02;
        let d = DataState::default();
        let line = ticker_line(&w, &d);
        let mut p = Project::blank();
        p.widgets.push(w.clone());
        for direction in [ScrollDirection::RightToLeft, ScrollDirection::LeftToRight] {
            p.widgets[0].scroll_direction = direction;
            let mut cache = FrameCache::default();
            // Less than a pixel of motion over four frames, all in the same old 30Hz bucket.
            let images: Vec<_> = (0..4)
                .map(|i| cache.frame_at(&p, &d, 250.001 + i as f64 / 240.))
                .collect();
            assert!(images.windows(2).all(|pair| pair[0] != pair[1]));
            let cached = &cache.ticker_lines[&1].1;
            assert_eq!(cached, &line);
            for i in 0..4 {
                assert_eq!(images[i], frame_at(&p, &d, 250.001 + i as f64 / 240.));
            }
        }
        let mut opaque = RgbaImage::from_pixel(4, 2, Rgba([100, 200, 80, 255]));
        opaque.put_pixel(0, 0, Rgba([100, 200, 80, 128]));
        w.w = 4.;
        w.h = 2.;
        w.scroll_speed = 10.;
        // width 4 + strip 4; t=.4375 => x=.5.
        let tile = ticker_tile(&w, &opaque, 0.4375);
        assert_eq!(tile.get_pixel(0, 0).0, [100, 200, 80, 64]);
        assert_eq!(tile.get_pixel(2, 1).0, [100, 200, 80, 255]);
        p.widgets.clear();
        let mut cache = FrameCache::default();
        cache.frame_at(&p, &d, 1.);
        assert!(cache.ticker_lines.is_empty());
    }
    #[test]
    fn ticker_combines_sources_roundtrips_and_animates_in_both_directions() {
        let mut w = Widget::new(1, Kind::Ticker, 12., 12.);
        w.w = 280.;
        w.h = 36.;
        w.ticker_bound = true;
        w.ticker_sources = vec![
            TickerSource {
                binding: "price".into(),
                unit_prefix: "$".into(),
                unit_suffix: " USD".into(),
                decimals: None,
            },
            TickerSource {
                binding: "weather".into(),
                unit_prefix: "Weather: ".into(),
                unit_suffix: "!".into(),
                decimals: None,
            },
        ];
        let mut d = DataState::default();
        d.values.insert("price".into(), "12.5".into());
        d.values.insert("weather".into(), "Sunny".into());
        assert_eq!(ticker_text(&w, &d), "$12.5 USD   •   Weather: Sunny!");
        let mut p = Project::blank();
        p.widgets.push(w.clone());
        let restored: Project = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
        assert!(restored.widgets[0] == w);
        let mut cache = FrameCache::default();
        for direction in [ScrollDirection::RightToLeft, ScrollDirection::LeftToRight] {
            p.widgets[0].scroll_direction = direction;
            p.widgets[0].scroll_speed = 10.;
            for time in [0.25, 0.5, 0.75, 1.25] {
                assert!(cache.frame_at(&p, &d, time) == frame_at(&p, &d, time));
            }
            assert!(frame_at(&p, &d, 0.25) != frame_at(&p, &d, 0.5));
            assert!(frame_at(&p, &d, 0.25) == frame_at(&p, &d, 1.25));
        }
        assert_eq!(
            ticker_x(100., 200., 10., ScrollDirection::RightToLeft, 0.25),
            25.
        );
        assert_eq!(
            ticker_x(100., 200., 10., ScrollDirection::LeftToRight, 0.25),
            -125.
        );
        assert_eq!(
            ticker_x(100., 200., 5., ScrollDirection::RightToLeft, 0.5),
            25.
        );
        p.widgets[0].scroll_speed = 0.;
        assert!(!has_animation(&p));
        let still = cache.frame_at(&p, &d, 0.);
        assert!(still == cache.frame_at(&p, &d, 10.));
        d.values.insert("weather".into(), "Rain".into());
        assert!(still != cache.frame_at(&p, &d, 10.));
        p.widgets[0].ticker_bound = false;
        p.widgets[0].text = "Static text".into();
        assert_eq!(ticker_text(&p.widgets[0], &d), "Static text");
        assert!(widget_tile(&p.widgets[0], &d).pixels().any(|p| p[3] > 0));
    }
    use imageproc::drawing::text_size;
    fn ink_bounds(img: &RgbaImage) -> (u32, u32, u32, u32) {
        let mut b = (img.width(), img.height(), 0, 0);
        for (x, y, p) in img.enumerate_pixels() {
            if p[3] > 0 {
                b.0 = b.0.min(x);
                b.1 = b.1.min(y);
                b.2 = b.2.max(x);
                b.3 = b.3.max(y);
            }
        }
        b
    }
    #[test]
    fn imported_size_scales_letters_and_digits_and_survives_json() {
        let font = crate::fonts::resolve("Microsoft YaHei Bold", None).unwrap();
        let metrics = LegacyText {
            cell_width: 10.,
            cell_height: 13.,
            advance: 9.,
            base_size: 20.,
        };
        let mut sheet = RgbaImage::from_pixel(480, 320, Rgba([10, 25, 31, 255]));
        for (row, content) in ["888.8", "AVATAR gyp", "CPU 19.5 GHz"].iter().enumerate() {
            let small = text_line(content, 20., [255; 4], &font, Some(&metrics), 0.);
            let large = text_line(content, 40., [255; 4], &font, Some(&metrics), 0.);
            let a = ink_bounds(&small);
            let b = ink_bounds(&large);
            assert!(
                (large.width() as i32 - small.width() as i32 * 2).abs() <= 3,
                "width scales: {content}"
            );
            assert!(b.3 - b.1 >= (a.3 - a.1) * 2 - 2, "height scales: {content}");
            image::imageops::overlay(&mut sheet, &small, 12, (row * 90 + 10) as i64);
            image::imageops::overlay(&mut sheet, &large, 180, (row * 90 + 10) as i64);
        }
        sheet.save("evidence/umeter-text-sizing.png").unwrap();
        let mut widget = Widget::new(1, Kind::Value, 100., 20.);
        widget.w = 40.;
        widget.h = 13.;
        widget.size = 20.;
        widget.legacy_text = Some(metrics);
        widget.resize_text(40.);
        assert_eq!((widget.x, widget.w, widget.h), (60., 80., 26.));
        let restored: Widget =
            serde_json::from_slice(&serde_json::to_vec(&widget).unwrap()).unwrap();
        assert_eq!(restored.legacy_text.unwrap().base_size, 20.);
    }
    #[test]
    fn numeric_fields_keep_their_right_edge_and_spacing_is_editable() {
        let font = crate::fonts::resolve("Segoe UI", None).unwrap();
        for content in ["1111", "8888", "AVATAR", "1.25 GHz"] {
            let a = text_line(content, 25., [255; 4], &font, None, 0.);
            let b = text_line(content, 25., [255; 4], &font, None, 2.);
            assert!(b.width() > a.width(), "tracking increases width: {content}");
        }
        for imported in [false, true] {
            let mut widget = Widget::new(1, Kind::Value, 0., 0.);
            widget.show_label = false;
            widget.w = 120.;
            widget.h = 40.;
            if imported {
                widget.legacy_text = Some(LegacyText {
                    cell_width: 10.,
                    cell_height: 13.,
                    advance: 9.,
                    base_size: 25.,
                });
            }
            for value in ["1", "88", "123.4", "N/A"] {
                widget.text = value.into();
                let b = ink_bounds(&widget_tile(&widget, &DataState::default()));
                if imported {
                    // Original TItem draws from its saved posX, independent
                    // of the editor selection rectangle's spare width.
                    widget.w = 160.;
                    let wider = ink_bounds(&widget_tile(&widget, &DataState::default()));
                    assert_eq!(b, wider, "saved origin: {value}");
                    widget.w = 120.;
                } else {
                    assert_eq!(b.2, 119, "editor right edge: {value}");
                }
            }
        }
    }
    #[test]
    fn imported_bar_uses_original_ceiling_and_integer_pixel_length() {
        let mut w = Widget::new(1, Kind::Bar, 0., 0.);
        w.binding = "cpu.usage".into();
        w.show_label = false;
        w.w = 150.;
        w.h = 13.;
        w.legacy_text = Some(LegacyText {
            cell_width: 10.,
            cell_height: 13.,
            advance: 9.,
            base_size: 25.,
        });
        let mut d = DataState::default();
        // Original controller RVA 0x12418 uses truncate(value * .01 * 150).
        // Postprocessor RVA 0x15b78 replaces values above 99 with 99.
        for (value, expected_pixels) in [
            ("0", 0),
            ("0.1", 0),
            ("42", 63),
            ("50", 75),
            ("58", 87),
            ("78", 117),
            ("99", 148),
            ("99.5", 148),
            ("100", 148),
            ("150", 148),
        ] {
            d.values.insert(w.binding.clone(), value.into());
            let img = widget_tile(&w, &d);
            let count = (0..150)
                .filter(|x| img.get_pixel(*x, 6).0 == w.color)
                .count();
            assert_eq!(count, expected_pixels, "reading {value}");
            assert_eq!(d.values[&w.binding], value);
        }
        w.legacy_text = None;
        d.values.insert(w.binding.clone(), "100".into());
        let img = widget_tile(&w, &d);
        assert!((0..150).all(|x| img.get_pixel(x, 6).0 == w.color));
    }

    #[test]
    fn imported_cpu_frequency_bar_uses_eight_ghz_scale() {
        let mut w = Widget::new(1, Kind::Bar, 0., 0.);
        w.binding = "cpu.frequency".into();
        w.show_label = false;
        w.w = 150.;
        w.h = 13.;
        w.legacy_text = Some(LegacyText {
            cell_width: 10.,
            cell_height: 13.,
            advance: 9.,
            base_size: 25.,
        });
        // A saved import from an older build still has the default max=100.
        let w: Widget = serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
        let mut d = DataState::default();
        for (value, expected_pixels) in [
            ("0", 0),
            ("0.001", 0),
            ("0.1", 1),
            ("1", 18),
            ("4", 75),
            ("4.8", 90),
            ("7.999", 149),
            ("8", 150),
            ("9", 150),
            ("-1", 0),
            ("N/A", 0),
            ("NaN", 0),
            ("inf", 0),
            ("-inf", 0),
        ] {
            d.values.insert(w.binding.clone(), value.into());
            let img = widget_tile(&w, &d);
            let count = (0..150)
                .filter(|x| img.get_pixel(*x, 6).0 == w.color)
                .count();
            assert_eq!(count, expected_pixels, "frequency {value} GHz");
            assert_eq!(d.value(&w), value);
        }
        // A newly created widget keeps its editable range.
        let mut w = w;
        w.legacy_text = None;
        w.max = 10.;
        d.values.insert(w.binding.clone(), "4".into());
        let img = widget_tile(&w, &d);
        assert_eq!(
            (0..150)
                .filter(|x| img.get_pixel(*x, 6).0 == w.color)
                .count(),
            60
        );
    }

    #[test]
    fn imported_sensor_text_truncates_while_bars_keep_fractional_readings() {
        for binding in [
            "cpu.usage",
            "cpu.temperature",
            "gpu.usage",
            "gpu.temperature",
            "memory.usage",
            "disk.usage",
            "cpu.power",
            "gpu.memory",
        ] {
            let mut w = Widget::new(1, Kind::Value, 0., 0.);
            w.binding = binding.into();
            w.show_label = false;
            w.decimals = Some(0);
            w.unit = match binding {
                "cpu.temperature" | "gpu.temperature" => "°C",
                "cpu.power" => " W",
                "gpu.memory" => " MB",
                _ => "%",
            }
            .into();
            w.legacy_text = Some(LegacyText {
                cell_width: 16.,
                cell_height: 28.,
                advance: 15.,
                base_size: w.size,
            });
            let mut expected = w.clone();
            expected.binding.clear();
            let mut data = DataState::default();
            for (reading, text) in [
                ("33.9", "33"),
                ("33.99", "33"),
                ("0.9", "0"),
                ("98.99", "98"),
            ] {
                data.values.insert(binding.into(), reading.into());
                let digits = if text == "0" && binding.ends_with("temperature") {
                    "00".to_owned()
                } else {
                    // Literal PadLeft widths from controller RVA0x12418.
                    let columns = match binding {
                        "cpu.power" => 3,
                        "gpu.memory" => 5,
                        _ => 2,
                    };
                    format!("{text:>columns$}")
                };
                // A literal with a suffix bypasses numeric formatting, so this
                // compares the real raster against the exact original text.
                expected.text = format!("{digits}{}", w.unit);
                assert!(
                    widget_tile(&w, &data) == widget_tile(&expected, &data),
                    "{binding}: {reading} must display {digits}"
                );
                assert_eq!(data.number(&w), reading.parse::<f64>().ok());
            }
            if matches!(binding, "cpu.power" | "gpu.memory") {
                // Neither amount has the separate percentage/temperature 99 cap.
                data.values.insert(binding.into(), "1024.99".into());
                let digits = if binding == "gpu.memory" {
                    " 1024"
                } else {
                    "1024"
                };
                expected.text = format!("{digits}{}", w.unit);
                assert!(widget_tile(&w, &data) == widget_tile(&expected, &data));
                assert_eq!(data.number(&w), Some(1024.99));
                // Existing JSON imports retain the recovered formatting rule.
                let restored: Widget =
                    serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
                assert!(widget_tile(&restored, &data) == widget_tile(&expected, &data));
            }
            data.values.insert(binding.into(), "33.9".into());
            w.decimals = Some(1);
            expected.decimals = Some(1);
            expected.text = format!("33.9{}", w.unit);
            assert!(widget_tile(&w, &data) == widget_tile(&expected, &data));
            w.decimals = Some(0);
            w.legacy_text = None;
            expected.decimals = Some(0);
            expected.legacy_text = None;
            expected.text = format!("34{}", w.unit);
            assert!(widget_tile(&w, &data) == widget_tile(&expected, &data));
        }
        let mut bar = Widget::new(1, Kind::Bar, 0., 0.);
        bar.binding = "cpu.usage".into();
        bar.w = 150.;
        bar.h = 13.;
        bar.show_label = false;
        bar.decimals = Some(0);
        bar.legacy_text = Some(LegacyText {
            cell_width: 16.,
            cell_height: 28.,
            advance: 15.,
            base_size: bar.size,
        });
        let mut data = DataState::default();
        data.values.insert("cpu.usage".into(), "33.9".into());
        let image = widget_tile(&bar, &data);
        assert_eq!(
            (0..150)
                .filter(|x| image.get_pixel(*x, 6).0 == bar.color)
                .count(),
            50
        );
    }

    #[test]
    fn imported_integer_sensor_fields_do_not_round_up_fractional_provider_readings() {
        for (binding, suffix, columns) in [
            ("disk.temperature", " °C", 2),
            ("cpu.fan", " RPM", 5),
            ("gpu.fan", "%", 5),
            ("volume.level", "%", 2),
        ] {
            let mut widget = Widget::new(1, Kind::Value, 0., 0.);
            widget.binding = binding.into();
            widget.show_label = false;
            widget.decimals = Some(0);
            widget.unit_prefix = "[".into();
            widget.unit = format!("{suffix}]");
            widget.w = 250.;
            widget.legacy_text = Some(LegacyText {
                cell_width: 16.,
                cell_height: 28.,
                advance: 15.,
                base_size: widget.size,
            });
            // JSON imports saved by earlier builds must get the same rule.
            widget = serde_json::from_str(&serde_json::to_string(&widget).unwrap()).unwrap();
            let mut expected = widget.clone();
            expected.binding.clear();
            let mut data = DataState::default();
            for (raw, digits) in [
                ("33.9", "33"),
                ("0.9", "0"),
                ("98.99", "98"),
                ("0.99999999", "0"),
                ("-0.9", "0"),
            ] {
                data.values.insert(binding.into(), raw.into());
                expected.text = format!("[{digits:>columns$}{suffix}]");
                assert!(
                    widget_tile(&widget, &data) == widget_tile(&expected, &data),
                    "{binding}: {raw} must display {digits}"
                );
                assert_eq!(data.number(&widget), raw.parse::<f64>().ok());
                assert_eq!(data.values[binding], raw);
            }
            // Disk temperature and fan speeds have no recovered 99 ceiling.
            if binding != "volume.level" {
                data.values.insert(binding.into(), "125.9".into());
                expected.text = format!("[{:>columns$}{suffix}]", "125");
                assert!(widget_tile(&widget, &data) == widget_tile(&expected, &data));
            }
            data.values.insert(binding.into(), "33.9".into());
            widget.decimals = Some(1);
            expected.text = format!("[33.9{suffix}]");
            assert!(widget_tile(&widget, &data) == widget_tile(&expected, &data));
            widget.decimals = Some(0);
            widget.legacy_text = None;
            expected.legacy_text = None;
            expected.text = format!("[34{suffix}]");
            assert!(widget_tile(&widget, &data) == widget_tile(&expected, &data));
            widget.legacy_text = Some(LegacyText {
                cell_width: 16.,
                cell_height: 28.,
                advance: 15.,
                base_size: widget.size,
            });
            expected.legacy_text = widget.legacy_text.clone();
            for raw in [None, Some("N/A")] {
                data.values.remove(binding);
                if let Some(raw) = raw {
                    data.values.insert(binding.into(), raw.into());
                }
                expected.text = format!("[N/A{suffix}]");
                assert!(widget_tile(&widget, &data) == widget_tile(&expected, &data));
            }
        }
    }

    #[test]
    fn imported_frequency_text_matches_original_dotnet_formula() {
        let fixtures: serde_json::Value = serde_json::from_str(
            include_str!("../evidence/cpu-frequency-dotnet-fixtures.json")
                .trim_start_matches('\u{feff}'),
        )
        .unwrap();
        let mut widget = Widget::new(1, Kind::Value, 0., 0.);
        widget.binding = "cpu.frequency".into();
        widget.show_label = false;
        widget.decimals = Some(1);
        widget.unit_prefix = "CPU ".into();
        widget.unit = " GHz".into();
        widget.w = 350.;
        widget.legacy_text = Some(LegacyText {
            cell_width: 16.,
            cell_height: 28.,
            advance: 15.,
            base_size: widget.size,
        });
        let mut expected = widget.clone();
        expected.binding.clear();
        let mut data = DataState::default();
        for fixture in fixtures.as_array().unwrap() {
            let mhz = fixture["MHz"].as_f64().unwrap();
            let text = fixture["Text"].as_str().unwrap();
            data.values
                .insert(widget.binding.clone(), (mhz / 1000.).to_string());
            expected.text = format!("CPU {text} GHz");
            assert!(
                widget_tile(&widget, &data) == widget_tile(&expected, &data),
                "{mhz} MHz must display {text}"
            );
            assert_eq!(data.number(&widget), Some(mhz / 1000.));
            let restored: Widget =
                serde_json::from_str(&serde_json::to_string(&widget).unwrap()).unwrap();
            assert!(widget_tile(&restored, &data) == widget_tile(&expected, &data));
        }
        data.values.remove(&widget.binding);
        expected.text = "CPU N/A GHz".into();
        assert!(widget_tile(&widget, &data) == widget_tile(&expected, &data));
        data.values.insert(widget.binding.clone(), "2.9".into());
        widget.decimals = Some(2);
        expected.text = "CPU 2.90 GHz".into();
        assert!(widget_tile(&widget, &data) == widget_tile(&expected, &data));
        widget.decimals = Some(1);
        widget.legacy_text = None;
        expected.legacy_text = None;
        expected.text = "CPU 2.9 GHz".into();
        assert!(widget_tile(&widget, &data) == widget_tile(&expected, &data));
    }

    #[test]
    fn imported_network_text_matches_original_tenths_and_thresholds() {
        for binding in ["network.download", "network.upload"] {
            let mut widget = Widget::new(1, Kind::Value, 0., 0.);
            widget.binding = binding.into();
            widget.show_label = false;
            widget.decimals = Some(1);
            widget.unit_prefix = "Rate ".into();
            widget.unit = " MiB/s".into();
            widget.w = 350.;
            widget.legacy_text = Some(LegacyText {
                cell_width: 16.,
                cell_height: 28.,
                advance: 15.,
                base_size: widget.size,
            });
            let mut expected = widget.clone();
            expected.binding.clear();
            let mut data = DataState::default();
            // Full sampler0x113dc -> controller0x12418 path. The sampler
            // raises every positive rate below0.1MiB/s before formatting.
            for (rate, normalized, text) in [
                (-1., 0., "0.0"),
                (0., 0., "0.0"),
                (0.000001, 0.1, "0.1"),
                (0.009, 0.1, "0.1"),
                (0.01, 0.1, "0.1"),
                (0.01001, 0.1, "0.1"),
                (0.099, 0.1, "0.1"),
                (0.1, 0.1, "0.1"),
                (1.29, 1.29, "1.2"),
                (12.39, 12.39, "12.3"),
                (98.99, 98.99, "98.9"),
                (99., 99., "99.0"),
                (99.0001, 99.0001, "99.9"),
                (120., 120., "99.9"),
            ] {
                // Shared bindings use decimal MB/s; imported fields use MiB/s.
                let shared = rate * 1024. * 1024. / 1_000_000.;
                data.values.insert(binding.into(), shared.to_string());
                expected.text = format!("Rate {text:>4} MiB/s");
                assert!(
                    widget_tile(&widget, &data) == widget_tile(&expected, &data),
                    "{binding}: {rate} MiB/s must display {text}"
                );
                assert!((data.number(&widget).unwrap() - normalized).abs() < 1e-10);
                assert_eq!(data.values[binding], shared.to_string());
            }
            let restored: Widget =
                serde_json::from_str(&serde_json::to_string(&widget).unwrap()).unwrap();
            assert!(widget_tile(&restored, &data) == widget_tile(&expected, &data));
            data.values.remove(binding);
            expected.text = "Rate N/A MiB/s".into();
            assert!(widget_tile(&widget, &data) == widget_tile(&expected, &data));

            let shared = 1.29 * 1024. * 1024. / 1_000_000.;
            data.values.insert(binding.into(), shared.to_string());
            widget.decimals = Some(2);
            expected.text = "Rate 1.29 MiB/s".into();
            assert!(widget_tile(&widget, &data) == widget_tile(&expected, &data));
            widget.decimals = Some(1);
            widget.legacy_text = None;
            expected.legacy_text = None;
            expected.text = "Rate 1.4 MiB/s".into();
            assert!(widget_tile(&widget, &data) == widget_tile(&expected, &data));
        }
    }

    #[test]
    fn text_binding_renders_the_live_value_and_suffix() {
        let mut p = Project::blank();
        let mut w = Widget::new(1, Kind::Text, 0., 0.);
        w.binding = "cpu.usage".into();
        w.unit = "%".into();
        w.text = "old text".into();
        w.decimals = Some(0);
        p.widgets.push(w);
        let mut d = DataState::default();
        d.values.insert("cpu.usage".into(), "42".into());
        let bound = frame(&p, &d);
        p.widgets[0].binding.clear();
        p.widgets[0].text = "42%".into();
        // Numeric text now aligns to the same right edge as a value field.
        p.widgets[0].kind = Kind::Value;
        p.widgets[0].show_label = false;
        assert!(bound == frame(&p, &d));
    }
    #[test]
    fn shapes_support_independent_fill_and_outline_and_lines_have_direction() {
        for kind in [
            Kind::Rectangle,
            Kind::RoundedRectangle,
            Kind::Circle,
            Kind::Oval,
            Kind::Polygon,
        ] {
            let mut w = Widget::new(1, kind, 0., 0.);
            w.w = 40.;
            w.h = 40.;
            w.fill_enabled = false;
            w.outline_enabled = false;
            w.stroke_width = 3.;
            w.corner_radius = 5.;
            let mut img = RgbaImage::new(40, 40);
            draw_widget(&mut img, &w, &DataState::default(), 0.);
            assert!(img.pixels().all(|p| p[3] == 0));
            w.outline_enabled = true;
            draw_widget(&mut img, &w, &DataState::default(), 0.);
            assert_eq!(img.get_pixel(20, 20)[3], 0);
            assert!(img.pixels().any(|p| p[3] > 0));
            w.fill_enabled = true;
            draw_widget(&mut img, &w, &DataState::default(), 0.);
            assert_eq!(img.get_pixel(20, 20).0, w.color);
        }
        let mut w = Widget::new(1, Kind::StraightLine, 0., 0.);
        w.outline_enabled = true;
        w.line_direction = LineDirection::Diagonal;
        for (width, height) in [(40, 1), (1, 40), (40, 40)] {
            w.w = width as f32;
            w.h = height as f32;
            let mut img = RgbaImage::new(width, height);
            draw_widget(&mut img, &w, &DataState::default(), 0.);
            assert!(img.pixels().any(|p| p[3] > 0));
        }
        let mut a = RgbaImage::new(40, 40);
        draw_widget(&mut a, &w, &DataState::default(), 0.);
        w.line_reverse = true;
        let mut b = RgbaImage::new(40, 40);
        draw_widget(&mut b, &w, &DataState::default(), 0.);
        assert_ne!(a, b);
    }
    #[test]
    fn rotated_shapes_keep_corners_and_use_partial_edge_coverage() {
        let data = DataState::default();
        for kind in [
            Kind::Rectangle,
            Kind::RoundedRectangle,
            Kind::Circle,
            Kind::Oval,
            Kind::Polygon,
            Kind::StraightLine,
        ] {
            let mut w = Widget::new(1, kind, 80., 80.);
            w.w = 60.;
            w.h = 24.;
            w.rotation_degrees = 32.;
            w.outline_enabled = true;
            w.stroke_width = 2.;
            let (x, y, width, height) = widget_bounds(&w);
            let tile = widget_tile(&w, &data);
            assert!(
                tile.pixels().any(|p| p[3] > 0 && p[3] < 255),
                "edges must be antialiased"
            );
            assert_eq!(tile.dimensions(), (width as u32, height as u32));
            assert!((x + width / 2. - 110.).abs() < 1.);
            assert!((y + height / 2. - 92.).abs() < 1.);
            let mut project = Project::blank();
            project.widgets.push(w);
            assert_eq!(
                FrameCache::default().frame(&project, &data),
                frame(&project, &data)
            );
        }
        let mut line = Widget::new(2, Kind::StraightLine, 0., 0.);
        line.w = 40.;
        line.h = 40.;
        line.outline_enabled = true;
        line.stroke_width = 2.;
        let horizontal = widget_tile(&line, &data);
        assert!(horizontal.get_pixel(10, 20)[3] > 0);
        assert_eq!(horizontal.get_pixel(20, 10)[3], 0);
        line.line_direction = LineDirection::Vertical;
        let vertical = widget_tile(&line, &data);
        assert!(vertical.get_pixel(20, 10)[3] > 0);
        assert_eq!(vertical.get_pixel(10, 20)[3], 0);
        line.rotation_degrees = 90.;
        let encoded = serde_json::to_vec(&line).unwrap();
        let restored: Widget = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(widget_tile(&line, &data), widget_tile(&restored, &data));
    }
    #[test]
    fn cached_frames_preserve_overlaps_and_remove_moved_or_deleted_layers() {
        let mut p = Project::blank();
        let mut a = Widget::new(1, Kind::Rectangle, 12., 12.);
        a.w = 40.;
        a.h = 40.;
        p.widgets.push(a);
        let mut b = Widget::new(2, Kind::Text, 15., 15.);
        b.binding = "test".into();
        p.widgets.push(b);
        let mut cache = FrameCache::default();
        let mut d = DataState::default();
        d.values.insert("test".into(), "one".into());
        assert_eq!(cache.frame(&p, &d), frame(&p, &d));
        d.values.insert("test".into(), "two".into());
        p.widgets[0].x = 30.;
        assert_eq!(cache.frame(&p, &d), frame(&p, &d));
        p.widgets.remove(0);
        assert_eq!(cache.frame(&p, &d), frame(&p, &d));
        p.widgets[0].visible = false;
        assert_eq!(cache.frame(&p, &d), frame(&p, &d));
    }
    #[test]
    fn technology_template_roundtrips_and_renders_live_gauges() {
        let p = Project::landscape_technology();
        assert_eq!(p.dimensions(), (480, 320));
        let saved = serde_json::to_vec(&p).unwrap();
        let restored: Project = serde_json::from_slice(&saved).unwrap();
        let mut ids = std::collections::BTreeSet::new();
        let mut clipped = vec![];
        for w in &restored.widgets {
            assert!(ids.insert(w.id), "duplicate layer id");
            assert!(
                w.x >= 0. && w.y >= 0. && w.x + w.w <= 480. && w.y + w.h <= 320.,
                "{} is outside the screen",
                w.name
            );
            if w.kind == Kind::Text {
                if let Some(font) = crate::fonts::resolve(&w.font, w.font_data.as_deref()) {
                    let (width, _) = text_size(w.size, &font, &w.text);
                    if width as f32 > w.w {
                        clipped.push(format!("{} needs {} px but has {} px", w.name, width, w.w));
                    }
                }
            }
        }
        assert!(clipped.is_empty(), "{}", clipped.join("; "));
        assert_eq!(p.widgets.iter().filter(|w| w.kind == Kind::Ring).count(), 3);
        let mut data = DataState::default();
        // Values from the supplied picture are fixtures, never runtime defaults.
        for (key, value) in [
            ("cpu.usage", "87"),
            ("gpu.usage", "10"),
            ("memory.usage", "47"),
            ("cpu.temperature", "44"),
            ("cpu.frequency", "4.8"),
            ("gpu.temperature", "59"),
            ("gpu.memory", "83325"),
            ("disk.temperature", "46"),
            ("disk.usage", "27"),
            ("network.download", "80.1"),
            ("network.upload", "80.1"),
            ("volume.level", "58"),
            ("weather.summary", "晴 西南风1级, -2°"),
        ] {
            data.values.insert(key.into(), value.into());
        }
        let image = frame(&restored, &data);
        data.values.insert("cpu.usage".into(), "0".into());
        let zero = frame(&restored, &data);
        assert_ne!(image, zero, "gauge must respond to its live binding");
        assert_eq!(image.get_pixel(5, 65).0, [105, 127, 140, 255]);
        if let Some(path) = std::env::var_os("COCKPIT_REFERENCE_PREVIEW") {
            image.save(path).unwrap();
        }
    }
    #[test]
    fn older_designs_keep_their_gauge_and_clock_defaults() {
        let mut value = serde_json::to_value(Widget::new(1, Kind::Ring, 0., 0.)).unwrap();
        for key in [
            "decimals",
            "ring_thickness",
            "ring_start",
            "ring_label_inside",
            "clock_format",
        ] {
            value.as_object_mut().unwrap().remove(key);
        }
        let restored: Widget = serde_json::from_value(value).unwrap();
        assert_eq!(restored.ring_thickness, 5.);
        assert_eq!(restored.ring_start, -90.);
        assert!(!restored.ring_label_inside);
        assert_eq!(restored.decimals, None);
        assert!(restored.clock_format.is_empty());
    }
    #[test]
    fn screen_render_is_populated_in_both_orientations() {
        let mut p = Project::starter();
        let mut d = DataState::default();
        d.values.insert("cpu.usage".into(), "42".into());
        for portrait in [false, true] {
            p.portrait = portrait;
            let img = frame(&p, &d);
            assert_eq!(img.dimensions(), p.dimensions());
            assert!(img.pixels().filter(|px| px.0 != p.background).count() > 1000);
            assert_eq!(rgb565(&img).len(), 480 * 320 * 2);
        }
    }
    #[test]
    fn rgb565_known_colors() {
        let mut img = RgbaImage::new(3, 1);
        for (x, c) in [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]]
            .iter()
            .enumerate()
        {
            img.put_pixel(x as u32, 0, Rgba(*c));
        }
        assert_eq!(rgb565(&img), vec![0, 248, 224, 7, 31, 0]);
    }
    #[test]
    fn imported_arc_keeps_original_segments_and_has_no_added_center_text() {
        let mut w = Widget::new(1, Kind::Ring, 0., 0.);
        w.legacy_arc = true;
        w.show_label = false;
        w.ring_thickness = 5.;
        w.ring_start = -54.;
        w.binding = "cpu.usage".into();
        let paint = |value: &str| {
            let mut img = RgbaImage::new(103, 103);
            let mut data = DataState::default();
            data.values.insert("cpu.usage".into(), value.into());
            draw_widget(&mut img, &w, &data, 0.);
            img
        };
        let zero = paint("0");
        assert!(!zero.pixels().any(|p| p.0 == w.color));
        let full = paint("100");
        assert!(full.pixels().filter(|p| p.0 == w.color).count() > 1000);
        assert_eq!(full.get_pixel(51, 51).0, [0, 0, 0, 0]);
        assert_eq!(paint("29"), paint("29.9"));
        assert_ne!(paint("29"), paint("30"));
        assert_eq!(paint("-1"), zero);
        assert_eq!(paint("101"), full);
    }
    #[test]
    fn imported_arc_interval_gaps_render_and_invalidate_the_cache() {
        let mut w = Widget::new(1, Kind::Ring, 10., 10.);
        w.legacy_arc = true;
        w.legacy_arc_gaps = true;
        w.show_label = false;
        w.w = 200.;
        w.h = 200.;
        w.ring_thickness = 10.;
        w.binding = "cpu.usage".into();
        w.color = [255, 60, 0, 255];
        w.background = [0, 30, 100, 255];
        let mut data = DataState::default();
        data.values.insert(w.binding.clone(), "99".into());
        for start in [-54., 0., 90.] {
            w.ring_start = start;
            let tile = widget_tile(&w, &data);
            // Sample the centers of known 3.6-degree slices, away from edges.
            for (slice, foreground) in [
                (0, false),
                (1, true),
                (3, true),
                (4, false),
                (5, true),
                (95, true),
                (96, false),
                (97, true),
                (98, true),
                (99, false),
            ] {
                let angle = (start + (slice as f32 + 0.5) * 3.6).to_radians();
                let x = (100. + 95. * angle.cos()).round() as u32;
                let y = (100. + 95. * angle.sin()).round() as u32;
                assert_eq!(
                    tile.get_pixel(x, y).0,
                    if foreground { w.color } else { w.background },
                    "slice {slice}, start {start}"
                );
            }
            data.values.insert(w.binding.clone(), "29.9".into());
            let fractional = widget_tile(&w, &data);
            data.values.insert(w.binding.clone(), "29".into());
            assert!(fractional == widget_tile(&w, &data));
            data.values.insert(w.binding.clone(), "99".into());
        }
        let mut p = Project::blank();
        p.widgets.push(w.clone());
        let mut cache = FrameCache::default();
        let gapped = cache.frame_at(&p, &data, 0.);
        assert!(gapped == frame_at(&p, &data, 0.));
        p.widgets[0].legacy_arc_gaps = false;
        let solid = cache.frame_at(&p, &data, 0.);
        assert!(solid != gapped);
        assert!(solid == frame_at(&p, &data, 0.));
        // Earlier JSON has no gap field and must retain its solid appearance.
        let mut saved = serde_json::to_value(&w).unwrap();
        saved.as_object_mut().unwrap().remove("legacy_arc_gaps");
        let restored: Widget = serde_json::from_value(saved).unwrap();
        assert!(!restored.legacy_arc_gaps);
        assert!(widget_tile(&restored, &data) == widget_tile(&p.widgets[0], &data));
        // Normal editor rings are continuous even if old/custom JSON has the flag.
        w.legacy_arc = false;
        let ordinary = widget_tile(&w, &data);
        w.legacy_arc_gaps = false;
        assert!(ordinary == widget_tile(&w, &data));
    }
    #[test]
    fn imported_punctuation_and_spaces_keep_the_saved_glyph_interval() {
        let font = crate::fonts::resolve("Segoe UI", None).expect("Windows UI font");
        let metrics = LegacyText {
            cell_width: 22.,
            cell_height: 28.,
            advance: 34.,
            base_size: 40.,
        };
        // FontConfig survives saving an imported design as JSON.
        let metrics: LegacyText =
            serde_json::from_slice(&serde_json::to_vec(&metrics).unwrap()).unwrap();
        for size in [20., 40., 60.] {
            for spacing in [0., 2.] {
                let paint = |text| text_line(text, size, [255; 4], &font, Some(&metrics), spacing);
                let digit = paint("8");
                let step = (metrics.advance * size / metrics.base_size + spacing) as u32;
                for text in ["8.8", "8:8", "8/8", "8 8", "8A8", "8V8"] {
                    let line = paint(text);
                    assert_eq!(
                        line.width(),
                        digit.width() + 2 * step,
                        "{text}, size {size}, extra spacing {spacing}"
                    );
                    for offset in [0, 2 * step] {
                        assert_eq!(
                            image::imageops::crop_imm(
                                &line,
                                offset,
                                0,
                                digit.width(),
                                digit.height()
                            )
                            .to_image(),
                            digit,
                            "glyph changed at {offset} in {text}"
                        );
                    }
                }
            }
        }
        let ordinary = |text| text_line(text, 40., [255; 4], &font, None, 0.);
        assert!(ordinary("8.8").width() < ordinary("888").width());
    }
    #[test]
    fn legacy_text_clips_without_changing_saved_glyph_size_or_advance() {
        let font = crate::fonts::resolve("Segoe UI", None);
        assert!(
            font.is_some(),
            "a font is required to verify legacy glyph placement"
        );
        let metrics = LegacyText {
            cell_width: 22.,
            cell_height: 28.,
            advance: 20.,
            base_size: 40.,
        };
        let paint = |width| {
            let mut image = RgbaImage::new(width, 28);
            legacy_text(
                &mut image,
                "888",
                0,
                0,
                40.,
                [255, 255, 255, 255],
                &font,
                &metrics,
            );
            image
        };
        let full = paint(100);
        assert!(full.pixels().any(|p| p[3] > 0));
        assert_eq!(
            paint(30),
            image::imageops::crop_imm(&full, 0, 0, 30, 28).to_image()
        );
    }
}
