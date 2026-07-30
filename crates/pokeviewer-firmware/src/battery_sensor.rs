//! Calibrated GPIO4 battery sampling for the supported V2 board.

use embedded_hal::delay::DelayNs;
use esp_hal::{
    analog::adc::{Adc, AdcCalCurve, AdcConfig, Attenuation},
    delay::Delay,
    peripherals::{ADC1, GPIO4},
};
use pokeviewer_core::{BATTERY_SAMPLE_COUNT, BatteryStatus, estimate_battery, filtered_battery_mv};
use portable_atomic::{AtomicU32, Ordering};

pub(crate) const BATTERY_VALID_DIAGNOSTIC_FLAG: u16 = 1 << 5;
pub(crate) const BATTERY_LOW_DIAGNOSTIC_FLAG: u16 = 1 << 6;

const RETAINED_MAGIC: u32 = 0x4254_0000;
const RETAINED_LOW: u32 = 1;
const RETAINED_MAGIC_MASK: u32 = !RETAINED_LOW;

#[esp_hal::ram(unstable(rtc_fast, persistent))]
static RETAINED_BATTERY_STATE: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BatteryObservation {
    pub(crate) status: BatteryStatus,
    pub(crate) diagnostic_flags: u16,
}

pub(crate) fn sample_battery(adc1: ADC1<'static>, gpio4: GPIO4<'static>) -> BatteryObservation {
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

    let previous = RETAINED_BATTERY_STATE.load(Ordering::Relaxed);
    let previous_low =
        previous & RETAINED_MAGIC_MASK == RETAINED_MAGIC && previous & RETAINED_LOW != 0;
    let estimate = estimate_battery(filtered_battery_mv(samples), previous_low);
    match estimate.status {
        BatteryStatus::Estimated { recharge, .. } => {
            RETAINED_BATTERY_STATE.store(RETAINED_MAGIC | u32::from(recharge), Ordering::Relaxed);
            BatteryObservation {
                status: estimate.status,
                diagnostic_flags: BATTERY_VALID_DIAGNOSTIC_FLAG
                    | if recharge {
                        BATTERY_LOW_DIAGNOSTIC_FLAG
                    } else {
                        0
                    },
            }
        }
        BatteryStatus::Unavailable => BatteryObservation {
            status: BatteryStatus::Unavailable,
            diagnostic_flags: 0,
        },
    }
}
