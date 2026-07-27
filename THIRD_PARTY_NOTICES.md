# Third-party notices

Pokeviewer is an unofficial, non-commercial fan project.

Pokémon names, characters, artwork, sprites, trademarks, and other Pokémon
media are owned by their respective rights holders. Nothing in this repository
grants permission to use or redistribute that material. The project is not
affiliated with, endorsed by, or sponsored by Nintendo, Creatures Inc.,
GAME FREAK inc., or The Pokémon Company International.

The committed content pack derives sprites from the Pokémon Yellow sprite set
at PokeAPI/sprites revision
`8dfa3d97e953caaafaafd4963eff7621811af08e`. Those files are not covered by
Pokeviewer's MIT license. Converting a sprite to a monochrome firmware format
does not remove the underlying third-party rights.

The Pokémon Company's published legal information says its intellectual
property is not granted for use beyond personal, non-commercial home use.
Publishing converted sprites in a public repository therefore carries a real
removal or infringement-claim risk. Maintainers and redistributors must make
their own rights assessment and must be prepared to remove the media pack.

Pokémon metadata is sourced through [PokéAPI][pokeapi]. PokéAPI's fair-use
policy asks clients to cache requested resources locally; Pokeviewer uses an
explicit maintainer-only cache refresh and never contacts PokéAPI at runtime.

## License boundary

The `LICENSE` file applies only to original Pokeviewer source code,
documentation, and artwork explicitly identified as original. It does not
apply to:

- Pokémon names, characters, trademarks, or sprites;
- cached PokéAPI responses or media referenced by them;
- vendor documentation, schematics, example code, or binary firmware; or
- any other third-party material carrying its own terms.

[pokeapi]: https://pokeapi.co/docs/v2
[sprites]: https://github.com/PokeAPI/sprites/tree/8dfa3d97e953caaafaafd4963eff7621811af08e/sprites/pokemon/versions/generation-i/yellow
