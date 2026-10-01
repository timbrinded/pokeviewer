//! V2 rail retention and active-low EXT1 deep-sleep boundary.

use embedded_hal::delay::DelayNs;
use esp_hal::{
    delay::Delay,
    gpio::{
        Input, InputConfig, Level, Output, OutputConfig, Pull, RtcFunction, RtcPin,
        RtcPinWithResistors,
    },
    peripherals::{GPIO0, GPIO5, GPIO6, GPIO17, GPIO18, GPIO42, LPWR},
    rtc_cntl::{
        Rtc as HalRtc,
        sleep::{Ext1WakeupSource, TimerWakeupSource, WakeupLevel},
    },
    time::Duration,
};
use pokeviewer_esp32s3_pad_hold::hold_audio_power_pad;

use crate::{SleepWakeSources, select_sleep_wake_sources};

pub(crate) const BOOT_BUTTON_WAKE_BIT: u32 = 1 << 0;
pub(crate) const RTC_INTERRUPT_WAKE_BIT: u32 = 1 << 5;
pub(crate) const POWER_BUTTON_WAKE_BIT: u32 = 1 << 18;
const WAKE_RELEASE_POLLS: u32 = 200;
const WAKE_RELEASE_POLL_MS: u32 = 50;

macro_rules! held_high_restorer {
    ($function:ident, $pin:ty) => {
        pub(crate) fn $function<'a>(pin: &'a mut $pin) -> Output<'a> {
            let configured = Output::new(pin.reborrow(), Level::High, OutputConfig::default());
            drop(configured);
            pin.rtc_set_config(false, false, RtcFunction::Rtc);
            pin.rtcio_pad_hold(false);
            Output::new(pin.reborrow(), Level::High, OutputConfig::default())
        }
    };
}

// Restore GPIO17's high output before releasing its retained pad state.
held_high_restorer!(restore_power_latch, GPIO17<'static>);

// Restore GPIO6's high output before releasing its retained pad state.
held_high_restorer!(restore_panel_power, GPIO6<'static>);

pub(crate) struct SleepResources {
    pub(crate) boot_button: GPIO0<'static>,
    pub(crate) rtc_interrupt: GPIO5<'static>,
    pub(crate) power_button: GPIO18<'static>,
    pub(crate) panel_power: GPIO6<'static>,
    pub(crate) power_latch: GPIO17<'static>,
    pub(crate) audio_power: GPIO42<'static>,
    pub(crate) low_power: LPWR<'static>,
}

impl SleepResources {
    /// Enter deep sleep until an adult holds BOOT to restart the firmware.
    pub(crate) fn sleep_until_restart(self) -> ! {
        self.sleep_ext1(SleepWakeSources {
            rtc_alarm: false,
            power_button: false,
            boot_button: true,
        })
    }

    /// Enter deep sleep with only the ESP32-S3 RTC timer as a wake source.
    pub(crate) fn sleep_with_timer(self, duration: Duration) -> ! {
        let Self {
            boot_button: _,
            rtc_interrupt: _,
            power_button: _,
            panel_power,
            power_latch,
            audio_power,
            low_power,
        } = self;
        retain_power_rails(panel_power, power_latch, audio_power);
        let timer = TimerWakeupSource::new(duration);
        let mut low_power = HalRtc::new(low_power);
        Delay::new().delay_ms(100);
        low_power.sleep_deep(&[&timer]);
    }

    /// Drop the board power latch and keep ESP wake sources disabled.
    pub(crate) fn power_off_for_storage(self) -> ! {
        let Self {
            boot_button: _,
            rtc_interrupt: _,
            power_button: _,
            panel_power,
            power_latch,
            audio_power,
            low_power,
        } = self;
        retain_storage_rails(panel_power, power_latch, audio_power);
        let mut low_power = HalRtc::new(low_power);
        low_power.sleep_deep(&[]);
    }

    /// Enter deep sleep until the RTC alarm, PWR, or BOOT becomes active-low.
    pub(crate) fn sleep(self) -> ! {
        self.sleep_ext1(SleepWakeSources {
            rtc_alarm: true,
            power_button: true,
            boot_button: true,
        })
    }

    /// Enter deep sleep until only the RTC alarm becomes active-low.
    ///
    /// The alarm-wake diagnostic uses this so a button press cannot be mistaken
    /// for an alarm wake.
    pub(crate) fn sleep_for_alarm(self) -> ! {
        self.sleep_ext1(SleepWakeSources {
            rtc_alarm: true,
            power_button: false,
            boot_button: false,
        })
    }

    /// Enter deep sleep until PWR or BOOT becomes active-low.
    pub(crate) fn sleep_for_setup(self) -> ! {
        self.sleep_ext1(SleepWakeSources {
            rtc_alarm: false,
            power_button: true,
            boot_button: true,
        })
    }

    fn sleep_ext1(self, requested: SleepWakeSources) -> ! {
        let Self {
            mut boot_button,
            mut rtc_interrupt,
            mut power_button,
            panel_power,
            power_latch,
            audio_power,
            low_power,
        } = self;
        restore_wake_pin(&mut boot_button);
        restore_wake_pin(&mut rtc_interrupt);
        restore_wake_pin(&mut power_button);
        let boot_input = Input::new(
            boot_button.reborrow(),
            InputConfig::default().with_pull(Pull::Up),
        );
        let rtc_input = Input::new(
            rtc_interrupt.reborrow(),
            InputConfig::default().with_pull(Pull::Up),
        );
        let power_input = Input::new(
            power_button.reborrow(),
            InputConfig::default().with_pull(Pull::Up),
        );
        let low_lines = || SleepWakeSources {
            rtc_alarm: requested.rtc_alarm && rtc_input.is_low(),
            power_button: requested.power_button && power_input.is_low(),
            boot_button: requested.boot_button && boot_input.is_low(),
        };
        let mut delay = Delay::new();
        for _ in 0..WAKE_RELEASE_POLLS {
            let low = low_lines();
            if !(low.rtc_alarm || low.power_button || low.boot_button) {
                break;
            }
            delay.delay_ms(WAKE_RELEASE_POLL_MS);
        }
        let low = low_lines();
        let sources = select_sleep_wake_sources(requested, low);
        if low.rtc_alarm || low.power_button || low.boot_button {
            esp_println::println!(
                "wake line remained low; requested={requested:?}; armed={sources:?}"
            );
        }
        drop(boot_input);
        drop(rtc_input);
        drop(power_input);

        retain_power_rails(panel_power, power_latch, audio_power);
        configure_wake_pin(&mut boot_button);
        configure_wake_pin(&mut rtc_interrupt);
        configure_wake_pin(&mut power_button);
        let mut low_power = HalRtc::new(low_power);
        // Move armed pins to the front so EXT1 receives one contiguous slice.
        let mut candidates: [(bool, &mut dyn RtcPin); 3] = [
            (sources.boot_button, &mut boot_button),
            (sources.rtc_alarm, &mut rtc_interrupt),
            (sources.power_button, &mut power_button),
        ];
        candidates.sort_unstable_by_key(|(armed, _)| !*armed);
        let armed_count = candidates.iter().filter(|(armed, _)| *armed).count();
        let mut wake_pins = candidates.map(|(_, pin)| pin);
        if armed_count == 0 {
            delay.delay_ms(100);
            low_power.sleep_deep(&[]);
        }
        let wake = Ext1WakeupSource::new(&mut wake_pins[..armed_count], WakeupLevel::Low);
        delay.delay_ms(100);
        low_power.sleep_deep(&[&wake]);
    }
}

pub(crate) fn ext1_wake_status() -> u32 {
    esp_hal::peripherals::LPWR::regs()
        .ext_wakeup1_status()
        .read()
        .ext_wakeup1_status()
        .bits()
}

pub(crate) fn restore_wake_pin(pin: &mut impl RtcPin) {
    pin.rtc_set_config(true, false, RtcFunction::Rtc);
    pin.rtcio_pad_hold(false);
}

fn configure_wake_pin(pin: &mut (impl RtcPin + RtcPinWithResistors)) {
    pin.rtcio_pullup(true);
    pin.rtcio_pulldown(false);
}

fn retain_power_rails(
    mut panel_power: GPIO6<'static>,
    mut power_latch: GPIO17<'static>,
    audio_power: GPIO42<'static>,
) {
    let panel_output = Output::new(panel_power.reborrow(), Level::High, OutputConfig::default());
    drop(panel_output);
    panel_power.rtcio_pad_hold(true);

    let latch_output = Output::new(power_latch.reborrow(), Level::High, OutputConfig::default());
    drop(latch_output);
    power_latch.rtcio_pad_hold(true);

    let _audio_power = Output::new(audio_power, Level::Low, OutputConfig::default());
    hold_audio_power_pad();
}

fn retain_storage_rails(
    mut panel_power: GPIO6<'static>,
    mut power_latch: GPIO17<'static>,
    audio_power: GPIO42<'static>,
) {
    let panel_output = Output::new(panel_power.reborrow(), Level::High, OutputConfig::default());
    drop(panel_output);
    panel_power.rtcio_pad_hold(true);

    let latch_output = Output::new(power_latch.reborrow(), Level::Low, OutputConfig::default());
    drop(latch_output);
    power_latch.rtcio_pad_hold(true);

    let _audio_power = Output::new(audio_power, Level::Low, OutputConfig::default());
    hold_audio_power_pad();
}
