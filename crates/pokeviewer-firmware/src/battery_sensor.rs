//! Calibrated GPIO4 battery sampling for the supported V2 board.

use embedded_hal::delay::DelayNs;
use esp_hal::{
    analog::adc::{Adc, AdcCalCurve, AdcConfig, Attenuation},
    delay::Delay,
    peripherals::{ADC1, GPIO4},
};
use pokeviewer_core::{BATTERY_SAMPLE_COUNT, filtered_battery_mv};

pub(crate) fn sample_battery_mv(adc1: ADC1<'static>, gpio4: GPIO4<'static>) -> u16 {
    let mut config = AdcConfig::new();
    let mut pin =
        config.enable_pin_with_cal::<_, AdcCalCurve<ADC1<'static>>>(gpio4, Attenuation::_11dB);
    let mut adc = Adc::new(adc1, config);
    let mut delay = Delay::new();

    delay.delay_ms(50);
    let _discarded = adc.read_blocking(&mut pin);
    let mut samples = [0; BATTERY_SAMPLE_COUNT];
    for (index, sample) in samples.iter_mut().enumerate() {
        *sample = adc.read_blocking(&mut pin);
        if index + 1 != BATTERY_SAMPLE_COUNT {
            delay.delay_ms(2);
        }
    }

    filtered_battery_mv(samples)
}
