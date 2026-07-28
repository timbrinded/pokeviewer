# V1 daily-card design

- Status: accepted for v1
- Design issue: [U14 / #15][issue-15]
- Renderer: `pokeviewer-core::render_daily_card`
- Last reviewed: 2026-07-28

The selected card is a quiet, full-screen character card with exactly the four
product-contract essentials. From top to bottom:

1. the display-day weekday, centered at 2× font scale;
2. the Pokémon Yellow front sprite, centered and rendered at 2× native pixels;
3. the English Pokémon name, centered at 3× font scale; and
4. the canonical type or types, centered at 2× font scale.

There is no border or decorative copy competing with the character. A
single-type card centers its type near the bottom. A dual-type card places the
primary and secondary types on separate lines, preserving the pack's canonical
order without requiring a small separator.

## Executable geometry

All coordinates are zero-based and the end value is exclusive:

| Group | Vertical pixels | Maximum content width |
| --- | ---: | ---: |
| Weekday | 3–17 | 106 pixels (`WEDNESDAY`) |
| Sprite canvas | 21–133 | 112 pixels |
| Name | 139–160 | 177 pixels (`FARFETCH’D`) |
| Single type | 177–191 | 94 pixels |
| Primary type | 166–180 | 94 pixels |
| Secondary type | 183–197 | 94 pixels |

The bands do not overlap. Each has white separation from its neighbors, and
all content remains inside the 200 × 200 panel. The renderer tests derive these
bounds from the production constants and audit all 151 committed records, so a
future name, font, scale, or content change cannot silently truncate a label.

## Review evidence

The [four representative actual-pixel cards][baseline] cover:

- Pikachu: short name and one long type;
- Charizard: dual types and a large sprite;
- Farfetch’d: the widest v1 name and punctuation; and
- Nidoran♀: a smaller source sprite and non-ASCII symbol.

The [151-card contact sheet][all-cards] places every exact 200 × 200 output in
National Pokédex order with eight-pixel gutters. It was visually inspected at
native pixels on 2026-07-28 after the content-revision-2 palette-split
regeneration. No label is truncated, no bands overlap, the sprites retain crisp
pixel edges without dither patterns, and the four required groups remain
distinct. The sheet intentionally contains no ID or caption inside a card
because those would be unapproved fifth elements.

An [actual-size print review page][print-review] renders the representative
cards at the panel's nominal 1.54-inch diagonal size when printed at 100%
scale. Physical printing and panel photography remain pending hardware
qualification evidence; desktop review is not a substitute for that sign-off.

## Retained-card rule

The renderer accepts one already-coherent `DailyCard`; it does not read a clock.
The runtime must derive both the weekday and Pokémon from the same
`DailySelection`. Before 07:00 it either retains the complete prior e-paper
card or renders the complete prior display day. It must never combine the new
calendar weekday with the prior Pokémon.

[all-cards]: ../evidence/daily-card-v1/README.md
[baseline]: ../evidence/renderer-baseline/README.md
[issue-15]: https://github.com/timbrinded/pokeviewer/issues/15
[print-review]: ../evidence/daily-card-v1/actual-size-review.html
