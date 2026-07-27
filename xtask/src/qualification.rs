//! Deterministic seven-day physical-run schedule generation.

use std::{fmt::Write as _, fs};

use pokeviewer_core::{ContentPack, LocalDateTime, Weekday, next_rollover, select_daily_pokemon};

use crate::render::render_record;

const PACK: &[u8] = include_bytes!("../../content/generated/pokeviewer-v1.pack");

pub(crate) fn schedule_command(start: &str, output: Option<&str>) -> Result<(), String> {
    let schedule = build_schedule(parse_date(start)?)?;
    match output {
        Some(path) => {
            fs::write(path, schedule)
                .map_err(|error| format!("failed to write {path}: {error}"))?;
        }
        None => print!("{schedule}"),
    }
    Ok(())
}

fn build_schedule(mut datetime: LocalDateTime) -> Result<String, String> {
    let pack = ContentPack::parse(PACK).map_err(|error| format!("invalid pack: {error:?}"))?;
    let mut output = String::from("day,date,weekday,dex_id,name,framebuffer_crc32,status\n");
    for day in 1..=7 {
        let selection = select_daily_pokemon(datetime)
            .map_err(|_| "qualification date is outside the schedule range")?;
        let record = pack
            .scheduled_record(selection.cycle_index)
            .map_err(|error| format!("invalid scheduled record: {error:?}"))?;
        let framebuffer = render_record(&pack, record.dex_id, selection.display_date.weekday)?;
        writeln!(
            output,
            "{day},{:04}-{:02}-{:02},{},{},{},{:08x},PENDING",
            selection.display_date.year,
            selection.display_date.month,
            selection.display_date.day,
            weekday_label(selection.display_date.weekday),
            record.dex_id,
            record.name,
            framebuffer.crc32(),
        )
        .map_err(|_| "failed to format qualification schedule")?;
        datetime = next_rollover(datetime)
            .map_err(|_| "seven-day schedule crosses the supported RTC range")?;
    }
    Ok(output)
}

fn parse_date(value: &str) -> Result<LocalDateTime, String> {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err("start date must use YYYY-MM-DD".to_owned());
    }
    let datetime = LocalDateTime {
        year: decimal(&bytes[0..4])?,
        month: u8::try_from(decimal(&bytes[5..7])?).map_err(|_| "invalid start date")?,
        day: u8::try_from(decimal(&bytes[8..10])?).map_err(|_| "invalid start date")?,
        hour: 7,
        minute: 0,
        second: 0,
    };
    datetime
        .validate()
        .map_err(|_| "start date is outside the RTC calendar range".to_owned())
}

fn decimal(bytes: &[u8]) -> Result<u16, String> {
    bytes.iter().try_fold(0_u16, |value, byte| {
        if byte.is_ascii_digit() {
            Ok(value * 10 + u16::from(byte - b'0'))
        } else {
            Err("start date contains a non-digit".to_owned())
        }
    })
}

const fn weekday_label(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "Monday",
        Weekday::Tuesday => "Tuesday",
        Weekday::Wednesday => "Wednesday",
        Weekday::Thursday => "Thursday",
        Weekday::Friday => "Friday",
        Weekday::Saturday => "Saturday",
        Weekday::Sunday => "Sunday",
    }
}

#[cfg(test)]
mod tests {
    use super::{build_schedule, parse_date};

    #[test]
    fn seven_day_schedule_is_complete_and_deterministic() {
        let schedule = build_schedule(parse_date("2026-01-01").unwrap()).unwrap();
        assert_eq!(schedule.lines().count(), 8);
        assert!(schedule.contains("1,2026-01-01,Thursday,1,Bulbasaur,"));
        assert!(schedule.contains("7,2026-01-07,Wednesday,"));
        assert_eq!(schedule.matches(",PENDING").count(), 7);
    }

    #[test]
    fn invalid_start_dates_are_rejected() {
        assert!(parse_date("2025-02-29").is_err());
        assert!(parse_date("2026/01/01").is_err());
    }
}
