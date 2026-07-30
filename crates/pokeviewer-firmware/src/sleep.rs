//! V2 rail retention and active-low EXT1 deep-sleep boundary.

use embedded_hal::delay::DelayNs;
use esp_hal::{
    delay::Delay,
    gpio::{
        Input, InputConfig, Level, Output, OutputConfig, Pull, RtcFunction, RtcPin,
        RtcPinWithResistors,
    },
    peripherals::{GPIO5, GPIO6, GPIO17, GPIO18, GPIO42, LPWR},
    rtc_cntl::{
        Rtc as HalRtc,
        sleep::{Ext1WakeupSource, TimerWakeupSource, WakeupLevel},
    },
    time::Duration,
};
use pokeviewer_esp32s3_pad_hold::hold_audio_power_pad;

pub(crate) const RTC_INTERRUPT_WAKE_BIT: u32 = 1 << 5;
pub(crate) const POWER_BUTTON_WAKE_BIT: u32 = 1 << 18;

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
    pub(crate) rtc_interrupt: GPIO5<'static>,
    pub(crate) power_button: GPIO18<'static>,
    pub(crate) panel_power: GPIO6<'static>,
    pub(crate) power_latch: GPIO17<'static>,
    pub(crate) audio_power: GPIO42<'static>,
    pub(crate) low_power: LPWR<'static>,
}

impl SleepResources {
    /// Enter deep sleep indefinitely, with external reset as the only exit.
    pub(crate) fn sleep_without_wake(self) -> ! {
        let Self {
            rtc_interrupt: _,
            power_button: _,
            panel_power,
            power_latch,
            audio_power,
            low_power,
        } = self;
        retain_power_rails(panel_power, power_latch, audio_power);
        let mut low_power = HalRtc::new(low_power);
        Delay::new().delay_ms(100);
        low_power.sleep_deep(&[]);
    }

    /// Enter deep sleep with only the ESP32-S3 RTC timer as a wake source.
    pub(crate) fn sleep_with_timer(self, duration: Duration) -> ! {
        let Self {
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

    /// Enter deep sleep until either the RTC alarm or PWR becomes active-low.
    pub(crate) fn sleep(self) -> ! {
        self.sleep_ext1(true)
    }

    /// Enter deep sleep until PWR becomes active-low.
    pub(crate) fn sleep_for_setup(self) -> ! {
        self.sleep_ext1(false)
    }

    fn sleep_ext1(self, include_rtc: bool) -> ! {
        let Self {
            mut rtc_interrupt,
            mut power_button,
            panel_power,
            power_latch,
            audio_power,
            low_power,
        } = self;
        restore_wake_pin(&mut rtc_interrupt);
        restore_wake_pin(&mut power_button);
        let rtc_input = Input::new(
            rtc_interrupt.reborrow(),
            InputConfig::default().with_pull(Pull::Up),
        );
        let power_input = Input::new(
            power_button.reborrow(),
            InputConfig::default().with_pull(Pull::Up),
        );
        if include_rtc && rtc_input.is_low() {
            esp_println::println!("sleep refused: RTC interrupt remained low");
            loop {
                core::hint::spin_loop();
            }
        }
        if power_input.is_low() {
            esp_println::println!("sleep refused: PWR remained low");
            loop {
                core::hint::spin_loop();
            }
        }
        drop(rtc_input);
        drop(power_input);

        retain_power_rails(panel_power, power_latch, audio_power);
        configure_wake_pin(&mut rtc_interrupt);
        configure_wake_pin(&mut power_button);
        let mut wake_pins: [&mut dyn RtcPin; 2];
        let wake_slice: &mut [&mut dyn RtcPin] = if include_rtc {
            wake_pins = [&mut rtc_interrupt, &mut power_button];
            &mut wake_pins
        } else {
            wake_pins = [&mut power_button, &mut rtc_interrupt];
            &mut wake_pins[..1]
        };
        let wake = Ext1WakeupSource::new(wake_slice, WakeupLevel::Low);
        let mut low_power = HalRtc::new(low_power);
        Delay::new().delay_ms(100);
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
