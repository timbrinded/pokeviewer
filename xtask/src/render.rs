//! Host evidence generation from the shared `no_std` renderer.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use pokeviewer_core::{
    ContentPack, DISPLAY_HEIGHT, DISPLAY_WIDTH, DailyCard, Framebuffer, Weekday, render_daily_card,
};

type TaskResult = Result<(), String>;

const PACK: &[u8] = include_bytes!("../../content/generated/pokeviewer-v1.pack");
const DEFAULT_OUTPUT: &str = "target/render-samples";
const DEFAULT_CONTACT_SHEET: &str = "target/all-cards-contact-sheet.png";
const SAMPLES: [(u8, Weekday); 4] = [
    (25, Weekday::Monday),
    (6, Weekday::Tuesday),
    (83, Weekday::Wednesday),
    (29, Weekday::Thursday),
];

pub(crate) fn samples_command(output_dir: Option<&str>) -> TaskResult {
    let output_dir = PathBuf::from(output_dir.unwrap_or(DEFAULT_OUTPUT));
    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("failed to create {}: {error}", output_dir.display()))?;
    let pack =
        ContentPack::parse(PACK).map_err(|error| format!("invalid content pack: {error:?}"))?;

    for (dex_id, weekday) in SAMPLES {
        let record = pack
            .record(dex_id)
            .map_err(|error| format!("invalid record {dex_id}: {error:?}"))?;
        let mut framebuffer = Framebuffer::default();
        render_daily_card(
            &mut framebuffer,
            DailyCard {
                weekday,
                name: record.name,
                primary_type: record.primary_type,
                secondary_type: record.secondary_type,
                sprite: record.sprite,
            },
        )
        .map_err(|error| format!("record {dex_id} did not render: {error:?}"))?;

        let stem = format!("{dex_id:03}");
        write_pbm(&output_dir.join(format!("{stem}.pbm")), &framebuffer)?;
        write_png(&output_dir.join(format!("{stem}.png")), &framebuffer)?;
        println!(
            "{stem}: {} (CRC-32 {:08x})",
            record.name,
            crc32fast::hash(framebuffer.as_bytes())
        );
    }
    Ok(())
}

pub(crate) fn contact_sheet_command(output_file: Option<&str>) -> TaskResult {
    const COLUMNS: usize = 10;
    const ROWS: usize = 16;
    const GUTTER: usize = 8;
    const SHEET_WIDTH: usize = COLUMNS * DISPLAY_WIDTH + (COLUMNS - 1) * GUTTER;
    const SHEET_HEIGHT: usize = ROWS * DISPLAY_HEIGHT + (ROWS - 1) * GUTTER;
    const SHEET_ROW_BYTES: usize = SHEET_WIDTH / 8;

    let output_file = PathBuf::from(output_file.unwrap_or(DEFAULT_CONTACT_SHEET));
    create_parent(&output_file)?;
    let pack =
        ContentPack::parse(PACK).map_err(|error| format!("invalid content pack: {error:?}"))?;
    let mut sheet = vec![u8::MAX; SHEET_ROW_BYTES * SHEET_HEIGHT];
    for dex_id in 1..=151 {
        let record = pack
            .record(dex_id)
            .map_err(|error| format!("invalid record {dex_id}: {error:?}"))?;
        let mut framebuffer = Framebuffer::default();
        render_daily_card(
            &mut framebuffer,
            DailyCard {
                weekday: weekday_for_id(dex_id),
                name: record.name,
                primary_type: record.primary_type,
                secondary_type: record.secondary_type,
                sprite: record.sprite,
            },
        )
        .map_err(|error| format!("record {dex_id} did not render: {error:?}"))?;

        let card_index = usize::from(dex_id - 1);
        let cell_x_bytes = card_index % COLUMNS * ((DISPLAY_WIDTH + GUTTER) / 8);
        let cell_y = card_index / COLUMNS * (DISPLAY_HEIGHT + GUTTER);
        for row in 0..DISPLAY_HEIGHT {
            let source_start = row * (DISPLAY_WIDTH / 8);
            let target_start = (cell_y + row) * SHEET_ROW_BYTES + cell_x_bytes;
            sheet[target_start..target_start + DISPLAY_WIDTH / 8].copy_from_slice(
                &framebuffer.as_bytes()[source_start..source_start + DISPLAY_WIDTH / 8],
            );
        }
    }

    write_one_bit_png(&output_file, SHEET_WIDTH, SHEET_HEIGHT, &sheet)?;
    println!(
        "{}: 151 cards, {}x{} actual pixels",
        output_file.display(),
        SHEET_WIDTH,
        SHEET_HEIGHT
    );
    Ok(())
}

fn write_pbm(path: &Path, framebuffer: &Framebuffer) -> TaskResult {
    let mut file = fs::File::create(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    write!(file, "P4\n{DISPLAY_WIDTH} {DISPLAY_HEIGHT}\n")
        .map_err(|error| format!("failed to write {} header: {error}", path.display()))?;
    for byte in framebuffer.as_bytes() {
        file.write_all(&[!byte])
            .map_err(|error| format!("failed to write {} pixels: {error}", path.display()))?;
    }
    Ok(())
}

fn write_png(path: &Path, framebuffer: &Framebuffer) -> TaskResult {
    write_one_bit_png(path, DISPLAY_WIDTH, DISPLAY_HEIGHT, framebuffer.as_bytes())
}

fn write_one_bit_png(path: &Path, width: usize, height: usize, pixels: &[u8]) -> TaskResult {
    create_parent(path)?;
    let file = fs::File::create(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    let mut encoder = png::Encoder::new(
        file,
        u32::try_from(width).map_err(|_| "image width exceeds u32")?,
        u32::try_from(height).map_err(|_| "image height exceeds u32")?,
    );
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::One);
    let mut writer = encoder
        .write_header()
        .map_err(|error| format!("failed to write {} header: {error}", path.display()))?;
    writer
        .write_image_data(pixels)
        .map_err(|error| format!("failed to write {} pixels: {error}", path.display()))
}

fn create_parent(path: &Path) -> TaskResult {
    let parent = path
        .parent()
        .ok_or_else(|| format!("output {} must have a parent directory", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))
}

const fn weekday_for_id(dex_id: u8) -> Weekday {
    match (dex_id - 1) % 7 {
        0 => Weekday::Monday,
        1 => Weekday::Tuesday,
        2 => Weekday::Wednesday,
        3 => Weekday::Thursday,
        4 => Weekday::Friday,
        5 => Weekday::Saturday,
        _ => Weekday::Sunday,
    }
}

#[cfg(test)]
mod tests {
    use super::{DISPLAY_HEIGHT, DISPLAY_WIDTH, Framebuffer, write_pbm};

    #[test]
    fn pbm_conversion_inverts_panel_native_polarity() {
        let output_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/render-tests");
        std::fs::create_dir_all(&output_dir).unwrap();
        let output = output_dir.join("white-frame.pbm");
        write_pbm(&output, &Framebuffer::default()).unwrap();
        let bytes = std::fs::read(&output).unwrap();
        let header = format!("P4\n{DISPLAY_WIDTH} {DISPLAY_HEIGHT}\n");
        assert_eq!(&bytes[..header.len()], header.as_bytes());
        assert!(bytes[header.len()..].iter().all(|byte| *byte == 0));
        std::fs::remove_file(output).unwrap();
    }
}
