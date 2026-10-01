//! Target-independent interpretation of ESP32-S3 wake evidence.

use pokeviewer_core::BatteryState;

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
        /// EXT1 status contained the BOOT button pin.
        boot_pin: bool,
        /// The PCF85063 alarm flag is asserted.
        alarm_pending: bool,
    },
    /// The ESP32-S3 RTC timer fired for the periodic battery check.
    Timer,
    /// A source that release firmware did not configure.
    Other,
}

/// Ordered work selected from wake evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent step the runtime executes in a fixed order"
)]
pub struct WakeDecision {
    /// Refresh the daily card before any parent-session work.
    pub refresh_daily: bool,
    /// Sample and commit the battery observation for validated daily work.
    pub sample_battery: bool,
    /// Check whether PWR remains held long enough to open a parent session.
    pub check_parent_session: bool,
    /// Check whether BOOT remains held long enough to restart the firmware.
    pub check_restart: bool,
}

impl WakeDecision {
    /// Return whether validated daily work can sample and commit the battery.
    #[must_use]
    pub const fn should_commit_battery(self, rtc_valid: bool, wake_plan_valid: bool) -> bool {
        self.sample_battery && rtc_valid && wake_plan_valid
    }

    /// Return whether the panel must be redrawn after any battery commit.
    ///
    /// A periodic battery check redraws only when the displayed battery state
    /// changed, so most checks leave the panel untouched.
    #[must_use]
    pub fn refresh_required(self, previous: BatteryState, current: BatteryState) -> bool {
        self.refresh_daily || self.check_restart || (self.sample_battery && previous != current)
    }

    /// Resume only the deferred parent session after simultaneous daily work.
    #[must_use]
    pub const fn parent_session_after_daily() -> Self {
        Self {
            refresh_daily: false,
            sample_battery: false,
            check_parent_session: true,
            check_restart: false,
        }
    }
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
            sample_battery: false,
            check_parent_session: false,
            check_restart: false,
        }),
        WakeInput::Ext1 {
            rtc_pin,
            power_pin,
            boot_pin,
            alarm_pending,
        } if (rtc_pin && alarm_pending) || power_pin || boot_pin => {
            let alarm = rtc_pin && alarm_pending;
            // An alarm refresh or parent session already covers a restart.
            let restart = boot_pin && !alarm && !power_pin;
            Ok(WakeDecision {
                refresh_daily: alarm,
                sample_battery: alarm || restart,
                check_parent_session: power_pin,
                check_restart: restart,
            })
        }
        WakeInput::Timer => Ok(WakeDecision {
            refresh_daily: false,
            sample_battery: true,
            check_parent_session: false,
            check_restart: false,
        }),
        WakeInput::Ext1 { .. } | WakeInput::Other => Err(UnexpectedWake),
    }
}

/// Active-low EXT1 inputs armed for one deep sleep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SleepWakeSources {
    /// Wake on the PCF85063 alarm line (GPIO5).
    pub rtc_alarm: bool,
    /// Wake on the PWR button (GPIO18).
    pub power_button: bool,
    /// Wake on the BOOT button (GPIO0).
    pub boot_button: bool,
}

/// Choose the EXT1 inputs to arm after the bounded release wait has ended.
///
/// `requested` lists the inputs this sleep mode wants. `low` lists the
/// requested inputs that are still active-low. EXT1 `ANY_LOW` wakes
/// immediately from a line that is already low, so arming a stuck line would
/// create a wake loop.
///
/// Each stuck line is dropped on its own, so a stuck RTC line still leaves BOOT
/// available for an adult restart.
#[must_use]
pub const fn select_sleep_wake_sources(
    requested: SleepWakeSources,
    low: SleepWakeSources,
) -> SleepWakeSources {
    SleepWakeSources {
        rtc_alarm: requested.rtc_alarm && !low.rtc_alarm,
        power_button: requested.power_button && !low.power_button,
        boot_button: requested.boot_button && !low.boot_button,
    }
}

#[cfg(test)]
mod tests {
    use pokeviewer_core::BatteryState;

    use super::{
        SleepWakeSources, WakeDecision, WakeInput, decide_wake, select_sleep_wake_sources,
    };

    #[test]
    fn reset_and_software_reset_refresh_without_sampling_or_parent_work() {
        assert_eq!(
            decide_wake(WakeInput::Reset),
            Ok(WakeDecision {
                refresh_daily: true,
                sample_battery: false,
                check_parent_session: false,
                check_restart: false,
            })
        );
    }

    #[test]
    fn rtc_alarm_and_power_are_independent_expected_ext1_sources() {
        assert_eq!(
            decide_wake(WakeInput::Ext1 {
                rtc_pin: true,
                power_pin: false,
                boot_pin: false,
                alarm_pending: true,
            }),
            Ok(WakeDecision {
                refresh_daily: true,
                sample_battery: true,
                check_parent_session: false,
                check_restart: false,
            })
        );
        assert_eq!(
            decide_wake(WakeInput::Ext1 {
                rtc_pin: false,
                power_pin: true,
                boot_pin: false,
                alarm_pending: false,
            }),
            Ok(WakeDecision {
                refresh_daily: false,
                sample_battery: false,
                check_parent_session: true,
                check_restart: false,
            })
        );
    }

    #[test]
    fn simultaneous_alarm_refreshes_before_parent_work() {
        assert_eq!(
            decide_wake(WakeInput::Ext1 {
                rtc_pin: true,
                power_pin: true,
                boot_pin: false,
                alarm_pending: true,
            }),
            Ok(WakeDecision {
                refresh_daily: true,
                sample_battery: true,
                check_parent_session: true,
                check_restart: false,
            })
        );
    }

    #[test]
    fn deferred_parent_session_does_not_repeat_daily_battery_work() {
        assert_eq!(
            WakeDecision::parent_session_after_daily(),
            WakeDecision {
                refresh_daily: false,
                sample_battery: false,
                check_parent_session: true,
                check_restart: false,
            }
        );
    }

    #[test]
    fn alarm_battery_commit_requires_valid_rtc_and_wake_plan() {
        let decision = decide_wake(WakeInput::Ext1 {
            rtc_pin: true,
            power_pin: true,
            boot_pin: false,
            alarm_pending: true,
        })
        .unwrap();

        assert!(!decision.should_commit_battery(false, true));
        assert!(!decision.should_commit_battery(true, false));
        assert!(!decision.should_commit_battery(false, false));
        assert!(decision.should_commit_battery(true, true));
    }

    #[test]
    fn stale_rtc_pin_and_unknown_sources_are_rejected() {
        assert!(decide_wake(WakeInput::Other).is_err());
        assert!(
            decide_wake(WakeInput::Ext1 {
                rtc_pin: true,
                power_pin: false,
                boot_pin: false,
                alarm_pending: false,
            })
            .is_err()
        );
    }

    #[test]
    fn boot_alone_requests_a_restart_check() {
        assert_eq!(
            decide_wake(WakeInput::Ext1 {
                rtc_pin: false,
                power_pin: false,
                boot_pin: true,
                alarm_pending: false,
            }),
            Ok(WakeDecision {
                refresh_daily: false,
                sample_battery: true,
                check_parent_session: false,
                check_restart: true,
            })
        );
    }

    #[test]
    fn alarm_or_parent_work_supersedes_a_boot_restart() {
        for (rtc_pin, power_pin, alarm_pending) in [(true, false, true), (false, true, false)] {
            let decision = decide_wake(WakeInput::Ext1 {
                rtc_pin,
                power_pin,
                boot_pin: true,
                alarm_pending,
            })
            .unwrap();
            assert!(!decision.check_restart);
        }
    }

    const fn lines(rtc_alarm: bool, power_button: bool, boot_button: bool) -> SleepWakeSources {
        SleepWakeSources {
            rtc_alarm,
            power_button,
            boot_button,
        }
    }

    #[test]
    fn every_requested_released_line_is_armed() {
        for requested in [
            lines(true, true, true),
            lines(false, true, true),
            lines(false, false, true),
        ] {
            assert_eq!(
                select_sleep_wake_sources(requested, lines(false, false, false)),
                requested
            );
        }
    }

    #[test]
    fn each_stuck_line_is_dropped_independently() {
        let all = lines(true, true, true);
        for rtc_low in [false, true] {
            for power_low in [false, true] {
                for boot_low in [false, true] {
                    assert_eq!(
                        select_sleep_wake_sources(all, lines(rtc_low, power_low, boot_low)),
                        lines(!rtc_low, !power_low, !boot_low)
                    );
                }
            }
        }
    }

    #[test]
    fn unrequested_lines_are_never_armed() {
        assert_eq!(
            select_sleep_wake_sources(lines(false, false, true), lines(false, false, false)),
            lines(false, false, true)
        );
        assert_eq!(
            select_sleep_wake_sources(lines(false, false, true), lines(false, false, true)),
            lines(false, false, false)
        );
    }

    #[test]
    fn timer_wake_samples_the_battery_without_daily_or_button_work() {
        assert_eq!(
            decide_wake(WakeInput::Timer),
            Ok(WakeDecision {
                refresh_daily: false,
                sample_battery: true,
                check_parent_session: false,
                check_restart: false,
            })
        );
    }

    #[test]
    fn timer_check_redraws_only_when_the_battery_state_changes() {
        let timer = decide_wake(WakeInput::Timer).unwrap();
        for state in [
            BatteryState::Normal,
            BatteryState::Recharge,
            BatteryState::Unavailable,
        ] {
            assert!(!timer.refresh_required(state, state));
        }
        assert!(timer.refresh_required(BatteryState::Recharge, BatteryState::Normal));
        assert!(timer.refresh_required(BatteryState::Normal, BatteryState::Recharge));
        assert!(timer.refresh_required(BatteryState::Normal, BatteryState::Unavailable));
    }

    #[test]
    fn alarm_reset_and_restart_always_redraw() {
        let restart = decide_wake(WakeInput::Ext1 {
            rtc_pin: false,
            power_pin: false,
            boot_pin: true,
            alarm_pending: false,
        })
        .unwrap();
        let alarm = decide_wake(WakeInput::Ext1 {
            rtc_pin: true,
            power_pin: false,
            boot_pin: false,
            alarm_pending: true,
        })
        .unwrap();
        let reset = decide_wake(WakeInput::Reset).unwrap();
        for decision in [restart, alarm, reset] {
            assert!(decision.refresh_required(BatteryState::Normal, BatteryState::Normal));
        }
    }
}
