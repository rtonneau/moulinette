# moulinette glossary

Moulinette is a native Rust replacement for the statically analysed MPA path of moulinetteb.exe: it parses FAST ComTec MPA-3 instrument files and converts them to plain-text spectrum formats, with a CLI and a Slint GUI sharing the same workflow.

<!-- Format adapted from mattpocock-skills' domain-modeling (GLOSSARY-FORMAT.md): https://github.com/mattpocock/skills.git -->

<!--
How to add a term (the moment it is resolved, not in a batch at the end):

**Term**:
One or two sentences: what it IS, not what it does.
_Avoid_: the other words people use for it

- Be opinionated: one word per concept; list the others under _Avoid_.
- Only terms specific to this project. General programming concepts and
  implementation details don't belong here: this is a glossary, not a spec.
- Group terms under "### <area>" subheadings once natural clusters emerge.
-->

## Language

**List-mode file (.lst)**:
An MPA-3 instrument file that records every detector event individually (timestamp plus per-ADC channel value) rather than a pre-binned histogram. Contrasts with an .mpa file, which stores only the final per-ADC channel/count histogram. Shares the same ASCII header conventions as .mpa (ADC sections, CALOFF/CALFACT, etc.) but ends the header at a `[LISTDATA]` marker, followed by a binary stream of little-endian 16-bit words encoding timer words, sync markers, and event records.
_Avoid_: "list file", "lst file" as a noun on its own without "list-mode"; "mode" alone (collides with the existing output-format `Mode` concept in gui_state::MODES).

**Region of interest (ROI)**:
A named channel range on one ADC's spectrum, given either as channel bounds or as an energy range (converted to channels via that ADC's linear calibration). Used to restrict a list-mode rate computation to a sub-band of the spectrum instead of the whole thing.

**Time-binned rate**:
The list-mode analysis's core derived series: events within an ROI (or a whole ADC) are grouped into fixed-width time bins, producing counts per bin, a rate (counts/second), and its Poisson error, over the acquisition's internal accumulated clock (not wall-clock time).
