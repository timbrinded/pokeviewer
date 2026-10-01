# V1 daily-card design

- Status: accepted for v1
- Design issue: [U14 / #15][issue-15]
- Renderer: `pokeviewer-core::render_daily_card`

The selected card is a quiet, full-screen character card with exactly the four
product-contract essentials. From top to bottom:

1. the display-day weekday, centered at 2× font scale;
2. the Pokémon Crystal front sprite, centered and rendered at 2× native pixels,
   with its four shades drawn as [dot patterns](../development/rendering.md#sprite-shading);
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
| Single type | 173–187 | 94 pixels |
| Primary type | 162–176 | 94 pixels |
| Secondary type | 178–192 | 94 pixels |
| `Recharge`: lightning icon and `CHARGE!` | 192–199 | 48 pixels, centered |
| `Unavailable`: `BAT ?` | 3–10 | 29 pixels, 3 pixels from the right edge |

The bands do not overlap, and all content stays inside the 200 × 200 panel.
`BAT ?` shares rows with the weekday but sits to its right. The renderer tests
derive these bounds from the production constants and check all 251 committed
records, so a name, font, scale, or content change cannot silently truncate a
label.

## Review evidence

The [four representative actual-pixel cards][baseline] cover:

- Pikachu: short name and one long type;
- Charizard: dual types and a large sprite;
- Farfetch’d: the widest name and punctuation; and
- Nidoran♀: a smaller source sprite and non-ASCII symbol.

The [251-card contact sheet][all-cards] shows every card in Pokédex order with
eight-pixel gutters, for checking truncation, overlap, and sprite conversion at
a glance. The [actual-size review page][print-review] prints the
representative cards at the panel's 1.54-inch size at 100% scale.

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
