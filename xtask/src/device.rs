//! README device views: exact renderer frames placed on a drawing of the V2 case.
//!
//! Case geometry is in tenths of a millimetre and comes from the Waveshare
//! ESP32-S3-ePaper-1.54 outline drawing: a 39.8 × 53.0 × 16.9 mm case with
//! 4.5 mm corners and a 27.8 mm square screen window 14.3 mm above the bottom
//! edge. The right side carries the microSD slot, the light window, `BOOT`,
//! and `PWR` from top to bottom; USB-C is on the bottom face. Only the screen
//! pixels are exact; the case is an illustration.

use std::{fmt::Write as _, fs, path::PathBuf};

use pokeviewer_core::{
    BatteryState, ContentPack, DISPLAY_HEIGHT, DISPLAY_WIDTH, Framebuffer, Weekday,
    render_setup_screen,
};
use pokeviewer_firmware::{FailureKind, render_failure_screen};

use crate::render::{PACK, render_record_with_battery};

type TaskResult = Result<(), String>;

const DEFAULT_OUTPUT: &str = "target/device-views";

const CASE_WIDTH: u32 = 398;
const CASE_HEIGHT: u32 = 530;
const CASE_DEPTH: u32 = 169;
const CASE_RADIUS: u32 = 45;
const WINDOW_X: u32 = 60;
const WINDOW_Y: u32 = 109;
const WINDOW_SIZE: u32 = 278;
/// One panel pixel is 27.8 mm / 200 = 0.139 mm.
const PIXEL_SCALE: &str = "1.39";

const SD_SLOT_Y: u32 = 88;
const LIGHT_Y: u32 = 144;
const BOOT_Y: u32 = 278;
const PWR_Y: u32 = 379;
const BUTTON_HEIGHT: u32 = 62;
const SIDE_GAP: u32 = 120;
const BOTTOM_GAP: u32 = 90;

/// The goldens' weekday cases, so the strip shows one card per weekday.
const WEEK: [(u8, Weekday); 7] = [
    (1, Weekday::Monday),
    (6, Weekday::Tuesday),
    (142, Weekday::Wednesday),
    (29, Weekday::Thursday),
    (122, Weekday::Friday),
    (25, Weekday::Saturday),
    (151, Weekday::Sunday),
];
const FEATURED: (u8, Weekday) = (25, Weekday::Monday);

#[derive(Clone, Copy)]
enum Light {
    Off,
    Green,
    GreenFlash,
    Orange,
}

pub(crate) fn device_views_command(output_dir: Option<&str>) -> TaskResult {
    let output_dir = PathBuf::from(output_dir.unwrap_or(DEFAULT_OUTPUT));
    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("failed to create {}: {error}", output_dir.display()))?;
    let pack =
        ContentPack::parse(PACK).map_err(|error| format!("invalid content pack: {error:?}"))?;
    let card =
        |battery_state| render_record_with_battery(&pack, FEATURED.0, FEATURED.1, battery_state);
    let failure = |kind| {
        let mut framebuffer = Framebuffer::default();
        render_failure_screen(&mut framebuffer, kind)
            .map_err(|error| format!("failed to render failure screen: {error:?}"))?;
        Ok::<_, String>(framebuffer)
    };
    let mut setup = Framebuffer::default();
    render_setup_screen(&mut setup);

    let mut week = Vec::with_capacity(WEEK.len());
    for (dex_id, weekday) in WEEK {
        week.push(render_record_with_battery(
            &pack,
            dex_id,
            weekday,
            BatteryState::Normal,
        )?);
    }

    let files = [
        ("anatomy.svg", anatomy(&card(BatteryState::Normal)?)),
        ("week.svg", week_strip(&week)),
        ("screen-daily.svg", screen(&card(BatteryState::Normal)?)),
        (
            "screen-recharge.svg",
            screen(&card(BatteryState::Recharge)?),
        ),
        (
            "screen-unavailable.svg",
            screen(&card(BatteryState::Unavailable)?),
        ),
        ("screen-set-time.svg", screen(&setup)),
        (
            "screen-error-reset.svg",
            screen(&failure(FailureKind::UnexpectedWake)?),
        ),
        (
            "screen-error-reflash.svg",
            screen(&failure(FailureKind::Content)?),
        ),
        ("light-off.svg", light(Light::Off)),
        ("light-green.svg", light(Light::Green)),
        ("light-green-flash.svg", light(Light::GreenFlash)),
        ("light-orange.svg", light(Light::Orange)),
    ];
    for (name, svg) in files {
        let path = output_dir.join(name);
        fs::write(&path, svg)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
        println!("{}", path.display());
    }
    Ok(())
}

fn anatomy(framebuffer: &Framebuffer) -> String {
    let side_x = CASE_WIDTH + SIDE_GAP;
    let bottom_y = CASE_HEIGHT + BOTTOM_GAP;
    let label_x = side_x + CASE_DEPTH + 70;
    let mut body = String::new();
    body.push_str(&front(0, 0, framebuffer));
    body.push_str(&side(side_x, 0, Light::Off));
    body.push_str(&bottom(0, bottom_y));
    // (feature x, feature y, label y, text): each leader runs from the feature to its label.
    for (feature_x, feature_y, label_y, text) in [
        (68, SD_SLOT_Y + 20, SD_SLOT_Y + 8, "microSD slot (not used)"),
        (128, LIGHT_Y + 17, LIGHT_Y + 30, "Light: green or orange"),
        (
            118,
            BOOT_Y + BUTTON_HEIGHT / 2,
            BOOT_Y + BUTTON_HEIGHT / 2,
            "BOOT",
        ),
        (
            118,
            PWR_Y + BUTTON_HEIGHT / 2,
            PWR_Y + BUTTON_HEIGHT / 2,
            "PWR",
        ),
    ] {
        let _ = write!(
            body,
            r#"<path class="leader" d="M{} {feature_y}L{} {label_y}"/><text class="label" x="{label_x}" y="{}">{text}</text>"#,
            side_x + feature_x,
            label_x - 16,
            label_y + 9,
        );
    }
    let usb_y = bottom_y + CASE_DEPTH / 2;
    let _ = write!(
        body,
        r#"<path class="leader" d="M{} {usb_y}H{}"/><text class="label" x="{label_x}" y="{}">USB-C</text>"#,
        CASE_WIDTH / 2 + 60,
        label_x - 16,
        usb_y + 9,
    );
    for (x, y, text) in [
        (CASE_WIDTH / 2, CASE_HEIGHT + 50, "Front"),
        (side_x + CASE_DEPTH / 2, CASE_HEIGHT + 50, "Right side"),
        (CASE_WIDTH / 2, bottom_y + CASE_DEPTH + 50, "Bottom"),
    ] {
        let _ = write!(
            body,
            r#"<text class="caption" x="{x}" y="{y}" text-anchor="middle">{text}</text>"#
        );
    }
    let width = label_x + 330;
    let height = bottom_y + CASE_DEPTH + 70;
    document(&body, -30, -30, width + 30, height + 30, 640)
}

fn week_strip(frames: &[Framebuffer]) -> String {
    const GAP: u32 = 50;
    let mut body = String::new();
    let mut x = 0;
    for framebuffer in frames {
        body.push_str(&front(x, 0, framebuffer));
        x += CASE_WIDTH + GAP;
    }
    document(&body, -30, -30, x - GAP + 60, CASE_HEIGHT + 70, 880)
}

/// The screen window and a little of the case, at about one image pixel per panel pixel.
fn screen(framebuffer: &Framebuffer) -> String {
    const MARGIN: u32 = 22;
    let size = WINDOW_SIZE + 2 * MARGIN;
    let origin = |edge: u32| i64::from(edge - MARGIN);
    document(
        &front(0, 0, framebuffer),
        origin(WINDOW_X),
        origin(WINDOW_Y),
        size,
        size,
        232,
    )
}

/// A close-up of the top of the right side, where the light window sits.
fn light(state: Light) -> String {
    document(&side(0, 0, state), -20, 40, CASE_DEPTH + 40, 200, 90)
}

fn front(x: u32, y: u32, framebuffer: &Framebuffer) -> String {
    let mut svg = format!(r#"<g transform="translate({x} {y})">"#);
    let _ = write!(
        svg,
        concat!(
            r#"<rect class="case" width="{w}" height="{h}" rx="{r}" filter="url(#shadow)"/>"#,
            r#"<rect class="lip" x="14" y="14" width="{lw}" height="{lh}" rx="{lr}"/>"#,
            r#"<rect class="bezel" x="{bx}" y="{by}" width="{bs}" height="{bs}" rx="8"/>"#,
            r#"<rect class="paper" x="{wx}" y="{wy}" width="{ws}" height="{ws}"/>"#,
            r#"<path class="ink" transform="translate({wx} {wy}) scale({scale})" d="{ink}"/>"#,
            "</g>"
        ),
        w = CASE_WIDTH,
        h = CASE_HEIGHT,
        r = CASE_RADIUS,
        lw = CASE_WIDTH - 28,
        lh = CASE_HEIGHT - 28,
        lr = CASE_RADIUS - 12,
        bx = WINDOW_X - 8,
        by = WINDOW_Y - 8,
        bs = WINDOW_SIZE + 16,
        wx = WINDOW_X,
        wy = WINDOW_Y,
        ws = WINDOW_SIZE,
        scale = PIXEL_SCALE,
        ink = ink_path(framebuffer),
    );
    svg
}

/// The right side, with the front face on the left edge.
fn side(x: u32, y: u32, light: Light) -> String {
    let mut svg = format!(r#"<g transform="translate({x} {y})">"#);
    let _ = write!(
        svg,
        concat!(
            r#"<rect class="case" width="{d}" height="{h}" rx="20" filter="url(#shadow)"/>"#,
            r#"<path class="seam" d="M118 6V{seam}"/>"#,
            r#"<circle class="hole" cx="63" cy="66" r="6"/>"#,
            r#"<rect class="hole" x="58" y="{sd}" width="10" height="114" rx="5"/>"#,
        ),
        d = CASE_DEPTH,
        h = CASE_HEIGHT,
        seam = CASE_HEIGHT - 6,
        sd = SD_SLOT_Y,
    );
    let window = r#"x="88" width="40" height="34" rx="5""#;
    let _ = write!(
        svg,
        r#"<rect class="light-window" {window} y="{LIGHT_Y}"/>"#
    );
    let lit = match light {
        Light::Off => None,
        Light::Green | Light::GreenFlash => Some("#2fd158"),
        Light::Orange => Some("#ff9419"),
    };
    if let Some(colour) = lit {
        let animation = if matches!(light, Light::GreenFlash) {
            r#"<animate attributeName="opacity" values="0;1;1;0;0" keyTimes="0;0.04;0.2;0.26;1" dur="3s" repeatCount="indefinite"/>"#
        } else {
            ""
        };
        let _ = write!(
            svg,
            concat!(
                r#"<g>{animation}"#,
                r#"<circle cx="108" cy="{cy}" r="32" fill="{c}" opacity="0.3"/>"#,
                r#"<rect {window} y="{y}" fill="{c}"/>"#,
                "</g>"
            ),
            animation = animation,
            cy = LIGHT_Y + 17,
            c = colour,
            window = window,
            y = LIGHT_Y,
        );
    }
    for button_y in [BOOT_Y, PWR_Y] {
        let _ = write!(
            svg,
            concat!(
                r#"<rect class="button" x="30" y="{y}" width="88" height="{h}" rx="14"/>"#,
                r#"<circle class="button-cap" cx="84" cy="{cy}" r="15"/>"#
            ),
            y = button_y,
            h = BUTTON_HEIGHT,
            cy = button_y + BUTTON_HEIGHT / 2,
        );
    }
    svg.push_str("</g>");
    svg
}

/// The bottom face, with the front face on the top edge.
fn bottom(x: u32, y: u32) -> String {
    let usb_x = CASE_WIDTH / 2 - 45;
    format!(
        concat!(
            r#"<g transform="translate({x} {y})">"#,
            r#"<rect class="case" width="{w}" height="{d}" rx="20" filter="url(#shadow)"/>"#,
            r#"<path class="seam" d="M6 118H{seam}"/>"#,
            r#"<rect class="hole" x="{ux}" y="52" width="90" height="33" rx="16"/>"#,
            r#"<rect class="usb-tongue" x="{tx}" y="64" width="58" height="9" rx="3"/>"#,
            "</g>"
        ),
        x = x,
        y = y,
        w = CASE_WIDTH,
        d = CASE_DEPTH,
        seam = CASE_WIDTH - 6,
        ux = usb_x,
        tx = usb_x + 16,
    )
}

/// One path in panel-pixel units covering every black pixel, one row run at a time.
fn ink_path(framebuffer: &Framebuffer) -> String {
    let mut path = String::new();
    for y in 0..DISPLAY_HEIGHT {
        let mut x = 0;
        while x < DISPLAY_WIDTH {
            if framebuffer.is_black(x, y) != Some(true) {
                x += 1;
                continue;
            }
            let start = x;
            while framebuffer.is_black(x, y) == Some(true) {
                x += 1;
            }
            let run = x - start;
            let _ = write!(path, "M{start} {y}h{run}v1h-{run}z");
        }
    }
    path
}

fn document(body: &str, x: i64, y: i64, width: u32, height: u32, pixel_width: u32) -> String {
    let pixel_height = u64::from(pixel_width) * u64::from(height) / u64::from(width);
    format!(
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{x} {y} {w} {h}" width="{pw}" height="{ph}">"#,
            "\n<style>",
            ".case{{fill:#fafaf8;stroke:#c9c9c3;stroke-width:2}}",
            ".lip{{fill:none;stroke:#e6e6e1;stroke-width:2}}",
            ".bezel{{fill:#cfd0cb}}",
            ".paper{{fill:#e9e8e1}}",
            ".ink{{fill:#232326;shape-rendering:crispEdges}}",
            ".seam{{fill:none;stroke:#deded8;stroke-width:2}}",
            ".hole{{fill:#3b3b3d}}",
            ".usb-tongue{{fill:#9a9a9c}}",
            ".light-window{{fill:#e4e4de;stroke:#b5b5ae;stroke-width:2}}",
            ".button{{fill:#f1f1ee;stroke:#b5b5ae;stroke-width:2}}",
            ".button-cap{{fill:#ffffff;stroke:#b5b5ae;stroke-width:2}}",
            ".label,.caption{{font-family:system-ui,-apple-system,'Segoe UI',Helvetica,Arial,sans-serif;fill:#1f2328}}",
            ".label{{font-size:28px;font-weight:600}}",
            ".caption{{font-size:24px;fill:#656d76}}",
            ".leader{{fill:none;stroke:#8c959f;stroke-width:2}}",
            "@media (prefers-color-scheme:dark){{.label{{fill:#e6edf3}}.caption{{fill:#9198a1}}.leader{{stroke:#6e7681}}}}",
            "</style>\n",
            r##"<defs><filter id="shadow" x="-20%" y="-20%" width="140%" height="140%"><feDropShadow dx="0" dy="6" stdDeviation="8" flood-color="#000" flood-opacity="0.16"/></filter></defs>"##,
            "\n{body}\n</svg>\n"
        ),
        x = x,
        y = y,
        w = width,
        h = height,
        pw = pixel_width,
        ph = pixel_height,
        body = body,
    )
}

#[cfg(test)]
mod tests {
    use pokeviewer_core::{DISPLAY_HEIGHT, DISPLAY_WIDTH, Framebuffer, render_setup_screen};

    use super::ink_path;

    #[test]
    fn ink_path_covers_exactly_the_black_pixels() {
        let mut framebuffer = Framebuffer::default();
        render_setup_screen(&mut framebuffer);
        let black = (0..DISPLAY_HEIGHT)
            .flat_map(|y| (0..DISPLAY_WIDTH).map(move |x| (x, y)))
            .filter(|&(x, y)| framebuffer.is_black(x, y) == Some(true))
            .count();
        let drawn: usize = ink_path(&framebuffer)
            .split('h')
            .skip(1)
            .step_by(2)
            .map(|run| run.split('v').next().unwrap().parse::<usize>().unwrap())
            .sum();
        assert!(black > 0);
        assert_eq!(drawn, black);
    }
}
