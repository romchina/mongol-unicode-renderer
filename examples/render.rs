//! Small PNG renderer; all filesystem/PNG work stays outside the core library.
use mongol_unicode_renderer::{
    Font, LayoutOptions, RasterImage, RasterMode, RasterOptions, render_layout,
};
use std::{error::Error, fs, io::BufWriter, path::PathBuf};

const HELP: &str = "Usage: cargo run --example render -- FONT.ttf OUTPUT.png [options]
  --text TEXT        Unicode Mongolian input (default: a short generic specimen)
  --text-file FILE   Read UTF-8 text from FILE (mutually exclusive with --text)
  --size PX          Font size, default 32
  --height PX        Maximum column ink height, default 256
  --pixel            Disable antialiasing and supersampling
  --transparent      Keep alpha (default PNG background is white)
  --help            Print this help

No font is installed or downloaded. Font embedding/distribution rights are yours
to check. Unbreakable words that exceed --height produce an error, not clipping.";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let Some(first) = args.next() else {
        println!("{HELP}");
        return Ok(());
    };
    if first == "--help" {
        println!("{HELP}");
        return Ok(());
    }
    let font_path = PathBuf::from(first);
    let output = PathBuf::from(args.next().ok_or("missing output PNG path; use --help")?);
    let mut options = LayoutOptions::default();
    let mut raster = RasterOptions {
        color: [38, 85, 155, 255],
        ..Default::default()
    };
    let mut text = None;
    let mut transparent = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--text" | "--text-file" => {
                if text.is_some() {
                    return Err("use only one --text or --text-file".into());
                }
                let value = args.next().ok_or("missing text/path argument")?;
                text = Some(if arg == "--text" {
                    value
                } else {
                    fs::read_to_string(value)?
                });
            }
            "--size" => options.font_size = args.next().ok_or("missing size")?.parse()?,
            "--height" => options.max_height = args.next().ok_or("missing height")?.parse()?,
            "--pixel" => raster.mode = RasterMode::Pixel,
            "--transparent" => transparent = true,
            _ => return Err(format!("unknown option {arg}; use --help").into()),
        }
    }
    let bytes = fs::read(font_path)?;
    let font = Font::new(&bytes)?;
    let text = text.unwrap_or_else(|| "ᠮᠣᠩᠭᠣᠯ ᠪᠢᠴᠢᠭ ᠮᠣᠩᠭᠣᠯ ᠤᠨ ᠪᠢᠴᠢᠭ".into());
    let layout = font.layout(&text, &options)?;
    let mut image = render_layout(&font, &layout, &raster)?;
    if !transparent {
        composite_white(&mut image);
    }
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let mut encoder = png::Encoder::new(
        BufWriter::new(fs::File::create(&output)?),
        image.width,
        image.height,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&image.rgba)?;
    println!(
        "{} columns, {}x{} pixels -> {}",
        layout.columns().len(),
        image.width,
        image.height,
        output.display()
    );
    Ok(())
}

fn composite_white(image: &mut RasterImage) {
    for pixel in image.rgba.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel[..3] {
            *channel = ((u32::from(*channel) * alpha + 255 * (255 - alpha) + 127) / 255) as u8;
        }
        pixel[3] = 255;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_background_handles_straight_alpha() {
        let mut image = RasterImage {
            width: 3,
            height: 1,
            rgba: vec![38, 85, 155, 0, 38, 85, 155, 128, 38, 85, 155, 255],
        };
        composite_white(&mut image);
        assert_eq!(
            image.rgba,
            vec![255, 255, 255, 255, 146, 170, 205, 255, 38, 85, 155, 255]
        );
    }
}
