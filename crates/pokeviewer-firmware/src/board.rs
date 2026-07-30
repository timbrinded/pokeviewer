//! Boundary around the pinned ESP HAL and supported V2 board.

use embassy_futures::block_on;
use embedded_hal::delay::DelayNs;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    i2c::master::{Config as I2cConfig, I2c},
    rtc_cntl::{reset_reason, wakeup_cause},
    system::{Cpu, SleepSource},
    time::{Duration, Rate},
};
use pokeviewer_core::Framebuffer;
use pokeviewer_esp32s3_pad_hold::release_audio_power_pad;
use portable_atomic::{AtomicU32, Ordering};

use crate::{
    FailureKind, LocalDateTime, Pcf85063Rtc, Rtc,
    es8311::suspend_audio_codec,
    panel::{PanelDiagnostic, refresh_panel_frame, run_panel_diagnostics},
    render_failure_screen,
    sleep::{
        RTC_INTERRUPT_WAKE_BIT, SleepResources, ext1_wake_status, restore_panel_power,
        restore_power_latch,
    },
    usb_protocol::UsbProtocolTransport,
};

const RTC_WAKE_DIAGNOSTIC_PASS_MAGIC: u32 = 0x4558_5431;
const RTC_WAKE_DIAGNOSTIC_NO_ALARM_MAGIC: u32 = 0x4E4F_4146;

#[esp_hal::ram(unstable(rtc_fast, persistent))]
static RTC_WAKE_DIAGNOSTIC_RESULT: AtomicU32 = AtomicU32::new(0);

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

/// Safely inject one terminal policy and prove its no-wake deep-sleep boundary.
pub fn run_failure_diagnostic(failure: FailureKind) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut power_latch_pin = peripherals.GPIO17;
    let power_latch = restore_power_latch(&mut power_latch_pin);
    let mut panel_power_pin = peripherals.GPIO6;
    let mut panel_power = restore_panel_power(&mut panel_power_pin);
    let mut audio_power_pin = peripherals.GPIO42;
    let audio_power = Output::new(
        audio_power_pin.reborrow(),
        Level::Low,
        OutputConfig::default(),
    );
    release_audio_power_pad();

    let codec_suspended = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(100)),
    )
    .map(|i2c| {
        i2c.with_sda(peripherals.GPIO47)
            .with_scl(peripherals.GPIO48)
            .into_async()
    })
    .is_ok_and(|mut i2c| block_on(suspend_audio_codec(&mut i2c)).is_ok());

    let display_refreshed = if failure == FailureKind::Panel {
        false
    } else {
        let mut framebuffer = Framebuffer::default();
        render_failure_screen(&mut framebuffer, failure)
            .expect("fixed recovery labels must render");
        panel_power.set_low();
        let refreshed = refresh_panel_frame(
            peripherals.SPI2,
            peripherals.GPIO8,
            peripherals.GPIO9,
            peripherals.GPIO10,
            peripherals.GPIO11,
            peripherals.GPIO12,
            peripherals.GPIO13,
            &framebuffer,
        )
        .is_ok();
        panel_power.set_high();
        refreshed
    };

    let policy = failure.policy();
    esp_println::println!(
        "failure diagnostic; injected_code={}; display_refreshed={display_refreshed}; retained_prior_frame={}; codec_suspended={codec_suspended}; attempts={}; terminal_deep_sleep=true; wake_sources=none",
        policy.code,
        failure == FailureKind::Panel,
        policy.max_attempts,
    );
    drop(panel_power);
    drop(power_latch);
    drop(audio_power);
    SleepResources {
        rtc_interrupt: peripherals.GPIO5,
        power_button: peripherals.GPIO18,
        panel_power: panel_power_pin,
        power_latch: power_latch_pin,
        audio_power: audio_power_pin,
        low_power: peripherals.LPWR,
    }
    .sleep_without_wake();
}

/// Refresh one frame, validate RTC state, and enter active-low RTC deep sleep.
pub fn run_sleep_diagnostic() -> ! {
    let cause = wakeup_cause();
    let wake_status = ext1_wake_status();
    match RTC_WAKE_DIAGNOSTIC_RESULT.load(Ordering::Relaxed) {
        RTC_WAKE_DIAGNOSTIC_PASS_MAGIC => {
            let mut delay = Delay::new();
            loop {
                esp_println::println!(
                    "sleep diagnostic retained result: passed; wake_cause=Ext1; rtc_status_bit=true; alarm_was_pending=true"
                );
                delay.delay_ms(1_000);
            }
        }
        RTC_WAKE_DIAGNOSTIC_NO_ALARM_MAGIC => {
            let mut delay = Delay::new();
            loop {
                esp_println::println!(
                    "sleep diagnostic retained result: failed; wake_cause=Ext1; rtc_status_or_alarm_missing=true"
                );
                delay.delay_ms(1_000);
            }
        }
        _ => {}
    }
    RTC_WAKE_DIAGNOSTIC_RESULT.store(0, Ordering::Relaxed);
    let configure_alarm = !matches!(cause, SleepSource::Ext1);
    match run_board_diagnostics(PanelDiagnostic::SingleFrame, configure_alarm) {
        Ok((report, resources)) => {
            if matches!(cause, SleepSource::Ext1) {
                if wake_status & RTC_INTERRUPT_WAKE_BIT == 0 || !report.alarm_was_pending {
                    RTC_WAKE_DIAGNOSTIC_RESULT
                        .store(RTC_WAKE_DIAGNOSTIC_NO_ALARM_MAGIC, Ordering::Relaxed);
                    let mut delay = Delay::new();
                    loop {
                        esp_println::println!(
                            "sleep diagnostic failed: EXT1 RTC status or alarm flag was absent"
                        );
                        delay.delay_ms(1_000);
                    }
                }
                RTC_WAKE_DIAGNOSTIC_RESULT.store(RTC_WAKE_DIAGNOSTIC_PASS_MAGIC, Ordering::Relaxed);
                let mut delay = Delay::new();
                loop {
                    esp_println::println!(
                        "sleep diagnostic passed; wake_cause=Ext1; rtc_status_bit=true; RTC={:04}-{:02}-{:02} {:02}:{:02}:{:02}; alarm_was_pending=true; panel_rail_off=true; audio_power_low=true; audio_codec_suspended=true",
                        report.rtc_datetime.year,
                        report.rtc_datetime.month,
                        report.rtc_datetime.day,
                        report.rtc_datetime.hour,
                        report.rtc_datetime.minute,
                        report.rtc_datetime.second,
                    );
                    delay.delay_ms(1_000);
                }
            }
            esp_println::println!(
                "sleep diagnostic ready; wake_cause={cause:?}; RTC={:04}-{:02}-{:02} {:02}:{:02}:{:02}; alarm_was_pending={}; panel_rail_off=true; audio_power_low=true; audio_codec_suspended=true",
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
            let mut delay = Delay::new();
            loop {
                esp_println::println!("sleep diagnostic failed: {error}");
                delay.delay_ms(1_000);
            }
        }
    }
}

/// Enter timer-only deep sleep once and remain awake after the validated wake.
pub fn run_timer_sleep_diagnostic() -> ! {
    let cause = wakeup_cause();
    let reset = reset_reason(Cpu::ProCpu);
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut power_latch_pin = peripherals.GPIO17;
    let power_latch = restore_power_latch(&mut power_latch_pin);
    let mut panel_power_pin = peripherals.GPIO6;
    let panel_power = restore_panel_power(&mut panel_power_pin);
    let mut audio_power_pin = peripherals.GPIO42;
    let audio_power = Output::new(
        audio_power_pin.reborrow(),
        Level::Low,
        OutputConfig::default(),
    );
    release_audio_power_pad();

    match cause {
        SleepSource::Timer => {
            let mut delay = Delay::new();
            loop {
                esp_println::println!(
                    "timer sleep diagnostic passed; wake_cause=Timer; reset_reason={reset:?}; panel_rail_off=true; power_latch_high=true; audio_power_low=true"
                );
                delay.delay_ms(1_000);
            }
        }
        SleepSource::Undefined => {
            esp_println::println!(
                "timer sleep diagnostic ready; wake_cause=Undefined; reset_reason={reset:?}; duration_seconds=10; panel_rail_off=true; power_latch_high=true; audio_power_low=true"
            );
            drop(panel_power);
            drop(power_latch);
            drop(audio_power);
            SleepResources {
                rtc_interrupt: peripherals.GPIO5,
                power_button: peripherals.GPIO18,
                panel_power: panel_power_pin,
                power_latch: power_latch_pin,
                audio_power: audio_power_pin,
                low_power: peripherals.LPWR,
            }
            .sleep_with_timer(Duration::from_secs(10));
        }
        other => {
            let mut delay = Delay::new();
            loop {
                esp_println::println!(
                    "timer sleep diagnostic failed; unexpected_wake_cause={other:?}; reset_reason={reset:?}; panel_rail_off=true; power_latch_high=true; audio_power_low=true"
                );
                delay.delay_ms(1_000);
            }
        }
    }
}

/// Verify the RTC alarm flag and active-low interrupt while remaining awake.
pub fn run_rtc_alarm_assertion_diagnostic() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let _power_latch = Output::new(peripherals.GPIO17, Level::High, OutputConfig::default());
    let _panel_power = Output::new(peripherals.GPIO6, Level::High, OutputConfig::default());
    let _audio_power = Output::new(peripherals.GPIO42, Level::Low, OutputConfig::default());
    release_audio_power_pad();

    let mut i2c = match I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(100)),
    ) {
        Ok(i2c) => i2c
            .with_sda(peripherals.GPIO47)
            .with_scl(peripherals.GPIO48)
            .into_async(),
        Err(_) => rtc_alarm_failure("invalid I2C configuration"),
    };
    if block_on(suspend_audio_codec(&mut i2c)).is_err() {
        rtc_alarm_failure("audio codec suspend failed");
    }

    let mut rtc = Pcf85063Rtc::new(i2c);
    let interrupt = esp_hal::gpio::Input::new(
        peripherals.GPIO5,
        esp_hal::gpio::InputConfig::default().with_pull(esp_hal::gpio::Pull::Up),
    );
    let start = match block_on(rtc.read_datetime()) {
        Ok(datetime) => datetime,
        Err(_) => rtc_alarm_failure("RTC read failed"),
    };
    if start.hour != 6 || start.minute != 59 || start.second < 30 {
        rtc_alarm_failure("set RTC to 06:59:30-06:59:59 before running");
    }
    if block_on(rtc.clear_alarm()).is_err() {
        rtc_alarm_failure("RTC alarm clear failed");
    }
    if block_on(rtc.configure_daily_alarm()).is_err() {
        rtc_alarm_failure("RTC alarm configuration failed");
    }

    let mut delay = Delay::new();
    loop {
        let datetime = match block_on(rtc.read_datetime()) {
            Ok(datetime) => datetime,
            Err(_) => rtc_alarm_failure("RTC poll failed"),
        };
        let pending = match block_on(rtc.alarm_pending()) {
            Ok(pending) => pending,
            Err(_) => rtc_alarm_failure("RTC alarm flag read failed"),
        };
        let interrupt_low = interrupt.is_low();

        if pending {
            if !interrupt_low {
                rtc_alarm_failure("alarm flag asserted without GPIO5 low");
            }
            if block_on(rtc.clear_alarm()).is_err() {
                rtc_alarm_failure("RTC alarm flag clear failed");
            }
            delay.delay_ms(10);
            if interrupt.is_low() {
                rtc_alarm_failure("GPIO5 remained low after clearing alarm");
            }
            loop {
                esp_println::println!(
                    "RTC alarm assertion diagnostic passed; RTC={:04}-{:02}-{:02} {:02}:{:02}:{:02}; alarm_flag_asserted=true; GPIO5_low=true; clear_released_GPIO5=true",
                    datetime.year,
                    datetime.month,
                    datetime.day,
                    datetime.hour,
                    datetime.minute,
                    datetime.second,
                );
                delay.delay_ms(1_000);
            }
        }

        if datetime.hour > 7 || (datetime.hour == 7 && (datetime.minute > 0 || datetime.second > 5))
        {
            rtc_alarm_failure("07:00:00 passed without alarm assertion");
        }

        esp_println::println!(
            "RTC alarm assertion diagnostic waiting; RTC={:04}-{:02}-{:02} {:02}:{:02}:{:02}; alarm_pending=false; GPIO5_low={interrupt_low}",
            datetime.year,
            datetime.month,
            datetime.day,
            datetime.hour,
            datetime.minute,
            datetime.second,
        );
        delay.delay_ms(250);
    }
}

fn rtc_alarm_failure(reason: &'static str) -> ! {
    let mut delay = Delay::new();
    loop {
        esp_println::println!("RTC alarm assertion diagnostic failed: {reason}");
        delay.delay_ms(1_000);
    }
}

/// Run the bounded wired provisioning server without enabling wireless radios.
pub fn run_usb_provisioning() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let _power_latch = Output::new(peripherals.GPIO17, Level::High, OutputConfig::default());
    let _panel_power = Output::new(peripherals.GPIO6, Level::High, OutputConfig::default());
    // The unpowered ES8311 clamps the shared RTC I²C bus low.
    let _audio_power = Output::new(peripherals.GPIO42, Level::Low, OutputConfig::default());
    release_audio_power_pad();
    let mut i2c = match I2c::new(
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
    if block_on(suspend_audio_codec(&mut i2c)).is_err() {
        loop {
            core::hint::spin_loop();
        }
    }
    let mut rtc = Pcf85063Rtc::new(i2c);
    let mut transport = UsbProtocolTransport::new(peripherals.USB_DEVICE);
    let mut delay = Delay::new();
    loop {
        if block_on(transport.poll(&mut rtc, 0, false)).is_err() {
            transport.reset_partial_frame();
        }
        delay.delay_ms(1);
    }
}

fn run_board_diagnostics(
    panel_diagnostic: PanelDiagnostic,
    configure_alarm: bool,
) -> Result<(HardwareDiagnosticReport, SleepResources), &'static str> {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut power_latch_pin = peripherals.GPIO17;
    let power_latch = restore_power_latch(&mut power_latch_pin);
    let mut panel_power_pin = peripherals.GPIO6;
    let mut panel_power = restore_panel_power(&mut panel_power_pin);
    let mut audio_power_pin = peripherals.GPIO42;
    // Keep the shared-bus codec powered until all RTC transactions complete.
    let audio_power = Output::new(
        audio_power_pin.reborrow(),
        Level::Low,
        OutputConfig::default(),
    );
    release_audio_power_pad();

    let mut i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(100)),
    )
    .map_err(|_| "invalid I2C configuration")?
    .with_sda(peripherals.GPIO47)
    .with_scl(peripherals.GPIO48)
    .into_async();
    block_on(suspend_audio_codec(&mut i2c)).map_err(|_| "audio codec suspend failed")?;
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
                power_button: peripherals.GPIO18,
                panel_power: panel_power_pin,
                power_latch: power_latch_pin,
                audio_power: audio_power_pin,
                low_power: peripherals.LPWR,
            },
        )
    })
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
