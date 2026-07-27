use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    CACHE_SCHEMA_VERSION, CacheManifest, FIRST_ID, LAST_ID, SPRITES_REVISION, SourceEntry,
    SourceFile, TaskResult, sha256_hex, source, write_json,
};

pub(super) fn refresh_cache(destination: &Path) -> TaskResult {
    if destination.exists() {
        return Err(format!(
            "refusing to replace existing cache {}; fetch into a new directory for review",
            destination.display()
        ));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| "cache destination must have a parent directory".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let temporary = temporary_path(parent, destination);
    if temporary.exists() {
        return Err(format!(
            "temporary refresh directory already exists: {}",
            temporary.display()
        ));
    }

    let result = fetch_all(&temporary);
    if let Err(error) = result {
        if temporary.exists() {
            fs::remove_dir_all(&temporary).map_err(|cleanup| {
                format!(
                    "{error}; also failed to remove {}: {cleanup}",
                    temporary.display()
                )
            })?;
        }
        return Err(error);
    }

    fs::rename(&temporary, destination).map_err(|error| {
        format!(
            "cache is complete but could not move {} to {}: {error}",
            temporary.display(),
            destination.display()
        )
    })?;
    println!(
        "cached Pokémon IDs {FIRST_ID}–{LAST_ID} in {}",
        destination.display()
    );
    Ok(())
}

fn fetch_all(temporary: &Path) -> TaskResult {
    for child in ["pokemon", "species", "sprites"] {
        fs::create_dir_all(temporary.join(child)).map_err(|error| {
            format!(
                "failed to create cache directory {}: {error}",
                temporary.join(child).display()
            )
        })?;
    }

    let mut entries = Vec::with_capacity(usize::from(LAST_ID));
    for id in FIRST_ID..=LAST_ID {
        let pokemon_url = format!("https://pokeapi.co/api/v2/pokemon/{id}/");
        let species_url = format!("https://pokeapi.co/api/v2/pokemon-species/{id}/");
        let sprite_url = format!(
            "https://raw.githubusercontent.com/PokeAPI/sprites/{SPRITES_REVISION}/sprites/pokemon/versions/generation-i/yellow/{id}.png"
        );
        let pokemon = fetch(&pokemon_url, id, "Pokémon API response")?;
        let species = fetch(&species_url, id, "species API response")?;
        let sprite = fetch(&sprite_url, id, "Yellow front sprite")?;
        source::validate_source_bytes(id, &pokemon, &species, &sprite)?;

        let pokemon_path = format!("pokemon/{id:03}.json");
        let species_path = format!("species/{id:03}.json");
        let sprite_path = format!("sprites/{id:03}.png");
        write_source(temporary, &pokemon_path, &pokemon)?;
        write_source(temporary, &species_path, &species)?;
        write_source(temporary, &sprite_path, &sprite)?;
        entries.push(SourceEntry {
            id,
            pokemon: source_file(pokemon_url, pokemon_path, &pokemon),
            species: source_file(species_url, species_path, &species),
            sprite: source_file(sprite_url, sprite_path, &sprite),
        });
        println!("cached Pokémon ID {id}");
    }

    let retrieved_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system time is before Unix epoch: {error}"))?
        .as_secs();
    write_json(
        &temporary.join("manifest.json"),
        &CacheManifest {
            schema_version: CACHE_SCHEMA_VERSION,
            sprites_revision: SPRITES_REVISION.to_owned(),
            retrieved_unix_seconds,
            entries,
        },
    )
}

fn fetch(url: &str, id: u16, label: &str) -> TaskResult<Vec<u8>> {
    let output = Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--proto",
            "=https",
            "--tlsv1.2",
            "--user-agent",
            "pokeviewer-content-tool/0.1 (+https://github.com/timbrinded/pokeviewer)",
            url,
        ])
        .output()
        .map_err(|error| format!("Pokémon ID {id}: {label}: failed to start curl: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Pokémon ID {id}: {label}: curl failed with {}: {}",
            output.status,
            stderr.trim()
        ));
    }
    if output.stdout.is_empty() {
        return Err(format!("Pokémon ID {id}: {label}: empty response"));
    }
    Ok(output.stdout)
}

fn write_source(root: &Path, relative: &str, bytes: &[u8]) -> TaskResult {
    let path = root.join(relative);
    fs::write(&path, bytes).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn source_file(url: String, path: String, bytes: &[u8]) -> SourceFile {
    SourceFile {
        url,
        path,
        sha256: sha256_hex(bytes),
    }
}

fn temporary_path(parent: &Path, destination: &Path) -> PathBuf {
    let name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("content-cache");
    parent.join(format!(".{name}.fetch-{}", std::process::id()))
}
