//! Passive daily refresh, PWR-gated parent setup, and EXT1 deep-sleep runtime.

use embassy_futures::block_on;
use embedded_hal::delay::DelayNs;
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    i2c::master::{Config as I2cConfig, I2c},
    rtc_cntl::wakeup_cause,
    system::SleepSource,
    time::Rate,
};
use pokeviewer_core::{BatteryStatus, Framebuffer, SetupReason};
use pokeviewer_esp32s3_pad_hold::release_audio_power_pad;
use portable_atomic::{AtomicU32, Ordering};

use crate::{
    FailureKind, Pcf85063Rtc, Pcf85063RtcError, ProtocolAction, Rtc, Screen, WakeDecision,
    WakeInput,
    application::planned_wake_reached,
    battery_sensor::{BatteryObservation, sample_battery},
    decide_wake,
    es8311::suspend_audio_codec,
    panel::refresh_panel_frame,
    plan_wake, render_failure_screen,
    sleep::{
        POWER_BUTTON_WAKE_BIT, RTC_INTERRUPT_WAKE_BIT, SleepResources, ext1_wake_status,
        restore_panel_power, restore_power_latch, restore_wake_pin,
    },
    usb_protocol::UsbProtocolTransport,
};

type BoardI2c = esp_hal::i2c::master::I2c<'static, esp_hal::Async>;
type BoardRtc = Pcf85063Rtc<BoardI2c>;

const PARENT_AFTER_DAILY_MAGIC: u32 = 0x5057_5201;
const POWER_HOLD_POLLS: usize = 60;
const POWER_HOLD_POLL_MS: u32 = 50;
const USB_FRAME_GATE_POLLS: usize = 15_000;
const PARENT_SESSION_POLLS: usize = 120_000;

#[esp_hal::ram(unstable(rtc_fast, persistent))]
static PARENT_AFTER_DAILY: AtomicU32 = AtomicU32::new(0);

/// Render one frame, then deep-sleep until the RTC alarm or PWR wakes the board.
pub fn run_pokeviewer() -> ! {
    let cause = wakeup_cause();
    let wake_status = ext1_wake_status();
    let parent_after_daily =
        PARENT_AFTER_DAILY.swap(0, Ordering::Relaxed) == PARENT_AFTER_DAILY_MAGIC;
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

    let mut rtc_interrupt_pin = peripherals.GPIO5;
    let mut power_button_pin = peripherals.GPIO18;
    restore_wake_pin(&mut rtc_interrupt_pin);
    restore_wake_pin(&mut power_button_pin);
    let battery = sample_battery(peripherals.ADC1, peripherals.GPIO4);

    macro_rules! sleep_resources {
        () => {
            SleepResources {
                rtc_interrupt: rtc_interrupt_pin,
                power_button: power_button_pin,
                panel_power: panel_power_pin,
                power_latch: power_latch_pin,
                audio_power: audio_power_pin,
                low_power: peripherals.LPWR,
            }
        };
    }

    macro_rules! terminal {
        ($failure:expr) => {{
            drop(panel_power);
            drop(power_latch);
            drop(audio_power);
            sleep_after_failure($failure, sleep_resources!());
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

    macro_rules! sleep_current_rtc {
        ($rtc:ident) => {{
            let sleep_mode = prepare_sleep(&mut $rtc);
            drop($rtc);
            drop(panel_power);
            drop(power_latch);
            drop(audio_power);
            let resources = sleep_resources!();
            match sleep_mode {
                RtcSleepMode::Daily => resources.sleep(),
                RtcSleepMode::Setup => resources.sleep_for_setup(),
                RtcSleepMode::AlarmFailure => {
                    sleep_after_failure(FailureKind::Alarm, resources);
                }
            }
        }};
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
    let alarm_pending = block_on(rtc.alarm_pending()).unwrap_or(false);
    let decision = if parent_after_daily {
        WakeDecision {
            refresh_daily: false,
            check_parent_session: true,
        }
    } else {
        let input = match cause {
            SleepSource::Undefined => WakeInput::Reset,
            SleepSource::Ext1 => WakeInput::Ext1 {
                rtc_pin: wake_status & RTC_INTERRUPT_WAKE_BIT != 0,
                power_pin: wake_status & POWER_BUTTON_WAKE_BIT != 0,
                alarm_pending,
            },
            _ => WakeInput::Other,
        };
        match decide_wake(input) {
            Ok(decision) => decision,
            Err(_) => display_terminal!(FailureKind::UnexpectedWake),
        }
    };

    if decision.check_parent_session && !decision.refresh_daily {
        if !power_held_for_parent_session(&mut power_button_pin) {
            wait_for_power_release(&mut power_button_pin);
            sleep_current_rtc!(rtc);
        }

        let Some((mut transport, first_action)) =
            wait_for_valid_usb_frame(&mut rtc, peripherals.USB_DEVICE, battery.diagnostic_flags)
        else {
            wait_for_power_release(&mut power_button_pin);
            sleep_current_rtc!(rtc);
        };
        if first_action == ProtocolAction::RtcSet {
            Delay::new().delay_ms(100);
            esp_hal::system::software_reset();
        }

        let mut framebuffer = Framebuffer::default();
        render_failure_screen(&mut framebuffer, FailureKind::InvalidRtc)
            .expect("fixed setup labels must render");
        panel_power.set_low();
        if refresh_panel_frame(
            peripherals.SPI2,
            peripherals.GPIO8,
            peripherals.GPIO9,
            peripherals.GPIO10,
            peripherals.GPIO11,
            peripherals.GPIO12,
            peripherals.GPIO13,
            &framebuffer,
        )
        .is_err()
        {
            panel_power.set_high();
            terminal!(FailureKind::Panel);
        }
        panel_power.set_high();

        match serve_parent_session(&mut rtc, &mut transport, battery.diagnostic_flags) {
            ProtocolAction::RtcSet => {
                Delay::new().delay_ms(100);
                esp_hal::system::software_reset();
            }
            ProtocolAction::EnterStorage => {
                if block_on(rtc.invalidate()).is_err() {
                    terminal!(FailureKind::InvalidRtc);
                }
                Delay::new().delay_ms(100);
                drop(rtc);
                drop(transport);
                drop(panel_power);
                drop(power_latch);
                drop(audio_power);
                sleep_resources!().power_off_for_storage();
            }
            ProtocolAction::None => {
                wait_for_power_release(&mut power_button_pin);
                if block_on(rtc.read_datetime()).is_ok() {
                    Delay::new().delay_ms(100);
                    esp_hal::system::software_reset();
                }
                drop(rtc);
                drop(transport);
                drop(panel_power);
                drop(power_latch);
                drop(audio_power);
                sleep_resources!().sleep_for_setup();
            }
        }
    }

    let reading = block_on(rtc.read_datetime()).map_err(map_rtc_error);
    let wake_plan = reading.ok().and_then(|now| plan_wake(now, None).ok());
    let mut framebuffer = Framebuffer::default();
    let mut frame_failure = None;
    let rendered = match crate::render_rtc_frame(reading, battery.status, &mut framebuffer) {
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
            "RTC setup required; framebuffer_crc32={:08x}; awake=true; timeout_seconds=120",
            rendered.crc32
        );
        let action = serve_initial_setup(
            &mut rtc,
            peripherals.USB_DEVICE,
            FailureKind::InvalidRtc.policy().diagnostic_flag | battery.diagnostic_flags,
        );
        if action == ProtocolAction::RtcSet {
            Delay::new().delay_ms(100);
            esp_hal::system::software_reset();
        }
        drop(rtc);
        drop(panel_power);
        drop(power_latch);
        drop(audio_power);
        sleep_resources!().sleep_for_setup();
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

    if decision.check_parent_session {
        PARENT_AFTER_DAILY.store(PARENT_AFTER_DAILY_MAGIC, Ordering::Relaxed);
        Delay::new().delay_ms(100);
        esp_hal::system::software_reset();
    }

    log_daily_ready(rendered.crc32, wake_plan.next_wake, battery);
    drop(rtc);
    drop(panel_power);
    drop(power_latch);
    drop(audio_power);
    sleep_resources!().sleep();
}

fn power_held_for_parent_session(
    power_button_pin: &mut esp_hal::peripherals::GPIO18<'static>,
) -> bool {
    let input = Input::new(
        power_button_pin.reborrow(),
        InputConfig::default().with_pull(Pull::Up),
    );
    if input.is_high() {
        return false;
    }
    let mut delay = Delay::new();
    for _ in 0..POWER_HOLD_POLLS {
        delay.delay_ms(POWER_HOLD_POLL_MS);
        if input.is_high() {
            return false;
        }
    }
    true
}

fn wait_for_power_release(power_button_pin: &mut esp_hal::peripherals::GPIO18<'static>) {
    let input = Input::new(
        power_button_pin.reborrow(),
        InputConfig::default().with_pull(Pull::Up),
    );
    let mut delay = Delay::new();
    while input.is_low() {
        delay.delay_ms(50);
    }
}

fn wait_for_valid_usb_frame(
    rtc: &mut BoardRtc,
    usb_device: esp_hal::peripherals::USB_DEVICE<'static>,
    diagnostic_flags: u16,
) -> Option<(UsbProtocolTransport, ProtocolAction)> {
    let mut transport = UsbProtocolTransport::new(usb_device);
    let mut delay = Delay::new();
    for _ in 0..USB_FRAME_GATE_POLLS {
        match block_on(transport.poll(rtc, diagnostic_flags, false)) {
            Ok(result) if result.handled > 0 => return Some((transport, result.action)),
            Ok(_) => {}
            Err(_) => transport.reset_partial_frame(),
        }
        delay.delay_ms(1);
    }
    None
}

fn serve_parent_session(
    rtc: &mut BoardRtc,
    transport: &mut UsbProtocolTransport,
    diagnostic_flags: u16,
) -> ProtocolAction {
    let mut delay = Delay::new();
    for _ in 0..PARENT_SESSION_POLLS {
        match block_on(transport.poll(rtc, diagnostic_flags, true)) {
            Ok(result) if result.action != ProtocolAction::None => return result.action,
            Ok(_) => {}
            Err(_) => transport.reset_partial_frame(),
        }
        delay.delay_ms(1);
    }
    ProtocolAction::None
}

fn serve_initial_setup(
    rtc: &mut BoardRtc,
    usb_device: esp_hal::peripherals::USB_DEVICE<'static>,
    diagnostic_flags: u16,
) -> ProtocolAction {
    let mut transport = UsbProtocolTransport::new(usb_device);
    let mut delay = Delay::new();
    for _ in 0..PARENT_SESSION_POLLS {
        match block_on(transport.poll(rtc, diagnostic_flags, false)) {
            Ok(result) if result.action == ProtocolAction::RtcSet => return result.action,
            Ok(_) => {}
            Err(_) => transport.reset_partial_frame(),
        }
        delay.delay_ms(1);
    }
    ProtocolAction::None
}

enum RtcSleepMode {
    Daily,
    Setup,
    AlarmFailure,
}

fn prepare_sleep(rtc: &mut BoardRtc) -> RtcSleepMode {
    let valid = block_on(rtc.read_datetime()).is_ok();
    let alarm_pending = block_on(rtc.alarm_pending()).unwrap_or(false);
    if valid && alarm_pending {
        Delay::new().delay_ms(100);
        esp_hal::system::software_reset();
    }
    if valid && block_on(rtc.configure_daily_alarm()).is_err() {
        return RtcSleepMode::AlarmFailure;
    }
    if valid {
        RtcSleepMode::Daily
    } else {
        RtcSleepMode::Setup
    }
}

fn log_daily_ready(
    crc32: u32,
    next_wake: pokeviewer_core::LocalDateTime,
    battery: BatteryObservation,
) {
    let (battery_percent, battery_recharge) = match battery.status {
        BatteryStatus::Estimated { percent, recharge } => (Some(percent), recharge),
        BatteryStatus::Unavailable => (None, false),
    };
    esp_println::println!(
        "daily card ready; framebuffer_crc32={crc32:08x}; refreshed=true; next_rollover={:04}-{:02}-{:02} 07:00:00; battery_percent={battery_percent:?}; battery_recharge={battery_recharge}; panel_rail_off=true; power_latch_high=true; audio_power_low=true; audio_codec_suspended=true; deep_sleep=true; wake_sources=ext1_gpio5_gpio18",
        next_wake.year,
        next_wake.month,
        next_wake.day,
    );
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
