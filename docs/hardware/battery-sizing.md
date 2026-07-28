# Battery sizing from measured current

Pokeviewer makes no universal runtime claim. Cell capacity, protection,
temperature, age, conversion losses, self-discharge, and the exact board
measurements all affect runtime.

After qualification, record:

- `I_sleep`: measured deep-sleep current in mA;
- `I_active`: average current before panel refresh in mA;
- `t_active`: measured active duration outside panel refresh in seconds;
- `I_refresh`: average current during the panel refresh in mA;
- `t_refresh`: measured panel-refresh duration in seconds; and
- `u`: conservative usable-capacity fraction from `0` to `1`.

V1 has one scheduled wake/refresh per day. For a 72-hour target, calculate:

```text
sleep_hours = 72 - (3 * (t_active + t_refresh) / 3600)
required_mAh =
  (
    I_sleep * sleep_hours
    + I_active * 3 * t_active / 3600
    + I_refresh * 3 * t_refresh / 3600
  ) / u
```

Example arithmetic must use measured V2-board values and the intended protected
cell's documented usable capacity. Until those measurements exist, leave the
inputs blank; substituting datasheet typicals would turn a sizing worksheet
into an unsupported runtime promise.

Use the exact command and thresholds in the
[release qualification procedure](release-qualification.md). Battery selection
and charging remain governed by the [safety guide](../safety.md).
