# Framework gaps hit while seeding the 6 Quy Nhơn properties

Snapshot of remaining gaps. Items 1–4 are now DONE (see "Resolved" at the bottom);
item 5 is next; 6/7/9 wait for the building/apartment split.

## Next up
5. **Drop hand-entered lat/lng for a Google Place reference.** Coords are still bare
   `Coords { lat, lng }`, hand-set and mostly approximate. Decided: store a Google
   **Place ID**, resolve the map pin via the Places API, and derive name/address/
   coords from it later. Touches `map.rs` (JS marker rendering), the Maps loader in
   `app.rs` (needs `libraries=places`), and the model (`Coords` → place id). Needs a
   real Place ID per property (only The Calla's is known so far).

## Deferred to the building/apartment model split
6. **No specs**: floors, unit count, area m², price/m², #towers, unit mix, year.
7. **No operator/brand field** (e.g. Wyndham-run Q1) and **no amenities list**.
9. **`FileKind` has no floor-plan/layout kind** (filed as `Pic`); no caption/order/
   cover designation on files.
These are *building* attributes — park them until Property splits into building +
apartment.

## Resolved (this pass)
1. **`developer` field — DONE.** Optional `Property.developer` (name) → new
   `developers` table (`name`, `note`, `page`), enforced by a SQLite FK. The note
   shows on hover over the developer field (details panel, `ev` Tooltip). A `//TODO`
   on `Developer::note` (domain.rs) tracks generalizing the note concept to an
   arbitrary (table,key) side-table.
2. **Construction status — DONE.** New `ConstructionStatus { UnderConstruction,
   Completed }` field, modeled like `PropertyState`. The two towers are now
   `UnderConstruction`; the four built are `Completed`.
3. **Optional price — DONE.** `price: Option<Money>`; `None` renders as a `?` in the
   new `--color-warn` amber across header/chart/details/embed. Dummy Q1/Triton
   prices removed.
4. **research_url — no change needed.** It points at *our* own article; developer
   homepage lives in `developers.page`, per-property brochures in documents, and the
   map pin becomes the Google Place (item 5).

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
    `data/app.db` silently ignores new entries (must be deleted first). Now also:
    adding the developers table + new NOT NULL columns means an OLD `data/app.db`
    won't migrate — delete it to re-seed.

## Root smell (partly addressed)
`additional_reasoning` was a dumping ground for developer/specs/brand/status/thesis.
Promoting developer + construction out (items 1–2) shrank it; the `reasoning` strings
were trimmed of their "Developer: …" prefixes. Specs/brand (6/7) still live there
until the building model exists.
