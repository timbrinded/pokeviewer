//! Passive daily refresh, provisioning, and RTC-alarm sleep runtime.

use embassy_futures::block_on;
use embedded_hal::delay::DelayNs;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    i2c::master::{Config as I2cConfig, I2c},
    rtc_cntl::wakeup_cause,
    system::SleepSource,
    time::Rate,
};
use pokeviewer_core::{Framebuffer, RecoveryState, SetupReason, assess_rtc};
use pokeviewer_esp32s3_pad_hold::release_audio_power_pad;

use crate::{
    FailureKind, Pcf85063Rtc, Pcf85063RtcError, Rtc, Screen,
    application::planned_wake_reached,
    es8311::suspend_audio_codec,
    panel::refresh_panel_frame,
    plan_wake, render_failure_screen,
    sleep::{SleepResources, restore_panel_power, restore_power_latch},
    usb_protocol::UsbProtocolTransport,
};

type BoardI2c = esp_hal::i2c::master::I2c<'static, esp_hal::Async>;
type BoardRtc = Pcf85063Rtc<BoardI2c>;

/// Render one frame, then deep-sleep until the next daily RTC alarm.
pub fn run_pokeviewer() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let mut power_latch_pin = peripherals.GPIO17;
    let power_latch = restore_power_latch(&mut power_latch_pin);
    let mut panel_power_pin = peripherals.GPIO6;
    let mut panel_power = restore_panel_power(&mut panel_power_pin);
    let mut audio_power_pin = peripherals.GPIO42;
    // The unpowered ES8311 clamps the shared RTC I²C bus low.
    let audio_power = Output::new(
        audio_power_pin.reborrow(),
        Level::Low,
        OutputConfig::default(),
    );
    release_audio_power_pad();

    macro_rules! terminal {
        ($failure:expr) => {{
            drop(panel_power);
            drop(power_latch);
            drop(audio_power);
            sleep_after_failure(
                $failure,
                SleepResources {
                    rtc_interrupt: peripherals.GPIO5,
                    panel_power: panel_power_pin,
                    power_latch: power_latch_pin,
                    audio_power: audio_power_pin,
                    low_power: peripherals.LPWR,
                },
            );
        }};
    }

    macro_rules! display_terminal {
        ($failure:expr) => {{
            let mut framebuffer = Framebuffer::default();
            render_failure_screen(&mut framebuffer, $failure)
                .expect("fixed recovery labels must render");
            panel_power.set_low();
            let _ = refresh_panel_frame(
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
            terminal!($failure);
        }};
    }

    if !matches!(wakeup_cause(), SleepSource::Undefined | SleepSource::Ext0) {
        display_terminal!(FailureKind::UnexpectedWake);
    }

    let mut i2c = match I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(100)),
    ) {
        Ok(i2c) => i2c
            .with_sda(peripherals.GPIO47)
            .with_scl(peripherals.GPIO48)
            .into_async(),
        Err(_) => display_terminal!(FailureKind::InvalidRtc),
    };
    if block_on(suspend_audio_codec(&mut i2c)).is_err() {
        display_terminal!(FailureKind::InvalidRtc);
    }
    let mut rtc = Pcf85063Rtc::new(i2c);
    let reading = block_on(rtc.read_datetime()).map_err(map_rtc_error);
    let wake_plan = reading.ok().and_then(|now| plan_wake(now, None).ok());
    let mut framebuffer = Framebuffer::default();
    let mut frame_failure = None;
    let rendered = match crate::render_rtc_frame(reading, &mut framebuffer) {
        Ok(rendered) => Some(rendered),
        Err(_) => {
            render_failure_screen(&mut framebuffer, FailureKind::Content)
                .expect("fixed recovery labels must render");
            frame_failure = Some(FailureKind::Content);
            None
        }
    };
    if reading.is_ok() && wake_plan.is_none() {
        render_failure_screen(&mut framebuffer, FailureKind::Alarm)
            .expect("fixed recovery labels must render");
        frame_failure = Some(FailureKind::Alarm);
    }
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
    if panel_result.is_err() {
        terminal!(FailureKind::Panel);
    }
    if let Some(failure) = frame_failure {
        terminal!(failure);
    }

    let rendered = rendered.expect("successful frame has a rendered state");
    let Screen::Daily(_) = rendered.screen else {
        esp_println::println!(
            "RTC setup required; framebuffer_crc32={:08x}; awake=true",
            rendered.crc32
        );
        serve_setup(
            &mut rtc,
            peripherals.USB_DEVICE,
            panel_power,
            power_latch,
            audio_power,
        );
    };
    let wake_plan = wake_plan.expect("daily frame has a validated wake plan");
    if block_on(rtc.configure_daily_alarm()).is_err() {
        terminal!(FailureKind::Alarm);
    }
    let after_alarm_configuration = match block_on(rtc.read_datetime()) {
        Ok(datetime) => datetime,
        Err(_) => terminal!(FailureKind::Alarm),
    };
    match planned_wake_reached(after_alarm_configuration, wake_plan.next_wake) {
        Ok(true) => {
            esp_println::println!("daily rollover crossed during refresh; restarting once");
            Delay::new().delay_ms(100);
            esp_hal::system::software_reset();
        }
        Ok(false) => {}
        Err(_) => terminal!(FailureKind::Alarm),
    }
    esp_println::println!(
        "daily card ready; framebuffer_crc32={:08x}; refreshed=true; next_rollover={:04}-{:02}-{:02} 07:00:00; panel_rail_off=true; power_latch_high=true; audio_power_low=true; audio_codec_suspended=true; deep_sleep=true",
        rendered.crc32,
        wake_plan.next_wake.year,
        wake_plan.next_wake.month,
        wake_plan.next_wake.day,
    );
    drop(rtc);
    drop(panel_power);
    drop(power_latch);
    drop(audio_power);
    SleepResources {
        rtc_interrupt: peripherals.GPIO5,
        panel_power: panel_power_pin,
        power_latch: power_latch_pin,
        audio_power: audio_power_pin,
        low_power: peripherals.LPWR,
    }
    .sleep();
}

fn serve_setup(
    rtc: &mut BoardRtc,
    usb_device: esp_hal::peripherals::USB_DEVICE<'static>,
    _panel_power: Output<'_>,
    _power_latch: Output<'_>,
    _audio_power: Output<'_>,
) -> ! {
    let mut transport = UsbProtocolTransport::new(usb_device);
    let mut delay = Delay::new();
    loop {
        match block_on(transport.poll(rtc, FailureKind::InvalidRtc.policy().diagnostic_flag)) {
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

fn sleep_after_failure(failure: FailureKind, resources: SleepResources) -> ! {
    let policy = failure.policy();
    esp_println::println!(
        "terminal failure; code={}; diagnostic_flag={:04x}; attempts={}; panel_rail_off=true; power_latch_high=true; audio_power_low=true; deep_sleep=true; wake_sources=none",
        policy.code,
        policy.diagnostic_flag,
        policy.max_attempts,
    );
    resources.sleep_without_wake();
}

fn map_rtc_error<BusError>(error: Pcf85063RtcError<BusError>) -> SetupReason {
    match error {
        Pcf85063RtcError::OscillatorStopped => SetupReason::OscillatorStopped,
        Pcf85063RtcError::InvalidDateTime => SetupReason::InvalidCalendar,
        Pcf85063RtcError::Driver(_) => SetupReason::ReadFailure,
    }
}
