//! Host evidence generation from the shared `no_std` renderer.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use pokeviewer_core::{
    BatteryStatus, ContentPack, DISPLAY_HEIGHT, DISPLAY_WIDTH, DailyCard, Framebuffer, Weekday,
    render_daily_card, render_setup_screen,
};
use pokeviewer_firmware::{FailureKind, render_failure_screen};

type TaskResult = Result<(), String>;

const PACK: &[u8] = include_bytes!("../../content/generated/pokeviewer-v1.pack");
const DEFAULT_OUTPUT: &str = "target/render-samples";
const DEFAULT_CONTACT_SHEET: &str = "target/all-cards-contact-sheet.png";
const DEFAULT_SETUP_SCREEN: &str = "target/setup-screen.png";
const DEFAULT_RECOVERY_SCREENS: &str = "target/recovery-screens";
const SAMPLES: [(u8, Weekday); 4] = [
    (25, Weekday::Monday),
    (6, Weekday::Tuesday),
    (83, Weekday::Wednesday),
    (29, Weekday::Thursday),
];
pub(crate) const REVIEW_BATTERY_STATUS: BatteryStatus = BatteryStatus::Estimated {
    percent: 50,
    recharge: false,
};

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
        let framebuffer = render_record(&pack, dex_id, weekday)?;

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
        let framebuffer = render_record(&pack, dex_id, weekday_for_id(dex_id))?;

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

pub(crate) fn setup_screen_command(output_file: Option<&str>) -> TaskResult {
    let output_file = PathBuf::from(output_file.unwrap_or(DEFAULT_SETUP_SCREEN));
    let mut framebuffer = Framebuffer::default();
    render_setup_screen(&mut framebuffer);
    write_png(&output_file, &framebuffer)?;
    println!(
        "{}: setup screen (CRC-32 {:08x})",
        output_file.display(),
        crc32fast::hash(framebuffer.as_bytes())
    );
    Ok(())
}

pub(crate) fn recovery_screens_command(output_dir: Option<&str>) -> TaskResult {
    let output_dir = PathBuf::from(output_dir.unwrap_or(DEFAULT_RECOVERY_SCREENS));
    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("failed to create {}: {error}", output_dir.display()))?;
    for (slug, failure) in [
        ("rtc", FailureKind::InvalidRtc),
        ("pack", FailureKind::Content),
        ("panel", FailureKind::Panel),
        ("alarm", FailureKind::Alarm),
        ("wake", FailureKind::UnexpectedWake),
    ] {
        let mut framebuffer = Framebuffer::default();
        render_failure_screen(&mut framebuffer, failure)
            .map_err(|error| format!("failed to render {slug}: {error:?}"))?;
        let output_file = output_dir.join(format!("{slug}.png"));
        write_png(&output_file, &framebuffer)?;
        println!(
            "{}: {} (CRC-32 {:08x})",
            output_file.display(),
            failure.policy().code,
            framebuffer.crc32()
        );
    }
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

pub(crate) fn render_record(
    pack: &ContentPack<'_>,
    dex_id: u8,
    weekday: Weekday,
) -> Result<Framebuffer, String> {
    render_record_with_battery(pack, dex_id, weekday, REVIEW_BATTERY_STATUS)
}

pub(crate) fn render_record_with_battery(
    pack: &ContentPack<'_>,
    dex_id: u8,
    weekday: Weekday,
    battery_status: BatteryStatus,
) -> Result<Framebuffer, String> {
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
            battery_status,
        },
    )
    .map_err(|error| format!("record {dex_id} did not render: {error:?}"))?;
    Ok(framebuffer)
}

pub(crate) fn write_one_bit_png(
    path: &Path,
    width: usize,
    height: usize,
    pixels: &[u8],
) -> TaskResult {
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
