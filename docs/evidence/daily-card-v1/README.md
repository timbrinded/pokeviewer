# V1 daily-card review evidence

`all-cards-contact-sheet.png` contains all 151 exact framebuffers in National
Pokédex order, left to right and then top to bottom. Each card remains exactly
200 × 200 pixels; eight white pixels separate adjacent cards.
Weekdays cycle Monday through Sunday only to exercise every label; the sheet is
a layout audit, not a calendar schedule.

Regenerate it with:

```console
cargo xtask render-contact-sheet \
  docs/evidence/daily-card-v1/all-cards-contact-sheet.png
```

| Property | Value |
| --- | --- |
| Cards | 151 |
| Sheet dimensions | 2,072 × 3,320 pixels |
| PNG format | one-bit grayscale, non-interlaced |
| SHA-256 | `8ee884bbd856dcda64c30b8239bba42e211057f04eb4861064a9b14f85000c74` |
| Visual inspection | content revision 2 reviewed at native pixels on 2026-07-28 |
| Physical panel/print review | pending hardware qualification |

The companion `actual-size-review.html` provides four representative cards at
the nominal physical panel size for a 100%-scale print check. Neither artifact
contains child, device, host-path, or USB-identifying data.
