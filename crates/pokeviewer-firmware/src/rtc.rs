//! RTC contract shared by hardware and deterministic tests.

use time::{Date, Month, PrimitiveDateTime, Time};

/// Day of the week derived from a validated local date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weekday {
    /// Monday.
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
    /// Sunday.
    Sunday,
}

/// Local civil date and time stored by the onboard RTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalDateTime {
    /// Full year in the supported 2000–2099 range.
    pub year: u16,
    /// Calendar month in the range 1–12.
    pub month: u8,
    /// Calendar day in the range 1–31.
    pub day: u8,
    /// Hour in the range 0–23.
    pub hour: u8,
    /// Minute in the range 0–59.
    pub minute: u8,
    /// Second in the range 0–59.
    pub second: u8,
}

impl LocalDateTime {
    /// Validate the complete calendar value and the RTC's supported year range.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidDateTime`] if any field is outside its calendar range.
    pub fn validate(self) -> Result<Self, InvalidDateTime> {
        self.to_primitive().map(|_| self)
    }

    /// Derive the weekday without storing redundant RTC state.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidDateTime`] if the complete datetime is not valid.
    pub fn weekday(self) -> Result<Weekday, InvalidDateTime> {
        self.to_primitive().map(|value| match value.weekday() {
            time::Weekday::Monday => Weekday::Monday,
            time::Weekday::Tuesday => Weekday::Tuesday,
            time::Weekday::Wednesday => Weekday::Wednesday,
            time::Weekday::Thursday => Weekday::Thursday,
            time::Weekday::Friday => Weekday::Friday,
            time::Weekday::Saturday => Weekday::Saturday,
            time::Weekday::Sunday => Weekday::Sunday,
        })
    }

    pub(crate) fn to_primitive(self) -> Result<PrimitiveDateTime, InvalidDateTime> {
        if !(2000..=2099).contains(&self.year) {
            return Err(InvalidDateTime);
        }

        let month = Month::try_from(self.month).map_err(|_| InvalidDateTime)?;
        let date = Date::from_calendar_date(i32::from(self.year), month, self.day)
            .map_err(|_| InvalidDateTime)?;
        let time =
            Time::from_hms(self.hour, self.minute, self.second).map_err(|_| InvalidDateTime)?;
        Ok(PrimitiveDateTime::new(date, time))
    }
}

/// A local datetime is outside the RTC's supported calendar range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidDateTime;

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
