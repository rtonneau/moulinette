---
bump: minor
floor: minor
---
- Add a home screen with Analyze/Convert cards so both actions are reachable up front.
- Make `.lst` files convertible (histogrammed into per-ADC spectra, then written with the existing legacy formats) and `.mpa` files analysable (ROI peak statistics: gross/net counts, centroid, width), so every loaded file supports both actions.
- Add interactive in-window charts (pan, zoom, hover tooltips) for the rate-vs-time, violin and spectrum views, replacing static previews; the existing CSV/PNG file outputs are unchanged.
- Add a charge-normalized (µC) output format that divides each channel's counts by a chosen monitor ADC's total counts, optionally scaled by a user-supplied counts-per-µC calibration.
- Add an opt-in charge-axis chart on the `.lst` Analyze screen, plotting each ROI's rate against accumulated monitor-ADC counts (or µC) instead of elapsed time.
