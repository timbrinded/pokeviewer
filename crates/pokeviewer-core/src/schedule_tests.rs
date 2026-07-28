extern crate std;

use std::collections::BTreeSet;
use time::{Date, Duration};

use super::{
    CYCLE_LENGTH, DailySelection, DisplayDate, InvalidDateTime, LocalDateTime, Weekday,
    next_rollover, select_daily_pokemon,
};
use crate::ContentPack;

const PACK: &[u8] = include_bytes!("../../../content/generated/pokeviewer-v1.pack");
const EPOCH_ROLLOVER: LocalDateTime = LocalDateTime {
    year: 2026,
    month: 1,
    day: 1,
    hour: 7,
    minute: 0,
    second: 0,
};

fn selection(local: LocalDateTime) -> DailySelection {
    select_daily_pokemon(local).unwrap()
}

fn at_noon(date: Date) -> LocalDateTime {
    LocalDateTime {
        year: u16::try_from(date.year()).unwrap(),
        month: date.month() as u8,
        day: date.day(),
        hour: 12,
        minute: 0,
        second: 0,
    }
}

#[test]
fn next_rollover_is_strict_and_crosses_calendar_boundaries() {
    for (now, expected) in [
        (
            LocalDateTime {
                year: 2026,
                month: 7,
                day: 27,
                hour: 6,
                minute: 59,
                second: 59,
            },
            LocalDateTime {
                year: 2026,
                month: 7,
                day: 27,
                hour: 7,
                minute: 0,
                second: 0,
            },
        ),
        (
            LocalDateTime {
                year: 2026,
                month: 12,
                day: 31,
                hour: 7,
                minute: 0,
                second: 0,
            },
            LocalDateTime {
                year: 2027,
                month: 1,
                day: 1,
                hour: 7,
                minute: 0,
                second: 0,
            },
        ),
        (
            LocalDateTime {
                year: 2024,
                month: 2,
                day: 28,
                hour: 23,
                minute: 59,
                second: 59,
            },
            LocalDateTime {
                year: 2024,
                month: 2,
                day: 29,
                hour: 7,
                minute: 0,
                second: 0,
            },
        ),
    ] {
        assert_eq!(next_rollover(now), Ok(expected));
    }
    assert_eq!(
        next_rollover(LocalDateTime {
            year: 2099,
            month: 12,
            day: 31,
            hour: 7,
            minute: 0,
            second: 0,
        }),
        Err(InvalidDateTime)
    );
}

#[test]
fn contract_vectors_are_exact() {
    let vectors = [
        (
            LocalDateTime {
                year: 2025,
                month: 12,
                day: 31,
                hour: 12,
                minute: 0,
                second: 0,
            },
            DisplayDate {
                year: 2025,
                month: 12,
                day: 31,
                weekday: Weekday::Wednesday,
            },
            150,
            79,
        ),
        (
            LocalDateTime {
                hour: 6,
                minute: 59,
                second: 59,
                ..EPOCH_ROLLOVER
            },
            DisplayDate {
                year: 2025,
                month: 12,
                day: 31,
                weekday: Weekday::Wednesday,
            },
            150,
            79,
        ),
        (
            EPOCH_ROLLOVER,
            DisplayDate {
                year: 2026,
                month: 1,
                day: 1,
                weekday: Weekday::Thursday,
            },
            0,
            1,
        ),
        (
            LocalDateTime {
                year: 2026,
                month: 6,
                day: 1,
                hour: 7,
                minute: 0,
                second: 0,
            },
            DisplayDate {
                year: 2026,
                month: 6,
                day: 1,
                weekday: Weekday::Monday,
            },
            0,
            1,
        ),
    ];

    for (local, display_date, cycle_index, dex_id) in vectors {
        assert_eq!(
            selection(local),
            DailySelection {
                display_date,
                cycle_index,
                dex_id
            }
        );
    }
}

#[test]
fn rollover_uses_one_atomic_display_date_across_boundaries() {
    let cases = [
        (
            LocalDateTime {
                year: 2024,
                month: 3,
                day: 1,
                hour: 6,
                minute: 59,
                second: 59,
            },
            (2024, 2, 29),
        ),
        (
            LocalDateTime {
                year: 2026,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            },
            (2025, 12, 31),
        ),
        (
            LocalDateTime {
                year: 2026,
                month: 2,
                day: 1,
                hour: 6,
                minute: 0,
                second: 0,
            },
            (2026, 1, 31),
        ),
    ];

    for (local, expected) in cases {
        let date = selection(local).display_date;
        assert_eq!((date.year, date.month, date.day), expected);
    }
}

#[test]
fn every_cycle_is_a_permutation_and_matches_the_committed_pack() {
    let pack = ContentPack::parse(PACK).unwrap();
    let epoch = EPOCH_ROLLOVER.to_primitive().unwrap().date();
    for cycle in -20..=20 {
        let start = epoch + Duration::days(cycle * CYCLE_LENGTH);
        let ids: BTreeSet<_> = (0..CYCLE_LENGTH)
            .map(|offset| {
                let selected = selection(at_noon(start + Duration::days(offset)));
                assert_eq!(
                    pack.scheduled_record(selected.cycle_index).unwrap().dex_id,
                    selected.dex_id
                );
                selected.dex_id
            })
            .collect();
        assert_eq!(ids.len(), usize::try_from(CYCLE_LENGTH).unwrap());
        assert_eq!(
            selection(at_noon(start)).dex_id,
            selection(at_noon(start + Duration::days(CYCLE_LENGTH))).dex_id
        );
    }
}

#[test]
fn rtc_range_extremes_and_large_offsets_are_exact() {
    let cases = [
        (
            LocalDateTime {
                year: 2000,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            },
            (1999, 12, 31, 15, 39),
        ),
        (
            LocalDateTime {
                year: 2099,
                month: 12,
                day: 31,
                hour: 23,
                minute: 59,
                second: 59,
            },
            (2099, 12, 31, 149, 6),
        ),
    ];

    for (local, expected) in cases {
        let selected = selection(local);
        assert_eq!(
            (
                selected.display_date.year,
                selected.display_date.month,
                selected.display_date.day,
                selected.cycle_index,
                selected.dex_id
            ),
            expected
        );
    }
}

#[test]
fn invalid_calendar_inputs_are_rejected() {
    for invalid in [
        LocalDateTime {
            year: 2025,
            month: 2,
            day: 29,
            ..EPOCH_ROLLOVER
        },
        LocalDateTime {
            hour: 24,
            ..EPOCH_ROLLOVER
        },
        LocalDateTime {
            year: 1999,
            ..EPOCH_ROLLOVER
        },
    ] {
        assert_eq!(select_daily_pokemon(invalid), Err(InvalidDateTime));
    }
}
