//! V2 rail retention and active-low RTC deep-sleep boundary.

use embedded_hal::delay::DelayNs;
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull, RtcPin},
    peripherals::{GPIO5, GPIO6, GPIO17, GPIO42, LPWR},
    rtc_cntl::{
        Rtc as HalRtc,
        sleep::{Ext0WakeupSource, WakeupLevel},
    },
};

pub(crate) struct SleepResources {
    pub(crate) rtc_interrupt: GPIO5<'static>,
    pub(crate) panel_power: GPIO6<'static>,
    pub(crate) power_latch: GPIO17<'static>,
    pub(crate) audio_power: GPIO42<'static>,
    pub(crate) low_power: LPWR<'static>,
}

impl SleepResources {
    pub(crate) fn sleep(self) -> ! {
        let mut rtc_interrupt = self.rtc_interrupt;
        let interrupt_input = Input::new(
            rtc_interrupt.reborrow(),
            InputConfig::default().with_pull(Pull::Up),
        );
        if interrupt_input.is_low() {
            esp_println::println!("sleep refused: RTC interrupt remained low");
            loop {
                core::hint::spin_loop();
            }
        }
        drop(interrupt_input);

        let mut panel_power = self.panel_power;
        let panel_output =
            Output::new(panel_power.reborrow(), Level::High, OutputConfig::default());
        drop(panel_output);
        panel_power.rtcio_pad_hold(true);

        let mut power_latch = self.power_latch;
        let latch_output =
            Output::new(power_latch.reborrow(), Level::High, OutputConfig::default());
        drop(latch_output);
        power_latch.rtcio_pad_hold(true);

        let _audio_power = Output::new(self.audio_power, Level::High, OutputConfig::default());
        let wake = Ext0WakeupSource::new(rtc_interrupt, WakeupLevel::Low);
        let mut low_power = HalRtc::new(self.low_power);
        Delay::new().delay_ms(100);
        low_power.sleep_deep(&[&wake]);
    }
}
