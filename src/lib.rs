//! Unicode Mongolian shaping and seamless top-to-bottom, left-to-right rendering.
//!
//! Supply an OpenType font containing Mongolian contextual substitutions. No font
//! is embedded in the library. See the README for scope and font licensing.
//!
//! ```
//! use mongol_unicode_renderer::{Font, LayoutOptions, RasterOptions, render_layout};
//! # fn demo(bytes: &[u8]) -> Result<(), mongol_unicode_renderer::Error> {
//! let font = Font::new(bytes)?;
//! let layout = font.layout("ᠮᠣᠩᠭᠣᠯ ᠪᠢᠴᠢᠭ", &LayoutOptions::default())?;
//! let image = render_layout(&font, &layout, &RasterOptions::default())?;
//! // Upload image.rgba (straight-alpha RGBA8) to your engine or encode it as PNG.
//! # Ok(()) }
//! ```

mod layout;
mod raster;
mod shape;

pub use layout::{Column, Layout, LayoutOptions, OverflowPolicy};
pub use raster::{RasterImage, RasterMode, RasterOptions, render_column, render_layout};
pub use shape::{Bounds, Font, Glyph, ShapedRun};

/// Upper bound on a single input, including UTF-8 control/selector bytes.
pub const MAX_TEXT_BYTES: usize = 65_536;

/// A recoverable validation, shaping, layout, or rendering failure.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    InvalidFont,
    InvalidOptions(&'static str),
    TextTooLong,
    PrivateUse { byte_index: usize },
    UnsupportedControl { byte_index: usize },
    MissingGlyph { cluster: u32 },
    WordTooTall { height: f32, limit: f32 },
    TooManyColumns,
    ImageTooLarge,
    RasterizationFailed,
    FontMismatch,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFont => write!(f, "invalid or unsupported OpenType font (use face 0)"),
            Self::InvalidOptions(why) => write!(f, "invalid options: {why}"),
            Self::TextTooLong => write!(f, "input exceeds {MAX_TEXT_BYTES} UTF-8 bytes"),
            Self::PrivateUse { byte_index } => write!(
                f,
                "private-use text at byte {byte_index}; convert legacy text to Unicode first"
            ),
            Self::UnsupportedControl { byte_index } => {
                write!(f, "unsupported control at byte {byte_index}")
            }
            Self::MissingGlyph { cluster } => {
                write!(f, "font produced .notdef for UTF-8 cluster {cluster}")
            }
            Self::WordTooTall { height, limit } => write!(
                f,
                "unbreakable word/group is {height:.2}px tall, limit {limit:.2}px"
            ),
            Self::TooManyColumns => write!(f, "layout exceeds configured maximum columns"),
            Self::ImageTooLarge => write!(f, "image exceeds dimension or pixel allocation limits"),
            Self::RasterizationFailed => {
                write!(f, "could not construct or rasterize glyph outlines")
            }
            Self::FontMismatch => write!(f, "the shaped run belongs to a different font"),
        }
    }
}

impl std::error::Error for Error {}

fn validate_text(text: &str, multiline: bool) -> Result<(), Error> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(Error::TextTooLong);
    }
    for (byte_index, c) in text.char_indices() {
        if matches!(c, '\u{e000}'..='\u{f8ff}' | '\u{f0000}'..='\u{ffffd}' | '\u{100000}'..='\u{10fffd}')
        {
            return Err(Error::PrivateUse { byte_index });
        }
        if c.is_control() && !(multiline && matches!(c, '\n' | '\r')) {
            return Err(Error::UnsupportedControl { byte_index });
        }
    }
    Ok(())
}
