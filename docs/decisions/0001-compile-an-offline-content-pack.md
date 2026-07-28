---
status: accepted
date: 2026-07-27
decision-makers:
  - Project maintainer
---

# Compile a versioned offline content pack into `no_std` firmware

## Context and Problem Statement

Pokeviewer must select and render all Generation I cards while battery-powered
and without depending on network availability, credentials, or changing remote
data. Should content be fetched at runtime, cached after first boot, or converted
and compiled into each release?

## Decision Drivers

- Normal operation must remain useful without internet access.
- Network radios and request handling would increase active time and failure
  states.
- Every device on a release must render the same content for the same date.
- PokéAPI asks clients to cache requested resources locally.
- Third-party media provenance and release contents must remain auditable.

## Considered Options

- Fetch content from PokéAPI at runtime
- Download and cache content on each device
- Compile a versioned offline content pack into firmware

## Decision Outcome

Chosen option: "Compile a versioned offline content pack into firmware",
because it removes runtime networking, makes a release deterministic, and lets
the complete third-party payload be reviewed before distribution.

### Consequences

- Good, because normal operation has no network, account, or service dependency.
- Good, because host tests and firmware can use the exact same versioned data.
- Bad, because updating metadata or sprites requires a new firmware build.
- Bad, because the repository and release process must explicitly manage the
  rights risk of the compiled Pokémon media.

### Confirmation

CI must build and test without fetching PokéAPI resources, validate exactly 151
records, and prove that two conversions from the same explicit cache are
byte-identical.

## Pros and Cons of the Options

### Fetch content from PokéAPI at runtime

- Good, because upstream changes appear without a firmware release.
- Bad, because the toy stops being fully offline.
- Bad, because radio, TLS, API, timeout, and retry states increase energy use
  and operational complexity.

### Download and cache content on each device

- Good, because the device can operate offline after setup.
- Bad, because first-run operation still depends on a network.
- Bad, because devices can hold different content versions and need writable
  storage recovery behavior.

### Compile a versioned offline content pack into firmware

- Good, because the runtime has one deterministic, read-only content source.
- Good, because radio stacks can remain disabled.
- Bad, because content refreshes require an explicit maintainer workflow and
  release.

## More Information

- [V1 product contract](../product-contract.md)
- [Content-pack and daily-schedule contract v1](../content-pack-v1.md)
- [PokéAPI v2 fair-use policy](https://pokeapi.co/docs/v2)
- [Decision issue #2](https://github.com/timbrinded/pokeviewer/issues/2)
