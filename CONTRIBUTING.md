# Contributing

Small, reproducible Unicode Mongolian rendering issues are welcome.

Please include:

1. A minimal UTF-8 text sample and its code points (especially selectors/joiners).
2. Font name/version and an official source/license link. Do not upload a font
   unless its license permits redistribution.
3. Font size, column height, raster mode, and expected vs actual result.
4. OS, Rust version, and a screenshot if it helps illustrate the issue.

Do not commit any font other than the existing, pinned OFL Noto test fixture.
Font files are ignored by default, and CI checks tracked font content and the
fixture/license checksums. Run `python tools/check_font_policy.py` after staging
changes to check the Git index (the content that would actually be committed).

Do not attach confidential game scripts/assets, evaluation-font screenshots
that cannot be shared publicly, access cookies, service keys, or private paths.

Run formatting, Clippy, tests and doc-tests before submitting a pull request:

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo test --locked --doc
```

New regression fixtures should be small and redistributable. Code contributions
are under the project's MIT license; third-party fonts retain their own license.
