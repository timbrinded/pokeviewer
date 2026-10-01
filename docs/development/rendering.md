# Renderer and visual goldens

The `pokeviewer-core` renderer is the single source of framebuffer bytes for
host screenshots and supported-board firmware. It has no board, filesystem,
clock, network, allocator, or driver access.

## Buffer contract

`Framebuffer` owns exactly 5,000 bytes for the 200 × 200 panel. Pixels are
row-major and most-significant-bit first. `1` is white and `0` is black, which
is the native polarity expected by the pinned Waveshare driver. The host PBM
writer inverts those bytes because raw PBM uses `1` for black; the one-bit PNG
writer uses the panel bytes directly.

`render_daily_card` accepts only a typed `DailyCard`: one `Weekday`, a borrowed
English name, one or two `PokemonType` values, a borrowed fixed-size sprite,
and one battery state: `Normal`, `Recharge`, or `Unavailable`.
It validates the complete input before clearing or drawing. Empty, oversized,
unsupported, duplicate-type, or over-wide input returns a bounded
`RenderError` and leaves the prior framebuffer unchanged.

The fixed font covers the complete committed name vocabulary, including the
curly apostrophe, the female and male signs, the hyphen, and digits. An
exhaustive host test renders all 251 committed records.

The renderer does not derive battery state from voltage. `Normal` shows no
battery text or icon, `Recharge` shows the lightning icon and `CHARGE!` at the
bottom, and `Unavailable` shows `BAT ?` in the top-right corner. The card
contains no battery percentage.

## Sprite shading

Pack sprites hold four shades, from `0` white to `3` black. The renderer draws
each sprite pixel as a 2 × 2 panel cell and sets 0, 1, 2, or 4 of its pixels
black for shades 0–3: white, 25 %, 50 %, and black. A black sprite pixel
that is one of the four centre pixels of an all-black 4 × 4 square gets 3 of
4 (75 %), so large black areas keep their form while outlines, eyes, spots,
and black detail up to three pixels wide stay solid. A fixed 2 × 2 ordered-dither threshold, `[[0, 2],
[3, 1]]`, chooses which pixels, so 25 % is one dot per cell, 50 % is a
checkerboard, and adjacent cells tile without seams.

Every panel pixel is still black or white, and the panel uses its normal full
refresh. `SHADE_INK` and `SOLID_INTERIOR_INK` in `render.rs` are the reviewed
tone table; changing them is a visual change that needs `golden-update`.

The Crystal palettes are why the shading exists. With two shades mapped to
black and two to white, 39 of the 251 sprites are more than 80 % ink inside
their outline: Vileplume's body and face, Gengar, Umbreon, and Murkrow become
solid black, and Snorlax loses its belly. With this table, no sprite is above
about 80 %, and light bodies such as Pikachu read as a 25 % stipple instead
of disappearing into the white background.

## Memory report

| Item | Storage | Allocation |
| --- | ---: | --- |
| Panel framebuffer | 5,000 bytes RAM | fixed value |
| Font bitmaps | 231 bytes read-only program data | fixed value |
| Current sprite | borrowed 784-byte pack slice | none |
| `DailyCard` strings and sprite | borrowed views | none |
| Renderer work buffer | 0 bytes | none |
| Heap | 0 bytes | none |

Rasterization uses only bounded scalar loop state. It never copies the font,
name, or sprite into a temporary buffer.

## Visual goldens

The goldens in `tests/goldens` are raw 5,000-byte framebuffers, so a check
compares exact panel bytes rather than screenshots. They cover every weekday,
layout edge cases, and the `Recharge` and `Unavailable` battery states. The
[goldens README](../../tests/goldens/README.md) lists the cases.

```console
cargo xtask golden-check target/visual-diff
```

The check renders each case from the committed pack and fails if any pixel
differs. For each changed case it writes `*-expected.png`, `*-actual.png`,
`*-diff.png` (black where pixels differ), and `*-report.txt` with the changed
coordinates and hashes. CI uploads this directory as `visual-diff`.

After a reviewed design change, regenerate the goldens and review every PNG
and the manifest diff before you commit:

```console
cargo xtask golden-update
cargo xtask golden-check target/visual-diff
```

`cargo xtask golden-demo-failure DIR` flips one pixel to show what a failure
looks like; the [committed example](../evidence/golden-failure/README.md) was
made this way.

## Review images

These commands regenerate the committed review images:

```console
cargo xtask render-samples docs/evidence/renderer-baseline
cargo xtask render-contact-sheet docs/evidence/daily-card-v1/all-cards-contact-sheet.png
cargo xtask render-setup-screen docs/evidence/setup-screen/invalid-rtc-setup.png
cargo xtask render-recovery-screens docs/evidence/recovery-screens
```

CI compares only the recovery screens byte for byte; the other images are
review aids. The [daily-card design](../design/daily-card-v1.md) explains the
layout.
