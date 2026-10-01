# Daily-card review evidence

`all-cards-contact-sheet.png` contains all 251 exact framebuffers in National
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
| Cards | 251 |
| Sheet dimensions | 2,072 × 5,400 pixels |
| PNG format | one-bit grayscale, non-interlaced |
| SHA-256 | `fe06ffdc36117298e71049b57ab52b5d15e9c0cad8ca0fbef71963bb603a685a` |
| Visual inspection | content revision 3 and renderer revision 4 reviewed at native pixels on 2026-10-01 |
| Physical panel/print review | pending hardware qualification |

The companion `actual-size-review.html` provides four representative cards at
the nominal physical panel size for a 100%-scale print check. Neither artifact
contains child, device, host-path, or USB-identifying data.
