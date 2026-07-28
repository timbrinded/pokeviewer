use std::io::Cursor;

use png::{BitDepth, ColorType, Transformations};
use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;

use super::TaskResult;

pub(super) const SPRITE_WIDTH: usize = 56;
pub(super) const SPRITE_HEIGHT: usize = 56;
pub(super) const SPRITE_BYTES: usize = SPRITE_WIDTH * SPRITE_HEIGHT / 8;
pub(super) const NO_SECONDARY_TYPE: u8 = 0xff;
const MAX_NAME_BYTES: usize = 16;
const SOURCE_PALETTE_COLORS: usize = 4;

#[derive(Deserialize)]
struct PokemonResponse {
    id: u16,
    types: Vec<TypeSlot>,
    sprites: Sprites,
}

#[derive(Deserialize)]
struct TypeSlot {
    slot: u8,
    #[serde(rename = "type")]
    kind: NamedResource,
}

#[derive(Deserialize)]
struct NamedResource {
    name: String,
}

#[derive(Deserialize)]
struct Sprites {
    versions: Versions,
}

#[derive(Deserialize)]
struct Versions {
    #[serde(rename = "generation-i")]
    generation_one: GenerationOne,
}

#[derive(Deserialize)]
struct GenerationOne {
    yellow: YellowSprites,
}

#[derive(Deserialize)]
struct YellowSprites {
    front_default: String,
}

#[derive(Deserialize)]
struct SpeciesResponse {
    id: u16,
    names: Vec<LocalizedName>,
}

#[derive(Deserialize)]
struct LocalizedName {
    name: String,
    language: NamedResource,
}

#[derive(Debug)]
pub(super) struct ConvertedRecord {
    pub(super) id: u16,
    pub(super) name: String,
    pub(super) primary_type: u8,
    pub(super) secondary_type: u8,
    pub(super) source_width: usize,
    pub(super) source_height: usize,
    pub(super) sprite: Vec<u8>,
}

#[derive(Debug)]
struct DecodedSprite {
    width: usize,
    height: usize,
    pixels: Vec<[u8; 4]>,
}

pub(super) fn validate_source_bytes(
    id: u16,
    pokemon: &[u8],
    species: &[u8],
    sprite: &[u8],
) -> TaskResult {
    parse_source(id, pokemon, species, sprite).map(|_| ())
}

pub(super) fn parse_source(
    id: u16,
    pokemon_bytes: &[u8],
    species_bytes: &[u8],
    sprite_bytes: &[u8],
) -> TaskResult<ConvertedRecord> {
    let pokemon: PokemonResponse = serde_json::from_slice(pokemon_bytes)
        .map_err(|error| format!("Pokémon ID {id}: Pokémon schema: {error}"))?;
    if pokemon.id != id {
        return Err(format!(
            "Pokémon ID {id}: Pokémon schema: response contains ID {}",
            pokemon.id
        ));
    }
    let sprite_suffix = format!("/{id}.png");
    if !pokemon
        .sprites
        .versions
        .generation_one
        .yellow
        .front_default
        .ends_with(&sprite_suffix)
    {
        return Err(format!(
            "Pokémon ID {id}: Pokémon schema: Yellow front sprite URL has an unexpected file"
        ));
    }
    let (primary_type, secondary_type) = parse_types(id, pokemon.types)?;

    let species: SpeciesResponse = serde_json::from_slice(species_bytes)
        .map_err(|error| format!("Pokémon ID {id}: species schema: {error}"))?;
    if species.id != id {
        return Err(format!(
            "Pokémon ID {id}: species schema: response contains ID {}",
            species.id
        ));
    }
    let english_names: Vec<_> = species
        .names
        .into_iter()
        .filter(|value| value.language.name == "en")
        .collect();
    if english_names.len() != 1 {
        return Err(format!(
            "Pokémon ID {id}: English name: expected one value, found {}",
            english_names.len()
        ));
    }
    let name: String = english_names[0].name.nfc().collect();
    validate_name(id, &name)?;

    let (sprite, source_width, source_height) = convert_sprite(id, sprite_bytes)?;
    Ok(ConvertedRecord {
        id,
        name,
        primary_type,
        secondary_type,
        source_width,
        source_height,
        sprite,
    })
}

fn parse_types(id: u16, mut types: Vec<TypeSlot>) -> TaskResult<(u8, u8)> {
    types.sort_by_key(|entry| entry.slot);
    if !(1..=2).contains(&types.len()) {
        return Err(format!(
            "Pokémon ID {id}: types: expected one or two entries, found {}",
            types.len()
        ));
    }
    for (index, entry) in types.iter().enumerate() {
        let expected_slot = u8::try_from(index + 1).map_err(|_| "type slot overflow")?;
        if entry.slot != expected_slot {
            return Err(format!(
                "Pokémon ID {id}: types: expected slot {expected_slot}, found {}",
                entry.slot
            ));
        }
    }
    let primary = type_code(id, &types[0].kind.name)?;
    let secondary = if types.len() == 2 {
        type_code(id, &types[1].kind.name)?
    } else {
        NO_SECONDARY_TYPE
    };
    if primary == secondary {
        return Err(format!("Pokémon ID {id}: types: duplicate type"));
    }
    Ok((primary, secondary))
}

fn type_code(id: u16, value: &str) -> TaskResult<u8> {
    [
        "normal", "fire", "water", "electric", "grass", "ice", "fighting", "poison", "ground",
        "flying", "psychic", "bug", "rock", "ghost", "dragon", "dark", "steel", "fairy",
    ]
    .iter()
    .position(|candidate| *candidate == value)
    .and_then(|index| u8::try_from(index).ok())
    .ok_or_else(|| format!("Pokémon ID {id}: types: unsupported type {value}"))
}

fn validate_name(id: u16, name: &str) -> TaskResult {
    if name.is_empty() || name.len() > MAX_NAME_BYTES {
        return Err(format!(
            "Pokémon ID {id}: English name: expected 1–{MAX_NAME_BYTES} UTF-8 bytes, found {}",
            name.len()
        ));
    }
    if name.chars().any(char::is_control) {
        return Err(format!(
            "Pokémon ID {id}: English name: control characters are forbidden"
        ));
    }
    Ok(())
}

fn convert_sprite(id: u16, bytes: &[u8]) -> TaskResult<(Vec<u8>, usize, usize)> {
    let decoded = decode_sprite(id, bytes)?;
    let palette = source_palette(id, &decoded.pixels)?;
    let mut output = vec![0; SPRITE_BYTES];
    let x_offset = (SPRITE_WIDTH - decoded.width) / 2;
    let y_offset = (SPRITE_HEIGHT - decoded.height) / 2;
    for (source_index, [red, green, blue, alpha]) in decoded.pixels.into_iter().enumerate() {
        let color = [red, green, blue];
        if alpha == 255 && palette[..SOURCE_PALETTE_COLORS / 2].contains(&color) {
            let source_x = source_index % decoded.width;
            let source_y = source_index / decoded.width;
            let output_index = (source_y + y_offset) * SPRITE_WIDTH + source_x + x_offset;
            output[output_index / 8] |= 1 << (7 - output_index % 8);
        }
    }
    Ok((output, decoded.width, decoded.height))
}

fn source_palette(id: u16, pixels: &[[u8; 4]]) -> TaskResult<Vec<[u8; 3]>> {
    let mut palette = Vec::with_capacity(SOURCE_PALETTE_COLORS);
    for &[red, green, blue, alpha] in pixels {
        if alpha >= 128 && alpha != 255 {
            return Err(format!(
                "Pokémon ID {id}: sprite palette: visible pixels must be fully opaque"
            ));
        }
        let color = [red, green, blue];
        if alpha == 255 && !palette.contains(&color) {
            palette.push(color);
        }
    }
    palette.sort_by_key(|&[red, green, blue]| (luminance(red, green, blue), red, green, blue));
    if palette.len() != SOURCE_PALETTE_COLORS {
        return Err(format!(
            "Pokémon ID {id}: sprite palette: expected exactly {SOURCE_PALETTE_COLORS} opaque colours, found {}",
            palette.len()
        ));
    }
    if palette.last() != Some(&[255, 255, 255]) {
        return Err(format!(
            "Pokémon ID {id}: sprite palette: lightest colour must be opaque white"
        ));
    }
    Ok(palette)
}

fn luminance(red: u8, green: u8, blue: u8) -> u32 {
    (299 * u32::from(red) + 587 * u32::from(green) + 114 * u32::from(blue) + 500) / 1000
}

fn decode_sprite(id: u16, bytes: &[u8]) -> TaskResult<DecodedSprite> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|error| format!("Pokémon ID {id}: sprite PNG: {error}"))?;
    let size = reader.output_buffer_size().ok_or_else(|| {
        format!("Pokémon ID {id}: sprite PNG: decoded buffer size is unavailable")
    })?;
    let mut decoded_buffer = vec![0; size];
    let info = reader
        .next_frame(&mut decoded_buffer)
        .map_err(|error| format!("Pokémon ID {id}: sprite PNG: {error}"))?;
    let source_width = usize::try_from(info.width)
        .map_err(|_| format!("Pokémon ID {id}: sprite width exceeds usize"))?;
    let source_height = usize::try_from(info.height)
        .map_err(|_| format!("Pokémon ID {id}: sprite height exceeds usize"))?;
    if source_width == 0
        || source_height == 0
        || source_width > SPRITE_WIDTH
        || source_height > SPRITE_HEIGHT
    {
        return Err(format!(
            "Pokémon ID {id}: sprite dimensions: expected non-empty dimensions no larger than {SPRITE_WIDTH}x{SPRITE_HEIGHT}, found {}x{}",
            info.width, info.height
        ));
    }
    if info.bit_depth != BitDepth::Eight {
        return Err(format!(
            "Pokémon ID {id}: sprite PNG: expected decoded 8-bit channels"
        ));
    }

    let pixels = rgba_pixels(
        id,
        info.color_type,
        &decoded_buffer[..info.buffer_size()],
        source_width * source_height,
    )?;
    Ok(DecodedSprite {
        width: source_width,
        height: source_height,
        pixels,
    })
}

fn rgba_pixels(
    id: u16,
    color: ColorType,
    bytes: &[u8],
    expected_pixels: usize,
) -> TaskResult<Vec<[u8; 4]>> {
    let pixels: Vec<[u8; 4]> = match color {
        ColorType::Rgba => bytes
            .chunks_exact(4)
            .map(|value| [value[0], value[1], value[2], value[3]])
            .collect(),
        ColorType::Rgb => bytes
            .chunks_exact(3)
            .map(|value| [value[0], value[1], value[2], 255])
            .collect(),
        ColorType::GrayscaleAlpha => bytes
            .chunks_exact(2)
            .map(|value| [value[0], value[0], value[0], value[1]])
            .collect(),
        ColorType::Grayscale => bytes
            .iter()
            .map(|value| [*value, *value, *value, 255])
            .collect(),
        ColorType::Indexed => {
            return Err(format!(
                "Pokémon ID {id}: sprite PNG: indexed color was not expanded"
            ));
        }
    };
    if pixels.len() != expected_pixels {
        return Err(format!(
            "Pokémon ID {id}: sprite PNG: decoded pixel count mismatch"
        ));
    }
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    const POKEMON: &[u8] = br#"{
      "id": 1,
      "types": [
        {"slot": 2, "type": {"name": "poison"}},
        {"slot": 1, "type": {"name": "grass"}}
      ],
      "sprites": {"versions": {"generation-i": {"yellow": {
        "front_default": "https://example.invalid/1.png"
      }}}}
    }"#;
    const SPECIES: &[u8] = br#"{
      "id": 1,
      "names": [{"name": "Bulbasaur", "language": {"name": "en"}}]
    }"#;

    #[test]
    fn representative_source_converts_to_contract_record() {
        let record = parse_source(1, POKEMON, SPECIES, &fixture_png()).unwrap();

        assert_eq!(record.id, 1);
        assert_eq!(record.name, "Bulbasaur");
        assert_eq!(record.primary_type, 4);
        assert_eq!(record.secondary_type, 7);
        assert_eq!(record.sprite.len(), SPRITE_BYTES);
        assert_eq!(record.sprite[0], 0b1100_1100);
    }

    #[test]
    fn malformed_source_reports_id_and_rule() {
        let error = parse_source(2, POKEMON, SPECIES, &fixture_png()).unwrap_err();

        assert!(error.contains("Pokémon ID 2"));
        assert!(error.contains("response contains ID 1"));
    }

    #[test]
    fn invalid_sprite_dimensions_report_id_and_rule() {
        let error =
            parse_source(1, POKEMON, SPECIES, &fixture_png_with_dimensions(57, 56)).unwrap_err();

        assert!(error.contains("Pokémon ID 1"));
        assert!(error.contains("sprite dimensions"));
    }

    #[test]
    fn native_sprite_is_centered_without_scaling() {
        let record =
            parse_source(1, POKEMON, SPECIES, &fixture_png_with_dimensions(40, 40)).unwrap();

        assert!(record.sprite[..57].iter().all(|byte| *byte == 0));
        assert_eq!(record.sprite[57], 0b1100_1100);
    }

    #[test]
    fn unexpected_source_palette_reports_id_and_rule() {
        let png = fixture_png_with_palette(
            56,
            56,
            &[[0, 0, 0, 255], [128, 128, 128, 255], [255, 255, 255, 255]],
        );
        let error = parse_source(1, POKEMON, SPECIES, &png).unwrap_err();

        assert!(error.contains("Pokémon ID 1"));
        assert!(error.contains("expected exactly 4 opaque colours"));
    }

    #[test]
    fn semitransparent_source_pixel_reports_id_and_rule() {
        let png = fixture_png_with_palette(
            56,
            56,
            &[
                [0, 0, 0, 255],
                [85, 85, 85, 200],
                [170, 170, 170, 255],
                [255, 255, 255, 255],
            ],
        );
        let error = parse_source(1, POKEMON, SPECIES, &png).unwrap_err();

        assert!(error.contains("Pokémon ID 1"));
        assert!(error.contains("visible pixels must be fully opaque"));
    }

    fn fixture_png() -> Vec<u8> {
        fixture_png_with_dimensions(56, 56)
    }

    fn fixture_png_with_dimensions(width: u32, height: u32) -> Vec<u8> {
        fixture_png_with_palette(
            width,
            height,
            &[
                [0, 0, 0, 255],
                [85, 85, 85, 255],
                [170, 170, 170, 255],
                [255, 255, 255, 255],
            ],
        )
    }

    fn fixture_png_with_palette(width: u32, height: u32, palette: &[[u8; 4]]) -> Vec<u8> {
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, width, height);
            encoder.set_color(ColorType::Rgba);
            encoder.set_depth(BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            let pixel_count = usize::try_from(width * height).unwrap();
            let pixels: Vec<u8> = (0..pixel_count)
                .flat_map(|index| palette[index % palette.len()])
                .collect();
            writer.write_image_data(&pixels).unwrap();
        }
        png
    }
}
