//! Deterministic golden-frame update, comparison, and failure evidence.

use std::{
    fmt::Write as _,
    fs,
    path::{Component, Path, PathBuf},
};

use pokeviewer_core::{ContentPack, DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAMEBUFFER_BYTES};

use crate::{
    content::{sha256_hex, write_json},
    render::{render_record, write_one_bit_png},
};

#[path = "golden_manifest.rs"]
mod manifest;

use manifest::{CASES, GoldenCase, GoldenManifest, GoldenSpec, weekday_label};

type TaskResult = Result<(), String>;

const PACK: &[u8] = include_bytes!("../../content/generated/pokeviewer-v1.pack");
const GOLDEN_ROOT: &str = "tests/goldens";
const DEFAULT_DIFF_DIR: &str = "target/visual-diff";
const DEFAULT_DEMO_DIR: &str = "target/golden-failure-demo";

pub(crate) fn update_command() -> TaskResult {
    let root = Path::new(GOLDEN_ROOT);
    let cards = root.join("cards");
    if cards.exists() {
        fs::remove_dir_all(&cards)
            .map_err(|error| format!("failed to replace {}: {error}", cards.display()))?;
    }
    fs::create_dir_all(&cards)
        .map_err(|error| format!("failed to create {}: {error}", cards.display()))?;

    let pack = parse_pack()?;
    let mut cases = Vec::with_capacity(CASES.len());
    for spec in CASES {
        let framebuffer = render_record(&pack, spec.dex_id, spec.weekday)?;
        let raw_relative = format!("cards/{}.bin", spec.slug);
        let png_relative = format!("cards/{}.png", spec.slug);
        let raw_path = root.join(&raw_relative);
        let png_path = root.join(&png_relative);
        fs::write(&raw_path, framebuffer.as_bytes())
            .map_err(|error| format!("failed to write {}: {error}", raw_path.display()))?;
        write_one_bit_png(
            &png_path,
            DISPLAY_WIDTH,
            DISPLAY_HEIGHT,
            framebuffer.as_bytes(),
        )?;
        let png = fs::read(&png_path)
            .map_err(|error| format!("failed to read {}: {error}", png_path.display()))?;
        let record = pack
            .record(spec.dex_id)
            .map_err(|error| format!("invalid record {}: {error:?}", spec.dex_id))?;
        cases.push(GoldenCase {
            case: spec.slug.to_owned(),
            dex_id: spec.dex_id,
            name: record.name.to_owned(),
            weekday: weekday_label(spec.weekday).to_owned(),
            framebuffer_file: raw_relative,
            png_file: png_relative,
            framebuffer_crc32: format!("{:08x}", crc32fast::hash(framebuffer.as_bytes())),
            framebuffer_sha256: sha256_hex(framebuffer.as_bytes()),
            png_sha256: sha256_hex(&png),
        });
    }
    write_json(
        &root.join("manifest.json"),
        &GoldenManifest {
            schema_version: 1,
            renderer_version: 1,
            cases,
        },
    )?;
    println!("updated {} reviewed golden cases", CASES.len());
    Ok(())
}

pub(crate) fn check_command(diff_dir: Option<&str>) -> TaskResult {
    let root = Path::new(GOLDEN_ROOT);
    let diff_dir = safe_relative_output(diff_dir.unwrap_or(DEFAULT_DIFF_DIR))?;
    clear_directory(&diff_dir)?;
    let manifest = read_manifest(&root.join("manifest.json"))?;
    if manifest.schema_version != 1
        || manifest.renderer_version != 1
        || manifest.cases.len() != CASES.len()
    {
        return Err("golden manifest version or case count is unsupported".to_owned());
    }

    let pack = parse_pack()?;
    let mut failures = Vec::new();
    for (spec, committed) in CASES.into_iter().zip(&manifest.cases) {
        let record = pack
            .record(spec.dex_id)
            .map_err(|error| format!("invalid record {}: {error:?}", spec.dex_id))?;
        validate_case_metadata(root, spec, record.name, committed)?;
        let expected = fs::read(root.join(&committed.framebuffer_file))
            .map_err(|error| format!("failed to read {}: {error}", committed.framebuffer_file))?;
        if expected.len() != FRAMEBUFFER_BYTES {
            return Err(format!(
                "{} has {} bytes; expected {FRAMEBUFFER_BYTES}",
                committed.framebuffer_file,
                expected.len()
            ));
        }
        validate_committed_hashes(root, committed, &expected)?;

        let actual = render_record(&pack, spec.dex_id, spec.weekday)?;
        if expected != actual.as_bytes() {
            let changed_pixels = changed_pixel_count(&expected, actual.as_bytes());
            write_failure_artifacts(&diff_dir, spec.slug, &expected, actual.as_bytes())?;
            failures.push(format!("{}: {changed_pixels} changed pixels", spec.slug));
        }
    }

    if failures.is_empty() {
        println!(
            "all {} visual goldens match exact framebuffer bytes",
            CASES.len()
        );
        Ok(())
    } else {
        Err(format!(
            "visual golden mismatch; artifacts: {}\n{}",
            diff_dir.display(),
            failures.join("\n")
        ))
    }
}

pub(crate) fn demo_failure_command(output_dir: Option<&str>) -> TaskResult {
    let output_dir = safe_relative_output(output_dir.unwrap_or(DEFAULT_DEMO_DIR))?;
    clear_demo_artifacts(&output_dir)?;
    let pack = parse_pack()?;
    let expected = render_record(&pack, CASES[0].dex_id, CASES[0].weekday)?;
    let mut actual = expected.as_bytes().to_vec();
    actual[100 * (DISPLAY_WIDTH / 8) + 100 / 8] ^= 0x80 >> (100 % 8);
    let changed_pixels = changed_pixel_count(expected.as_bytes(), &actual);
    if changed_pixels != 1 {
        return Err(format!(
            "failure demonstration changed {changed_pixels} pixels instead of one"
        ));
    }
    write_failure_artifacts(&output_dir, CASES[0].slug, expected.as_bytes(), &actual)?;
    fs::write(
        output_dir.join("README.txt"),
        concat!(
            "Deliberate visual-golden failure demonstration.\n",
            "The actual frame differs from the expected frame by one pixel at (100, 100).\n",
            "golden-check uses the same comparison and artifact writer, then exits non-zero.\n",
        ),
    )
    .map_err(|error| format!("failed to write demo README: {error}"))?;
    println!("captured one-pixel failure in {}", output_dir.display());
    Ok(())
}

fn validate_case_metadata(
    root: &Path,
    spec: GoldenSpec,
    expected_name: &str,
    committed: &GoldenCase,
) -> TaskResult {
    let expected_raw = format!("cards/{}.bin", spec.slug);
    let expected_png = format!("cards/{}.png", spec.slug);
    if committed.case != spec.slug
        || committed.dex_id != spec.dex_id
        || committed.name != expected_name
        || committed.weekday != weekday_label(spec.weekday)
        || committed.framebuffer_file != expected_raw
        || committed.png_file != expected_png
    {
        return Err(format!(
            "golden manifest metadata changed for {}",
            spec.slug
        ));
    }
    if !root.join(&expected_raw).is_file() || !root.join(&expected_png).is_file() {
        return Err(format!("golden files are missing for {}", spec.slug));
    }
    Ok(())
}

fn validate_committed_hashes(
    root: &Path,
    committed: &GoldenCase,
    framebuffer: &[u8],
) -> TaskResult {
    let crc = format!("{:08x}", crc32fast::hash(framebuffer));
    let raw_sha = sha256_hex(framebuffer);
    let png = fs::read(root.join(&committed.png_file))
        .map_err(|error| format!("failed to read {}: {error}", committed.png_file))?;
    let png_sha = sha256_hex(&png);
    if crc != committed.framebuffer_crc32
        || raw_sha != committed.framebuffer_sha256
        || png_sha != committed.png_sha256
    {
        return Err(format!("committed hash mismatch for {}", committed.case));
    }
    Ok(())
}

fn write_failure_artifacts(
    output_dir: &Path,
    slug: &str,
    expected: &[u8],
    actual: &[u8],
) -> TaskResult {
    fs::create_dir_all(output_dir)
        .map_err(|error| format!("failed to create {}: {error}", output_dir.display()))?;
    let diff: Vec<_> = expected
        .iter()
        .zip(actual)
        .map(|(before, after)| !(before ^ after))
        .collect();
    write_one_bit_png(
        &output_dir.join(format!("{slug}-expected.png")),
        DISPLAY_WIDTH,
        DISPLAY_HEIGHT,
        expected,
    )?;
    write_one_bit_png(
        &output_dir.join(format!("{slug}-actual.png")),
        DISPLAY_WIDTH,
        DISPLAY_HEIGHT,
        actual,
    )?;
    write_one_bit_png(
        &output_dir.join(format!("{slug}-diff.png")),
        DISPLAY_WIDTH,
        DISPLAY_HEIGHT,
        &diff,
    )?;
    fs::write(
        output_dir.join(format!("{slug}-report.txt")),
        difference_report(expected, actual),
    )
    .map_err(|error| format!("failed to write {slug} difference report: {error}"))
}

fn changed_pixel_count(expected: &[u8], actual: &[u8]) -> u32 {
    expected
        .iter()
        .zip(actual)
        .map(|(before, after)| (before ^ after).count_ones())
        .sum()
}

fn difference_report(expected: &[u8], actual: &[u8]) -> String {
    let mut coordinates = String::new();
    for y in 0..DISPLAY_HEIGHT {
        for x in 0..DISPLAY_WIDTH {
            let index = y * (DISPLAY_WIDTH / 8) + x / 8;
            let mask = 0x80 >> (x % 8);
            if (expected[index] ^ actual[index]) & mask != 0 {
                if !coordinates.is_empty() {
                    coordinates.push_str(", ");
                }
                let _ = write!(coordinates, "({x},{y})");
            }
        }
    }
    format!(
        "changed_pixels={}\nchanged_coordinates={coordinates}\nexpected_sha256={}\nactual_sha256={}\n",
        changed_pixel_count(expected, actual),
        sha256_hex(expected),
        sha256_hex(actual)
    )
}

fn parse_pack() -> Result<ContentPack<'static>, String> {
    ContentPack::parse(PACK).map_err(|error| format!("invalid content pack: {error:?}"))
}

fn read_manifest(path: &Path) -> Result<GoldenManifest, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn clear_directory(path: &Path) -> TaskResult {
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("failed to clear {}: {error}", path.display()))?;
    }
    Ok(())
}

fn clear_demo_artifacts(path: &Path) -> TaskResult {
    fs::create_dir_all(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    for filename in [
        "monday-001-expected.png",
        "monday-001-actual.png",
        "monday-001-diff.png",
        "monday-001-report.txt",
        "README.txt",
    ] {
        let artifact = path.join(filename);
        if artifact.exists() {
            fs::remove_file(&artifact)
                .map_err(|error| format!("failed to replace {}: {error}", artifact.display()))?;
        }
    }
    Ok(())
}

fn safe_relative_output(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("golden output must be a repository-relative child path".to_owned());
    }
    Ok(path)
}

#[cfg(test)]
#[path = "golden_tests.rs"]
mod tests;
