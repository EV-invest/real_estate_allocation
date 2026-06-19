# Amazon product-page UX — principles we're stealing

Reference: `docs/refs/amazon_product_page.html` (a real Amazon DP). Goal: their
seamless utility — easy perception of what's shown, disciplined hiding of what
isn't. `[x]` = applied, `[ ]` = todo.

## Principles

- [x] **One centered, capped container.** `#dp-container.a-container` ≈ `max-width:1500px; margin:0 auto`, `body{padding:20px 40px}`. Nothing full-bleed → identical read on laptop and 4K. Done via `max-w-[1200px] mx-auto` + `px-6 lg:px-8` gutters on header + body (header border stays full-bleed). This was the "flat/sparse on wide monitors" fix.

- [ ] **3-column F-pattern, one job per column.** left 45% = visual (gallery); center = identity/decision (title → price → about); right = the *one* boxed, isolated buy box. Eye flows image → what → cost → act; weight spent only on the CTA. We box every panel equally — nothing is prioritized, no single primary-action zone.

- [ ] **Tight named spacing scale, small end dominant.** `a-spacing-{none,mini,micro,small,base,medium,large}`; usage = none ×254, small ×90, base ×56, large rare. High density, deliberate small gaps — never a sparse void. Tighten inter-row spacing; stop leaning on uniform `gap-6`.

- [ ] **Cheap sections, not heavy cards.** section = thin `<hr a-divider-normal>` + compact bold heading (`a-size-base-plus a-text-bold`). Hairlines + type separate; borders/shadows are signal, not decoration. Partial: deal-terms already uses hairline rows; everything else is still wrapped in equally-heavy cards.

- [x] **Typography carries hierarchy.** big title, huge colored price, small muted labels, base body — scan headings to find anything. Serif title, colored price/accents, muted labels in place.

- [ ] **Progressive disclosure.** expanders / "see more" / collapsed bullets — show the 20% that decides, hide the rest. We render everything flat; no expanders yet.

## Next (when picked up)

- [ ] Card-weight pass: reserve borders/box for one primary element; demote the rest to hairline-divided sections (principles 2 + 4).
- [ ] Density pass: re-scale gaps toward the small end (principle 3).
- [ ] Add expanders for long terms/notes/media (principle 6).
