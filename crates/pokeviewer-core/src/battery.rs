//! Deterministic battery filtering, state transitions, and retention codec.

/// Number of calibrated divider samples used by one battery observation.
pub const BATTERY_SAMPLE_COUNT: usize = 16;
/// Minimum plausible single-cell voltage.
pub const MIN_BATTERY_MV: u16 = 2_500;
/// Maximum plausible single-cell voltage.
pub const MAX_BATTERY_MV: u16 = 4_500;
/// Voltage below which a normal battery enters recharge state.
pub const ENTER_RECHARGE_MV: u16 = 3_750;
/// Voltage at or above which a recharge battery returns to normal state.
pub const CLEAR_RECHARGE_MV: u16 = 3_850;
/// Version of the retained battery snapshot format.
pub const BATTERY_SNAPSHOT_VERSION: u8 = 1;
/// Encoded retained battery snapshot length.
pub const BATTERY_SNAPSHOT_BYTES: usize = 4;
// Twelve retained tag bits derived from the ASCII `BT` marker (0x4254).
const BATTERY_SNAPSHOT_MAGIC: u16 = 0x0254;
// The version occupies the low two tag bits.
const _: () = assert!(BATTERY_SNAPSHOT_VERSION < 4);
const BATTERY_SNAPSHOT_TAG: u16 =
    (BATTERY_SNAPSHOT_MAGIC << 2) | u16::from_be_bytes([0, BATTERY_SNAPSHOT_VERSION]);

/// Coarse battery state used by the application and renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BatteryState {
    /// The last valid voltage does not require a recharge warning.
    Normal = 0,
    /// The recharge warning is latched.
    Recharge = 1,
    /// No valid voltage is available.
    Unavailable = 2,
}

impl BatteryState {
    pub(crate) const fn to_wire(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::Recharge => 1,
            Self::Unavailable => 2,
        }
    }

    pub(crate) const fn from_wire(value: u8) -> Result<Self, BatteryError> {
        match value {
            0 => Ok(Self::Normal),
            1 => Ok(Self::Recharge),
            2 => Ok(Self::Unavailable),
            _ => Err(BatteryError::InvalidState),
        }
    }
}

/// A validated battery state and its associated cell voltage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatteryReading {
    state: BatteryState,
    cell_mv: u16,
}

impl BatteryReading {
    /// Canonical value used when no plausible reading is available.
    pub const UNAVAILABLE: Self = Self {
        state: BatteryState::Unavailable,
        cell_mv: 0,
    };

    /// Construct a reading that satisfies the battery contract.
    ///
    /// # Errors
    ///
    /// Returns [`BatteryError::InvalidMillivolts`] unless the state and voltage
    /// satisfy the exact normal, recharge, or unavailable ranges.
    pub const fn new(state: BatteryState, cell_mv: u16) -> Result<Self, BatteryError> {
        let valid = match state {
            BatteryState::Normal => cell_mv >= ENTER_RECHARGE_MV && cell_mv <= MAX_BATTERY_MV,
            BatteryState::Recharge => cell_mv >= MIN_BATTERY_MV && cell_mv < CLEAR_RECHARGE_MV,
            BatteryState::Unavailable => cell_mv == 0,
        };
        if valid {
            Ok(Self { state, cell_mv })
        } else {
            Err(BatteryError::InvalidMillivolts)
        }
    }

    /// Return the coarse battery state.
    #[must_use]
    pub const fn state(self) -> BatteryState {
        self.state
    }

    /// Return the validated cell voltage, or zero when unavailable.
    #[must_use]
    pub const fn cell_mv(self) -> u16 {
        self.cell_mv
    }
}

impl Default for BatteryReading {
    fn default() -> Self {
        Self::UNAVAILABLE
    }
}

/// Retained battery snapshot validation failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryError {
    /// The snapshot length is not the fixed v1 length.
    InvalidLength,
    /// The retained snapshot version is unsupported.
    UnsupportedVersion,
    /// The encoded battery state is unknown.
    InvalidState,
    /// State and millivolts do not form a valid reading.
    InvalidMillivolts,
}

/// Apply the board's 2:1 divider to the median of 16 calibrated ADC readings.
///
/// The readings are divider millivolts. For an even sample count, the median
/// is the integer average of the two central readings.
#[must_use]
pub fn filtered_battery_mv(mut divider_samples_mv: [u16; BATTERY_SAMPLE_COUNT]) -> u16 {
    divider_samples_mv.sort_unstable();
    let middle_sum = u32::from(divider_samples_mv[7]) + u32::from(divider_samples_mv[8]);
    let divider_mv = middle_sum / 2;
    u16::try_from(divider_mv.saturating_mul(2)).unwrap_or(u16::MAX)
}

/// Apply the battery range and recharge hysteresis contract to one sample.
///
/// An invalid sample preserves a prior recharge reading, including its last
/// valid millivolts. Other invalid samples become unavailable.
#[must_use]
pub fn update_battery_reading(cell_mv: u16, previous: BatteryReading) -> BatteryReading {
    if !(MIN_BATTERY_MV..=MAX_BATTERY_MV).contains(&cell_mv) {
        return if previous.state == BatteryState::Recharge {
            previous
        } else {
            BatteryReading::UNAVAILABLE
        };
    }

    let state = if previous.state == BatteryState::Recharge {
        if cell_mv >= CLEAR_RECHARGE_MV {
            BatteryState::Normal
        } else {
            BatteryState::Recharge
        }
    } else if cell_mv < ENTER_RECHARGE_MV {
        BatteryState::Recharge
    } else {
        BatteryState::Normal
    };
    BatteryReading { state, cell_mv }
}

/// Encode one validated reading for retained memory.
#[must_use]
pub fn encode_battery_snapshot(reading: BatteryReading) -> [u8; BATTERY_SNAPSHOT_BYTES] {
    let packed = u32::from(reading.cell_mv)
        | (u32::from(reading.state.to_wire()) << 16)
        | (u32::from(BATTERY_SNAPSHOT_TAG) << 18);
    packed.to_le_bytes()
}

/// Decode and validate one retained battery snapshot.
///
/// # Errors
///
/// Returns a [`BatteryError`] for invalid length, version, state, or voltage.
pub fn decode_battery_snapshot(bytes: &[u8]) -> Result<BatteryReading, BatteryError> {
    if bytes.len() != BATTERY_SNAPSHOT_BYTES {
        return Err(BatteryError::InvalidLength);
    }
    let tag = u16::from(bytes[2] >> 2) | (u16::from(bytes[3]) << 6);
    if tag != BATTERY_SNAPSHOT_TAG {
        return Err(BatteryError::UnsupportedVersion);
    }
    BatteryReading::new(
        BatteryState::from_wire(bytes[2] & 0x03)?,
        u16::from_le_bytes([bytes[0], bytes[1]]),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        BATTERY_SAMPLE_COUNT, BATTERY_SNAPSHOT_TAG, BatteryError, BatteryReading, BatteryState,
        decode_battery_snapshot, encode_battery_snapshot, filtered_battery_mv,
        update_battery_reading,
    };

    fn reading(state: BatteryState, cell_mv: u16) -> BatteryReading {
        BatteryReading::new(state, cell_mv).unwrap()
    }

    #[test]
    fn filter_discards_order_and_uses_the_two_middle_values() {
        let mut samples = [1_900; BATTERY_SAMPLE_COUNT];
        samples[0] = 100;
        samples[1] = 3_000;
        samples[7] = 1_800;
        samples[8] = 2_000;
        assert_eq!(filtered_battery_mv(samples), 3_800);
    }

    #[test]
    fn recharge_thresholds_are_exact_and_hysteretic() {
        let unavailable = BatteryReading::UNAVAILABLE;
        assert_eq!(
            update_battery_reading(3_749, unavailable),
            reading(BatteryState::Recharge, 3_749)
        );
        assert_eq!(
            update_battery_reading(3_750, unavailable),
            reading(BatteryState::Normal, 3_750)
        );

        let recharge = reading(BatteryState::Recharge, 3_700);
        assert_eq!(
            update_battery_reading(3_849, recharge),
            reading(BatteryState::Recharge, 3_849)
        );
        assert_eq!(
            update_battery_reading(3_850, recharge),
            reading(BatteryState::Normal, 3_850)
        );
    }

    #[test]
    fn invalid_sample_preserves_only_a_prior_recharge_reading() {
        let recharge = reading(BatteryState::Recharge, 3_700);
        for cell_mv in [0, 2_499, 4_501, u16::MAX] {
            assert_eq!(update_battery_reading(cell_mv, recharge), recharge);
            assert_eq!(
                update_battery_reading(cell_mv, reading(BatteryState::Normal, 4_000)),
                BatteryReading::UNAVAILABLE
            );
            assert_eq!(
                update_battery_reading(cell_mv, BatteryReading::UNAVAILABLE),
                BatteryReading::UNAVAILABLE
            );
        }
    }

    #[test]
    fn reading_validation_enforces_state_voltage_pairs() {
        assert_eq!(
            BatteryReading::new(BatteryState::Normal, 3_749),
            Err(BatteryError::InvalidMillivolts)
        );
        assert!(BatteryReading::new(BatteryState::Normal, 3_750).is_ok());
        assert!(BatteryReading::new(BatteryState::Normal, 4_500).is_ok());
        assert_eq!(
            BatteryReading::new(BatteryState::Normal, 4_501),
            Err(BatteryError::InvalidMillivolts)
        );

        assert_eq!(
            BatteryReading::new(BatteryState::Recharge, 2_499),
            Err(BatteryError::InvalidMillivolts)
        );
        assert!(BatteryReading::new(BatteryState::Recharge, 2_500).is_ok());
        assert!(BatteryReading::new(BatteryState::Recharge, 3_849).is_ok());
        assert_eq!(
            BatteryReading::new(BatteryState::Recharge, 3_850),
            Err(BatteryError::InvalidMillivolts)
        );
        assert_eq!(
            BatteryReading::new(BatteryState::Unavailable, 1),
            Err(BatteryError::InvalidMillivolts)
        );
        assert_eq!(
            BatteryReading::new(BatteryState::Unavailable, 0),
            Ok(BatteryReading::UNAVAILABLE)
        );
    }

    #[test]
    fn retained_snapshot_is_versioned_and_validated() {
        let normal = reading(BatteryState::Normal, 4_123);
        let recharge = reading(BatteryState::Recharge, 3_700);
        assert_eq!(encode_battery_snapshot(normal), [0x1b, 0x10, 0x44, 0x25]);
        assert_eq!(
            encode_battery_snapshot(BatteryReading::UNAVAILABLE),
            [0, 0, 0x46, 0x25]
        );
        for reading in [normal, recharge, BatteryReading::UNAVAILABLE] {
            let encoded = encode_battery_snapshot(reading);
            assert_eq!(decode_battery_snapshot(&encoded), Ok(reading));
        }

        assert_eq!(
            decode_battery_snapshot(&[1, 0, 0]),
            Err(BatteryError::InvalidLength)
        );
        for stale in [0_u32, 0x4254_0000, u32::MAX] {
            assert_eq!(
                decode_battery_snapshot(&stale.to_le_bytes()),
                Err(BatteryError::UnsupportedVersion)
            );
        }

        let invalid_state = (3_u32 << 16) | (u32::from(BATTERY_SNAPSHOT_TAG) << 18) | 0x0e74;
        assert_eq!(
            decode_battery_snapshot(&invalid_state.to_le_bytes()),
            Err(BatteryError::InvalidState)
        );
        let invalid_unavailable = (2_u32 << 16) | (u32::from(BATTERY_SNAPSHOT_TAG) << 18) | 1;
        assert_eq!(
            decode_battery_snapshot(&invalid_unavailable.to_le_bytes()),
            Err(BatteryError::InvalidMillivolts)
        );
    }
}
