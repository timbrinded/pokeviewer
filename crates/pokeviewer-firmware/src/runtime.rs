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
    LocalDateTime, Pcf85063Rtc, Pcf85063RtcError, RetainedCard, Rtc, Screen,
    panel::refresh_panel_frame, plan_wake, sleep::SleepResources,
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
}

/// Run the complete passive application and enter its terminal low-power state.
pub fn run_pokeviewer() -> ! {
    match run_application_once() {
        Ok(ApplicationRun::Daily {
            crc32,
            next_wake,
            refreshed,
            resources,
        }) => {
            esp_println::println!(
                "daily card ready; framebuffer_crc32={crc32:08x}; refreshed={refreshed}; next_wake={:04}-{:02}-{:02} 07:00:00; rails_off=true",
                next_wake.year,
                next_wake.month,
                next_wake.day,
            );
            resources.sleep();
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

fn run_application_once() -> Result<ApplicationRun, &'static str> {
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

    let alarm_pending = if matches!(rendered.screen, Screen::Daily(_)) {
        block_on(rtc.alarm_pending()).map_err(|_| "RTC alarm flag read failed")?
    } else {
        false
    };
    let retained = match rendered.screen {
        Screen::Daily(selection) if matches!(cause, SleepSource::Ext0) && !alarm_pending => {
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
        .map_err(|_| "next 07:00 wake is outside the RTC range")?;
    let refreshed = match rendered.screen {
        Screen::Setup(_) => true,
        Screen::Daily(_) => wake_plan.is_none_or(|plan| plan.refresh_required),
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
        panel_result?;
    }

    let Screen::Daily(_) = rendered.screen else {
        return Ok(ApplicationRun::Setup {
            crc32: rendered.crc32,
            rtc,
            usb_device: peripherals.USB_DEVICE,
        });
    };
    let wake_plan = wake_plan.ok_or("valid daily card has no wake plan")?;
    block_on(rtc.configure_daily_alarm()).map_err(|_| "RTC alarm configuration failed")?;
    drop(rtc);
    drop(panel_power);
    drop(power_latch);
    drop(audio_power);
    Ok(ApplicationRun::Daily {
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
    })
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
