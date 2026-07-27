# Shared renderer

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
English name, one or two `PokemonType` values, and a borrowed fixed-size sprite.
It validates the complete input before clearing or drawing. Empty, oversized,
unsupported, duplicate-type, or over-wide input returns a bounded
`RenderError` and leaves the prior framebuffer unchanged.

The fixed font covers the complete committed v1 name vocabulary, including the
curly apostrophe and the female and male signs. An exhaustive host test renders
all 151 committed records.

## Memory report

| Item | Storage | Allocation |
| --- | ---: | --- |
| Panel framebuffer | 5,000 bytes RAM | fixed value |
| Font bitmaps | 231 bytes read-only program data | fixed value |
| Current sprite | borrowed 392-byte pack slice | none |
| `DailyCard` strings and sprite | borrowed views | none |
| Renderer work buffer | 0 bytes | none |
| Heap | 0 bytes | none |

Rasterization uses only bounded scalar loop state. It never copies the font,
name, or sprite into a temporary buffer.

## Host evidence

Generate deterministic representative PBM and one-bit PNG files with:

```console
cargo xtask render-samples target/render-samples
```

The committed [baseline evidence](../evidence/renderer-baseline/README.md)
contains Pikachu, Charizard, Farfetch’d, and Nidoran♀. These exercise
single/dual types, short/long names, punctuation, a non-ASCII glyph, and
different source-sprite dimensions. Visual styling is reviewed and locked by
the separate daily-card design issue.
