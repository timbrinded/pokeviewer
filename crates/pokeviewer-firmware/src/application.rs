//! Allocation-free composition of RTC, content, schedule, and rendering.

use pokeviewer_core::{
    ContentPack, DailyCard, DailySelection, Framebuffer, LocalDateTime, PackError, RecoveryState,
    RenderError, SetupReason, assess_rtc, render_daily_card, render_setup_screen,
};

const PACK: &[u8] = include_bytes!("../../../content/generated/pokeviewer-v1.pack");

/// The retained screen produced by one complete application pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    /// The scheduled daily card was rendered.
    Daily(DailySelection),
    /// Adult RTC setup instructions were rendered instead of a card.
    Setup(SetupReason),
}

/// Traceable result passed unchanged to the panel boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderedFrame {
    /// Semantic content of the frame.
    pub screen: Screen,
    /// CRC-32 of the exact 5,000 panel-native bytes.
    pub crc32: u32,
}

/// Bounded integration failures unrelated to RTC setup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationError {
    /// The committed offline pack is incompatible or corrupt.
    Content(PackError),
    /// The selected card violates the locked renderer contract.
    Render(RenderError),
    /// Pack schedule and scheduling-domain identifiers disagree.
    ScheduleMismatch,
}

/// Render one complete application frame from a fresh RTC assessment.
///
/// # Errors
///
/// Returns a bounded content or rendering failure. Invalid RTC readings render
/// the adult setup screen and are successful application outcomes.
pub fn render_rtc_frame(
    reading: Result<LocalDateTime, SetupReason>,
    framebuffer: &mut Framebuffer,
) -> Result<RenderedFrame, ApplicationError> {
    let screen = match assess_rtc(reading) {
        RecoveryState::SetupRequired(reason) => {
            render_setup_screen(framebuffer);
            Screen::Setup(reason)
        }
        RecoveryState::Ready(selection) => {
            let pack = ContentPack::parse(PACK).map_err(ApplicationError::Content)?;
            render_selection(&pack, selection, framebuffer)?;
            Screen::Daily(selection)
        }
    };
    Ok(RenderedFrame {
        screen,
        crc32: framebuffer.crc32(),
    })
}

fn render_selection(
    pack: &ContentPack<'_>,
    selection: DailySelection,
    framebuffer: &mut Framebuffer,
) -> Result<(), ApplicationError> {
    let record = pack
        .scheduled_record(selection.cycle_index)
        .map_err(ApplicationError::Content)?;
    if record.dex_id != selection.dex_id {
        return Err(ApplicationError::ScheduleMismatch);
    }
    render_daily_card(
        framebuffer,
        DailyCard {
            weekday: selection.display_date.weekday,
            name: record.name,
            primary_type: record.primary_type,
            secondary_type: record.secondary_type,
            sprite: record.sprite,
        },
    )
    .map_err(ApplicationError::Render)
}

#[cfg(test)]
mod tests {
    use pokeviewer_core::{
        ContentPack, DailySelection, DisplayDate, Framebuffer, LocalDateTime, SetupReason, Weekday,
    };

    use super::{PACK, Screen, render_rtc_frame, render_selection};

    const MONDAY_BULBASAUR: &[u8; 5_000] =
        include_bytes!("../../../tests/goldens/cards/monday-001.bin");
    const TUESDAY_CHARIZARD: &[u8; 5_000] =
        include_bytes!("../../../tests/goldens/cards/tuesday-006.bin");
    const WEDNESDAY_AERODACTYL: &[u8; 5_000] =
        include_bytes!("../../../tests/goldens/cards/wednesday-142.bin");
    const FRIDAY_MR_MIME: &[u8; 5_000] =
        include_bytes!("../../../tests/goldens/cards/friday-122.bin");
    const SATURDAY_PIKACHU: &[u8; 5_000] =
        include_bytes!("../../../tests/goldens/cards/saturday-025.bin");

    #[test]
    fn published_epoch_vector_renders_the_expected_complete_card() {
        let mut framebuffer = Framebuffer::default();
        let result = render_rtc_frame(
            Ok(LocalDateTime {
                year: 2026,
                month: 1,
                day: 1,
                hour: 7,
                minute: 0,
                second: 0,
            }),
            &mut framebuffer,
        )
        .unwrap();

        let Screen::Daily(selection) = result.screen else {
            panic!("valid RTC must render a daily card");
        };
        assert_eq!(selection.cycle_index, 0);
        assert_eq!(selection.dex_id, 1);
        assert_eq!(selection.display_date.weekday, Weekday::Thursday);
        assert_eq!(result.crc32, framebuffer.crc32());
    }

    #[test]
    fn representative_integrated_frames_match_reviewed_goldens() {
        let pack = ContentPack::parse(PACK).unwrap();
        for (cycle_index, dex_id, weekday, expected) in [
            (0, 1, Weekday::Monday, MONDAY_BULBASAUR),
            (149, 6, Weekday::Tuesday, TUESDAY_CHARIZARD),
            (4, 142, Weekday::Wednesday, WEDNESDAY_AERODACTYL),
            (12, 122, Weekday::Friday, FRIDAY_MR_MIME),
            (81, 25, Weekday::Saturday, SATURDAY_PIKACHU),
        ] {
            let mut framebuffer = Framebuffer::default();
            render_selection(
                &pack,
                selection(cycle_index, dex_id, weekday),
                &mut framebuffer,
            )
            .unwrap();
            assert_eq!(framebuffer.as_bytes(), expected);
        }
    }

    #[test]
    fn every_packed_entry_renders_through_the_integrated_path() {
        let pack = ContentPack::parse(PACK).unwrap();
        for cycle_index in 0..151 {
            let dex_id = u8::try_from((73 * cycle_index) % 151 + 1).unwrap();
            let mut framebuffer = Framebuffer::default();
            render_selection(
                &pack,
                selection(u8::try_from(cycle_index).unwrap(), dex_id, Weekday::Sunday),
                &mut framebuffer,
            )
            .unwrap();
        }
    }

    #[test]
    fn invalid_rtc_renders_setup_instead_of_a_card() {
        let mut framebuffer = Framebuffer::default();
        let result =
            render_rtc_frame(Err(SetupReason::OscillatorStopped), &mut framebuffer).unwrap();

        assert_eq!(result.screen, Screen::Setup(SetupReason::OscillatorStopped));
        assert_eq!(result.crc32, 0x063c_ff9d);
    }

    const fn selection(cycle_index: u8, dex_id: u8, weekday: Weekday) -> DailySelection {
        DailySelection {
            display_date: DisplayDate {
                year: 2026,
                month: 1,
                day: 1,
                weekday,
            },
            cycle_index,
            dex_id,
        }
    }
}
