# API and design notes

## Public API

- `Font::new(&bytes)` borrows face-0 font bytes and caches HarfRust shaping data.
- `Font::shape(text, px)` returns a read-only `ShapedRun`, glyph IDs, UTF-8 cluster
  offsets, positioned advances, and horizontal ink bounds in pixels.
- `Font::layout(text, &LayoutOptions)` returns ordered `Column`s with preserved
  text, shaped runs, and overflow flags. It uses actual ink height, not a character
  count or a sum of isolated character widths.
- `render_column` fills one complete run's outlines into a single texture.
- `render_layout` composes column textures at integer coordinates, left to right.
- `RasterImage` exposes `width`, `height`, and straight-alpha `rgba` bytes.

Keep the original font bytes alive for `Font`. Shaped runs own their glyphs and
metrics. Rendering with unrelated font bytes is rejected by a non-cryptographic
identity guard; that guard is not a security primitive. `font_size` is an em size
in output pixels, not a fixed visual glyph height.

## Coordinate system

The shaper runs LTR in a horizontal font coordinate system. For the Mongolian
outline fonts targeted here, apply:

```text
x_screen = y_font + glyph_y - ink_min_y + padding
y_screen = x_font + glyph_x - ink_min_x + padding
```

`glyph_x/y` and ink bounds above are in scaled pixel units. Screen Y grows down.
This is **not** `ink_max_y - y_font`: that would mirror the glyph across the
stem. Do not rotate the resulting image again. This does not implement general
CSS vertical text orientation for arbitrary scripts or OpenType TTB/vmtx layout.

## Why a whole-run path?

Independently rasterized glyph-atlas rectangles can acquire translucent edges
when rotated or sampled. Mongolian has a connected vertical stem, so those
edges appear as repeated horizontal cracks. Here every positioned glyph outline
goes into one `tiny-skia` path using non-zero winding fill. Smooth mode rasterizes
the complete path and only then downsamples it. Pixel mode renders once with AA
off. Neither path introduces per-glyph bitmap boundaries.

The texture fits actual outline extrema, including overhangs. `padding` is added
outside those bounds. A column's vertical drawing height is ink width in the
horizontal shaping coordinate system; advance is reported separately.

## Wrapping and overflow

U+0020 is the only soft break. U+202F and U+00A0 remain inside unbreakable groups;
selectors, joiners and MVS stay with the surrounding text. This is intentionally
more conservative than the original game's fallback wrapping. Explicit line
breaks are hard breaks, including blank columns. ASCII repeated/edge spaces are
normalized; other non-breaking characters are never stripped.

The default overflow behavior is an error. Applications can request `Keep`,
then paginate, widen the available space, or ask the user to choose a smaller
size. The library does not shrink individual columns: doing so would produce
inconsistent typography and destroy a pixel font's grid alignment.

## Engine integration

- Upload as RGBA8/sRGB with **straight-alpha** blending. Do not treat the output
  as premultiplied just because tiny-skia uses premultiplied pixels internally.
- For tintable text, set raster color to `[255,255,255,255]` and tint in the engine.
- Pixel mode needs nearest filtering, whole-pixel positioning, and a native-grid
  em size. Avoid fractional viewport or UI scaling after rendering.
- Smooth mode generally works with linear sampling. Preserve the original
  aspect ratio; do not stretch columns to a fixed box.
- Keep the CPU asset alive if your typewriter animation updates the same texture.
  Cache whole-word/column runs if measuring frequently; shaping every character
  in isolation loses contextual glyphs.
- Core code has no filesystem, network, platform-font, GPU, or Bevy dependency.
  The PNG example is a separate native frontend; WASM needs a host-side upload or
  image encoding layer.

## Validation and testing

Tests use only an OFL font with pinned checksums. They cover contextual vs isolated
shapes, UTF-8 clusters, selectors/controls, word and suffix-group preservation,
explicit overflow, missing glyphs, non-mirrored matrix, continuous connected-word
ink, transparent edges, tint/alpha, deterministic output and resource guards.
They are regression checks, not a complete Mongolian orthographic conformance
suite. Different fonts may have different typographic behavior.

Technical references:

- [HarfRust 0.4.1 API](https://docs.rs/harfrust/0.4.1/harfrust/)
- [ttf-parser 0.25.1 Face API](https://docs.rs/ttf-parser/0.25.1/ttf_parser/struct.Face.html)
- [Noto Mongolian upstream](https://github.com/notofonts/mongolian)

This project is an independent extraction/rework of romchina's game-side Unicode
renderer. Legacy PUA mapping, game text, UI geometry, commercial font selection,
engine state, and deployment/authentication code were deliberately excluded.
