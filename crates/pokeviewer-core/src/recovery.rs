//! Invalid-RTC gate that prevents plausible but incorrect cards.

use crate::{DailySelection, LocalDateTime, select_daily_pokemon};

/// Sanitized reason that adult setup is required.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupReason {
    /// RTC oscillator-stop or power-loss flag was asserted.
    OscillatorStopped,
    /// RTC transport or register read failed.
    ReadFailure,
    /// Calendar or BCD fields were impossible.
    InvalidCalendar,
    /// Year is outside the supported 2000–2099 scheduling range.
    OutsideScheduleRange,
}

/// Complete result of assessing one fresh RTC reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryState {
    /// No card may be selected until wired provisioning succeeds.
    SetupRequired(SetupReason),
    /// A newly validated reading produced a coherent display-day selection.
    Ready(DailySelection),
}

/// Assess one fresh RTC result without retaining or inventing a fallback date.
#[must_use]
pub fn assess_rtc(reading: Result<LocalDateTime, SetupReason>) -> RecoveryState {
    let datetime = match reading {
        Ok(datetime) => datetime,
        Err(reason) => return RecoveryState::SetupRequired(reason),
    };
    if !(2000..=2099).contains(&datetime.year) {
        return RecoveryState::SetupRequired(SetupReason::OutsideScheduleRange);
    }
    match select_daily_pokemon(datetime) {
        Ok(selection) => RecoveryState::Ready(selection),
        Err(_) => RecoveryState::SetupRequired(SetupReason::InvalidCalendar),
    }
}

#[cfg(test)]
mod tests {
    use crate::{LocalDateTime, RecoveryState, SetupReason, assess_rtc};

    const NOW: LocalDateTime = LocalDateTime {
        year: 2026,
        month: 7,
        day: 27,
        hour: 7,
        minute: 0,
        second: 0,
    };

    #[test]
    fn every_invalid_source_requires_setup_without_a_selection() {
        for reading in [
            Err(SetupReason::OscillatorStopped),
            Err(SetupReason::ReadFailure),
            Ok(LocalDateTime {
                year: 2025,
                month: 2,
                day: 29,
                ..NOW
            }),
            Ok(LocalDateTime { year: 2100, ..NOW }),
        ] {
            assert!(matches!(
                assess_rtc(reading),
                RecoveryState::SetupRequired(_)
            ));
        }
    }

    #[test]
    fn fresh_valid_readback_recovers_with_the_correct_pre_rollover_day() {
        let recovered = assess_rtc(Ok(LocalDateTime {
            year: 2026,
            month: 1,
            day: 1,
            hour: 6,
            minute: 59,
            second: 59,
        }));
        let RecoveryState::Ready(selection) = recovered else {
            panic!("valid readback must leave setup mode");
        };
        assert_eq!(
            (
                selection.display_date.year,
                selection.display_date.month,
                selection.display_date.day,
                selection.cycle_index,
                selection.dex_id
            ),
            (2025, 12, 31, 150, 79)
        );
    }

    #[test]
    fn valid_boot_bypasses_setup() {
        assert!(matches!(assess_rtc(Ok(NOW)), RecoveryState::Ready(_)));
    }
}
