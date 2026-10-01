//! Two-bit content-sprite pixel format.

/// Width and height of one content sprite in source pixels.
pub const CONTENT_SPRITE_SIZE: usize = 56;

/// Bytes in one 56 × 56 content sprite with two bits per pixel.
pub const CONTENT_SPRITE_BYTES: usize = CONTENT_SPRITE_SIZE * CONTENT_SPRITE_SIZE / 4;

/// Darkest sprite shade; `0` is white.
pub const BLACK_SHADE: u8 = 3;

/// Read the two-bit shade at `(x, y)`, from `0` white to [`BLACK_SHADE`].
///
/// Pixels are row-major, and the most-significant bit pair of each byte is the
/// leftmost pixel.
///
/// # Panics
///
/// Panics if `x` or `y` is not below [`CONTENT_SPRITE_SIZE`].
#[must_use]
pub fn sprite_shade(sprite: &[u8; CONTENT_SPRITE_BYTES], x: usize, y: usize) -> u8 {
    let (index, shift) = shade_location(x, y);
    (sprite[index] >> shift) & 0b11
}

/// Write `shade` at `(x, y)`, replacing the previous value.
///
/// # Panics
///
/// Panics if `x` or `y` is not below [`CONTENT_SPRITE_SIZE`], or if `shade`
/// is above [`BLACK_SHADE`].
pub fn set_sprite_shade(sprite: &mut [u8; CONTENT_SPRITE_BYTES], x: usize, y: usize, shade: u8) {
    assert!(shade <= BLACK_SHADE, "sprite shade {shade} is not two bits");
    let (index, shift) = shade_location(x, y);
    sprite[index] = (sprite[index] & !(0b11 << shift)) | (shade << shift);
}

fn shade_location(x: usize, y: usize) -> (usize, usize) {
    assert!(
        x < CONTENT_SPRITE_SIZE && y < CONTENT_SPRITE_SIZE,
        "sprite pixel is outside the canvas"
    );
    let pixel = y * CONTENT_SPRITE_SIZE + x;
    (pixel / 4, 6 - 2 * (pixel % 4))
}
