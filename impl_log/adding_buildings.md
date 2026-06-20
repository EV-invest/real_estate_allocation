# Framework gaps hit while seeding the 6 Quy Nhơn properties

Snapshot after adding 4 built + 2 under-construction listings. Domain model
(`domain.rs`) unchanged from when the work started. Everything below is data I
had to drop or cram into free text because there was no field for it.

## High priority — structural model gaps
1. **No `developer` field.** Issue #3 is organized *by developer*, but developer
   names live buried in `additional_reasoning`. Can't group or filter by it.
2. **No construction status / handover date.** `PropertyState`
   (Purchased / Interesting / Purchasing) is an *acquisition* lifecycle, orthogonal
   to *built vs under-construction*. I overloaded `Purchasing` to mean "under-
   construction prospect" (Q1, Triton). Built-vs-not and handover date have no home.
3. **Price model too thin.** Single required non-negative USD `Money`. No native
   VND/currency, no range, no per-unit-vs-whole-project basis, and **no way to say
   "price unknown/provisional"** — so Q1 ($150K) and Triton ($110K) are fabricated
   placeholders just to satisfy the field.
4. **Single `research_url`.** Can't store multiple sources (developer page +
   aggregator + Google Maps pin + brochure). Extras dropped or inlined as prose.

## Medium — missing structured attributes (all currently crammed into reasoning)
5. **No address/location text** — only `lat/lng`. Real addresses stored nowhere;
   geocoding done by hand and approximate (3 of the first 4 + both new coords are
   district-level estimates; only The Calla is exact).
6. **No specs**: floors, unit count, area m², price/m², #towers, unit mix, year.
7. **No operator/brand field** (e.g. Wyndham-run Q1) and **no amenities list**.
8. **Coords have no provenance/confidence** — can't flag "approximate".

## Files / media
9. **`FileKind` has no "floor plan / layout"** — filed as `Pic`, indistinguishable
   from photos. Also no caption, ordering, or cover-image designation on files.

## Process
10. **No runtime ingestion.** `seed()` is the only path in — every new listing is a
    code change + rebuild. And `seed()` self-skips a non-empty DB, so a stale local
    `data/app.db` silently ignores new entries (must be deleted first).

## Root smell
`additional_reasoning` has become a dumping ground for developer, specs, brand,
status, and thesis. That's the symptom; the fix is promoting items 1–8 to real
fields and letting reasoning go back to being just the thesis.
