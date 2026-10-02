#![no_std]
#![forbid(unsafe_code)]
#![doc = "Deterministic domain and rendering logic shared by firmware and host tests."]

mod battery;
mod content;
mod font;
mod protocol;
mod recovery;
mod render;
mod schedule;
mod sprite;

pub use battery::{
    BATTERY_SAMPLE_COUNT, BATTERY_SNAPSHOT_BYTES, BATTERY_SNAPSHOT_VERSION, BatteryError,
    BatteryReading, BatteryState, CLEAR_RECHARGE_MV, ENTER_RECHARGE_MV, MAX_BATTERY_MV,
    MIN_BATTERY_MV, decode_battery_snapshot, encode_battery_snapshot, filtered_battery_mv,
    update_battery_reading,
};
pub use content::{
    CONTENT_FORMAT_VERSION, CONTENT_REVISION, ContentPack, POKEMON_COUNT, PackError, PokemonRecord,
    PokemonType,
};
pub use protocol::{
    BATTERY_PAYLOAD_BYTES, CAP_DIAGNOSTICS, CAP_ENTER_STORAGE, CAP_HANDSHAKE, CAP_READ_BATTERY,
    CAP_READ_RTC, CAP_SET_RTC, CAPABILITIES, Command, EncodedFrame, FIRMWARE_VERSION,
    FrameAccumulator, FrameError, FrameKind, ProtocolFrame, Status, decode_battery_reading,
    decode_datetime, encode_battery_reading, encode_datetime,
};
pub use recovery::{RecoveryState, SetupReason, assess_rtc};
pub use render::{
    DailyCard, Framebuffer, RenderError, render_daily_card, render_recovery_screen,
    render_setup_screen,
};
pub use schedule::{
    DailySelection, DisplayDate, InvalidDateTime, LocalDateTime, SCHEDULE_VERSION, Weekday,
    next_rollover, scheduled_dex_id, select_daily_pokemon,
};
pub use sprite::{
    BLACK_SHADE, CONTENT_SPRITE_BYTES, CONTENT_SPRITE_SIZE, set_sprite_shade, sprite_shade,
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
