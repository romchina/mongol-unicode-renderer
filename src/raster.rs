use crate::{Bounds, Error, Font, Layout, ShapedRun};
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};

const MAX_DIMENSION: u32 = 8192;
const MAX_PIXELS: u64 = 16_777_216;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterMode {
    /// Whole-run antialiasing followed by box downsampling; samples in [1, 4].
    Smooth { samples: u8 },
    /// No AA, supersampling, or rescaling. Use your font's native grid multiples.
    Pixel,
}

#[derive(Debug, Clone)]
pub struct RasterOptions {
    pub mode: RasterMode,
    pub padding: u32,
    /// Straight-alpha sRGB RGBA8 tint.
    pub color: [u8; 4],
}

impl Default for RasterOptions {
    fn default() -> Self {
        Self {
            mode: RasterMode::Smooth { samples: 4 },
            padding: 2,
            color: [0, 0, 0, 255],
        }
    }
}

/// CPU texture suitable for GPU upload. Pixels are straight-alpha RGBA8, not
/// tiny-skia's premultiplied format; no engine-specific asset type is required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasterImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

fn checked_image(width: u32, height: u32) -> Result<usize, Error> {
    let pixels = u64::from(width) * u64::from(height);
    if width == 0
        || height == 0
        || width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || pixels > MAX_PIXELS
    {
        return Err(Error::ImageTooLarge);
    }
    Ok((pixels * 4) as usize)
}

fn dimensions(run: &ShapedRun, options: &RasterOptions) -> Result<(u32, u32, u32), Error> {
    if options.padding > 128 {
        return Err(Error::InvalidOptions("padding must be <= 128"));
    }
    let samples = match options.mode {
        RasterMode::Pixel => 1,
        RasterMode::Smooth {
            samples: samples @ 1..=4,
        } => u32::from(samples),
        _ => return Err(Error::InvalidOptions("smooth samples must be in [1, 4]")),
    };
    let (w, h) = run.vertical_ink_size();
    let available = (MAX_DIMENSION - options.padding * 2) as f32;
    if !w.is_finite() || !h.is_finite() || w.ceil() > available || h.ceil() > available {
        return Err(Error::ImageTooLarge);
    }
    let width = w.ceil().max(1.0) as u32 + options.padding * 2;
    let height = h.ceil().max(1.0) as u32 + options.padding * 2;
    checked_image(width, height)?;
    checked_image(width * samples, height * samples)?;
    Ok((width, height, samples))
}

/// The Mongolian text matrix is x_screen = y_font and y_screen = x_font.
/// Negating y_font would mirror the letters, although column order still looks right.
struct VerticalOutline<'a> {
    builder: &'a mut PathBuilder,
    x: f32,
    y: f32,
    bounds: Bounds,
    scale: f32,
    samples: f32,
    padding: f32,
}

impl VerticalOutline<'_> {
    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        (
            (y * self.scale + self.y - self.bounds.min_y + self.padding) * self.samples,
            (x * self.scale + self.x - self.bounds.min_x + self.padding) * self.samples,
        )
    }
}

impl ttf_parser::OutlineBuilder for VerticalOutline<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.point(x, y);
        self.builder.move_to(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.point(x, y);
        self.builder.line_to(x, y);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (x1, y1) = self.point(x1, y1);
        let (x, y) = self.point(x, y);
        self.builder.quad_to(x1, y1, x, y);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (x1, y1) = self.point(x1, y1);
        let (x2, y2) = self.point(x2, y2);
        let (x, y) = self.point(x, y);
        self.builder.cubic_to(x1, y1, x2, y2, x, y);
    }
    fn close(&mut self) {
        self.builder.close();
    }
}

/// Fill all glyph outlines in one path, avoiding per-glyph atlas-edge seams.
/// Actual outline bounds determine texture size, including negative bearings.
pub fn render_column(
    font: &Font<'_>,
    run: &ShapedRun,
    options: &RasterOptions,
) -> Result<RasterImage, Error> {
    if run.font_id != font.id {
        return Err(Error::FontMismatch);
    }
    let (width, height, samples) = dimensions(run, options)?;
    let mut rgba = vec![0; checked_image(width, height)?];
    let Some(bounds) = run.bounds else {
        return Ok(RasterImage {
            width,
            height,
            rgba,
        });
    };
    let mut builder = PathBuilder::new();
    let scale = run.font_size / f32::from(font.face.units_per_em());
    for glyph in &run.glyphs {
        let mut outline = VerticalOutline {
            builder: &mut builder,
            x: glyph.x,
            y: glyph.y,
            bounds,
            scale,
            samples: samples as f32,
            padding: options.padding as f32,
        };
        font.face
            .outline_glyph(ttf_parser::GlyphId(glyph.id), &mut outline);
    }
    let path = builder.finish().ok_or(Error::RasterizationFailed)?;
    let mut pixmap = Pixmap::new(width * samples, height * samples).ok_or(Error::ImageTooLarge)?;
    let mut paint = Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);
    paint.anti_alias = matches!(options.mode, RasterMode::Smooth { .. });
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    for y in 0..height {
        for x in 0..width {
            let mut coverage = 0u32;
            for sy in 0..samples {
                for sx in 0..samples {
                    let index = (((y * samples + sy) * width * samples + x * samples + sx) * 4 + 3)
                        as usize;
                    coverage += u32::from(pixmap.data()[index]);
                }
            }
            let alpha = (coverage / (samples * samples) * u32::from(options.color[3]) / 255) as u8;
            let index = ((y * width + x) * 4) as usize;
            // Keep RGB unpremultiplied for callers using standard alpha blending.
            rgba[index..index + 4].copy_from_slice(&[
                options.color[0],
                options.color[1],
                options.color[2],
                alpha,
            ]);
        }
    }
    Ok(RasterImage {
        width,
        height,
        rgba,
    })
}

/// Compose columns at integer positions in left-to-right order. No clipping,
/// scaling, or automatic pixel-font size adjustment is performed.
pub fn render_layout(
    font: &Font<'_>,
    layout: &Layout,
    options: &RasterOptions,
) -> Result<RasterImage, Error> {
    // Check the entire canvas before allocating any per-column raster buffers.
    let mut sizes = Vec::new();
    let mut width = 0u32;
    let mut height = 1;
    for column in layout.columns() {
        if column.run().font_id != font.id {
            return Err(Error::FontMismatch);
        }
        let (w, h, _) = dimensions(column.run(), options)?;
        if !sizes.is_empty() {
            width = width
                .checked_add(layout.column_gap())
                .ok_or(Error::ImageTooLarge)?;
        }
        width = width.checked_add(w).ok_or(Error::ImageTooLarge)?;
        height = height.max(h);
        sizes.push((w, h));
    }
    let width = width.max(1);
    let mut rgba = vec![0; checked_image(width, height)?];
    let mut x = 0u32;
    for (column, (w, h)) in layout.columns().iter().zip(sizes) {
        let image = render_column(font, column.run(), options)?;
        for y in 0..h {
            let start = ((y * width + x) * 4) as usize;
            let src = (y * w * 4) as usize;
            rgba[start..start + (w * 4) as usize]
                .copy_from_slice(&image.rgba[src..src + (w * 4) as usize]);
        }
        x += w + layout.column_gap();
    }
    Ok(RasterImage {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_matrix_is_not_mirrored() {
        let mut builder = PathBuilder::new();
        let outline = VerticalOutline {
            builder: &mut builder,
            x: 4.0,
            y: 7.0,
            bounds: Bounds {
                min_x: -2.0,
                min_y: -3.0,
                max_x: 40.0,
                max_y: 20.0,
            },
            scale: 1.0,
            samples: 1.0,
            padding: 2.0,
        };
        assert_eq!(outline.point(10.0, 8.0), (20.0, 18.0));
        assert!(outline.point(10.0, 9.0).0 > outline.point(10.0, 8.0).0);
        assert!(outline.point(11.0, 8.0).1 > outline.point(10.0, 8.0).1);
    }

    #[test]
    fn allocation_limits_are_checked() {
        assert_eq!(checked_image(8193, 1), Err(Error::ImageTooLarge));
        assert_eq!(checked_image(8192, 8192), Err(Error::ImageTooLarge));
        assert_eq!(checked_image(0, 1), Err(Error::ImageTooLarge));
    }
}
