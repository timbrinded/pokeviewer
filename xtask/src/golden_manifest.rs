//! Reviewed golden-case definitions and serialized manifest shape.

use pokeviewer_core::Weekday;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
pub(super) struct GoldenSpec {
    pub(super) slug: &'static str,
    pub(super) dex_id: u8,
    pub(super) weekday: Weekday,
}

pub(super) const CASES: [GoldenSpec; 7] = [
    GoldenSpec {
        slug: "monday-001",
        dex_id: 1,
        weekday: Weekday::Monday,
    },
    GoldenSpec {
        slug: "tuesday-006",
        dex_id: 6,
        weekday: Weekday::Tuesday,
    },
    GoldenSpec {
        slug: "wednesday-142",
        dex_id: 142,
        weekday: Weekday::Wednesday,
    },
    GoldenSpec {
        slug: "thursday-029",
        dex_id: 29,
        weekday: Weekday::Thursday,
    },
    GoldenSpec {
        slug: "friday-122",
        dex_id: 122,
        weekday: Weekday::Friday,
    },
    GoldenSpec {
        slug: "saturday-025",
        dex_id: 25,
        weekday: Weekday::Saturday,
    },
    GoldenSpec {
        slug: "sunday-151",
        dex_id: 151,
        weekday: Weekday::Sunday,
    },
];

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct GoldenManifest {
    pub(super) schema_version: u8,
    pub(super) renderer_version: u8,
    pub(super) cases: Vec<GoldenCase>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub(super) struct GoldenCase {
    pub(super) case: String,
    pub(super) dex_id: u8,
    pub(super) name: String,
    pub(super) weekday: String,
    pub(super) framebuffer_file: String,
    pub(super) png_file: String,
    pub(super) framebuffer_crc32: String,
    pub(super) framebuffer_sha256: String,
    pub(super) png_sha256: String,
}

pub(super) const fn weekday_label(weekday: Weekday) -> &'static str {
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
