# Ticket 01: lst-binary-parser

**Model:** heavy
**Model (Jev):** opus-5.5 (confidence 0.15)
**Effort:** high

**Acceptance Criteria:**
- [ ] `lst::parse(bytes: &[u8]) -> Result<ListFile, LstParseError>` parses the ASCII header (up to and including a `[LISTDATA]` marker) and the following binary event stream.
- [ ] Header parsing recognizes `[ADCn]` sections and `caloff`/`calfact`/`calfact2`/`calfact3`/`nchannels`/`timerreduce` key=value lines, populating one `AdcHeader { adc_index: u8, caloff: Option<f64>, calfact: Option<f64>, calfact2: Option<f64>, calfact3: Option<f64>, n_channels: u32 }` per declared ADC (values stored as-read; ticket 2 decides how `calfact2`/`calfact3` affect ROI resolution).
- [ ] A missing `[LISTDATA]` marker returns `LstParseError::MissingListData`.
- [ ] Timer words (`high == 0x4000`) accumulate `timer_reduce` ms (default 1, overridden by the header's `timerreduce` key) onto a running `u64` millisecond clock, advancing 2 words.
- [ ] Sync markers (`low == 0xFFFF && high == 0xFFFF`) are skipped, advancing 2 words.
- [ ] Event records decode the ADC bitmask (`low & 0xFF`, bits 0-7), the dummy-word flag (bit 15 of `high`) and the RTC-present flag (bit 12 of `high`); when RTC is present the next 3 words are consumed and discarded (not stored); when the dummy flag is set, one extra word is skipped before ADC values; exactly one 16-bit channel value is read per set ADC-mask bit, in ascending bit order, producing one `Event { time_ms: u64, adc_values: [Option<u16>; 8] }`.
- [ ] An unrecognized word (not a timer word, sync marker, or a valid event header) returns `LstParseError::UnrecognizedWord { word_index: usize }`.
- [ ] EOF while reading RTC words or ADC channel values returns `LstParseError::TruncatedEvent { word_index: usize }`.
- [ ] `docs/lst_format.md` documents the header dialect and the binary record layout (timer/sync/event discrimination bits, RTC/dummy flag positions, ADC bitmask) in enough detail that ticket 1's test fixtures are traceable to it.
- [ ] Unit tests (hand-built `&[u8]` fixtures) cover: header-only input (zero events), a single timer word, a sync marker, an event with 1 active ADC and no RTC/dummy flags, an event with RTC present, an event with the dummy flag set, an event with 3 active ADCs, a truncated event (EOF mid-read, asserts `TruncatedEvent`), an unrecognized word (asserts `UnrecognizedWord`), and a missing `[LISTDATA]` marker (asserts `MissingListData`).

**Files to Touch:**
- `src/lst.rs` (new)
- `src/lib.rs` (add `pub mod lst;`)
- `docs/lst_format.md` (new)

**Verification Step:**

Run:
```bash
cargo test lst::
```
Expected: `test result: ok.` with 10+ passed, 0 failed (one test per record-type/error case listed above).

**Notes:**

Mirror `src/mpa.rs`'s existing style: a `ParseErrorKind`-like enum (`LstParseError`) with `Display`/`std::error::Error` impls, and an inline `#[cfg(test)] mod tests` at the bottom of the file, consistent with `mpa.rs`'s own test module. `.lst` and `.mpa` share the same `key=value`/`[ADCn]` header dialect but terminate differently (`[LISTDATA]` vs `[DATAn,N]`); reuse `mpa.rs`'s header-parsing approach where practical, but a standalone implementation is acceptable given the differing terminator. This parser is intentionally strict (errors on anything unrecognized) — a deliberate divergence from `mpa3kit`'s lenient skip-and-count Python reference, per the approved grill design.
