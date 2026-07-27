//! Explicit cache refresh and deterministic offline content-pack tooling.

mod fetch;
mod serializer;
mod source;

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(crate) const FIRST_ID: u16 = 1;
pub(crate) const LAST_ID: u16 = 151;
pub(crate) const CACHE_SCHEMA_VERSION: u16 = 1;
pub(crate) const FORMAT_VERSION: u16 = 1;
pub(crate) const CONTENT_REVISION: u32 = 1;
pub(crate) const SCHEDULE_VERSION: u16 = 1;
pub(crate) const SPRITES_REVISION: &str = "8dfa3d97e953caaafaafd4963eff7621811af08e";
pub(crate) const DEFAULT_CACHE: &str = "content/cache-v1";
pub(crate) const DEFAULT_PACK: &str = "content/generated/pokeviewer-v1.pack";
pub(crate) const DEFAULT_PACK_MANIFEST: &str = "content/generated/pokeviewer-v1.json";

pub(crate) type TaskResult<T = ()> = Result<T, String>;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct CacheManifest {
    pub(crate) schema_version: u16,
    pub(crate) sprites_revision: String,
    pub(crate) retrieved_unix_seconds: u64,
    pub(crate) entries: Vec<SourceEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct SourceEntry {
    pub(crate) id: u16,
    pub(crate) pokemon: SourceFile,
    pub(crate) species: SourceFile,
    pub(crate) sprite: SourceFile,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct SourceFile {
    pub(crate) url: String,
    pub(crate) path: String,
    pub(crate) sha256: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct PackManifest {
    pub(crate) format_version: u16,
    pub(crate) content_revision: u32,
    pub(crate) schedule_version: u16,
    pub(crate) source_manifest_sha256: String,
    pub(crate) sprites_revision: String,
    pub(crate) pack_path: String,
    pub(crate) pack_length: usize,
    pub(crate) pack_sha256: String,
    pub(crate) entries: Vec<PackManifestEntry>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PackManifestEntry {
    pub(crate) id: u16,
    pub(crate) name: String,
    pub(crate) pokemon_sha256: String,
    pub(crate) species_sha256: String,
    pub(crate) sprite_sha256: String,
}

pub(crate) fn fetch_command(cache_dir: Option<&str>) -> TaskResult {
    fetch::refresh_cache(Path::new(cache_dir.unwrap_or(DEFAULT_CACHE)))
}

pub(crate) fn build_command(arguments: &[String]) -> TaskResult {
    if arguments.len() > 3 {
        return Err("content-build accepts at most CACHE_DIR, PACK_FILE, and MANIFEST_FILE".into());
    }
    let cache = PathBuf::from(arguments.first().map_or(DEFAULT_CACHE, String::as_str));
    let pack = PathBuf::from(arguments.get(1).map_or(DEFAULT_PACK, String::as_str));
    let manifest = PathBuf::from(
        arguments
            .get(2)
            .map_or(DEFAULT_PACK_MANIFEST, String::as_str),
    );
    serializer::convert_cache(&cache, &pack, &manifest)
}

pub(crate) fn read(path: &Path, id: u16, rule: &str) -> TaskResult<Vec<u8>> {
    fs::read(path).map_err(|error| {
        format!(
            "Pokémon ID {id}: {rule}: failed to read {}: {error}",
            path.display()
        )
    })
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> TaskResult {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("failed to serialize {}: {error}", path.display()))?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|error| format!("failed to write {}: {error}", path.display()))
}
