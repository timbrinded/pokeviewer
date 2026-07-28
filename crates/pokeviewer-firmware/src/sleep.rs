//! V2 rail retention and active-low RTC deep-sleep boundary.

use embedded_hal::delay::DelayNs;
use esp_hal::{
    delay::Delay,
    gpio::{
        Input, InputConfig, Level, Output, OutputConfig, Pull, RtcFunction, RtcPin,
        RtcPinWithResistors,
    },
    peripherals::{GPIO5, GPIO6, GPIO17, GPIO42, LPWR},
    rtc_cntl::{
        Rtc as HalRtc,
        sleep::{Ext0WakeupSource, TimerWakeupSource, WakeupLevel},
    },
    time::Duration,
};
use pokeviewer_esp32s3_pad_hold::hold_audio_power_pad;

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

    pub(crate) fn sleep(self) -> ! {
        let Self {
            mut rtc_interrupt,
            panel_power,
            power_latch,
            audio_power,
            low_power,
        } = self;
        // Deep-sleep wake restarts the digital core, so the prior EXT0
        // source's Drop cannot restore this retained pad to the GPIO path.
        rtc_interrupt.rtc_set_config(true, false, RtcFunction::Rtc);
        let interrupt_input = Input::new(
            rtc_interrupt.reborrow(),
            InputConfig::default().with_pull(Pull::Up),
        );
        let rtc_interrupt_low = interrupt_input.is_low();
        if rtc_interrupt_low {
            esp_println::println!("sleep refused: RTC interrupt remained low");
            loop {
                core::hint::spin_loop();
            }
        }
        drop(interrupt_input);

        retain_power_rails(panel_power, power_latch, audio_power);
        rtc_interrupt.rtcio_pullup(true);
        rtc_interrupt.rtcio_pulldown(false);
        let wake = Ext0WakeupSource::new(rtc_interrupt, WakeupLevel::Low);
        let mut low_power = HalRtc::new(low_power);
        Delay::new().delay_ms(100);
        low_power.sleep_deep(&[&wake]);
    }
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
