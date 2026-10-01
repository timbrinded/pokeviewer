//! Trusted retained battery state and wake-gated observation commits.

use pokeviewer_core::{
    BATTERY_SNAPSHOT_BYTES, BatteryReading, BatteryState, decode_battery_snapshot,
    encode_battery_snapshot, update_battery_reading,
};

#[cfg(target_arch = "xtensa")]
use portable_atomic::{AtomicU32, Ordering};

pub(crate) const BATTERY_VALID_DIAGNOSTIC_FLAG: u16 = 1 << 5;
pub(crate) const BATTERY_LOW_DIAGNOSTIC_FLAG: u16 = 1 << 6;

const _: () = assert!(BATTERY_SNAPSHOT_BYTES == size_of::<u32>());

#[cfg(target_arch = "xtensa")]
#[esp_hal::ram(unstable(rtc_fast, persistent))]
static RETAINED_BATTERY_SNAPSHOT: AtomicU32 = AtomicU32::new(0);

pub(crate) const fn diagnostic_flags(reading: BatteryReading) -> u16 {
    match reading.state() {
        BatteryState::Normal => BATTERY_VALID_DIAGNOSTIC_FLAG,
        BatteryState::Recharge => BATTERY_VALID_DIAGNOSTIC_FLAG | BATTERY_LOW_DIAGNOSTIC_FLAG,
        BatteryState::Unavailable => 0,
    }
}

#[cfg(target_arch = "xtensa")]
pub(crate) fn load_retained_battery() -> BatteryReading {
    decode_snapshot_word(RETAINED_BATTERY_SNAPSHOT.load(Ordering::Relaxed))
}

#[cfg(target_arch = "xtensa")]
pub(crate) fn commit_battery_observation(cell_mv: u16) -> BatteryReading {
    let committed =
        commit_snapshot_word(RETAINED_BATTERY_SNAPSHOT.load(Ordering::Relaxed), cell_mv);
    RETAINED_BATTERY_SNAPSHOT.store(committed, Ordering::Relaxed);
    decode_snapshot_word(committed)
}

fn decode_snapshot_word(word: u32) -> BatteryReading {
    decode_battery_snapshot(&word.to_le_bytes()).unwrap_or(BatteryReading::UNAVAILABLE)
}

fn encode_snapshot_word(reading: BatteryReading) -> u32 {
    u32::from_le_bytes(encode_battery_snapshot(reading))
}

fn commit_snapshot_word(previous: u32, cell_mv: u16) -> u32 {
    let next = update_battery_reading(cell_mv, decode_snapshot_word(previous));
    encode_snapshot_word(next)
}

#[cfg(test)]
mod tests {
    use pokeviewer_core::{
        BatteryReading, BatteryState, encode_battery_snapshot, update_battery_reading,
    };

    use super::{commit_snapshot_word, decode_snapshot_word, diagnostic_flags};

    #[test]
    fn erased_and_invalid_snapshots_are_unavailable() {
        assert_eq!(decode_snapshot_word(0), BatteryReading::UNAVAILABLE);

        let mut invalid_tag = encode_battery_snapshot(BatteryReading::UNAVAILABLE);
        invalid_tag[3] ^= 0x80;
        assert_eq!(
            decode_snapshot_word(u32::from_le_bytes(invalid_tag)),
            BatteryReading::UNAVAILABLE
        );
    }

    #[test]
    fn scheduled_observation_commits_through_the_versioned_codec() {
        let word = commit_snapshot_word(0, 4_000);
        let reading = decode_snapshot_word(word);
        assert_eq!(reading.state(), BatteryState::Normal);
        assert_eq!(reading.cell_mv(), 4_000);
        assert_eq!(word.to_le_bytes(), encode_battery_snapshot(reading));
    }

    #[test]
    fn invalid_observation_preserves_the_complete_recharge_snapshot() {
        let recharge = update_battery_reading(3_700, BatteryReading::UNAVAILABLE);
        assert_eq!(recharge.state(), BatteryState::Recharge);
        let retained = u32::from_le_bytes(encode_battery_snapshot(recharge));

        assert_eq!(commit_snapshot_word(retained, 0), retained);
        assert_eq!(decode_snapshot_word(retained).cell_mv(), 3_700);
    }

    #[test]
    fn invalid_observation_replaces_non_recharge_state_with_unavailable() {
        let normal = update_battery_reading(4_000, BatteryReading::UNAVAILABLE);
        let retained = u32::from_le_bytes(encode_battery_snapshot(normal));
        let committed = commit_snapshot_word(retained, u16::MAX);

        assert_eq!(decode_snapshot_word(committed), BatteryReading::UNAVAILABLE);
        assert_eq!(
            committed.to_le_bytes(),
            encode_battery_snapshot(BatteryReading::UNAVAILABLE)
        );
    }

    #[test]
    fn diagnostics_are_derived_only_from_trusted_state() {
        let normal = update_battery_reading(4_000, BatteryReading::UNAVAILABLE);
        let recharge = update_battery_reading(3_700, BatteryReading::UNAVAILABLE);

        assert_eq!(diagnostic_flags(BatteryReading::UNAVAILABLE), 0);
        assert_eq!(diagnostic_flags(normal), 1 << 5);
        assert_eq!(diagnostic_flags(recharge), (1 << 5) | (1 << 6));
    }
}
