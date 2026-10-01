//! Sleep command for the unused always-powered SHTC3 sensor.

use embedded_hal_async::i2c::I2c;

const DEVICE_ADDRESS: u8 = 0x70;
const SLEEP_COMMAND: [u8; 2] = [0xb0, 0x98];

/// Move the SHTC3 from its 45 µA power-up idle state to 0.3 µA sleep.
///
/// The sensor shares the always-on 3V3 rail and stays asleep until power is
/// removed or it receives a wakeup command. A sleeping SHTC3 may not
/// acknowledge another sleep command, so callers treat failure as harmless.
pub(crate) async fn sleep_humidity_sensor<I2cBus>(i2c: &mut I2cBus) -> Result<(), I2cBus::Error>
where
    I2cBus: I2c,
{
    i2c.write(DEVICE_ADDRESS, &SLEEP_COMMAND).await
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::sleep_humidity_sensor;
    use crate::test_i2c::{RecordingI2c, block_on_ready};

    #[test]
    fn sends_only_the_datasheet_sleep_command() {
        let mut i2c = RecordingI2c::new();

        block_on_ready(sleep_humidity_sensor(&mut i2c)).unwrap();

        assert_eq!(
            i2c.attempted_writes,
            std::vec![(0x70, std::vec![0xb0, 0x98])]
        );
    }
}
