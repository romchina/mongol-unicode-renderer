# Third-party notices

Project code is MIT licensed. Fonts and dependencies keep their own licenses.

## Test font

Noto Sans Mongolian, Copyright 2022 The Noto Project Authors, is redistributed
unmodified under **SIL OFL 1.1**. Its complete license and pinned source/checksums
are in [tests/fonts](tests/fonts/README.md). Only this open font is checked in.
The generated `docs/demo.png` is a rendered specimen, not a font file.

## Rust dependencies

Cargo downloads these separately; their complete notices/licenses are included
in their upstream crate distributions. Check transitive licenses as appropriate
when distributing a compiled application.

| Component | Purpose | Declared license | Source |
| --- | --- | --- | --- |
| harfrust 0.4.1 | OpenType contextual shaping | MIT | https://github.com/harfbuzz/harfrust |
| ttf-parser 0.25 | Font metrics/outlines | MIT OR Apache-2.0 | https://github.com/harfbuzz/ttf-parser |
| tiny-skia 0.11.4 | Whole-run path rasterization | BSD-3-Clause | https://github.com/linebender/tiny-skia |
| png 0.17 (example only) | PNG encoding | MIT OR Apache-2.0 | https://github.com/image-rs/image-png |

This renderer is extracted and reworked from romchina's game-side Unicode
Mongolian rendering implementation. It includes no game scripts, artwork,
audio, proprietary fonts, engine assets, credentials, or deployment settings.
