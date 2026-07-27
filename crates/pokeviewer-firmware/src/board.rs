//! Boundary around the pinned ESP HAL and supported V2 board.

use embassy_futures::block_on;
use embedded_graphics::{
    Drawable,
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    prelude::{DrawTarget, Point, Primitive},
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use embedded_hal::{delay::DelayNs, digital::OutputPin, spi::SpiDevice};
use embedded_hal_bus::spi::ExclusiveDevice;
use epd_waveshare::{
    epd1in54_v2::{Display1in54, Epd1in54},
    prelude::{Color, WaveshareDisplay},
};
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig},
    i2c::master::{Config as I2cConfig, I2c},
    spi::{
        Mode,
        master::{Config, Spi},
    },
    time::Rate,
};

use crate::{
    LocalDateTime, Pcf85063Rtc, Rtc,
    bounded_busy::{BoundedBusy, BusyState},
};

const BUSY_POLL_US: u32 = 10_000;
const BUSY_MAX_POLLS: u32 = 500;
static PANEL_BUSY_STATE: BusyState = BusyState::new();

/// Successful RTC observations from the combined hardware diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardwareDiagnosticReport {
    /// RTC value read back after writing the same valid value.
    pub rtc_datetime: LocalDateTime,
    /// Whether an alarm was pending before it was cleared and reconfigured.
    pub alarm_was_pending: bool,
}

/// Run RTC and panel bring-up diagnostics while enforcing board power states.
pub fn run_hardware_diagnostics() -> Result<HardwareDiagnosticReport, &'static str> {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let _power_latch = Output::new(peripherals.GPIO17, Level::High, OutputConfig::default());
    let mut panel_power = Output::new(peripherals.GPIO6, Level::High, OutputConfig::default());
    let _audio_power = Output::new(peripherals.GPIO42, Level::High, OutputConfig::default());

    let i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(100)),
    )
    .map_err(|_| "invalid I2C configuration")?
    .with_sda(peripherals.GPIO47)
    .with_scl(peripherals.GPIO48)
    .into_async();
    let rtc_result = run_rtc_diagnostics(Pcf85063Rtc::new(i2c));

    panel_power.set_low();
    let panel_result = (|| {
        let busy = BoundedBusy::new(
            Input::new(peripherals.GPIO8, InputConfig::default()),
            BUSY_MAX_POLLS,
            &PANEL_BUSY_STATE,
        );
        let dc = Output::new(peripherals.GPIO10, Level::Low, OutputConfig::default());
        let reset_pin = Output::new(peripherals.GPIO9, Level::High, OutputConfig::default());
        let spi = Spi::new(
            peripherals.SPI2,
            Config::default()
                .with_frequency(Rate::from_mhz(10))
                .with_mode(Mode::_0),
        )
        .map_err(|_| "invalid SPI configuration")?
        .with_sck(peripherals.GPIO12)
        .with_mosi(peripherals.GPIO13);
        let cs = Output::new(peripherals.GPIO11, Level::High, OutputConfig::default());
        let mut spi =
            ExclusiveDevice::new_no_delay(spi, cs).map_err(|_| "SPI device setup failed")?;
        let mut delay = Delay::new();
        let mut panel = Epd1in54::new(
            &mut spi,
            busy,
            dc,
            reset_pin,
            &mut delay,
            Some(BUSY_POLL_US),
        )
        .map_err(|_| "panel initialization failed")?;
        check_busy()?;

        let mut frame = Display1in54::default();
        frame
            .clear(Color::White)
            .map_err(|_| "white frame failed")?;
        refresh(&mut panel, &mut spi, &mut delay, &frame)?;
        delay.delay_ms(2_000);

        frame
            .clear(Color::Black)
            .map_err(|_| "black frame failed")?;
        refresh(&mut panel, &mut spi, &mut delay, &frame)?;
        delay.delay_ms(2_000);

        draw_checkerboard(&mut frame)?;
        refresh(&mut panel, &mut spi, &mut delay, &frame)?;
        delay.delay_ms(2_000);

        draw_border(&mut frame)?;
        refresh(&mut panel, &mut spi, &mut delay, &frame)?;
        delay.delay_ms(2_000);

        draw_text(&mut frame)?;
        refresh(&mut panel, &mut spi, &mut delay, &frame)?;

        wait_until_idle(&mut panel, &mut spi, &mut delay)?;
        PANEL_BUSY_STATE.reset();
        panel
            .sleep(&mut spi, &mut delay)
            .map_err(|_| "panel sleep failed")?;
        check_busy()
    })();
    panel_power.set_high();
    panel_result?;
    rtc_result
}

fn run_rtc_diagnostics<I2cBus>(
    mut rtc: Pcf85063Rtc<I2cBus>,
) -> Result<HardwareDiagnosticReport, &'static str>
where
    I2cBus: embedded_hal_async::i2c::I2c,
{
    block_on(async {
        let datetime = rtc
            .read_datetime()
            .await
            .map_err(|_| "RTC read or oscillator validation failed")?;
        rtc.set_datetime(datetime)
            .await
            .map_err(|_| "RTC write failed")?;
        let readback = rtc
            .read_datetime()
            .await
            .map_err(|_| "RTC readback failed")?;
        if readback != datetime {
            return Err("RTC set/read mismatch");
        }

        let alarm_was_pending = rtc
            .alarm_pending()
            .await
            .map_err(|_| "RTC alarm flag read failed")?;
        rtc.clear_alarm()
            .await
            .map_err(|_| "RTC alarm flag clear failed")?;
        if rtc
            .alarm_pending()
            .await
            .map_err(|_| "RTC cleared flag readback failed")?
        {
            return Err("RTC alarm flag remained asserted");
        }
        rtc.configure_daily_alarm()
            .await
            .map_err(|_| "RTC alarm configuration failed")?;

        Ok(HardwareDiagnosticReport {
            rtc_datetime: readback,
            alarm_was_pending,
        })
    })
}

fn refresh<SpiDeviceType, BusyPin, DcPin, ResetPin, DelayType>(
    panel: &mut Epd1in54<SpiDeviceType, BoundedBusy<'static, BusyPin>, DcPin, ResetPin, DelayType>,
    spi: &mut SpiDeviceType,
    delay: &mut DelayType,
    frame: &Display1in54,
) -> Result<(), &'static str>
where
    SpiDeviceType: SpiDevice,
    BusyPin: embedded_hal::digital::InputPin,
    DcPin: OutputPin,
    ResetPin: OutputPin,
    DelayType: DelayNs,
{
    wait_until_idle(panel, spi, delay)?;
    PANEL_BUSY_STATE.reset();
    panel
        .update_frame(spi, frame.buffer(), delay)
        .map_err(|_| "panel refresh failed")?;
    check_busy()?;
    PANEL_BUSY_STATE.reset();
    panel
        .display_frame(spi, delay)
        .map_err(|_| "panel refresh failed")?;
    check_busy()?;
    wait_until_idle(panel, spi, delay)
}

fn wait_until_idle<SpiDeviceType, BusyPin, DcPin, ResetPin, DelayType>(
    panel: &mut Epd1in54<SpiDeviceType, BoundedBusy<'static, BusyPin>, DcPin, ResetPin, DelayType>,
    spi: &mut SpiDeviceType,
    delay: &mut DelayType,
) -> Result<(), &'static str>
where
    SpiDeviceType: SpiDevice,
    BusyPin: embedded_hal::digital::InputPin,
    DcPin: OutputPin,
    ResetPin: OutputPin,
    DelayType: DelayNs,
{
    PANEL_BUSY_STATE.reset();
    panel
        .wait_until_idle(spi, delay)
        .map_err(|_| "panel BUSY wait failed")?;
    check_busy()
}

fn check_busy() -> Result<(), &'static str> {
    if PANEL_BUSY_STATE.timed_out() {
        Err("panel BUSY timeout")
    } else {
        Ok(())
    }
}

fn draw_checkerboard(frame: &mut Display1in54) -> Result<(), &'static str> {
    frame
        .clear(Color::White)
        .map_err(|_| "checkerboard clear failed")?;
    for y in 0..10 {
        for x in 0..10 {
            if (x + y) % 2 == 0 {
                Rectangle::new(Point::new(x * 20, y * 20), (20, 20).into())
                    .into_styled(PrimitiveStyle::with_fill(Color::Black))
                    .draw(frame)
                    .map_err(|_| "checkerboard draw failed")?;
            }
        }
    }
    Ok(())
}

fn draw_border(frame: &mut Display1in54) -> Result<(), &'static str> {
    frame
        .clear(Color::White)
        .map_err(|_| "border clear failed")?;
    Rectangle::new(Point::zero(), (200, 200).into())
        .into_styled(PrimitiveStyle::with_stroke(Color::Black, 2))
        .draw(frame)
        .map_err(|_| "border draw failed")
}

fn draw_text(frame: &mut Display1in54) -> Result<(), &'static str> {
    frame.clear(Color::White).map_err(|_| "text clear failed")?;
    let style = MonoTextStyle::new(&FONT_6X10, Color::Black);
    Text::new("POKEVIEWER\n1.54 V2\n200 x 200", Point::new(48, 80), style)
        .draw(frame)
        .map(|_| ())
        .map_err(|_| "text draw failed")
}
