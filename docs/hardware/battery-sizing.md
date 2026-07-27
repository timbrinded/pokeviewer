# Battery sizing from measured current

Pokeviewer makes no universal runtime claim. Cell capacity, protection,
temperature, age, conversion losses, self-discharge, and the exact board
measurements all affect runtime.

After qualification, record:

- `I_sleep`: measured deep-sleep current in mA;
- `I_refresh`: average current during one complete wake and refresh in mA;
- `t_refresh`: measured wake and refresh duration in seconds;
- `N`: refreshes per day, normally `1`; and
- `u`: conservative usable-capacity fraction from `0` to `1`.

For a 72-hour target, calculate:

```text
sleep_hours = 72 - (3 * N * t_refresh / 3600)
required_mAh =
  (I_sleep * sleep_hours + I_refresh * 3 * N * t_refresh / 3600) / u
```

Example arithmetic must use measured V2-board values and the intended protected
cell's documented usable capacity. Until those measurements exist, leave the
inputs blank; substituting datasheet typicals would turn a sizing worksheet
into an unsupported runtime promise.

The separate [V2 board contract](v2-board-contract.md) remains the authority
for polarity and protected-cell safety checks.
