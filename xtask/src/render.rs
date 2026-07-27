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
    let file = fs::File::create(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    let mut encoder = png::Encoder::new(
        file,
        u32::try_from(DISPLAY_WIDTH).map_err(|_| "display width exceeds u32")?,
        u32::try_from(DISPLAY_HEIGHT).map_err(|_| "display height exceeds u32")?,
    );
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::One);
    let mut writer = encoder
        .write_header()
        .map_err(|error| format!("failed to write {} header: {error}", path.display()))?;
    writer
        .write_image_data(framebuffer.as_bytes())
        .map_err(|error| format!("failed to write {} pixels: {error}", path.display()))
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
