//! Software-suspend boundary for the powered shared-bus ES8311 codec.

use embedded_hal_async::i2c::I2c;

const DEVICE_ADDRESS: u8 = 0x18;
const SUSPEND_SEQUENCE: [(u8, u8); 15] = [
    (0x32, 0x00),
    (0x17, 0x00),
    (0x0e, 0xff),
    (0x12, 0x02),
    (0x14, 0x00),
    (0x0d, 0xfa),
    (0x15, 0x00),
    (0x02, 0x10),
    (0x00, 0x00),
    (0x00, 0x1f),
    (0x01, 0x30),
    (0x01, 0x00),
    (0x45, 0x00),
    (0x0d, 0xfc),
    (0x02, 0x00),
];

/// Apply the vendor suspend sequence while keeping the I²C control port alive.
pub(crate) async fn suspend_audio_codec<I2cBus>(i2c: &mut I2cBus) -> Result<(), I2cBus::Error>
where
    I2cBus: I2c,
{
    let (&(first_register, first_value), remaining) = SUSPEND_SEQUENCE
        .split_first()
        .expect("sequence is not empty");
    if i2c
        .write(DEVICE_ADDRESS, &[first_register, first_value])
        .await
        .is_err()
    {
        // The vendor driver documents occasional failure of the codec's first
        // I²C transaction after power-up. Retry that transaction once only.
        i2c.write(DEVICE_ADDRESS, &[first_register, first_value])
            .await?;
    }
    for &(register, value) in remaining {
        i2c.write(DEVICE_ADDRESS, &[register, value]).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{DEVICE_ADDRESS, SUSPEND_SEQUENCE, suspend_audio_codec};
    use crate::test_i2c::{RecordingI2c, TestI2cError, block_on_ready};
    use std::vec::Vec;

    const EXPECTED_SUSPEND_SEQUENCE: [(u8, u8); 15] = [
        (0x32, 0x00),
        (0x17, 0x00),
        (0x0e, 0xff),
        (0x12, 0x02),
        (0x14, 0x00),
        (0x0d, 0xfa),
        (0x15, 0x00),
        (0x02, 0x10),
        (0x00, 0x00),
        (0x00, 0x1f),
        (0x01, 0x30),
        (0x01, 0x00),
        (0x45, 0x00),
        (0x0d, 0xfc),
        (0x02, 0x00),
    ];

    fn expected_attempts() -> Vec<(u8, Vec<u8>)> {
        EXPECTED_SUSPEND_SEQUENCE
            .iter()
            .map(|&(register, value)| (0x18, std::vec![register, value]))
            .collect()
    }

    fn expected_register_writes() -> Vec<(u8, u8, u8)> {
        EXPECTED_SUSPEND_SEQUENCE
            .iter()
            .map(|&(register, value)| (0x18, register, value))
            .collect()
    }

    #[test]
    fn vendor_suspend_sequence_is_exact() {
        let mut i2c = RecordingI2c::new();

        block_on_ready(suspend_audio_codec(&mut i2c)).unwrap();

        assert_eq!(DEVICE_ADDRESS, 0x18);
        assert_eq!(SUSPEND_SEQUENCE, EXPECTED_SUSPEND_SEQUENCE);
        assert_eq!(i2c.attempted_writes, expected_attempts());
        assert_eq!(i2c.register_writes, expected_register_writes());
        assert_eq!(i2c.attempts(), SUSPEND_SEQUENCE.len());
    }

    #[test]
    fn first_transaction_is_retried_exactly_once() {
        let mut i2c = RecordingI2c::new().with_fail_attempts(&[1]);

        block_on_ready(suspend_audio_codec(&mut i2c)).unwrap();

        let mut expected = expected_attempts();
        expected.insert(0, (0x18, std::vec![0x32, 0x00]));
        assert_eq!(i2c.attempts(), SUSPEND_SEQUENCE.len() + 1);
        assert_eq!(i2c.attempted_writes, expected);
        assert_eq!(i2c.register_writes, expected_register_writes());
    }

    #[test]
    fn second_first_transaction_failure_stops_the_sequence() {
        let mut i2c = RecordingI2c::new().with_fail_attempts(&[1, 2]);

        let error = block_on_ready(suspend_audio_codec(&mut i2c)).unwrap_err();

        assert_eq!(error, TestI2cError::Injected);
        assert_eq!(i2c.attempts(), 2);
        assert_eq!(
            i2c.attempted_writes,
            std::vec![(0x18, std::vec![0x32, 0x00]), (0x18, std::vec![0x32, 0x00]),]
        );
        assert!(i2c.register_writes.is_empty());
    }

    #[test]
    fn later_transaction_failure_is_not_retried() {
        let mut i2c = RecordingI2c::new().with_fail_attempts(&[5]);

        let error = block_on_ready(suspend_audio_codec(&mut i2c)).unwrap_err();

        assert_eq!(error, TestI2cError::Injected);
        assert_eq!(i2c.attempts(), 5);
        assert_eq!(i2c.attempted_writes, expected_attempts()[..5]);
        assert_eq!(i2c.register_writes, expected_register_writes()[..4]);
    }
}
