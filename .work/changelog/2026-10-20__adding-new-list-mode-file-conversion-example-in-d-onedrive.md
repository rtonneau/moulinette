---
bump: minor
floor: minor
---
- Added a `moulinette list` CLI command and a matching GUI panel that load MPA-3 list-mode (`.lst`) files and compute time-binned acquisition rate per region of interest.
- Rate-vs-time and violin plots (PNG) plus per-ROI rate CSVs are written for list-mode files; violin plots are generated per time window when `--window`/time windows are given.
- The GUI now switches to a list-mode analysis panel (ROI editor, bin width, time windows, output directory) when a `.lst` file is opened or dropped, alongside the existing `.mpa` converter panel.
- CLI-only builds now depend on the `plotters` crate (no longer dependency-free), needed to render the new plots.
