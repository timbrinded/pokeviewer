//! Normal battery-powered wake, refresh, provisioning, and sleep runtime.

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
use pokeviewer_core::{Framebuffer, RecoveryState, SetupReason, assess_rtc};

use crate::{
    FailureKind, LocalDateTime, Pcf85063Rtc, Pcf85063RtcError, RetainedCard, Rtc, Screen,
    panel::refresh_panel_frame, plan_wake, render_failure_screen, sleep::SleepResources,
    usb_protocol::UsbProtocolTransport,
};

type BoardI2c = esp_hal::i2c::master::I2c<'static, esp_hal::Async>;
type BoardRtc = Pcf85063Rtc<BoardI2c>;

enum ApplicationRun {
    Daily {
        crc32: u32,
        next_wake: LocalDateTime,
        refreshed: bool,
        resources: SleepResources,
    },
    Setup {
        crc32: u32,
        rtc: BoardRtc,
        usb_device: esp_hal::peripherals::USB_DEVICE<'static>,
    },
    Failure {
        failure: FailureKind,
        resources: SleepResources,
    },
}

/// Run the complete passive application and enter its terminal low-power state.
pub fn run_pokeviewer() -> ! {
    match run_application_once() {
        ApplicationRun::Daily {
            crc32,
            next_wake,
            refreshed,
            resources,
        } => {
            esp_println::println!(
                "daily card ready; framebuffer_crc32={crc32:08x}; refreshed={refreshed}; next_wake={:04}-{:02}-{:02} 07:00:00; rails_off=true",
                next_wake.year,
                next_wake.month,
                next_wake.day,
            );
            resources.sleep();
        }
        ApplicationRun::Setup {
            crc32,
            mut rtc,
            usb_device,
        } => {
            esp_println::println!("RTC setup required; framebuffer_crc32={crc32:08x}");
            serve_setup(&mut rtc, usb_device);
        }
        ApplicationRun::Failure { failure, resources } => {
            let policy = failure.policy();
            esp_println::println!(
                "terminal failure; code={}; diagnostic_flag={:04x}; attempts={}; rails_off=true",
                policy.code,
                policy.diagnostic_flag,
                policy.max_attempts,
            );
            resources.sleep_without_wake();
        }
    }
}

fn run_application_once() -> ApplicationRun {
    let cause = wakeup_cause();
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

    macro_rules! terminal {
        ($failure:expr) => {{
            drop(panel_power);
            drop(power_latch);
            drop(audio_power);
            return ApplicationRun::Failure {
                failure: $failure,
                resources: SleepResources {
                    rtc_interrupt: peripherals.GPIO5,
                    panel_power: panel_power_pin,
                    power_latch: power_latch_pin,
                    audio_power: audio_power_pin,
                    low_power: peripherals.LPWR,
                },
            };
        }};
    }

    let i2c = match I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(100)),
    ) {
        Ok(i2c) => i2c
            .with_sda(peripherals.GPIO47)
            .with_scl(peripherals.GPIO48)
            .into_async(),
        Err(_) => {
            let mut framebuffer = Framebuffer::default();
            render_failure_screen(&mut framebuffer, FailureKind::InvalidRtc)
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
            terminal!(FailureKind::InvalidRtc);
        }
    };
    let mut rtc = Pcf85063Rtc::new(i2c);
    let reading = block_on(rtc.read_datetime()).map_err(map_rtc_error);
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
    if rendered.is_some_and(|rendered| matches!(rendered.screen, Screen::Daily(_)))
        && !matches!(cause, SleepSource::Undefined | SleepSource::Ext0)
    {
        render_failure_screen(&mut framebuffer, FailureKind::UnexpectedWake)
            .expect("fixed recovery labels must render");
        frame_failure = Some(FailureKind::UnexpectedWake);
    }

    let alarm_pending = if rendered.is_some_and(|value| matches!(value.screen, Screen::Daily(_))) {
        match block_on(rtc.alarm_pending()) {
            Ok(pending) => pending,
            Err(_) => {
                render_failure_screen(&mut framebuffer, FailureKind::Alarm)
                    .expect("fixed recovery labels must render");
                frame_failure = Some(FailureKind::Alarm);
                false
            }
        }
    } else {
        false
    };
    let retained = match rendered.map(|value| value.screen) {
        Some(Screen::Daily(selection)) if matches!(cause, SleepSource::Ext0) && !alarm_pending => {
            Some(RetainedCard {
                display_date: selection.display_date,
            })
        }
        _ => None,
    };
    let wake_plan = reading
        .ok()
        .map(|now| plan_wake(now, retained))
        .transpose()
        .ok()
        .flatten();
    if reading.is_ok() && wake_plan.is_none() {
        render_failure_screen(&mut framebuffer, FailureKind::Alarm)
            .expect("fixed recovery labels must render");
        frame_failure = Some(FailureKind::Alarm);
    }
    let refreshed = match rendered.map(|value| value.screen) {
        Some(Screen::Daily(_)) if frame_failure.is_none() => {
            wake_plan.is_none_or(|plan| plan.refresh_required)
        }
        _ => true,
    };
    if refreshed {
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
    }
    if let Some(failure) = frame_failure {
        terminal!(failure);
    }

    let rendered = rendered.expect("successful frame has a rendered state");
    let Screen::Daily(_) = rendered.screen else {
        return ApplicationRun::Setup {
            crc32: rendered.crc32,
            rtc,
            usb_device: peripherals.USB_DEVICE,
        };
    };
    let wake_plan = wake_plan.expect("daily frame has a validated wake plan");
    if block_on(rtc.configure_daily_alarm()).is_err() {
        terminal!(FailureKind::Alarm);
    }
    drop(rtc);
    drop(panel_power);
    drop(power_latch);
    drop(audio_power);
    ApplicationRun::Daily {
        crc32: rendered.crc32,
        next_wake: wake_plan.next_wake,
        refreshed,
        resources: SleepResources {
            rtc_interrupt: peripherals.GPIO5,
            panel_power: panel_power_pin,
            power_latch: power_latch_pin,
            audio_power: audio_power_pin,
            low_power: peripherals.LPWR,
        },
    }
}

fn serve_setup(rtc: &mut BoardRtc, usb_device: esp_hal::peripherals::USB_DEVICE<'static>) -> ! {
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

fn map_rtc_error<BusError>(error: Pcf85063RtcError<BusError>) -> SetupReason {
    match error {
        Pcf85063RtcError::OscillatorStopped => SetupReason::OscillatorStopped,
        Pcf85063RtcError::InvalidDateTime => SetupReason::InvalidCalendar,
        Pcf85063RtcError::Driver(_) => SetupReason::ReadFailure,
    }
}
