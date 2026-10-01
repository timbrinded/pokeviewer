---
status: proposed
date: 2026-10-01
decision-makers:
  - Project maintainer
---

# Show Generations I and II with shaded Crystal sprites

## Context and Problem Statement

The [product contract](../product-contract.md) limits the card to National
Pokédex 1–151 with Pokémon Yellow sprites. [Issue #40][issue-40] asks for all
251 Generation I and II Pokémon. Generation II has no Yellow sprite, so the
sprite source, the pack format, and the schedule must change together.

The Pokémon Crystal sprites are four-colour Game Boy Color images. Each one has
white, black, and two hues; the art was drawn as four grey shades. The v1 rule
maps the two darkest colours to black and the two lightest to white. Applied
to Crystal, that rule turns large regions into solid black. 39 of the 251
sprites would be more than 80 % ink inside their outline. Vileplume's body and
face, Gengar, Umbreon, Houndoom, and Murkrow lose most of their detail, and
Snorlax loses its belly. The panel is a 1.54-inch, 200 × 200, one-bit display,
and each sprite pixel is drawn as a 2 × 2 block of panel pixels.

Which sprites should the card use, and how should four shades appear on a
black-and-white panel?

## Decision Drivers

- One consistent art style across all 251 cards.
- A child must recognise the Pokémon at 1.54 inches; no face may become a
  black blob.
- Keep the panel in its existing one-bit, full-refresh mode.
- Keep the pack offline, deterministic, and validated before the panel is
  powered.

## Considered Options

- Sprite source:
  - mix Yellow (Gen I) and Crystal (Gen II) sprites; or
  - use Crystal sprites for all 251.
- Tone mapping:
  - keep the v1 one-bit threshold;
  - store four shades and draw each as a 2 × 2 ordered-dither pattern; or
  - also lighten solid-black interiors to 75 % while keeping outlines solid.

## Decision Outcome

Chosen options: "use Crystal sprites for all 251", and "store four shades and
draw each as a 2 × 2 ordered-dither pattern, with solid-black interiors at
75 %".

- **Content.** The pack holds IDs 1–251 with Crystal front sprites from the
  same pinned PokeAPI/sprites commit. Shades are ranked by colour luminance:
  `0` white, `1` light, `2` dark, `3` black, two bits per pixel, 784 bytes per
  sprite.
- **Rendering.** The renderer draws each sprite pixel as a 2 × 2 cell with
  0, 1, 2, or 4 black pixels for shades 0–3, using a 2 × 2 Bayer threshold so
  that adjacent cells tile without seams. A black pixel whose eight neighbours
  are all black gets 3 black pixels. Outlines, pupils, and black detail up to
  two pixels thick stay solid. Inside the outline, the darkest sprite drops to
  about 79 % ink, and none exceeds 80 %.
- **Panel.** Every panel pixel is still black or white. The panel keeps its
  one-bit full refresh, with no greyscale waveform. The contract's exclusion of
  greyscale effects is unchanged in that sense, and it is reworded to say so.
- **Schedule v2.** The 2026-01-01 epoch and the 07:00 rollover stay.
  `dex_id = (94 × cycle_index) mod 251 + 1` with a 251-day cycle. Within any
  seven days, every two cards are at least 31 Pokédex numbers apart, the
  maximum for a 251-day multiplier schedule. The card shown on a given date
  changes once when a device updates.
- **Format.** Pack format 2, content revision 3, schedule version 2. The pack
  budget rises from 64 KiB to 256 KiB. The firmware text budget applies to
  text without the pack and stays at the 134,464 bytes of code that the
  previous 200,000-byte text budget allowed beside a full pack.

Ranking by luminance differs from the original Game Boy shade order for 8 of
the 251 sprites: Spearow, Fearow, Farfetch'd, Hitmonlee, Scyther, Jynx,
Porygon, and Porygon2. The pret/pokecrystal build marks exactly these
palettes `--reverse`. Neither order looked consistently better on the panel,
so the contract keeps one luminance rule and no exception list.

### Consequences

- Good, because all 251 cards share one art style, and shaded areas keep their
  form on a one-bit panel.
- Good, because the pack stays semantic and the look is set in one reviewed
  renderer table, with exact-pixel goldens.
- Bad, because the pack grows from 61,390 to 200,410 bytes of flash.
- Bad, because the daily Pokémon changes once on update, and the cycle
  repeats after 251 days instead of 151.
- Bad, because 25 % and 50 % dot patterns add texture that a solid fill would
  not. Physical-panel review is still needed.

### Confirmation

`cargo xtask content-build` reproduces the committed pack byte for byte from
`content/cache-v2`. Host tests render all 251 records through the integrated
path and check the dither ink per cell. Goldens include Umbreon (Gen II
single type), Celebi (Gen II dual type), and Vileplume. One device check shows
a Gen II card after a synthetic rollover.

## More Information

- Partly supersedes the 151-record scope of
  [ADR 0001](0001-compile-an-offline-content-pack.md); its offline-pack
  decision remains.
- [Content-pack and daily-schedule contract v2](../content-pack-v2.md)
- [Tone-mapping evidence](../evidence/crystal-sprites/README.md)
- [pret/pokecrystal palette build rules][pret-makefile]

[issue-40]: https://github.com/timbrinded/pokeviewer/issues/40
[pret-makefile]: https://github.com/pret/pokecrystal/blob/master/Makefile
