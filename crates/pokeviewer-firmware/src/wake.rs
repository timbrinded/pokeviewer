//! Target-independent interpretation of ESP32-S3 wake evidence.

/// Wake category supplied by the hardware boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WakeInput {
    /// Cold boot, external reset, or a software reset.
    Reset,
    /// EXT1 resumed the device from deep sleep.
    Ext1 {
        /// EXT1 status contained the RTC interrupt pin.
        rtc_pin: bool,
        /// EXT1 status contained the PWR button pin.
        power_pin: bool,
        /// The PCF85063 alarm flag is asserted.
        alarm_pending: bool,
    },
    /// A source that release firmware did not configure.
    Other,
}

/// Ordered work selected from wake evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WakeDecision {
    /// Refresh the daily card before any parent-session work.
    pub refresh_daily: bool,
    /// Check whether PWR remains held long enough to open a parent session.
    pub check_parent_session: bool,
}

/// Release firmware did not configure the reported wake evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnexpectedWake;

/// Select ordered wake work from the source register and RTC alarm flag.
///
/// # Errors
///
/// Returns `UnexpectedWake` when the evidence does not identify an expected
/// release wake.
pub const fn decide_wake(input: WakeInput) -> Result<WakeDecision, UnexpectedWake> {
    match input {
        WakeInput::Reset => Ok(WakeDecision {
            refresh_daily: true,
            check_parent_session: false,
        }),
        WakeInput::Ext1 {
            rtc_pin,
            power_pin,
            alarm_pending,
        } if (rtc_pin && alarm_pending) || power_pin => Ok(WakeDecision {
            refresh_daily: rtc_pin && alarm_pending,
            check_parent_session: power_pin,
        }),
        WakeInput::Ext1 { .. } | WakeInput::Other => Err(UnexpectedWake),
    }
}

#[cfg(test)]
mod tests {
    use super::{WakeDecision, WakeInput, decide_wake};

    #[test]
    fn reset_refreshes_without_opening_a_parent_session() {
        assert_eq!(
            decide_wake(WakeInput::Reset),
            Ok(WakeDecision {
                refresh_daily: true,
                check_parent_session: false,
            })
        );
    }

    #[test]
    fn rtc_alarm_and_power_are_independent_expected_ext1_sources() {
        assert_eq!(
            decide_wake(WakeInput::Ext1 {
                rtc_pin: true,
                power_pin: false,
                alarm_pending: true,
            }),
            Ok(WakeDecision {
                refresh_daily: true,
                check_parent_session: false,
            })
        );
        assert_eq!(
            decide_wake(WakeInput::Ext1 {
                rtc_pin: false,
                power_pin: true,
                alarm_pending: false,
            }),
            Ok(WakeDecision {
                refresh_daily: false,
                check_parent_session: true,
            })
        );
    }

    #[test]
    fn simultaneous_alarm_refreshes_before_parent_work() {
        assert_eq!(
            decide_wake(WakeInput::Ext1 {
                rtc_pin: true,
                power_pin: true,
                alarm_pending: true,
            }),
            Ok(WakeDecision {
                refresh_daily: true,
                check_parent_session: true,
            })
        );
    }

    #[test]
    fn stale_rtc_pin_and_unknown_sources_are_rejected() {
        assert!(decide_wake(WakeInput::Other).is_err());
        assert!(
            decide_wake(WakeInput::Ext1 {
                rtc_pin: true,
                power_pin: false,
                alarm_pending: false,
            })
            .is_err()
        );
    }
}
