---
status: accepted
date: 2026-07-27
decision-makers:
  - Project maintainer
---

# Use a passive card with a 07:00 local display-day boundary

## Context and Problem Statement

The e-paper display can retain an image without MCU power, and the intended user
does not need menus or controls. When should a new daily card appear, and what
should remain visible while the device sleeps?

## Decision Drivers

- The card should be ready near the start of a child's day.
- The device should spend almost all of its time asleep.
- The weekday and Pokémon must always describe the same display day.
- The retained e-paper image should remain useful through resets and sleep.
- V1 should have as few child-facing states as possible.

## Considered Options

- Refresh at calendar midnight
- Refresh at 07:00 local time and retain the complete prior card until then
- Refresh when the user presses a button

## Decision Outcome

Chosen option: "Refresh at 07:00 local time and retain the complete prior card
until then", because it aligns the change with the intended morning experience,
avoids interaction, and uses e-paper retention to minimize active time.

### Consequences

- Good, because the runtime normally has one scheduled wake and refresh per day.
- Good, because the device remains entirely passive for the child.
- Bad, because between midnight and 06:59:59 the visible weekday intentionally
  remains the prior calendar weekday.
- Bad, because adults must understand the display-day rule when testing or
  provisioning before 07:00.

### Confirmation

State-machine tests must cover both sides of 07:00 and calendar boundaries. A
physical qualification run must show that the complete prior card is retained
before the alarm and changes exactly once after it.

## Pros and Cons of the Options

### Refresh at calendar midnight

- Good, because the visible weekday matches the wall-clock date immediately.
- Bad, because the display changes when the intended user is likely asleep.
- Bad, because a midnight test is less convenient during physical qualification.

### Refresh at 07:00 local time and retain the complete prior card until then

- Good, because the refresh occurs near the intended use period.
- Good, because weekday and Pokémon remain one atomic card.
- Bad, because display-day and calendar-day differ for seven hours.

### Refresh when the user presses a button

- Good, because a refresh only happens on demand.
- Bad, because it makes the experience interactive and can cause repeated
  energy-intensive e-paper refreshes.

## More Information

- [V1 product contract](../product-contract.md)
- [Decision issue #2](https://github.com/timbrinded/pokeviewer/issues/2)
