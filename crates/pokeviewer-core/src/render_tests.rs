use super::{
    DailyCard, Framebuffer, NAME_SCALE, NAME_Y, PRIMARY_TYPE_Y, RECHARGE_Y, RenderError,
    SECONDARY_TYPE_Y, SINGLE_TYPE_Y, SPRITE_SCALE, SPRITE_Y, TYPE_SCALE, WEEKDAY_SCALE, WEEKDAY_Y,
    draw_battery_state, draw_recharge_status, render_daily_card, render_recovery_screen,
    render_setup_screen, text_width, type_label,
};
use crate::{
    BatteryState, CONTENT_SPRITE_BYTES, CONTENT_SPRITE_SIZE, ContentPack, DISPLAY_WIDTH,
    FRAMEBUFFER_BYTES, PokemonType, Weekday, set_sprite_shade,
};

const PACK: &[u8] = include_bytes!("../../../content/generated/pokeviewer-v2.pack");
const WHITE_SPRITE: [u8; CONTENT_SPRITE_BYTES] = [0; CONTENT_SPRITE_BYTES];
const BLACK_SPRITE: [u8; CONTENT_SPRITE_BYTES] = [u8::MAX; CONTENT_SPRITE_BYTES];

fn card<'a>(name: &'a str, sprite: &'a [u8; CONTENT_SPRITE_BYTES]) -> DailyCard<'a> {
    DailyCard {
        weekday: Weekday::Wednesday,
        name,
        primary_type: PokemonType::Electric,
        secondary_type: Some(PokemonType::Flying),
        sprite,
        battery_state: BatteryState::Normal,
    }
}

#[test]
fn framebuffer_matches_panel_memory_and_polarity() {
    let mut framebuffer = Framebuffer::default();

    assert_eq!(core::mem::size_of::<Framebuffer>(), FRAMEBUFFER_BYTES);
    assert!(framebuffer.as_bytes().iter().all(|byte| *byte == u8::MAX));
    framebuffer.set_black(0, 0);
    framebuffer.set_black(199, 199);
    assert_eq!(framebuffer.as_bytes()[0], 0x7f);
    assert_eq!(framebuffer.as_bytes()[FRAMEBUFFER_BYTES - 1], 0xfe);
    assert_eq!(framebuffer.is_black(0, 0), Some(true));
    assert_eq!(framebuffer.is_black(200, 0), None);
}

#[test]
fn long_name_dual_types_and_sprite_extremes_are_deterministic() {
    let mut black_first = Framebuffer::default();
    let mut black_second = Framebuffer::default();
    render_daily_card(&mut black_first, card("Farfetch’d", &BLACK_SPRITE)).unwrap();
    render_daily_card(&mut black_second, card("Farfetch’d", &BLACK_SPRITE)).unwrap();
    assert_eq!(black_first, black_second);

    let mut white = Framebuffer::default();
    render_daily_card(&mut white, card("Nidoran♀", &WHITE_SPRITE)).unwrap();
    assert_eq!(crc32fast::hash(white.as_bytes()), 0xa077_1a7a);
    assert_ne!(black_first, white);
}

fn sprite_ink(framebuffer: &Framebuffer) -> usize {
    let size = CONTENT_SPRITE_SIZE * SPRITE_SCALE;
    let x = (DISPLAY_WIDTH - size) / 2;
    (SPRITE_Y..SPRITE_Y + size)
        .flat_map(|y| (x..x + size).map(move |x| (x, y)))
        .filter(|&(x, y)| framebuffer.is_black(x, y) == Some(true))
        .count()
}

#[test]
fn sprite_shades_render_as_dithered_cells() {
    let cells = CONTENT_SPRITE_SIZE * CONTENT_SPRITE_SIZE;
    for (shade_byte, ink_per_cell) in [(0x00, 0), (0x55, 1), (0xaa, 2)] {
        let sprite = [shade_byte; CONTENT_SPRITE_BYTES];
        let mut framebuffer = Framebuffer::default();
        render_daily_card(&mut framebuffer, card("Mew", &sprite)).unwrap();
        assert_eq!(sprite_ink(&framebuffer), cells * ink_per_cell);
    }

    let mut black = Framebuffer::default();
    render_daily_card(&mut black, card("Mew", &BLACK_SPRITE)).unwrap();
    let interior_cells = (CONTENT_SPRITE_SIZE - 2) * (CONTENT_SPRITE_SIZE - 2);
    let outline_cells = cells - interior_cells;
    assert_eq!(sprite_ink(&black), outline_cells * 4 + interior_cells * 3);
    let left = (DISPLAY_WIDTH - CONTENT_SPRITE_SIZE * SPRITE_SCALE) / 2;
    assert_eq!(black.is_black(left, SPRITE_Y), Some(true));
    assert_eq!(black.is_black(left + 2, SPRITE_Y + 3), Some(false));
}

#[test]
fn black_features_narrower_than_four_pixels_stay_solid() {
    let mut sprite = WHITE_SPRITE;
    // A 3 × 3 eye and a 4 × 4 patch, well apart.
    for (left, size) in [(4, 3), (20, 4)] {
        for y in 10..10 + size {
            for x in left..left + size {
                set_sprite_shade(&mut sprite, x, y, 3);
            }
        }
    }
    let mut framebuffer = Framebuffer::default();
    render_daily_card(&mut framebuffer, card("Mew", &sprite)).unwrap();

    let eye_cells = 3 * 3;
    let patch_centre_cells = 2 * 2;
    let patch_edge_cells = 4 * 4 - patch_centre_cells;
    assert_eq!(
        sprite_ink(&framebuffer),
        (eye_cells + patch_edge_cells) * 4 + patch_centre_cells * 3
    );
}

#[test]
fn invalid_input_is_rejected_without_changing_the_buffer() {
    let initial = Framebuffer::default();
    for (invalid, expected) in [
        (card("", &WHITE_SPRITE), RenderError::EmptyName),
        (
            card("ABCDEFGHIJKLMNOPQ", &WHITE_SPRITE),
            RenderError::NameTooLong,
        ),
        (
            card("Missing@", &WHITE_SPRITE),
            RenderError::UnsupportedGlyph,
        ),
        (
            DailyCard {
                secondary_type: Some(PokemonType::Electric),
                ..card("Pikachu", &WHITE_SPRITE)
            },
            RenderError::DuplicateType,
        ),
    ] {
        let mut framebuffer = initial.clone();
        assert_eq!(render_daily_card(&mut framebuffer, invalid), Err(expected));
        assert_eq!(framebuffer, initial);
    }
}

#[test]
fn clipping_at_every_edge_never_changes_out_of_range_storage() {
    let mut framebuffer = Framebuffer::default();
    for (x, y) in [
        (0, 0),
        (199, 0),
        (0, 199),
        (199, 199),
        (200, 0),
        (0, 200),
        (usize::MAX, usize::MAX),
    ] {
        framebuffer.set_black(x, y);
    }

    assert_eq!(
        framebuffer
            .as_bytes()
            .iter()
            .map(|byte| byte.count_zeros())
            .sum::<u32>(),
        4
    );
}

#[test]
fn every_committed_name_and_type_combination_renders() {
    let pack = ContentPack::parse(PACK).unwrap();
    let mut framebuffer = Framebuffer::default();
    for dex_id in 1..=251 {
        let record = pack.record(dex_id).unwrap();
        render_daily_card(
            &mut framebuffer,
            DailyCard {
                weekday: Weekday::Saturday,
                name: record.name,
                primary_type: record.primary_type,
                secondary_type: record.secondary_type,
                sprite: record.sprite,
                battery_state: BatteryState::Unavailable,
            },
        )
        .unwrap();
    }
}

#[test]
fn fixed_layout_bands_are_disjoint_and_fit_every_label() {
    let weekday_end = WEEKDAY_Y + crate::font::HEIGHT * WEEKDAY_SCALE;
    let sprite_end = SPRITE_Y + 56 * SPRITE_SCALE;
    let name_end = NAME_Y + crate::font::HEIGHT * NAME_SCALE;
    let primary_type_end = PRIMARY_TYPE_Y + crate::font::HEIGHT * TYPE_SCALE;
    let single_type_end = SINGLE_TYPE_Y + crate::font::HEIGHT * TYPE_SCALE;
    let secondary_type_end = SECONDARY_TYPE_Y + crate::font::HEIGHT * TYPE_SCALE;
    assert!(weekday_end <= SPRITE_Y);
    assert!(sprite_end <= NAME_Y);
    assert!(name_end <= PRIMARY_TYPE_Y);
    assert!(primary_type_end <= SECONDARY_TYPE_Y);
    assert!(single_type_end <= 200);
    assert!(secondary_type_end <= RECHARGE_Y);
    const {
        assert!(RECHARGE_Y + crate::font::HEIGHT <= 200);
    }

    let pack = ContentPack::parse(PACK).unwrap();
    for dex_id in 1..=251 {
        let record = pack.record(dex_id).unwrap();
        assert!(text_width(record.name.chars().count(), NAME_SCALE) <= 200);
        assert!(text_width(type_label(record.primary_type).chars().count(), TYPE_SCALE) <= 200);
        if let Some(secondary) = record.secondary_type {
            assert!(text_width(type_label(secondary).chars().count(), TYPE_SCALE) <= 200);
        }
    }
}

#[test]
fn battery_state_variants_are_distinct_and_bounded() {
    let mut hashes = [0; 3];
    for (index, battery_state) in [
        BatteryState::Normal,
        BatteryState::Recharge,
        BatteryState::Unavailable,
    ]
    .into_iter()
    .enumerate()
    {
        let mut framebuffer = Framebuffer::default();
        render_daily_card(
            &mut framebuffer,
            DailyCard {
                battery_state,
                ..card("Pikachu", &WHITE_SPRITE)
            },
        )
        .unwrap();
        hashes[index] = framebuffer.crc32();
    }
    for (index, hash) in hashes.into_iter().enumerate() {
        assert!(!hashes[..index].contains(&hash));
    }
}

#[test]
fn battery_state_draws_only_the_required_warning_content() {
    let blank = Framebuffer::default();

    let mut normal = blank.clone();
    draw_battery_state(&mut normal, BatteryState::Normal);
    assert_eq!(normal, blank);

    let mut recharge = blank.clone();
    draw_battery_state(&mut recharge, BatteryState::Recharge);
    assert_eq!(recharge, blank);
    draw_recharge_status(&mut recharge);
    assert_ne!(recharge, blank);

    let mut unavailable = blank.clone();
    draw_battery_state(&mut unavailable, BatteryState::Unavailable);
    assert_ne!(unavailable, blank);
    assert_eq!(unavailable.crc32(), 0xbbaf_f9be);
}

#[test]
fn setup_screen_is_deterministic_and_identifies_the_recovery_tool() {
    let mut first = Framebuffer::default();
    let mut second = Framebuffer::default();
    render_setup_screen(&mut first);
    render_setup_screen(&mut second);

    assert_eq!(first, second);
    assert_eq!(crc32fast::hash(first.as_bytes()), 0x34e3_1d2e);
}

#[test]
fn recovery_screen_validation_is_atomic_and_deterministic() {
    let mut framebuffer = Framebuffer::default();
    render_recovery_screen(&mut framebuffer, "PACK", "REFLASH").unwrap();
    assert_eq!(framebuffer.crc32(), 0xee18_1690);
    let before = framebuffer.clone();
    assert_eq!(
        render_recovery_screen(&mut framebuffer, "PACK", "@"),
        Err(RenderError::UnsupportedGlyph)
    );
    assert_eq!(framebuffer, before);
}
