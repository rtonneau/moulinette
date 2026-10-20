# Ticket 04: violin-plot

**Model:** heavy
**Model (Jev):** opus-5.5 (confidence 0.24)
**Effort:** high

**Acceptance Criteria:**
- [ ] `kde::gaussian_kde(samples: &[f64], eval_points: &[f64]) -> Vec<f64>` computes a Gaussian kernel density estimate using Silverman's rule-of-thumb bandwidth (`0.9 * min(std_dev, iqr/1.34) * n.powf(-0.2)`, matching SciPy/`matplotlib`'s default closely enough to produce a recognizable violin shape), returning one density value per `eval_points` entry.
- [ ] `plot::violin(groups: &[(String, Vec<f64>)], output_path: &Path) -> Result<(), PlotError>` renders a PNG with one filled-polygon violin per named group (each built from `gaussian_kde` over that group's values), positioned side by side with its label on the x-axis.
- [ ] `moulinette list ... --window START_S:END_S` (repeatable) is wired into the existing `list` subcommand from ticket 3: when at least one `--window` is given, for each `--roi`, the series' `RateBin`s are split by `time_s` into one group per window, and one violin PNG per ROI is written to `--output-dir` (one violin per window, in window order). Omitting `--window` entirely still produces exactly ticket 3's rate-vs-time CSV/PNG output, with no violin files written.
- [ ] Unit tests: `gaussian_kde` against a small known sample set, asserting density values at a few evaluation points within a documented tolerance of hand-computed (or `scipy`-cross-checked, with the cross-check documented in a comment) expected values; a `list --window` integration test asserting a violin PNG file is created and non-empty for a tiny synthetic fixture with two windows.

**Files to Touch:**
- `src/kde.rs` (new)
- `src/plot.rs`
- `src/main.rs`

**Verification Step:**

Run:
```bash
cargo test kde:: && cargo test plot::
```
Expected: `test result: ok.` for both module test suites, including the new `--window`/violin integration test.

**Notes:**

Depends on ticket 3's `list` subcommand and `plot.rs` module, and ticket 2's `RateBin`. Explicitly out of scope: `ruptures`-style changepoint detection (tracked as a separate follow-up, not part of this plan).
