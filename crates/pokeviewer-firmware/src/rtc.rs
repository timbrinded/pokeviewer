//! RTC contract shared by hardware and deterministic tests.

#[cfg(test)]
use pokeviewer_core::Weekday;
use pokeviewer_core::{InvalidDateTime, LocalDateTime};

/// Project-owned real-time clock operations.
#[allow(async_fn_in_trait)]
pub trait Rtc {
    /// Driver-specific error.
    type Error;

    /// Read a trustworthy local datetime.
    async fn read_datetime(&mut self) -> Result<LocalDateTime, Self::Error>;

    /// Set the complete local datetime.
    async fn set_datetime(&mut self, datetime: LocalDateTime) -> Result<(), Self::Error>;

    /// Configure and enable the fixed daily 07:00:00 alarm.
    async fn configure_daily_alarm(&mut self) -> Result<(), Self::Error>;

    /// Report whether the RTC alarm flag is asserted.
    async fn alarm_pending(&mut self) -> Result<bool, Self::Error>;

    /// Clear the RTC alarm flag.
    async fn clear_alarm(&mut self) -> Result<(), Self::Error>;
}

/// Deterministic RTC error used by host tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FakeRtcError {
    /// The simulated oscillator has stopped.
    OscillatorStopped,
    /// A supplied datetime is outside the supported calendar range.
    InvalidDateTime,
}

/// In-memory RTC for state-machine and schedule tests.
pub struct FakeRtc {
    now: LocalDateTime,
    oscillator_valid: bool,
    alarm_pending: bool,
    alarm_configured: bool,
}

impl FakeRtc {
    /// Create a valid fake RTC at the supplied local time.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidDateTime`] if `now` is outside the supported calendar.
    pub fn new(now: LocalDateTime) -> Result<Self, InvalidDateTime> {
        now.validate()?;
        Ok(Self {
            now,
            oscillator_valid: true,
            alarm_pending: false,
            alarm_configured: false,
        })
    }

    /// Simulate oscillator-stop invalidity.
    pub fn stop_oscillator(&mut self) {
        self.oscillator_valid = false;
    }

    /// Simulate the daily alarm firing.
    pub fn trigger_alarm(&mut self) {
        self.alarm_pending = true;
    }

    /// Report whether the fixed alarm has been configured.
    #[must_use]
    pub const fn alarm_configured(&self) -> bool {
        self.alarm_configured
    }
}

impl Rtc for FakeRtc {
    type Error = FakeRtcError;

    async fn read_datetime(&mut self) -> Result<LocalDateTime, Self::Error> {
        if self.oscillator_valid {
            Ok(self.now)
        } else {
            Err(FakeRtcError::OscillatorStopped)
        }
    }

    async fn set_datetime(&mut self, datetime: LocalDateTime) -> Result<(), Self::Error> {
        datetime
            .validate()
            .map_err(|_| FakeRtcError::InvalidDateTime)?;
        self.now = datetime;
        self.oscillator_valid = true;
        Ok(())
    }

    async fn configure_daily_alarm(&mut self) -> Result<(), Self::Error> {
        self.alarm_configured = true;
        self.alarm_pending = false;
        Ok(())
    }

    async fn alarm_pending(&mut self) -> Result<bool, Self::Error> {
        Ok(self.alarm_pending)
    }

    async fn clear_alarm(&mut self) -> Result<(), Self::Error> {
        self.alarm_pending = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };

    use super::{FakeRtc, FakeRtcError, LocalDateTime, Rtc, Weekday};

    fn block_on_ready<Output>(future: impl Future<Output = Output>) -> Output {
        let mut context = Context::from_waker(Waker::noop());
        let mut future = pin!(future);
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("fake RTC unexpectedly yielded"),
        }
    }

    const NOW: LocalDateTime = LocalDateTime {
        year: 2026,
        month: 7,
        day: 27,
        hour: 7,
        minute: 0,
        second: 0,
    };

    #[test]
    fn oscillator_stop_is_not_silently_trusted() {
        let mut rtc = FakeRtc::new(NOW).unwrap();
        rtc.stop_oscillator();

        assert_eq!(
            block_on_ready(rtc.read_datetime()),
            Err(FakeRtcError::OscillatorStopped)
        );
    }

    #[test]
    fn alarm_can_be_configured_triggered_and_cleared() {
        let mut rtc = FakeRtc::new(NOW).unwrap();

        block_on_ready(rtc.configure_daily_alarm()).unwrap();
        assert!(rtc.alarm_configured());
        rtc.trigger_alarm();
        assert!(block_on_ready(rtc.alarm_pending()).unwrap());
        block_on_ready(rtc.clear_alarm()).unwrap();
        assert!(!block_on_ready(rtc.alarm_pending()).unwrap());
    }

    #[test]
    fn calendar_validation_handles_leap_days_and_boundaries() {
        for valid in [
            LocalDateTime {
                year: 2000,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            },
            LocalDateTime {
                year: 2024,
                month: 2,
                day: 29,
                hour: 23,
                minute: 59,
                second: 59,
            },
            LocalDateTime {
                year: 2099,
                month: 12,
                day: 31,
                hour: 23,
                minute: 59,
                second: 59,
            },
        ] {
            assert_eq!(valid.validate(), Ok(valid));
        }

        for invalid in [
            LocalDateTime { year: 1999, ..NOW },
            LocalDateTime { year: 2100, ..NOW },
            LocalDateTime {
                year: 2025,
                month: 2,
                day: 29,
                ..NOW
            },
            LocalDateTime { hour: 24, ..NOW },
            LocalDateTime { minute: 60, ..NOW },
            LocalDateTime { second: 60, ..NOW },
        ] {
            assert_eq!(invalid.validate(), Err(super::InvalidDateTime));
        }
    }

    #[test]
    fn weekday_is_derived_from_the_complete_date() {
        assert_eq!(NOW.weekday(), Ok(Weekday::Monday));
        assert_eq!(
            LocalDateTime {
                year: 2024,
                month: 2,
                day: 29,
                ..NOW
            }
            .weekday(),
            Ok(Weekday::Thursday)
        );
    }

    #[test]
    fn fake_rejects_invalid_datetime_without_changing_state() {
        let mut rtc = FakeRtc::new(NOW).unwrap();
        let invalid = LocalDateTime {
            year: 2025,
            month: 2,
            day: 29,
            ..NOW
        };

        assert_eq!(
            block_on_ready(rtc.set_datetime(invalid)),
            Err(FakeRtcError::InvalidDateTime)
        );
        assert_eq!(block_on_ready(rtc.read_datetime()), Ok(NOW));
    }

    #[test]
    fn fake_rejects_an_invalid_initial_datetime() {
        let invalid = LocalDateTime {
            year: 2025,
            month: 2,
            day: 29,
            ..NOW
        };

        assert!(FakeRtc::new(invalid).is_err());
    }
}
