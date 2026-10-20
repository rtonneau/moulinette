# MPA-3 list-mode (`.lst`) format and the Rust parser

Status: reconstructed from `mpa3kit`'s working Python reference parser
(`C:\DEV\python\mpa3kit\src\mpa3kit\parser.py`, `ListFileParser.parse`), not
from a vendor spec. This document is deliberately precise about bit
positions and word order so that `src/lst.rs`'s hand-built test fixtures are
traceable back to it. See `docs/mpa_format.md` for the related `.mpa`
aggregated-histogram carrier.

`src/lst.rs`'s `lst::parse(bytes: &[u8]) -> Result<ListFile, LstParseError>`
implements everything described here. Unlike `mpa3kit`, the Rust parser is
**strict**: any word it cannot classify, or any EOF reached mid-record, is a
hard error rather than a silently skipped/truncated result. This is a
deliberate divergence, approved during grill design, from `mpa3kit`'s
lenient skip-and-count behavior.

## Overall file layout

```
<ASCII header, up to and including "[LISTDATA]">
<binary event stream: little-endian 16-bit words>
```

`lst::parse` finds the first case-insensitive occurrence of `[LISTDATA]` in
the input. Everything up to and including that marker is the header; a
missing marker returns `LstParseError::MissingListData`. Everything after
the marker (no newline-skipping, no further header interpretation) is
treated as the binary word stream, read as a flat sequence of little-endian
`u16` values two at a time (`low`, `high`).

## ASCII header dialect

Shares `.mpa`'s `[ADCn]` / `key=value` dialect (see `src/mpa.rs`,
`docs/mpa_format.md`) but terminates with `[LISTDATA]` instead of a `[DATAn,N]`
section. Lines are split on `\n`, with an optional trailing `\r` stripped,
then leading content is trimmed of trailing ASCII spaces (`mpa.rs`'s
`trim_spaces`, reused here).

- A line matching `[ADCn]` (case-insensitive `ADC` prefix, digits, closing
  `]`) opens a section for ADC index `n`. `n` is the literal number written
  after `ADC` — 1-based, matching `src/mpa.rs`'s `SectionKind::Adc`
  convention (not `mpa3kit`'s 0-based `adc_num`). Re-encountering the same
  `[ADCn]` later in the header resumes editing the same `AdcHeader` instead
  of creating a duplicate.
- Inside an ADC section, recognized `key=value` lines (key matched
  case-insensitively):
  - `caloff`, `calfact`, `calfact2`, `calfact3` — parsed as `f64`, stored in
    `AdcHeader` as-read. Ticket 1 does not interpret `calfact2`/`calfact3`;
    a later ticket decides how (or whether) they affect ROI resolution.
  - `nchannels` — parsed as `u32`, overriding the default of `2048`
    (matching `mpa3kit`'s default when the key is absent).
  - Any other key is ignored.
- `timerreduce=<integer>` is a **global** key (not tied to the current ADC
  section): it sets the number of milliseconds each timer word adds to the
  running clock, overriding the default of `1`. It can appear anywhere in
  the header, inside or outside an `[ADCn]` section.
- Any other section header or key=value line is ignored.
- One `AdcHeader { adc_index, caloff, calfact, calfact2, calfact3,
  n_channels }` is produced per distinct `[ADCn]` section seen, in first-seen
  order. Fields not set by a `key=value` line keep their default
  (`None` for the calibration fields, `2048` for `n_channels`).

## Binary event stream

Read as `u16` little-endian words, two at a time (`low = words[i]`,
`high = words[i+1]`). If fewer than two words remain, parsing stops
(trailing odd bytes, or a single trailing word, are not treated as an
error — there is nothing left to classify). Each pair is classified in this
order:

1. **Timer word** — `high == 0x4000` exactly (not just bit 14 set — e.g.
   `0xC000` is *not* a timer word, since bit 15 is also set). Adds the
   header's `timerreduce` (default `1`) milliseconds onto a running `u64`
   millisecond clock. Advances 2 words. No event is produced.
2. **Sync marker** — `low == 0xFFFF && high == 0xFFFF`. Skipped; advances 2
   words. No event is produced.
3. **Event record** — otherwise, valid only when bit 14 of `high` is `0`
   (`(high >> 14) & 1 == 0`). Anything that reaches this point with bit 14
   set is not a timer word (already ruled out by the exact-value check
   above), not a sync marker, and not a valid event header, so it is
   `LstParseError::UnrecognizedWord { word_index }`, where `word_index` is
   the index of the pair's `low` word.

   For a valid event record:
   - `adc_mask = low & 0xFF` — bits 0-7, one bit per possible ADC (0-7).
   - `dummy = (high >> 15) & 1` — a dummy word was inserted into the stream.
   - `rtc_present = (high >> 12) & 1` — a 3-word real-time-counter value
     follows.
   - The cursor starts right after the two header words (`word_index + 2`).
   - If `rtc_present`, the next 3 words are the RTC value; they are consumed
     for stream alignment only and never stored anywhere (not even in
     `Event`). EOF here (fewer than 3 words available) is
     `LstParseError::TruncatedEvent { word_index }`, where `word_index` is
     the index of the first RTC word the parser attempted to read.
   - If `dummy`, one more word is skipped (its value is discarded). EOF here
     is `TruncatedEvent { word_index }` at the skipped word's index.
   - Then, for each bit `0..8` set in `adc_mask`, in ascending bit order,
     exactly one 16-bit channel value is read and stored at that bit's slot.
     EOF while reading any of these is `TruncatedEvent { word_index }` at
     the specific missing word's index.
   - Exactly one `Event { time_ms, adc_values }` is produced per event
     record, regardless of how many (including zero) bits are set in
     `adc_mask` — `time_ms` is the running clock's current value, and
     `adc_values[b]` is `Some(value)` for each set bit `b`, `None`
     otherwise. (This differs from `mpa3kit`, which silently drops an event
     whose mask has no bits set; the Rust parser always records it.)

## Error summary

| Error | Condition |
| --- | --- |
| `MissingListData` | No `[LISTDATA]` marker (case-insensitive) anywhere in the input. |
| `UnrecognizedWord { word_index }` | A word pair is not a timer word, sync marker, or valid event header (bit 14 of `high` is set, but `high != 0x4000` and the pair isn't `0xFFFF, 0xFFFF`). |
| `TruncatedEvent { word_index }` | EOF reached while reading an event's RTC words, its skipped dummy word, or an ADC channel value. |

## Fixture traceability

`src/lst.rs`'s `#[cfg(test)] mod tests` builds every fixture directly from
the rules above using a `le(&[u16]) -> Vec<u8>` helper (little-endian word
packing), covering: header-only input (zero events), a lone timer word,
timer words with a header `timerreduce` override accumulating onto a
following event's `time_ms`, a sync marker, a single-ADC event with no
RTC/dummy flags, an event with RTC present, an event with the dummy flag
set, an event with three active ADCs, a truncated event (EOF mid-read), an
unrecognized word, and a missing `[LISTDATA]` marker.
