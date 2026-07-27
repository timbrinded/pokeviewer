//! Deterministic local-calendar and daily-selection rules.

use time::{Date, Month, PrimitiveDateTime, Time};

const EPOCH: Date = match Date::from_ordinal_date(2026, 1) {
    Ok(date) => date,
    Err(_) => panic!("the fixed schedule epoch must be valid"),
};
const CYCLE_LENGTH: i64 = 151;
const ROLLOVER_HOUR: u8 = 7;

/// Version of the repository-owned daily schedule.
pub const SCHEDULE_VERSION: u16 = 1;

/// Day of the week derived from a validated display date.
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

impl From<time::Weekday> for Weekday {
    fn from(value: time::Weekday) -> Self {
        match value {
            time::Weekday::Monday => Self::Monday,
            time::Weekday::Tuesday => Self::Tuesday,
            time::Weekday::Wednesday => Self::Wednesday,
            time::Weekday::Thursday => Self::Thursday,
            time::Weekday::Friday => Self::Friday,
            time::Weekday::Saturday => Self::Saturday,
            time::Weekday::Sunday => Self::Sunday,
        }
    }
}

/// Local civil date and time stored by the onboard RTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalDateTime {
    /// Full year in the supported 2000–2099 RTC range.
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

    /// Derive the weekday of the RTC calendar date.
    ///
    /// This is distinct from the display weekday before the 07:00 rollover.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidDateTime`] if the complete datetime is not valid.
    pub fn weekday(self) -> Result<Weekday, InvalidDateTime> {
        self.to_primitive()
            .map(|value| Weekday::from(value.weekday()))
    }

    /// Convert to the pinned allocation-free calendar representation.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidDateTime`] if the complete datetime is not valid.
    pub fn to_primitive(self) -> Result<PrimitiveDateTime, InvalidDateTime> {
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

/// Calendar date whose weekday and Pokémon belong together on the retained card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayDate {
    /// Proleptic Gregorian year.
    pub year: i32,
    /// Calendar month in the range 1–12.
    pub month: u8,
    /// Calendar day in the range 1–31.
    pub day: u8,
    /// Weekday derived from this display date.
    pub weekday: Weekday,
}

impl From<Date> for DisplayDate {
    fn from(value: Date) -> Self {
        Self {
            year: value.year(),
            month: value.month() as u8,
            day: value.day(),
            weekday: value.weekday().into(),
        }
    }
}

/// Complete schedule-v1 result for one valid RTC reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DailySelection {
    /// Date and weekday that must be rendered together.
    pub display_date: DisplayDate,
    /// Zero-based position in the 151-day schedule.
    pub cycle_index: u8,
    /// National Pokédex ID selected by schedule v1.
    pub dex_id: u8,
}

/// Select the deterministic schedule-v1 entry for a local RTC reading.
///
/// Times before 07:00:00 retain the prior display date. The calculation uses
/// Euclidean modulo, so every valid RTC date before and after the epoch is
/// defined without underflow.
///
/// # Errors
///
/// Returns [`InvalidDateTime`] for any invalid field or year outside 2000–2099.
pub fn select_daily_pokemon(local: LocalDateTime) -> Result<DailySelection, InvalidDateTime> {
    let local = local.to_primitive()?;
    let mut display_date = local.date();
    if local.hour() < ROLLOVER_HOUR {
        display_date = display_date.previous_day().ok_or(InvalidDateTime)?;
    }

    let epoch_offset = (display_date - EPOCH).whole_days();
    let cycle_index =
        u8::try_from(epoch_offset.rem_euclid(CYCLE_LENGTH)).map_err(|_| InvalidDateTime)?;
    let dex_id = u8::try_from((73 * i64::from(cycle_index)) % CYCLE_LENGTH + 1)
        .map_err(|_| InvalidDateTime)?;

    Ok(DailySelection {
        display_date: display_date.into(),
        cycle_index,
        dex_id,
    })
}

#[cfg(test)]
#[path = "schedule_tests.rs"]
mod tests;
