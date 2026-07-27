//! Hardware-independent renderer for the 200 × 200 monochrome panel buffer.

use crate::{
    CONTENT_SPRITE_BYTES, DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAMEBUFFER_BYTES, PokemonType, Weekday,
    font,
};

const NAME_MAX_BYTES: usize = 16;
const WEEKDAY_SCALE: usize = 2;
const NAME_SCALE: usize = 3;
const TYPE_SCALE: usize = 2;
const SPRITE_SCALE: usize = 2;
const WEEKDAY_Y: usize = 3;
const SPRITE_Y: usize = 21;
const NAME_Y: usize = 139;
const SINGLE_TYPE_Y: usize = 177;
const PRIMARY_TYPE_Y: usize = 166;
const SECONDARY_TYPE_Y: usize = 183;

/// Typed input accepted by the shared daily-card renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DailyCard<'a> {
    /// Weekday belonging to the complete display day.
    pub weekday: Weekday,
    /// Validated English display name.
    pub name: &'a str,
    /// Canonical primary type.
    pub primary_type: PokemonType,
    /// Distinct canonical secondary type, when present.
    pub secondary_type: Option<PokemonType>,
    /// Decoded 56 × 56 sprite, with `1` representing black.
    pub sprite: &'a [u8; CONTENT_SPRITE_BYTES],
}

/// Predictable renderer failures detected before the framebuffer is changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderError {
    /// The English display name is empty.
    EmptyName,
    /// The UTF-8 name exceeds the content-pack v1 limit.
    NameTooLong,
    /// The name contains a character absent from the fixed font.
    UnsupportedGlyph,
    /// Primary and secondary types are identical.
    DuplicateType,
    /// Text does not fit the fixed 200-pixel layout.
    TextTooWide,
}

/// Panel-native 200 × 200 one-bit framebuffer.
///
/// Pixels are row-major and most-significant-bit first. A set bit is white and
/// a cleared bit is black, matching the supported Waveshare driver.
#[repr(transparent)]
#[derive(Clone, PartialEq, Eq)]
pub struct Framebuffer {
    bytes: [u8; FRAMEBUFFER_BYTES],
}

impl core::fmt::Debug for Framebuffer {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Framebuffer")
            .field("byte_length", &self.bytes.len())
            .finish_non_exhaustive()
    }
}

impl Default for Framebuffer {
    fn default() -> Self {
        Self {
            bytes: [u8::MAX; FRAMEBUFFER_BYTES],
        }
    }
}

impl Framebuffer {
    /// Borrow the exact bytes passed to the panel driver.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; FRAMEBUFFER_BYTES] {
        &self.bytes
    }

    /// Report one pixel for host evidence and tests.
    #[must_use]
    pub fn is_black(&self, x: usize, y: usize) -> Option<bool> {
        pixel_location(x, y).map(|(index, mask)| self.bytes[index] & mask == 0)
    }

    fn clear_white(&mut self) {
        self.bytes.fill(u8::MAX);
    }

    fn set_black(&mut self, x: usize, y: usize) {
        if let Some((index, mask)) = pixel_location(x, y) {
            self.bytes[index] &= !mask;
        }
    }
}

/// Render one validated daily card into panel-native bytes.
///
/// Validation is atomic: on error, `framebuffer` remains byte-for-byte
/// unchanged.
///
/// # Errors
///
/// Returns [`RenderError`] when the name or type combination violates the v1
/// contract or cannot fit the fixed layout.
pub fn render_daily_card(
    framebuffer: &mut Framebuffer,
    card: DailyCard<'_>,
) -> Result<(), RenderError> {
    validate_card(card)?;

    framebuffer.clear_white();
    draw_centered_text(
        framebuffer,
        weekday_label(card.weekday),
        WEEKDAY_Y,
        WEEKDAY_SCALE,
    );
    draw_sprite(framebuffer, card.sprite);
    draw_centered_text(framebuffer, card.name, NAME_Y, NAME_SCALE);
    match card.secondary_type {
        Some(secondary) => {
            draw_centered_text(
                framebuffer,
                type_label(card.primary_type),
                PRIMARY_TYPE_Y,
                TYPE_SCALE,
            );
            draw_centered_text(
                framebuffer,
                type_label(secondary),
                SECONDARY_TYPE_Y,
                TYPE_SCALE,
            );
        }
        None => draw_centered_text(
            framebuffer,
            type_label(card.primary_type),
            SINGLE_TYPE_Y,
            TYPE_SCALE,
        ),
    }
    Ok(())
}

fn validate_card(card: DailyCard<'_>) -> Result<(), RenderError> {
    if card.name.is_empty() {
        return Err(RenderError::EmptyName);
    }
    if card.name.len() > NAME_MAX_BYTES {
        return Err(RenderError::NameTooLong);
    }
    if card.secondary_type == Some(card.primary_type) {
        return Err(RenderError::DuplicateType);
    }
    validate_text(card.name, NAME_SCALE)?;
    Ok(())
}

fn validate_text(text: &str, scale: usize) -> Result<(), RenderError> {
    let mut character_count = 0;
    for character in text.chars() {
        font::glyph(character).ok_or(RenderError::UnsupportedGlyph)?;
        character_count += 1;
    }
    if text_width(character_count, scale) > DISPLAY_WIDTH {
        return Err(RenderError::TextTooWide);
    }
    Ok(())
}

fn draw_centered_text(framebuffer: &mut Framebuffer, text: &str, y: usize, scale: usize) {
    let width = text_width(text.chars().count(), scale);
    draw_text(framebuffer, text, (DISPLAY_WIDTH - width) / 2, y, scale);
}

fn draw_text(framebuffer: &mut Framebuffer, text: &str, x: usize, y: usize, scale: usize) {
    let advance = (font::WIDTH + 1) * scale;
    for (character_index, character) in text.chars().enumerate() {
        let glyph = font::glyph(character).expect("text is validated before drawing");
        for (row, row_bits) in glyph.iter().copied().enumerate() {
            for column in 0..font::WIDTH {
                if row_bits & (1 << (font::WIDTH - 1 - column)) != 0 {
                    fill_scaled_pixel(
                        framebuffer,
                        x + character_index * advance + column * scale,
                        y + row * scale,
                        scale,
                    );
                }
            }
        }
    }
}

fn draw_sprite(framebuffer: &mut Framebuffer, sprite: &[u8; CONTENT_SPRITE_BYTES]) {
    let sprite_width = 56 * SPRITE_SCALE;
    let x = (DISPLAY_WIDTH - sprite_width) / 2;
    for source_y in 0..56 {
        for source_x in 0..56 {
            let index = source_y * 7 + source_x / 8;
            let mask = 0x80 >> (source_x % 8);
            if sprite[index] & mask != 0 {
                fill_scaled_pixel(
                    framebuffer,
                    x + source_x * SPRITE_SCALE,
                    SPRITE_Y + source_y * SPRITE_SCALE,
                    SPRITE_SCALE,
                );
            }
        }
    }
}

fn fill_scaled_pixel(framebuffer: &mut Framebuffer, x: usize, y: usize, scale: usize) {
    for offset_y in 0..scale {
        for offset_x in 0..scale {
            framebuffer.set_black(x + offset_x, y + offset_y);
        }
    }
}

const fn text_width(character_count: usize, scale: usize) -> usize {
    if character_count == 0 {
        0
    } else {
        (character_count * (font::WIDTH + 1) - 1) * scale
    }
}

fn pixel_location(x: usize, y: usize) -> Option<(usize, u8)> {
    if x >= DISPLAY_WIDTH || y >= DISPLAY_HEIGHT {
        return None;
    }
    Some((y * (DISPLAY_WIDTH / 8) + x / 8, 0x80 >> (x % 8)))
}

const fn weekday_label(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "MONDAY",
        Weekday::Tuesday => "TUESDAY",
        Weekday::Wednesday => "WEDNESDAY",
        Weekday::Thursday => "THURSDAY",
        Weekday::Friday => "FRIDAY",
        Weekday::Saturday => "SATURDAY",
        Weekday::Sunday => "SUNDAY",
    }
}

const fn type_label(pokemon_type: PokemonType) -> &'static str {
    match pokemon_type {
        PokemonType::Normal => "NORMAL",
        PokemonType::Fire => "FIRE",
        PokemonType::Water => "WATER",
        PokemonType::Electric => "ELECTRIC",
        PokemonType::Grass => "GRASS",
        PokemonType::Ice => "ICE",
        PokemonType::Fighting => "FIGHTING",
        PokemonType::Poison => "POISON",
        PokemonType::Ground => "GROUND",
        PokemonType::Flying => "FLYING",
        PokemonType::Psychic => "PSYCHIC",
        PokemonType::Bug => "BUG",
        PokemonType::Rock => "ROCK",
        PokemonType::Ghost => "GHOST",
        PokemonType::Dragon => "DRAGON",
        PokemonType::Dark => "DARK",
        PokemonType::Steel => "STEEL",
        PokemonType::Fairy => "FAIRY",
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
