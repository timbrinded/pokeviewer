#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![doc = "Audited ESP32-S3 GPIO42 deep-sleep hold shim for Pokeviewer."]

#[cfg(any(target_arch = "xtensa", test))]
const GPIO42_DIGITAL_HOLD_MASK: u32 = 1 << (42 - 21);

/// Hold GPIO42 at its configured level through ESP32-S3 deep sleep.
///
/// Configure GPIO42 as a low output before calling this function.
#[cfg(target_arch = "xtensa")]
pub fn hold_audio_power_pad() {
    critical_section::with(|_| {
        set_audio_power_pad_hold(true);
        esp_hal::peripherals::LPWR::regs()
            .dig_iso()
            .modify(|_, writer| {
                writer
                    .dg_pad_force_unhold()
                    .clear_bit()
                    .dg_pad_autohold_en()
                    .set_bit()
            });
    });
}

/// Release the GPIO42 hold after its low output configuration is restored.
#[cfg(target_arch = "xtensa")]
pub fn release_audio_power_pad() {
    critical_section::with(|_| {
        esp_hal::peripherals::LPWR::regs()
            .dig_iso()
            .modify(|_, writer| writer.dg_pad_autohold_en().clear_bit());
        set_audio_power_pad_hold(false);
    });
}

#[cfg(target_arch = "xtensa")]
#[allow(
    unsafe_code,
    reason = "the PAC marks every raw 32-bit field value unsafe; this helper changes only the documented GPIO42 hold bit"
)]
fn set_audio_power_pad_hold(held: bool) {
    esp_hal::peripherals::LPWR::regs()
        .dig_pad_hold()
        .modify(|reader, writer| {
            let bits = update_audio_power_hold_bits(reader.dig_pad_hold().bits(), held);
            // SAFETY: `bits` preserves every field bit except the documented
            // ESP32-S3 GPIO42 digital-pad hold bit.
            unsafe { writer.dig_pad_hold().bits(bits) }
        });
}

#[cfg(any(target_arch = "xtensa", test))]
const fn update_audio_power_hold_bits(bits: u32, held: bool) -> u32 {
    if held {
        bits | GPIO42_DIGITAL_HOLD_MASK
    } else {
        bits & !GPIO42_DIGITAL_HOLD_MASK
    }
}

#[cfg(test)]
mod tests {
    use super::{GPIO42_DIGITAL_HOLD_MASK, update_audio_power_hold_bits};

    #[test]
    fn changes_only_gpio42_hold_bit() {
        assert_eq!(GPIO42_DIGITAL_HOLD_MASK, 1 << 21);
        let other_bits = 0xa5a5_5a5a & !GPIO42_DIGITAL_HOLD_MASK;
        assert_eq!(
            update_audio_power_hold_bits(other_bits, true),
            other_bits | GPIO42_DIGITAL_HOLD_MASK
        );
        assert_eq!(
            update_audio_power_hold_bits(u32::MAX, false),
            !GPIO42_DIGITAL_HOLD_MASK
        );
    }
}
