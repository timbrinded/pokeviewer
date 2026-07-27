#![no_std]
#![forbid(unsafe_code)]
#![doc = "Deterministic domain and rendering logic shared by firmware and host tests."]

mod content;
mod font;
mod protocol;
mod recovery;
mod render;
mod schedule;

pub use content::{CONTENT_SPRITE_BYTES, ContentPack, PackError, PokemonRecord, PokemonType};
pub use protocol::{
    Command, EncodedFrame, FrameAccumulator, FrameError, FrameKind, ProtocolFrame, Status,
    decode_datetime, encode_datetime,
};
pub use recovery::{RecoveryState, SetupReason, assess_rtc};
pub use render::{DailyCard, Framebuffer, RenderError, render_daily_card, render_setup_screen};
pub use schedule::{
    DailySelection, DisplayDate, InvalidDateTime, LocalDateTime, SCHEDULE_VERSION, Weekday,
    select_daily_pokemon,
};

/// Width of the supported e-paper panel in pixels.
pub const DISPLAY_WIDTH: usize = 200;

/// Height of the supported e-paper panel in pixels.
pub const DISPLAY_HEIGHT: usize = 200;

/// Bytes required for a one-bit full-screen framebuffer.
pub const FRAMEBUFFER_BYTES: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT / 8;

#[cfg(test)]
mod tests {
    use super::{DISPLAY_HEIGHT, DISPLAY_WIDTH, FRAMEBUFFER_BYTES};

    #[test]
    fn framebuffer_matches_board_contract() {
        assert_eq!(DISPLAY_WIDTH, 200);
        assert_eq!(DISPLAY_HEIGHT, 200);
        assert_eq!(FRAMEBUFFER_BYTES, 5_000);
    }
}
