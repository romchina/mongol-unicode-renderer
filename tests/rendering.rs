use mongol_unicode_renderer::*;

const FONT: &[u8] = include_bytes!("fonts/NotoSansMongolian-Regular.ttf");
const WORD: &str = "ᠮᠣᠩᠭᠣᠯ";

fn font() -> Font<'static> {
    Font::new(FONT).unwrap()
}

#[test]
fn invalid_fonts_are_errors() {
    assert!(matches!(Font::new(&[]), Err(Error::InvalidFont)));
    assert!(matches!(Font::new(b"not a font"), Err(Error::InvalidFont)));
}

#[test]
fn contextual_shaping_is_not_per_character_lookup() {
    let font = font();
    let connected = font.shape(WORD, 32.0).unwrap();
    let isolated: Vec<_> = WORD
        .chars()
        .flat_map(|c| font.shape(&c.to_string(), 32.0).unwrap().glyphs().to_vec())
        .collect();
    let ids = |glyphs: &[Glyph]| glyphs.iter().map(|g| g.id).collect::<Vec<_>>();
    assert_ne!(ids(connected.glyphs()), ids(&isolated));
    assert!(connected.glyphs().iter().all(|g| g.id != 0));
}

#[test]
fn shaping_retains_utf8_cluster_offsets() {
    let run = font().shape("ᠮᠣᠩᠭᠣᠯ ᠪᠢᠴᠢᠭ", 32.0).unwrap();
    let mut last = 0;
    for glyph in run.glyphs() {
        assert!(glyph.cluster >= last);
        last = glyph.cluster;
    }
    assert!(last > 10); // bytes, not a sequence of scalar indices
}

#[test]
fn selectors_mvs_and_joiners_are_accepted() {
    let font = font();
    for text in [
        "ᠠ\u{180b}",
        "ᠠ\u{180c}",
        "ᠠ\u{180d}",
        "ᠠ\u{180f}",
        "ᠨ\u{180e}ᠠ",
        "ᠮ\u{200d}ᠣ",
        "ᠮ\u{200c}ᠣ",
    ] {
        let run = font.shape(text, 32.0).unwrap();
        assert!(!run.glyphs().is_empty(), "{text}");
    }
}

#[test]
fn pua_is_rejected_in_all_private_planes() {
    for text in ["\u{e255}", "\u{f0000}", "\u{100000}"] {
        assert!(matches!(
            font().shape(text, 32.0),
            Err(Error::PrivateUse { byte_index: 0 })
        ));
    }
}

#[test]
fn missing_glyphs_are_not_silently_rendered() {
    assert!(matches!(
        font().shape("🦀", 32.0),
        Err(Error::MissingGlyph { .. })
    ));
}

#[test]
fn invalid_font_sizes_are_errors_even_for_empty_text() {
    for size in [0.0, -1.0, f32::NAN, f32::INFINITY, 5000.0] {
        assert!(matches!(
            font().shape("", size),
            Err(Error::InvalidOptions(_))
        ));
        assert!(
            font()
                .layout(
                    "",
                    &LayoutOptions {
                        font_size: size,
                        ..Default::default()
                    }
                )
                .is_err()
        );
    }
}

#[test]
fn controls_and_oversized_input_are_errors() {
    assert!(matches!(
        font().shape("\t", 32.0),
        Err(Error::UnsupportedControl { .. })
    ));
    assert!(matches!(
        font().shape("\n", 32.0),
        Err(Error::UnsupportedControl { .. })
    ));
    assert_eq!(
        font()
            .shape(&"a".repeat(MAX_TEXT_BYTES + 1), 32.0)
            .unwrap_err(),
        Error::TextTooLong
    );
}

#[test]
fn bounds_and_advance_scale_together() {
    let font = font();
    let a = font.shape(WORD, 16.0).unwrap();
    let b = font.shape(WORD, 32.0).unwrap();
    assert!((b.advance() - a.advance() * 2.0).abs() < 0.01);
    assert!((b.vertical_ink_size().1 - a.vertical_ink_size().1 * 2.0).abs() < 0.01);
}

#[test]
fn words_wrap_without_loss_or_internal_splits() {
    let font = font();
    let text = format!("{WORD} {WORD} {WORD}");
    let height = font.shape(WORD, 32.0).unwrap().vertical_ink_size().1 + 1.0;
    let layout = font
        .layout(
            &text,
            &LayoutOptions {
                max_height: height,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(layout.columns().len(), 3);
    assert!(
        layout
            .columns()
            .iter()
            .all(|c| c.text() == WORD && !c.overflows())
    );
    assert_eq!(
        layout
            .columns()
            .iter()
            .map(|c| c.text())
            .collect::<Vec<_>>()
            .join(" "),
        text
    );
}

#[test]
fn nonbreaking_spaces_keep_suffix_groups_together() {
    let font = font();
    for separator in ['\u{202f}', '\u{00a0}'] {
        let group = format!("{WORD}{separator}ᠤᠨ");
        let text = format!("{group} {group}");
        let height = font.shape(&group, 32.0).unwrap().vertical_ink_size().1 + 1.0;
        let layout = font
            .layout(
                &text,
                &LayoutOptions {
                    max_height: height,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(layout.columns().len(), 2);
        assert!(layout.columns().iter().all(|c| c.text() == group));
    }
}

#[test]
fn long_words_fail_or_explicitly_overflow_without_scaling() {
    let font = font();
    let mut options = LayoutOptions {
        max_height: 1.0,
        ..Default::default()
    };
    assert!(matches!(
        font.layout(WORD, &options),
        Err(Error::WordTooTall { .. })
    ));
    options.overflow = OverflowPolicy::Keep;
    let layout = font.layout(WORD, &options).unwrap();
    assert!(layout.columns()[0].overflows());
    assert_eq!(layout.columns()[0].text(), WORD);
    assert_eq!(layout.columns()[0].run().font_size(), 32.0);
}

#[test]
fn explicit_newlines_keep_blank_columns_and_normalize_crlf() {
    let font = font();
    let a = font
        .layout(
            &format!("{WORD}\r\n\r\n{WORD}\r"),
            &LayoutOptions::default(),
        )
        .unwrap();
    assert_eq!(
        a.columns().iter().map(|c| c.text()).collect::<Vec<_>>(),
        vec![WORD, "", WORD, ""]
    );
}

#[test]
fn ascii_spaces_are_normalized_but_controls_are_not() {
    let layout = font()
        .layout(&format!("  {WORD}   {WORD}  "), &LayoutOptions::default())
        .unwrap();
    assert_eq!(layout.columns()[0].text(), format!("{WORD} {WORD}"));
    assert!(font().layout("\t", &LayoutOptions::default()).is_err());
}

#[test]
fn invalid_layout_options_are_errors() {
    for height in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(
            font()
                .layout(
                    WORD,
                    &LayoutOptions {
                        max_height: height,
                        ..Default::default()
                    }
                )
                .is_err()
        );
    }
    for max in [0, 4097] {
        assert!(
            font()
                .layout(
                    WORD,
                    &LayoutOptions {
                        max_columns: max,
                        ..Default::default()
                    }
                )
                .is_err()
        );
    }
    assert_eq!(
        font()
            .layout(
                &format!("{WORD}\n{WORD}"),
                &LayoutOptions {
                    max_columns: 1,
                    ..Default::default()
                }
            )
            .unwrap_err(),
        Error::TooManyColumns
    );
}

#[test]
fn smooth_output_contains_fractional_coverage_and_straight_rgb() {
    let font = font();
    let run = font.shape(WORD, 32.0).unwrap();
    let options = RasterOptions {
        color: [12, 90, 160, 128],
        ..Default::default()
    };
    let image = render_column(&font, &run, &options).unwrap();
    assert_eq!(image.rgba.len(), (image.width * image.height * 4) as usize);
    assert!(image.rgba.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 128));
    for pixel in image.rgba.chunks_exact(4).filter(|p| p[3] > 0) {
        assert_eq!(&pixel[..3], &[12, 90, 160]);
        assert!(pixel[3] <= 128);
    }
}

#[test]
fn pixel_output_has_only_binary_coverage() {
    let font = font();
    let run = font.shape(WORD, 32.0).unwrap();
    let image = render_column(
        &font,
        &run,
        &RasterOptions {
            mode: RasterMode::Pixel,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(image.rgba.chunks_exact(4).any(|p| p[3] == 255));
    assert!(image.rgba.chunks_exact(4).all(|p| p[3] == 0 || p[3] == 255));
}

#[test]
fn outline_bounds_leave_transparent_padding_on_all_edges() {
    let font = font();
    for text in [WORD, "ᠠ", "ᠭᠠᠵᠠᠷ", "ᠨ\u{180e}ᠠ"] {
        let image = render_column(
            &font,
            &font.shape(text, 31.5).unwrap(),
            &RasterOptions::default(),
        )
        .unwrap();
        let alpha = |x, y| image.rgba[((y * image.width + x) * 4 + 3) as usize];
        for x in 0..image.width {
            assert_eq!(alpha(x, 0), 0);
            assert_eq!(alpha(x, image.height - 1), 0);
        }
        for y in 0..image.height {
            assert_eq!(alpha(0, y), 0);
            assert_eq!(alpha(image.width - 1, y), 0);
        }
    }
}

#[test]
fn blank_text_produces_a_transparent_valid_image() {
    let font = font();
    let layout = font.layout("", &LayoutOptions::default()).unwrap();
    let image = render_layout(&font, &layout, &RasterOptions::default()).unwrap();
    assert!(image.width > 0 && image.height > 0);
    assert!(image.rgba.iter().all(|b| *b == 0));
}

#[test]
fn columns_are_composited_left_to_right_without_scaling() {
    let font = font();
    let layout = font
        .layout("ᠮᠣᠩᠭᠣᠯ\nᠪᠢᠴᠢᠭ", &LayoutOptions::default())
        .unwrap();
    let options = RasterOptions::default();
    let first = render_column(&font, layout.columns()[0].run(), &options).unwrap();
    let second = render_column(&font, layout.columns()[1].run(), &options).unwrap();
    let all = render_layout(&font, &layout, &options).unwrap();
    assert_eq!(all.width, first.width + layout.column_gap() + second.width);
    for y in 0..first.height {
        let start = (y * all.width * 4) as usize;
        let src = (y * first.width * 4) as usize;
        assert_eq!(
            &all.rgba[start..start + (first.width * 4) as usize],
            &first.rgba[src..src + (first.width * 4) as usize]
        );
    }
    for y in 0..second.height {
        let start = ((y * all.width + first.width + layout.column_gap()) * 4) as usize;
        let src = (y * second.width * 4) as usize;
        assert_eq!(
            &all.rgba[start..start + (second.width * 4) as usize],
            &second.rgba[src..src + (second.width * 4) as usize]
        );
    }
}

#[test]
fn raster_is_deterministic() {
    let font = font();
    let run = font.shape(WORD, 32.0).unwrap();
    assert_eq!(
        render_column(&font, &run, &RasterOptions::default()),
        render_column(&font, &run, &RasterOptions::default())
    );
}

#[test]
fn connected_word_has_no_blank_horizontal_atlas_seams() {
    let font = font();
    let run = font.shape(WORD, 32.0).unwrap();
    let image = render_column(&font, &run, &RasterOptions::default()).unwrap();
    let ink_rows: Vec<_> = (0..image.height)
        .filter(|y| {
            (0..image.width).any(|x| image.rgba[((y * image.width + x) * 4 + 3) as usize] > 0)
        })
        .collect();
    assert!(!ink_rows.is_empty());
    assert!(ink_rows.windows(2).all(|pair| pair[1] == pair[0] + 1));
}

#[test]
fn invalid_raster_options_and_huge_images_are_errors() {
    let font = font();
    let run = font.shape(WORD, 32.0).unwrap();
    for samples in [0, 5, 255] {
        assert!(
            render_column(
                &font,
                &run,
                &RasterOptions {
                    mode: RasterMode::Smooth { samples },
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    assert!(
        render_column(
            &font,
            &run,
            &RasterOptions {
                padding: 129,
                ..Default::default()
            }
        )
        .is_err()
    );
    let run = font.shape(&WORD.repeat(100), 4096.0).unwrap();
    assert_eq!(
        render_column(&font, &run, &RasterOptions::default()).unwrap_err(),
        Error::ImageTooLarge
    );
}

#[test]
fn same_font_bytes_in_another_allocation_can_render_a_run() {
    let bytes = FONT.to_vec();
    let a = font();
    let b = Font::new(&bytes).unwrap();
    assert!(render_column(&b, &a.shape(WORD, 32.0).unwrap(), &RasterOptions::default()).is_ok());
}

#[test]
fn another_font_identity_is_rejected() {
    // Append harmless trailing bytes: same font outlines but distinct identity.
    let mut bytes = FONT.to_vec();
    bytes.push(0);
    let a = font();
    let b = Font::new(&bytes).unwrap();
    assert_eq!(
        render_column(&b, &a.shape(WORD, 32.0).unwrap(), &RasterOptions::default()).unwrap_err(),
        Error::FontMismatch
    );
}
