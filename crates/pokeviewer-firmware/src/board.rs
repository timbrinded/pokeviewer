//! Boundary around the pinned ESP HAL and supported V2 board.

use embassy_futures::block_on;
use embedded_hal::delay::DelayNs;
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull, RtcPin},
    i2c::master::{Config as I2cConfig, I2c},
    peripherals::{GPIO5, GPIO6, GPIO17, GPIO42, LPWR},
    rtc_cntl::{
        Rtc as HalRtc,
        sleep::{Ext0WakeupSource, WakeupLevel},
        wakeup_cause,
    },
    system::SleepSource,
    time::Rate,
};

use crate::{
    LocalDateTime, Pcf85063Rtc, Pcf85063RtcError, Rtc, Screen,
    panel::{PanelDiagnostic, refresh_panel_frame, run_panel_diagnostics},
    usb_protocol::UsbProtocolTransport,
};
use pokeviewer_core::{Framebuffer, RecoveryState, SetupReason, assess_rtc};

struct SleepResources {
    rtc_interrupt: GPIO5<'static>,
    panel_power: GPIO6<'static>,
    power_latch: GPIO17<'static>,
    audio_power: GPIO42<'static>,
    low_power: LPWR<'static>,
}

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
    run_board_diagnostics(PanelDiagnostic::Full, true).map(|(report, _)| report)
}

/// Render one complete offline application frame on the supported V2 board.
///
/// A valid RTC produces a daily card. An invalid RTC produces the retained
/// setup screen and starts bounded wired provisioning; a successful set and
/// read-back restarts into the normal boot path.
pub fn run_pokeviewer() -> ! {
    match run_application_once() {
        Ok(ApplicationRun::Daily { crc32 }) => {
            esp_println::println!("daily card retained; framebuffer_crc32={crc32:08x}");
            loop {
                core::hint::spin_loop();
            }
        }
        Ok(ApplicationRun::Setup {
            crc32,
            mut rtc,
            usb_device,
        }) => {
            esp_println::println!("RTC setup required; framebuffer_crc32={crc32:08x}");
            serve_setup(&mut rtc, usb_device);
        }
        Err(error) => {
            esp_println::println!("application failed: {error}");
            loop {
                core::hint::spin_loop();
            }
        }
    }
}

/// Refresh one frame, validate RTC state, and enter active-low RTC deep sleep.
pub fn run_sleep_diagnostic() -> ! {
    let cause = wakeup_cause();
    let configure_alarm = !matches!(cause, SleepSource::Ext0);
    match run_board_diagnostics(PanelDiagnostic::SingleFrame, configure_alarm) {
        Ok((report, resources)) => {
            if matches!(cause, SleepSource::Ext0) && !report.alarm_was_pending {
                esp_println::println!(
                    "sleep diagnostic failed: RTC wake had no asserted alarm flag"
                );
                loop {
                    core::hint::spin_loop();
                }
            }
            esp_println::println!(
                "sleep diagnostic ready; wake_cause={cause:?}; RTC={:04}-{:02}-{:02} {:02}:{:02}:{:02}; alarm_was_pending={}; rails_off=true",
                report.rtc_datetime.year,
                report.rtc_datetime.month,
                report.rtc_datetime.day,
                report.rtc_datetime.hour,
                report.rtc_datetime.minute,
                report.rtc_datetime.second,
                report.alarm_was_pending,
            );
            resources.sleep();
        }
        Err(error) => {
            esp_println::println!("sleep diagnostic failed: {error}");
            loop {
                core::hint::spin_loop();
            }
        }
    }
}

/// Run the bounded wired provisioning server without enabling wireless radios.
pub fn run_usb_provisioning() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let _power_latch = Output::new(peripherals.GPIO17, Level::High, OutputConfig::default());
    let _panel_power = Output::new(peripherals.GPIO6, Level::High, OutputConfig::default());
    let _audio_power = Output::new(peripherals.GPIO42, Level::High, OutputConfig::default());
    let i2c = match I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(100)),
    ) {
        Ok(i2c) => i2c
            .with_sda(peripherals.GPIO47)
            .with_scl(peripherals.GPIO48)
            .into_async(),
        Err(_) => loop {
            core::hint::spin_loop();
        },
    };
    let mut rtc = Pcf85063Rtc::new(i2c);
    let mut transport = UsbProtocolTransport::new(peripherals.USB_DEVICE);
    let mut delay = Delay::new();
    loop {
        if block_on(transport.poll(&mut rtc, 0)).is_err() {
            transport.reset_partial_frame();
        }
        delay.delay_ms(1);
    }
}

type BoardI2c = esp_hal::i2c::master::I2c<'static, esp_hal::Async>;
type BoardRtc = Pcf85063Rtc<BoardI2c>;

enum ApplicationRun {
    Daily {
        crc32: u32,
    },
    Setup {
        crc32: u32,
        rtc: BoardRtc,
        usb_device: esp_hal::peripherals::USB_DEVICE<'static>,
    },
}

fn run_application_once() -> Result<ApplicationRun, &'static str> {
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
    let mut rtc = Pcf85063Rtc::new(i2c);
    let reading = block_on(rtc.read_datetime()).map_err(map_rtc_error);
    let mut framebuffer = Framebuffer::default();
    let rendered = crate::render_rtc_frame(reading, &mut framebuffer)
        .map_err(|_| "offline content or rendering failed")?;

    panel_power.set_low();
    let panel_result = refresh_panel_frame(
        peripherals.SPI2,
        peripherals.GPIO8,
        peripherals.GPIO9,
        peripherals.GPIO10,
        peripherals.GPIO11,
        peripherals.GPIO12,
        peripherals.GPIO13,
        &framebuffer,
    );
    panel_power.set_high();
    panel_result?;

    match rendered.screen {
        Screen::Daily(_) => Ok(ApplicationRun::Daily {
            crc32: rendered.crc32,
        }),
        Screen::Setup(_) => Ok(ApplicationRun::Setup {
            crc32: rendered.crc32,
            rtc,
            usb_device: peripherals.USB_DEVICE,
        }),
    }
}

fn serve_setup(rtc: &mut BoardRtc, usb_device: esp_hal::peripherals::USB_DEVICE<'static>) -> ! {
    let mut transport = UsbProtocolTransport::new(usb_device);
    let mut delay = Delay::new();
    loop {
        match block_on(transport.poll(rtc, 0)) {
            Ok(handled) if handled > 0 => {
                let reading = block_on(rtc.read_datetime()).map_err(map_rtc_error);
                if matches!(assess_rtc(reading), RecoveryState::Ready(_)) {
                    delay.delay_ms(100);
                    esp_hal::system::software_reset();
                }
            }
            Ok(_) => {}
            Err(_) => transport.reset_partial_frame(),
        }
        delay.delay_ms(1);
    }
}

fn map_rtc_error<BusError>(error: Pcf85063RtcError<BusError>) -> SetupReason {
    match error {
        Pcf85063RtcError::OscillatorStopped => SetupReason::OscillatorStopped,
        Pcf85063RtcError::InvalidDateTime => SetupReason::InvalidCalendar,
        Pcf85063RtcError::Driver(_) => SetupReason::ReadFailure,
    }
}

fn run_board_diagnostics(
    panel_diagnostic: PanelDiagnostic,
    configure_alarm: bool,
) -> Result<(HardwareDiagnosticReport, SleepResources), &'static str> {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut power_latch_pin = peripherals.GPIO17;
    power_latch_pin.rtcio_pad_hold(false);
    let power_latch = Output::new(
        power_latch_pin.reborrow(),
        Level::High,
        OutputConfig::default(),
    );
    let mut panel_power_pin = peripherals.GPIO6;
    panel_power_pin.rtcio_pad_hold(false);
    let mut panel_power = Output::new(
        panel_power_pin.reborrow(),
        Level::High,
        OutputConfig::default(),
    );
    let mut audio_power_pin = peripherals.GPIO42;
    let audio_power = Output::new(
        audio_power_pin.reborrow(),
        Level::High,
        OutputConfig::default(),
    );

    let i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(100)),
    )
    .map_err(|_| "invalid I2C configuration")?
    .with_sda(peripherals.GPIO47)
    .with_scl(peripherals.GPIO48)
    .into_async();
    let rtc_result = run_rtc_diagnostics(Pcf85063Rtc::new(i2c), configure_alarm);

    panel_power.set_low();
    let panel_result = run_panel_diagnostics(
        peripherals.SPI2,
        peripherals.GPIO8,
        peripherals.GPIO9,
        peripherals.GPIO10,
        peripherals.GPIO11,
        peripherals.GPIO12,
        peripherals.GPIO13,
        panel_diagnostic,
    );
    panel_power.set_high();
    drop(panel_power);
    drop(power_latch);
    drop(audio_power);
    panel_result?;
    rtc_result.map(|report| {
        (
            report,
            SleepResources {
                rtc_interrupt: peripherals.GPIO5,
                panel_power: panel_power_pin,
                power_latch: power_latch_pin,
                audio_power: audio_power_pin,
                low_power: peripherals.LPWR,
            },
        )
    })
}

impl SleepResources {
    fn sleep(self) -> ! {
        let mut rtc_interrupt = self.rtc_interrupt;
        let interrupt_input = Input::new(
            rtc_interrupt.reborrow(),
            InputConfig::default().with_pull(Pull::Up),
        );
        if interrupt_input.is_low() {
            esp_println::println!("sleep diagnostic failed: RTC interrupt remained low");
            loop {
                core::hint::spin_loop();
            }
        }
        drop(interrupt_input);

        let mut panel_power = self.panel_power;
        let panel_output =
            Output::new(panel_power.reborrow(), Level::High, OutputConfig::default());
        drop(panel_output);
        panel_power.rtcio_pad_hold(true);

        let mut power_latch = self.power_latch;
        let latch_output =
            Output::new(power_latch.reborrow(), Level::High, OutputConfig::default());
        drop(latch_output);
        power_latch.rtcio_pad_hold(true);

        let _audio_power = Output::new(self.audio_power, Level::High, OutputConfig::default());
        let wake = Ext0WakeupSource::new(rtc_interrupt, WakeupLevel::Low);
        let mut low_power = HalRtc::new(self.low_power);
        Delay::new().delay_ms(100);
        low_power.sleep_deep(&[&wake]);
    }
}

fn run_rtc_diagnostics<I2cBus>(
    mut rtc: Pcf85063Rtc<I2cBus>,
    configure_alarm: bool,
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
        if configure_alarm {
            rtc.configure_daily_alarm()
                .await
                .map_err(|_| "RTC alarm configuration failed")?;
        }

        Ok(HardwareDiagnosticReport {
            rtc_datetime: readback,
            alarm_was_pending,
        })
    })
}
