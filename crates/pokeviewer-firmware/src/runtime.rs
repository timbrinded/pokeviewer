//! Passive daily refresh, PWR-gated parent setup, and EXT1 deep-sleep runtime.

use embassy_futures::block_on;
use embedded_hal::delay::DelayNs;
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, InputPin, Level, Output, OutputConfig, Pull},
    i2c::master::{Config as I2cConfig, I2c},
    peripherals::{GPIO3, GPIO5},
    rtc_cntl::wakeup_cause,
    system::SleepSource,
    time::{Duration, Instant, Rate},
};
use pokeviewer_core::{BatteryReading, Framebuffer, SetupReason};
use pokeviewer_esp32s3_pad_hold::release_audio_power_pad;
use portable_atomic::{AtomicU32, Ordering};

use crate::{
    FailureKind, Pcf85063Rtc, Pcf85063RtcError, ProtocolAction, Rtc, Screen, WakeDecision,
    WakeInput,
    application::planned_wake_reached,
    battery::{commit_battery_observation, diagnostic_flags, load_retained_battery},
    battery_sensor::sample_battery_mv,
    decide_wake,
    es8311::suspend_audio_codec,
    panel::refresh_panel_frame,
    plan_wake, render_failure_screen,
    shtc3::sleep_humidity_sensor,
    sleep::{
        BOOT_BUTTON_WAKE_BIT, POWER_BUTTON_WAKE_BIT, RTC_INTERRUPT_WAKE_BIT, SleepResources,
        ext1_wake_status, restore_panel_power, restore_power_latch, restore_wake_pin,
    },
    usb_protocol::UsbProtocolTransport,
};

type BoardI2c = esp_hal::i2c::master::I2c<'static, esp_hal::Async>;
type BoardRtc = Pcf85063Rtc<BoardI2c>;

const PARENT_AFTER_DAILY_MAGIC: u32 = 0x5057_5201;
const TERMINAL_SLEEP_MAGIC: u32 = 0x5445_524d;
const BUTTON_POLL_MS: u32 = 50;
// Hold thresholds count from wake, which is when the press began.
const POWER_HOLD_MS: u64 = 3_000;
const RESTART_HOLD_MS: u64 = 1_000;
const BUTTON_RELEASE_POLLS: usize = 200;
const RTC_INTERRUPT_RELEASE_POLLS: usize = 10;
const RESTART_LIGHT_MS: u32 = 300;
const USB_FRAME_GATE_POLLS: usize = 15_000;
const PARENT_SESSION_POLLS: usize = 120_000;

#[esp_hal::ram(unstable(rtc_fast, persistent))]
static PARENT_AFTER_DAILY: AtomicU32 = AtomicU32::new(0);

// Set only while a terminal failure sleeps, so a short BOOT press can return to it.
#[esp_hal::ram(unstable(rtc_fast, persistent))]
static TERMINAL_SLEEP: AtomicU32 = AtomicU32::new(0);

/// Render one frame, then deep-sleep until the RTC alarm, PWR, or BOOT wakes the board.
pub fn run_pokeviewer() -> ! {
    let cause = wakeup_cause();
    let wake_status = ext1_wake_status();
    let parent_after_daily =
        PARENT_AFTER_DAILY.swap(0, Ordering::Relaxed) == PARENT_AFTER_DAILY_MAGIC;
    let after_terminal = TERMINAL_SLEEP.swap(0, Ordering::Relaxed) == TERMINAL_SLEEP_MAGIC;
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

    let mut status_light_pin = peripherals.GPIO3;
    set_status_light(&mut status_light_pin, false);

    let mut boot_button_pin = peripherals.GPIO0;
    let mut rtc_interrupt_pin = peripherals.GPIO5;
    let mut power_button_pin = peripherals.GPIO18;
    restore_wake_pin(&mut boot_button_pin);
    restore_wake_pin(&mut rtc_interrupt_pin);
    restore_wake_pin(&mut power_button_pin);
    macro_rules! sleep_resources {
        () => {
            SleepResources {
                boot_button: boot_button_pin,
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
            let sleep_mode = prepare_sleep(&mut $rtc, &mut rtc_interrupt_pin);
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
    // A sleeping sensor may reject the repeated command; it is already in the
    // low-power state this call requests.
    let _ = block_on(sleep_humidity_sensor(&mut i2c));
    let mut rtc = Pcf85063Rtc::new(i2c);
    let alarm_pending = block_on(rtc.alarm_pending()).unwrap_or(false);
    let decision = if parent_after_daily {
        WakeDecision::parent_session_after_daily()
    } else {
        let input = match cause {
            SleepSource::Undefined => WakeInput::Reset,
            SleepSource::Ext1 => WakeInput::Ext1 {
                rtc_pin: wake_status & RTC_INTERRUPT_WAKE_BIT != 0,
                power_pin: wake_status & POWER_BUTTON_WAKE_BIT != 0,
                boot_pin: wake_status & BOOT_BUTTON_WAKE_BIT != 0,
                alarm_pending,
            },
            _ => WakeInput::Other,
        };
        match decide_wake(input) {
            Ok(decision) => decision,
            Err(_) => display_terminal!(FailureKind::UnexpectedWake),
        }
    };
    let mut battery = load_retained_battery();
    let mut battery_diagnostic_flags = diagnostic_flags(battery);

    if decision.check_restart {
        if !button_held(boot_button_pin.reborrow(), RESTART_HOLD_MS) {
            if after_terminal {
                drop(rtc);
                drop(panel_power);
                drop(power_latch);
                drop(audio_power);
                sleep_until_restart(sleep_resources!());
            }
            sleep_current_rtc!(rtc);
        }
        // Acknowledge the restart before the slower panel refresh begins.
        set_status_light(&mut status_light_pin, true);
        Delay::new().delay_ms(RESTART_LIGHT_MS);
        set_status_light(&mut status_light_pin, false);
        wait_for_release(boot_button_pin.reborrow());
    }

    if decision.check_parent_session && !decision.refresh_daily {
        if !button_held(power_button_pin.reborrow(), POWER_HOLD_MS) {
            sleep_current_rtc!(rtc);
        }

        // The light stays on while the device listens for, or serves, a computer.
        set_status_light(&mut status_light_pin, true);
        let Some((mut transport, first_action)) = wait_for_valid_usb_frame(
            &mut rtc,
            peripherals.USB_DEVICE,
            battery_diagnostic_flags,
            battery,
        ) else {
            set_status_light(&mut status_light_pin, false);
            sleep_current_rtc!(rtc);
        };
        if first_action == ProtocolAction::RtcSet {
            set_status_light(&mut status_light_pin, false);
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
            set_status_light(&mut status_light_pin, false);
            terminal!(FailureKind::Panel);
        }
        panel_power.set_high();

        let action =
            serve_parent_session(&mut rtc, &mut transport, battery_diagnostic_flags, battery);
        set_status_light(&mut status_light_pin, false);
        match action {
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
                wait_for_release(power_button_pin.reborrow());
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
    if decision.should_commit_battery(reading.is_ok(), wake_plan.is_some()) {
        battery =
            commit_battery_observation(sample_battery_mv(peripherals.ADC1, peripherals.GPIO4));
        battery_diagnostic_flags = diagnostic_flags(battery);
    }
    let mut framebuffer = Framebuffer::default();
    let mut frame_failure = None;
    let rendered = match crate::render_rtc_frame(reading, battery.state(), &mut framebuffer) {
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
        set_status_light(&mut status_light_pin, true);
        let action = serve_initial_setup(
            &mut rtc,
            peripherals.USB_DEVICE,
            FailureKind::InvalidRtc.policy().diagnostic_flag | battery_diagnostic_flags,
            battery,
        );
        set_status_light(&mut status_light_pin, false);
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
    if block_on(rtc.configure_daily_alarm()).is_err()
        || !rtc_interrupt_released(rtc_interrupt_pin.reborrow())
    {
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

/// Report whether an active-low button stays pressed until `held_ms` after wake.
fn button_held<'a>(pin: impl InputPin + 'a, held_ms: u64) -> bool {
    let input = Input::new(pin, InputConfig::default().with_pull(Pull::Up));
    let mut delay = Delay::new();
    loop {
        if input.is_high() {
            return false;
        }
        if Instant::now().duration_since_epoch() >= Duration::from_millis(held_ms) {
            return true;
        }
        delay.delay_ms(BUTTON_POLL_MS);
    }
}

/// Report whether the PCF85063 interrupt line rises after the alarm flag clears.
///
/// A line that stays low would wake EXT1 immediately, so it is an alarm failure.
fn rtc_interrupt_released<'a>(pin: impl InputPin + 'a) -> bool {
    let input = Input::new(pin, InputConfig::default().with_pull(Pull::Up));
    let mut delay = Delay::new();
    for _ in 0..RTC_INTERRUPT_RELEASE_POLLS {
        if input.is_high() {
            return true;
        }
        delay.delay_ms(10);
    }
    input.is_high()
}

/// Wait a bounded time for an active-low button to be released.
fn wait_for_release<'a>(pin: impl InputPin + 'a) {
    let input = Input::new(pin, InputConfig::default().with_pull(Pull::Up));
    let mut delay = Delay::new();
    for _ in 0..BUTTON_RELEASE_POLLS {
        if input.is_high() {
            return;
        }
        delay.delay_ms(BUTTON_POLL_MS);
    }
}

/// Drive the active-low green LED on GPIO3. The level persists after the
/// temporary driver is dropped.
fn set_status_light(pin: &mut GPIO3<'static>, on: bool) {
    let level = if on { Level::Low } else { Level::High };
    let _light = Output::new(pin.reborrow(), level, OutputConfig::default());
}

fn wait_for_valid_usb_frame(
    rtc: &mut BoardRtc,
    usb_device: esp_hal::peripherals::USB_DEVICE<'static>,
    diagnostic_flags: u16,
    battery: BatteryReading,
) -> Option<(UsbProtocolTransport, ProtocolAction)> {
    let mut transport = UsbProtocolTransport::new(usb_device);
    let mut delay = Delay::new();
    for _ in 0..USB_FRAME_GATE_POLLS {
        match block_on(transport.poll(rtc, diagnostic_flags, battery, false)) {
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
    battery: BatteryReading,
) -> ProtocolAction {
    let mut delay = Delay::new();
    for _ in 0..PARENT_SESSION_POLLS {
        match block_on(transport.poll(rtc, diagnostic_flags, battery, true)) {
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
    battery: BatteryReading,
) -> ProtocolAction {
    let mut transport = UsbProtocolTransport::new(usb_device);
    let mut delay = Delay::new();
    for _ in 0..PARENT_SESSION_POLLS {
        match block_on(transport.poll(rtc, diagnostic_flags, battery, false)) {
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

fn prepare_sleep(rtc: &mut BoardRtc, rtc_interrupt: &mut GPIO5<'static>) -> RtcSleepMode {
    let valid = block_on(rtc.read_datetime()).is_ok();
    let alarm_pending = block_on(rtc.alarm_pending()).unwrap_or(false);
    if valid && alarm_pending {
        Delay::new().delay_ms(100);
        esp_hal::system::software_reset();
    }
    if valid
        && (block_on(rtc.configure_daily_alarm()).is_err()
            || !rtc_interrupt_released(rtc_interrupt.reborrow()))
    {
        return RtcSleepMode::AlarmFailure;
    }
    if valid {
        RtcSleepMode::Daily
    } else {
        RtcSleepMode::Setup
    }
}

fn log_daily_ready(crc32: u32, next_wake: pokeviewer_core::LocalDateTime, battery: BatteryReading) {
    esp_println::println!(
        "daily card ready; framebuffer_crc32={crc32:08x}; refreshed=true; next_rollover={:04}-{:02}-{:02} 07:00:00; battery_state={:?}; battery_cell_mv={}; panel_rail_off=true; power_latch_high=true; audio_power_low=true; audio_codec_suspended=true; deep_sleep=true; wake_sources=ext1_gpio0_gpio5_gpio18",
        next_wake.year,
        next_wake.month,
        next_wake.day,
        battery.state(),
        battery.cell_mv(),
    );
}

fn sleep_after_failure(failure: FailureKind, resources: SleepResources) -> ! {
    let policy = failure.policy();
    esp_println::println!(
        "terminal failure; code={}; diagnostic_flag={:04x}; attempts={}; panel_rail_off=true; power_latch_high=true; audio_power_low=true; deep_sleep=true; wake_sources=ext1_gpio0",
        policy.code,
        policy.diagnostic_flag,
        policy.max_attempts,
    );
    sleep_until_restart(resources);
}

fn sleep_until_restart(resources: SleepResources) -> ! {
    TERMINAL_SLEEP.store(TERMINAL_SLEEP_MAGIC, Ordering::Relaxed);
    resources.sleep_until_restart();
}

fn map_rtc_error<BusError>(error: Pcf85063RtcError<BusError>) -> SetupReason {
    match error {
        Pcf85063RtcError::OscillatorStopped => SetupReason::OscillatorStopped,
        Pcf85063RtcError::InvalidDateTime => SetupReason::InvalidCalendar,
        Pcf85063RtcError::Driver(_) => SetupReason::ReadFailure,
    }
}
