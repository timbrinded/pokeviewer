use super::{DailyCard, Framebuffer, RenderError, render_daily_card};
use crate::{CONTENT_SPRITE_BYTES, ContentPack, FRAMEBUFFER_BYTES, PokemonType, Weekday};

const PACK: &[u8] = include_bytes!("../../../content/generated/pokeviewer-v1.pack");
const WHITE_SPRITE: [u8; CONTENT_SPRITE_BYTES] = [0; CONTENT_SPRITE_BYTES];
const BLACK_SPRITE: [u8; CONTENT_SPRITE_BYTES] = [u8::MAX; CONTENT_SPRITE_BYTES];

fn card<'a>(name: &'a str, sprite: &'a [u8; CONTENT_SPRITE_BYTES]) -> DailyCard<'a> {
    DailyCard {
        weekday: Weekday::Wednesday,
        name,
        primary_type: PokemonType::Electric,
        secondary_type: Some(PokemonType::Flying),
        sprite,
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
    assert_eq!(crc32fast::hash(black_first.as_bytes()), 0x9e66_9b73);

    let mut white = Framebuffer::default();
    render_daily_card(&mut white, card("Nidoran♀", &WHITE_SPRITE)).unwrap();
    assert_eq!(crc32fast::hash(white.as_bytes()), 0x1916_b0fa);
    assert_ne!(black_first, white);
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
            card("Missing?", &WHITE_SPRITE),
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
    for dex_id in 1..=151 {
        let record = pack.record(dex_id).unwrap();
        render_daily_card(
            &mut framebuffer,
            DailyCard {
                weekday: Weekday::Saturday,
                name: record.name,
                primary_type: record.primary_type,
                secondary_type: record.secondary_type,
                sprite: record.sprite,
            },
        )
        .unwrap();
    }
}
