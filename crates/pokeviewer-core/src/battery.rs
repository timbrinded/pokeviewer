//! Deterministic generic `LiPo` voltage estimate and display policy.

/// Number of calibrated divider samples used by one battery observation.
pub const BATTERY_SAMPLE_COUNT: usize = 16;

const MIN_PLAUSIBLE_MV: u16 = 2_500;
const MAX_PLAUSIBLE_MV: u16 = 4_500;
const ENTER_RECHARGE_PER_MILLE: u16 = 150;
const CLEAR_RECHARGE_PER_MILLE: u16 = 200;

/// Generic `LiPo` open-circuit-voltage points from Zephyr's default battery
/// profile. Each entry is `(state_of_charge_percent, microvolts)`.
pub const GENERIC_LIPO_OCV_UV: [(u8, u32); 11] = [
    (0, 3_305_545),
    (10, 3_686_654),
    (20, 3_741_018),
    (30, 3_775_129),
    (40, 3_793_250),
    (50, 3_820_965),
    (60, 3_884_009),
    (70, 3_945_074),
    (80, 4_008_118),
    (90, 4_085_934),
    (100, 4_177_454),
];

/// Battery information shown on a daily card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryStatus {
    /// Approximate battery state in coarse 10% steps.
    Estimated {
        /// Display percentage. Valid values are 0, 10, through 100.
        percent: u8,
        /// Whether the persistent recharge warning is active.
        recharge: bool,
    },
    /// No plausible calibrated battery observation was available.
    Unavailable,
}

impl BatteryStatus {
    /// Report whether this value satisfies the renderer contract.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        match self {
            Self::Estimated { percent, .. } => percent <= 100 && percent % 10 == 0,
            Self::Unavailable => true,
        }
    }
}

/// Result of one voltage estimate, including the next retained hysteresis bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatteryEstimate {
    /// Status to render.
    pub status: BatteryStatus,
    /// Recharge state to retain for the next valid observation.
    pub recharge_latched: bool,
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

/// Convert one filtered cell voltage into a coarse display estimate.
///
/// The input is rejected only when it is outside a broad physically plausible
/// single-cell range. Voltages outside the generic OCV table are clamped to
/// its 0% and 100% endpoints.
#[must_use]
pub fn estimate_battery(battery_mv: u16, recharge_was_latched: bool) -> BatteryEstimate {
    if !(MIN_PLAUSIBLE_MV..=MAX_PLAUSIBLE_MV).contains(&battery_mv) {
        return BatteryEstimate {
            status: BatteryStatus::Unavailable,
            recharge_latched: recharge_was_latched,
        };
    }

    let raw_per_mille = interpolate_per_mille(u32::from(battery_mv) * 1_000);
    let recharge_latched = if recharge_was_latched {
        raw_per_mille < CLEAR_RECHARGE_PER_MILLE
    } else {
        raw_per_mille < ENTER_RECHARGE_PER_MILLE
    };
    let mut percent = u8::try_from(((raw_per_mille + 50) / 100) * 10).unwrap_or(100);
    if recharge_latched && percent > 10 {
        percent = 10;
    }
    BatteryEstimate {
        status: BatteryStatus::Estimated {
            percent,
            recharge: recharge_latched,
        },
        recharge_latched,
    }
}

fn interpolate_per_mille(battery_uv: u32) -> u16 {
    let first = GENERIC_LIPO_OCV_UV[0];
    if battery_uv <= first.1 {
        return u16::from(first.0) * 10;
    }
    let last = GENERIC_LIPO_OCV_UV[GENERIC_LIPO_OCV_UV.len() - 1];
    if battery_uv >= last.1 {
        return u16::from(last.0) * 10;
    }

    for points in GENERIC_LIPO_OCV_UV.windows(2) {
        let lower = points[0];
        let upper = points[1];
        if battery_uv <= upper.1 {
            let voltage_offset = u64::from(battery_uv - lower.1);
            let voltage_span = u64::from(upper.1 - lower.1);
            let state_offset = u64::from(upper.0 - lower.0) * 10;
            let interpolated = u64::from(lower.0) * 10
                + (voltage_offset * state_offset + voltage_span / 2) / voltage_span;
            return u16::try_from(interpolated).expect("table estimate fits u16");
        }
    }
    unreachable!("the endpoint checks cover the complete table")
}

#[cfg(test)]
mod tests {
    use super::{
        BATTERY_SAMPLE_COUNT, BatteryStatus, GENERIC_LIPO_OCV_UV, estimate_battery,
        filtered_battery_mv, interpolate_per_mille,
    };

    #[test]
    fn every_published_ocv_point_maps_to_its_exact_state() {
        for (percent, voltage_uv) in GENERIC_LIPO_OCV_UV {
            assert_eq!(interpolate_per_mille(voltage_uv), u16::from(percent) * 10);
        }
    }

    #[test]
    fn interpolation_and_endpoints_are_bounded() {
        assert_eq!(interpolate_per_mille(0), 0);
        assert_eq!(interpolate_per_mille(u32::MAX), 1_000);
        assert_eq!(
            interpolate_per_mille(u32::midpoint(3_820_965, 3_884_009)),
            550
        );
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
    fn estimate_rejects_implausible_samples() {
        for voltage in [0, 2_499, 4_501, u16::MAX] {
            assert_eq!(
                estimate_battery(voltage, true),
                super::BatteryEstimate {
                    status: BatteryStatus::Unavailable,
                    recharge_latched: true,
                }
            );
        }
    }

    #[test]
    fn recharge_hysteresis_is_conservative_and_coarse() {
        let low = estimate_battery(3_700, false);
        assert_eq!(
            low.status,
            BatteryStatus::Estimated {
                percent: 10,
                recharge: true,
            }
        );
        assert!(low.recharge_latched);

        let retained = estimate_battery(3_720, true);
        assert_eq!(
            retained.status,
            BatteryStatus::Estimated {
                percent: 10,
                recharge: true,
            }
        );

        let cleared = estimate_battery(3_742, true);
        assert_eq!(
            cleared.status,
            BatteryStatus::Estimated {
                percent: 20,
                recharge: false,
            }
        );
        assert!(!cleared.recharge_latched);
    }

    #[test]
    fn normal_display_uses_ten_percent_steps() {
        for voltage in 2_500..=4_500 {
            let status = estimate_battery(voltage, false).status;
            assert!(status.is_valid());
        }
    }
}
