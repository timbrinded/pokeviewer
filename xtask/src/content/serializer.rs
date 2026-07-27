use std::{fs, path::Path};

use super::{
    CACHE_SCHEMA_VERSION, CONTENT_REVISION, CacheManifest, FIRST_ID, FORMAT_VERSION, LAST_ID,
    PackManifest, PackManifestEntry, SCHEDULE_VERSION, SPRITES_REVISION, SourceEntry, TaskResult,
    read, sha256_hex,
    source::{ConvertedRecord, SPRITE_BYTES, SPRITE_HEIGHT, SPRITE_WIDTH, parse_source},
    write_json,
};

const HEADER_LENGTH: usize = 32;
const RECORD_LENGTH: usize = 6;
const RECORD_COUNT: usize = 151;
const MAX_PACK_BYTES: usize = 65_536;

pub(super) fn convert_cache(
    cache_dir: &Path,
    pack_path: &Path,
    pack_manifest_path: &Path,
) -> TaskResult {
    let source_manifest_path = cache_dir.join("manifest.json");
    let source_manifest_bytes = fs::read(&source_manifest_path).map_err(|error| {
        format!(
            "failed to read source manifest {}: {error}",
            source_manifest_path.display()
        )
    })?;
    let source_manifest: CacheManifest = serde_json::from_slice(&source_manifest_bytes)
        .map_err(|error| format!("cache manifest: invalid JSON: {error}"))?;
    validate_manifest(&source_manifest)?;

    let mut converted = Vec::with_capacity(RECORD_COUNT);
    let mut provenance = Vec::with_capacity(RECORD_COUNT);
    for expected_id in FIRST_ID..=LAST_ID {
        let entry = &source_manifest.entries[usize::from(expected_id - FIRST_ID)];
        validate_entry_contract(entry, expected_id)?;
        let pokemon = read_and_hash(cache_dir, expected_id, &entry.pokemon, "Pokémon response")?;
        let species = read_and_hash(cache_dir, expected_id, &entry.species, "species response")?;
        let sprite = read_and_hash(cache_dir, expected_id, &entry.sprite, "Yellow sprite")?;
        let record = parse_source(expected_id, &pokemon, &species, &sprite)?;
        provenance.push(PackManifestEntry {
            id: expected_id,
            name: record.name.clone(),
            pokemon_sha256: entry.pokemon.sha256.clone(),
            species_sha256: entry.species.sha256.clone(),
            sprite_sha256: entry.sprite.sha256.clone(),
        });
        converted.push(record);
    }

    let pack = build_pack(&converted)?;
    let reproducibility_check = build_pack(&converted)?;
    if pack != reproducibility_check {
        return Err("pack: repeated conversion produced different bytes".into());
    }
    create_parent(pack_path)?;
    fs::write(pack_path, &pack)
        .map_err(|error| format!("failed to write {}: {error}", pack_path.display()))?;
    create_parent(pack_manifest_path)?;
    let pack_file_name = pack_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "pack output must have a UTF-8 file name".to_owned())?;
    write_json(
        pack_manifest_path,
        &PackManifest {
            format_version: FORMAT_VERSION,
            content_revision: CONTENT_REVISION,
            schedule_version: SCHEDULE_VERSION,
            source_manifest_sha256: sha256_hex(&source_manifest_bytes),
            sprites_revision: source_manifest.sprites_revision,
            pack_path: pack_file_name.to_owned(),
            pack_length: pack.len(),
            pack_sha256: sha256_hex(&pack),
            entries: provenance,
        },
    )?;
    println!(
        "wrote deterministic {}-byte pack to {}",
        pack.len(),
        pack_path.display()
    );
    Ok(())
}

fn validate_manifest(manifest: &CacheManifest) -> TaskResult {
    if manifest.schema_version != CACHE_SCHEMA_VERSION {
        return Err(format!(
            "cache manifest: unsupported schema version {}",
            manifest.schema_version
        ));
    }
    if manifest.sprites_revision != SPRITES_REVISION {
        return Err(format!(
            "cache manifest: sprite revision must be {SPRITES_REVISION}"
        ));
    }
    if manifest.entries.len() != RECORD_COUNT {
        return Err(format!(
            "cache manifest: expected {RECORD_COUNT} entries, found {}",
            manifest.entries.len()
        ));
    }
    for (index, entry) in manifest.entries.iter().enumerate() {
        let expected = u16::try_from(index + 1).map_err(|_| "entry index overflow")?;
        if entry.id != expected {
            return Err(format!(
                "Pokémon ID {expected}: manifest order: found ID {}",
                entry.id
            ));
        }
    }
    Ok(())
}

fn validate_entry_contract(entry: &SourceEntry, id: u16) -> TaskResult {
    let expected_pokemon_url = format!("https://pokeapi.co/api/v2/pokemon/{id}/");
    let expected_species_url = format!("https://pokeapi.co/api/v2/pokemon-species/{id}/");
    let expected_sprite_url = format!(
        "https://raw.githubusercontent.com/PokeAPI/sprites/{SPRITES_REVISION}/sprites/pokemon/versions/generation-i/yellow/{id}.png"
    );
    let expected_pokemon_path = format!("pokemon/{id:03}.json");
    let expected_species_path = format!("species/{id:03}.json");
    let expected_sprite_path = format!("sprites/{id:03}.png");
    for (actual, expected, label) in [
        (&entry.pokemon.url, &expected_pokemon_url, "Pokémon URL"),
        (&entry.species.url, &expected_species_url, "species URL"),
        (&entry.sprite.url, &expected_sprite_url, "sprite URL"),
        (&entry.pokemon.path, &expected_pokemon_path, "Pokémon path"),
        (&entry.species.path, &expected_species_path, "species path"),
        (&entry.sprite.path, &expected_sprite_path, "sprite path"),
    ] {
        if actual != expected {
            return Err(format!(
                "Pokémon ID {id}: {label}: expected {expected}, found {actual}"
            ));
        }
    }
    Ok(())
}

fn read_and_hash(
    cache_dir: &Path,
    id: u16,
    source: &super::SourceFile,
    rule: &str,
) -> TaskResult<Vec<u8>> {
    let bytes = read(&cache_dir.join(&source.path), id, rule)?;
    let actual = sha256_hex(&bytes);
    if actual != source.sha256 {
        return Err(format!(
            "Pokémon ID {id}: {rule}: SHA-256 mismatch, expected {}, found {actual}",
            source.sha256
        ));
    }
    Ok(bytes)
}

fn build_pack(records: &[ConvertedRecord]) -> TaskResult<Vec<u8>> {
    if records.len() != RECORD_COUNT {
        return Err(format!(
            "pack: expected {RECORD_COUNT} records, found {}",
            records.len()
        ));
    }
    let mut record_bytes = Vec::with_capacity(RECORD_COUNT * RECORD_LENGTH);
    let mut names = Vec::new();
    let mut sprites = Vec::with_capacity(RECORD_COUNT * SPRITE_BYTES);
    for (index, record) in records.iter().enumerate() {
        let expected_id = u16::try_from(index + 1).map_err(|_| "record index overflow")?;
        if record.id != expected_id {
            return Err(format!(
                "Pokémon ID {expected_id}: pack order: found ID {}",
                record.id
            ));
        }
        let name_offset =
            u16::try_from(names.len()).map_err(|_| "pack: names section exceeds u16")?;
        let name_length = u8::try_from(record.name.len())
            .map_err(|_| format!("Pokémon ID {}: name length exceeds u8", record.id))?;
        record_bytes.push(
            u8::try_from(record.id)
                .map_err(|_| format!("Pokémon ID {} cannot fit in the v1 record", record.id))?,
        );
        record_bytes.extend([record.primary_type, record.secondary_type, name_length]);
        record_bytes.extend_from_slice(&name_offset.to_le_bytes());
        names.extend_from_slice(record.name.as_bytes());
        if record.sprite.len() != SPRITE_BYTES {
            return Err(format!(
                "Pokémon ID {}: converted sprite must be {SPRITE_BYTES} bytes",
                record.id
            ));
        }
        sprites.extend_from_slice(&record.sprite);
    }

    let schedule: Vec<u8> = (0..RECORD_COUNT)
        .map(|index| {
            u8::try_from((73 * index) % RECORD_COUNT + 1).map_err(|_| "schedule value exceeds u8")
        })
        .collect::<Result<_, _>>()?;
    let mut payload = record_bytes;
    payload.extend_from_slice(&names);
    payload.extend_from_slice(&schedule);
    payload.extend_from_slice(&sprites);

    let mut pack = Vec::with_capacity(HEADER_LENGTH + payload.len());
    let header_length = u16::try_from(HEADER_LENGTH).map_err(|_| "header length exceeds u16")?;
    let record_count = u16::try_from(RECORD_COUNT).map_err(|_| "record count exceeds u16")?;
    let sprite_width = u8::try_from(SPRITE_WIDTH).map_err(|_| "sprite width exceeds u8")?;
    let sprite_height = u8::try_from(SPRITE_HEIGHT).map_err(|_| "sprite height exceeds u8")?;
    let record_length = u8::try_from(RECORD_LENGTH).map_err(|_| "record length exceeds u8")?;
    pack.extend_from_slice(b"PKVW");
    pack.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    pack.extend_from_slice(&header_length.to_le_bytes());
    pack.extend_from_slice(&CONTENT_REVISION.to_le_bytes());
    pack.extend_from_slice(&SCHEDULE_VERSION.to_le_bytes());
    pack.extend_from_slice(&record_count.to_le_bytes());
    pack.extend_from_slice(&record_count.to_le_bytes());
    pack.extend([sprite_width, sprite_height, record_length, 0]);
    pack.extend_from_slice(
        &u16::try_from(names.len())
            .map_err(|_| "pack: names section exceeds u16")?
            .to_le_bytes(),
    );
    pack.extend_from_slice(
        &u32::try_from(payload.len())
            .map_err(|_| "pack: payload exceeds u32")?
            .to_le_bytes(),
    );
    pack.extend_from_slice(&crc32fast::hash(&payload).to_le_bytes());
    pack.extend_from_slice(&payload);
    if pack.len() > MAX_PACK_BYTES {
        return Err(format!(
            "pack: {} bytes exceeds {MAX_PACK_BYTES}-byte budget",
            pack.len()
        ));
    }
    Ok(pack)
}

fn create_parent(path: &Path) -> TaskResult {
    let parent = path
        .parent()
        .ok_or_else(|| format!("output {} must have a parent directory", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::source::NO_SECONDARY_TYPE;

    #[test]
    fn identical_records_produce_identical_pack_bytes() {
        let records_a = fixture_records();
        let records_b = fixture_records();

        let first = build_pack(&records_a).unwrap();
        let second = build_pack(&records_b).unwrap();

        assert_eq!(first, second);
        assert!(first.len() <= MAX_PACK_BYTES);
        assert_eq!(&first[..4], b"PKVW");
    }

    fn fixture_records() -> Vec<ConvertedRecord> {
        (FIRST_ID..=LAST_ID)
            .map(|id| ConvertedRecord {
                id,
                name: format!("P{id}"),
                primary_type: 0,
                secondary_type: NO_SECONDARY_TYPE,
                sprite: vec![0; SPRITE_BYTES],
            })
            .collect()
    }
}
