# E-paper panel bring-up

This procedure qualifies the non-touch 1.54-inch V2 panel before application
integration. It is diagnostic firmware, not the final daily-card experience.

## Fixed configuration

- driver: `epd_waveshare::epd1in54_v2::Epd1in54`;
- SPI2 at 10 MHz, mode 0;
- SCK GPIO12, MOSI GPIO13, CS GPIO11;
- DC GPIO10, reset GPIO9, BUSY GPIO8;
- active-low panel power on GPIO6;
- one 5,000-byte, one-bit framebuffer; and
- full refresh only.

BUSY is active high. The board adapter polls every 10 ms and terminates a wait
after 500 polls, giving a five-second upper bound. A timeout is printed as a
failure and the panel rail is switched off. The audio rail remains disabled on
GPIO42 throughout.

## Run

Build and flash using the pinned toolchain:

```sh
cargo xtask firmware-build
cargo xtask firmware-flash
```

The diagnostic displays, in order:

1. full white;
2. full black;
3. a 20-pixel checkerboard;
4. a two-pixel border; and
5. a centered text identification frame.

The first four frames remain visible for two seconds after refresh. The final
text frame is retained after the driver enters sleep and GPIO6 disables the
panel rail.

Expected terminal output is one of:

```text
display diagnostics complete; panel rail off
display diagnostics failed: <bounded reason>
```

## Qualification evidence

Do not mark hardware bring-up complete until all rows have sanitized evidence.

| Check | Required evidence | Status |
| --- | --- | --- |
| five patterns | clear, correctly oriented photos | pending |
| clipping | border visible on all four edges | pending |
| BUSY polarity | logic trace or measured levels | pending |
| refresh bound | measured duration below five seconds | pending |
| passive image | final frame visible with GPIO6 high | pending |
| panel current | refresh and rail-off measurements | pending |

Follow the [privacy and evidence rules](../privacy-and-evidence.md). Exclude USB
serials, MAC addresses, home paths, and unsanitized terminal output.
