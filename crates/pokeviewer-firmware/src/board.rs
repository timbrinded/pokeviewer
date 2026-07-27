//! Boundary around the pinned ESP HAL and supported V2 board.

use embassy_futures::block_on;
use embedded_hal::delay::DelayNs;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig, RtcPin},
    i2c::master::{Config as I2cConfig, I2c},
    rtc_cntl::wakeup_cause,
    system::SleepSource,
    time::Rate,
};

use crate::{
    LocalDateTime, Pcf85063Rtc, Rtc,
    panel::{PanelDiagnostic, run_panel_diagnostics},
    sleep::SleepResources,
    usb_protocol::UsbProtocolTransport,
};

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
