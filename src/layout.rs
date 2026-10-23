use crate::{Error, Font, ShapedRun, validate_text};

/// Behavior when a whole word or non-breaking suffix group is taller than a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowPolicy {
    /// Return WordTooTall (default); never silently scale or split a word.
    Error,
    /// Keep the word at its requested size and mark the column as overflowing.
    Keep,
}

#[derive(Debug, Clone)]
pub struct LayoutOptions {
    pub font_size: f32,
    /// Maximum vertical *ink* height, excluding raster padding.
    pub max_height: f32,
    /// Horizontal gap in output pixels, applied by render_layout.
    pub column_gap: u32,
    pub max_columns: usize,
    pub overflow: OverflowPolicy,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            font_size: 32.0,
            max_height: 256.0,
            column_gap: 12,
            max_columns: 256,
            overflow: OverflowPolicy::Error,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Column {
    text: String,
    run: ShapedRun,
    overflow: bool,
}

impl Column {
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn run(&self) -> &ShapedRun {
        &self.run
    }
    pub fn overflows(&self) -> bool {
        self.overflow
    }
}

/// Ordered top-to-bottom columns flowing left-to-right.
#[derive(Debug, Clone)]
pub struct Layout {
    columns: Vec<Column>,
    column_gap: u32,
}

impl Layout {
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }
    pub fn column_gap(&self) -> u32 {
        self.column_gap
    }
}

impl Font<'_> {
    /// Wrap at U+0020 only, preserving whole words, FVS, MVS, joiners, NNBSP,
    /// and NBSP groups. Repeated/edge ASCII spaces are collapsed/trimmed.
    /// CRLF and CR normalize to LF; explicit empty paragraphs become blank columns.
    pub fn layout(&self, text: &str, options: &LayoutOptions) -> Result<Layout, Error> {
        validate_text(text, true)?;
        if !options.max_height.is_finite() || options.max_height <= 0.0 {
            return Err(Error::InvalidOptions(
                "max_height must be positive and finite",
            ));
        }
        if options.max_columns == 0 || options.max_columns > 4096 || options.column_gap > 8192 {
            return Err(Error::InvalidOptions(
                "max_columns in [1, 4096], column_gap <= 8192 required",
            ));
        }
        // Validate font_size even for empty input.
        self.shape("", options.font_size)?;
        let mut columns = Vec::new();
        let mut push = |text: String, run: ShapedRun| -> Result<(), Error> {
            if columns.len() >= options.max_columns {
                return Err(Error::TooManyColumns);
            }
            let height = run.vertical_ink_size().1;
            let overflow = height > options.max_height;
            if overflow && options.overflow == OverflowPolicy::Error {
                return Err(Error::WordTooTall {
                    height,
                    limit: options.max_height,
                });
            }
            columns.push(Column {
                text,
                run,
                overflow,
            });
            Ok(())
        };
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        for paragraph in normalized.split('\n') {
            let mut current = String::new();
            let mut current_run = self.shape("", options.font_size)?;
            for word in paragraph.split(' ').filter(|word| !word.is_empty()) {
                let candidate = if current.is_empty() {
                    word.to_owned()
                } else {
                    format!("{current} {word}")
                };
                let run = self.shape(&candidate, options.font_size)?;
                if !current.is_empty() && run.vertical_ink_size().1 > options.max_height {
                    push(current, current_run)?;
                    current = word.to_owned();
                    current_run = self.shape(word, options.font_size)?;
                } else {
                    current = candidate;
                    current_run = run;
                }
            }
            push(current, current_run)?;
        }
        Ok(Layout {
            columns,
            column_gap: options.column_gap,
        })
    }
}
