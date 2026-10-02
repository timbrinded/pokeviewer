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

Pack sprites hold four shades, from `0` white to `3` black. Mapping two shades
to black and two to white would leave 39 of the 251 Crystal sprites more than
80 % ink inside their outline: Vileplume's body and face, Gengar, Umbreon, and
Murkrow become solid black, and Snorlax loses its belly. The renderer therefore
draws each sprite pixel as a 2 × 2 cell of panel pixels and blackens part of
the cell.

Shades 0–3 blacken 0, 1, 2, or 4 of the cell's panel pixels: white, 25 %,
50 %, and black. A panel pixel is black when the cell's ink exceeds its entry
in the threshold matrix `[[0, 2], [3, 1]]`, indexed `[row][column]`. So 25 %
is the top-left dot, 50 % is a checkerboard, and adjacent cells tile without
seams.

One exception keeps large black areas readable. A black sprite pixel that is
one of the four centre pixels of an all-black 4 × 4 block of sprite pixels,
lying entirely within the 56 × 56 sprite, gets 3 of 4 (75 %); its bottom-left
panel pixel stays white. Black detail up to three sprite pixels wide, such as
outlines, eyes, and spots, stays solid. With this table, only Murkrow is above
80 % ink, at 80.4 %, and light bodies such as Pikachu read as a 25 % stipple
instead of disappearing into the white background.

Every panel pixel is black or white, and the panel uses its normal full
refresh. `SHADE_INK`, `SOLID_INTERIOR_INK`, `SOLID_BLOCK`, and
`DITHER_THRESHOLDS` in `render.rs` are the reviewed tone table; changing any of
them is a visual change that needs `golden-update`.

## Memory report

| Item | Storage | Allocation |
| --- | ---: | --- |
| Panel framebuffer | 5,000 bytes RAM | fixed value |
| Font bitmaps | 322 bytes read-only program data: 45 glyphs and the lightning icon | fixed value |
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

The README device views place exact frames on a drawing of the V2 case:

```console
cargo xtask render-device-views docs/images/device
```

The case follows the Waveshare outline drawing: 39.8 × 53.0 × 16.9 mm, a
27.8 mm screen window, `BOOT` above `PWR` on the right side, and USB-C on the
bottom. Only the screen pixels are exact. Regenerate the views with any
reviewed golden change.

CI compares the recovery screens and the device views byte for byte; the
other images are review aids. The [daily-card design](../design/daily-card-v1.md) explains the
layout.
