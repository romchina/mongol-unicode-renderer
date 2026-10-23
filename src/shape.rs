use crate::{Error, validate_text};

/// Outline bounds in horizontal shaped coordinates, in pixels (positive Y up).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl Bounds {
    pub fn width(self) -> f32 {
        self.max_x - self.min_x
    }
    pub fn height(self) -> f32 {
        self.max_y - self.min_y
    }
}

/// One contextual glyph with positions in horizontal pixel coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    pub id: u16,
    /// UTF-8 byte offset in the shaped input, not a character index.
    pub cluster: u32,
    pub x: f32,
    pub y: f32,
    pub advance: f32,
}

/// Full shaped run. Read-only accessors prevent accidental metric corruption.
#[derive(Debug, Clone)]
pub struct ShapedRun {
    pub(crate) glyphs: Vec<Glyph>,
    pub(crate) bounds: Option<Bounds>,
    pub(crate) font_size: f32,
    pub(crate) advance: f32,
    pub(crate) font_id: u64,
}

impl ShapedRun {
    pub fn glyphs(&self) -> &[Glyph] {
        &self.glyphs
    }
    pub fn bounds(&self) -> Option<Bounds> {
        self.bounds
    }
    pub fn font_size(&self) -> f32 {
        self.font_size
    }
    pub fn advance(&self) -> f32 {
        self.advance
    }
    /// Vertical ink (width, height) after the Mongolian text matrix.
    pub fn vertical_ink_size(&self) -> (f32, f32) {
        self.bounds.map_or((0.0, 0.0), |b| (b.height(), b.width()))
    }
}

/// Borrowed OpenType font, face 0. No platform font API or engine dependency.
pub struct Font<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) face: ttf_parser::Face<'a>,
    pub(crate) id: u64,
    data: harfrust::ShaperData,
}

impl<'a> Font<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, Error> {
        let face = ttf_parser::Face::parse(bytes, 0).map_err(|_| Error::InvalidFont)?;
        let reference = harfrust::FontRef::new(bytes).map_err(|_| Error::InvalidFont)?;
        let data = harfrust::ShaperData::new(&reference);
        // Stable, non-cryptographic identity is only a misuse guard, not security.
        let id = bytes.iter().fold(0xcbf29ce484222325u64, |hash, b| {
            (hash ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        });
        Ok(Self {
            bytes,
            face,
            id,
            data,
        })
    }

    /// Shape as one LTR Mongolian run. Selectors and joiners are passed unchanged.
    /// This is not a bidi/itemization engine; split mixed-script runs upstream.
    pub fn shape(&self, text: &str, font_size: f32) -> Result<ShapedRun, Error> {
        validate_text(text, false)?;
        if !font_size.is_finite() || !(0.0..=4096.0).contains(&font_size) || font_size == 0.0 {
            return Err(Error::InvalidOptions(
                "font_size must be finite and in (0, 4096]",
            ));
        }
        let font = harfrust::FontRef::new(self.bytes).map_err(|_| Error::InvalidFont)?;
        let shaper = self.data.shaper(&font).build();
        let mut buffer = harfrust::UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_direction(harfrust::Direction::LeftToRight);
        buffer.set_script(harfrust::script::MONGOLIAN);
        buffer.set_language("mn".parse().expect("constant BCP 47 language tag"));
        buffer.guess_segment_properties();
        let shaped = shaper.shape(buffer, &[]);
        let scale = font_size / f32::from(self.face.units_per_em());
        let mut glyphs = Vec::with_capacity(shaped.len());
        let mut bounds: Option<Bounds> = None;
        let mut pen = 0.0;
        for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
            if info.glyph_id == 0 {
                return Err(Error::MissingGlyph {
                    cluster: info.cluster,
                });
            }
            let id = u16::try_from(info.glyph_id).map_err(|_| Error::InvalidFont)?;
            let x = pen + pos.x_offset as f32 * scale;
            let y = pos.y_offset as f32 * scale;
            let advance = pos.x_advance as f32 * scale;
            if let Some(b) = self.face.glyph_bounding_box(ttf_parser::GlyphId(id)) {
                let next = Bounds {
                    min_x: x + f32::from(b.x_min) * scale,
                    min_y: y + f32::from(b.y_min) * scale,
                    max_x: x + f32::from(b.x_max) * scale,
                    max_y: y + f32::from(b.y_max) * scale,
                };
                bounds = Some(bounds.map_or(next, |old| Bounds {
                    min_x: old.min_x.min(next.min_x),
                    min_y: old.min_y.min(next.min_y),
                    max_x: old.max_x.max(next.max_x),
                    max_y: old.max_y.max(next.max_y),
                }));
            }
            glyphs.push(Glyph {
                id,
                cluster: info.cluster,
                x,
                y,
                advance,
            });
            pen += advance;
        }
        Ok(ShapedRun {
            glyphs,
            bounds,
            font_size,
            advance: pen,
            font_id: self.id,
        })
    }
}
