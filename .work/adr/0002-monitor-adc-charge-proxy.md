# Monitor-ADC counts as an integrated-charge proxy

_ADR 0002, recorded 2026-10-21 (session 2026-10-21__clean-review-current-state-of-the-gui-layout-need-a-home-screen)._

<!-- Format adapted from mattpocock-skills' domain-modeling (ADR-FORMAT.md): https://github.com/mattpocock/skills.git -->

Neither `.mpa` nor `.lst` carries a literal beam-charge (coulomb) field; `docs/conversion_algorithm.md` previously recorded that the legacy tool adds no beam/charge normalization at all. For the new charge-normalized conversion format and Analyze's charge-axis chart, we treat one user-chosen ADC/spectrum's own accumulated counts as a proxy for integrated charge, converting to µC only when the user supplies a counts-per-µC calibration (left blank, the output stays a dimensionless ratio against the monitor's raw counts). This is an approximation specific to moulinette, not a reconstruction of original-tool behavior, and only holds while the monitor channel's response stays linear with beam current over the run.
