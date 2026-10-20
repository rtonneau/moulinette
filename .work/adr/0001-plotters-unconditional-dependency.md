# Make plotters an unconditional dependency for list-mode plotting

_ADR 0001, recorded 2026-10-20 (session 2026-10-20__adding-new-list-mode-file-conversion-example-in-d-onedrive)._

<!-- Format adapted from mattpocock-skills' domain-modeling (ADR-FORMAT.md): https://github.com/mattpocock/skills.git -->

The README documents `cargo build --release --no-default-features --bin moulinette` as a "dependency-free CLI-only build," since only the `gui` feature pulls in slint/rfd/winsafe. The new `list` CLI command needs to render PNG plots (rate-vs-time, violin) regardless of whether the `gui` feature is enabled, so it requires a plotting crate (`plotters`, pure-Rust, no unsafe code) unconditionally rather than gated behind a feature.

Decision: add `plotters` as an unconditional dependency. The CLI-only build (`--no-default-features`) still excludes the GUI toolkit but is no longer dependency-free; the README's "dependency-free CLI-only build" claim needs updating to reflect that `plotters` is now always present.

Rejected: gating `list` behind a new feature flag to preserve the zero-dependency CLI-only build, because `list` is core CLI functionality (like `convert`) and users who only need `convert`/`formats` are not expected to build without it.
