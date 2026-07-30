//! PCF85063A implementation of the project RTC contract.

use embedded_hal_async::i2c::I2c;
use pcf85063a::{BitFlags, Control, Error, PCF85063, Register};
use time::Time;

use crate::{LocalDateTime, Rtc};

const CONTROL_2_IDLE: u8 = 0x07;
const CONTROL_2_ALARM_ENABLED: u8 = 0x87;
const TIMER_DISABLED_LOW_POWER: u8 = 0x18;
const SOFTWARE_RESET: u8 = 0x58;

/// Errors surfaced by the PCF85063A adapter.
#[derive(Debug)]
pub enum Pcf85063RtcError<BusError> {
    /// The RTC reported that its oscillator stopped.
    OscillatorStopped,
    /// A supplied datetime is outside the supported calendar range.
    InvalidDateTime,
    /// The underlying driver or I²C bus failed.
    Driver(Error<BusError>),
}

/// Project adapter around the exact-pinned PCF85063A driver.
pub struct Pcf85063Rtc<I2cBus> {
    driver: PCF85063<I2cBus>,
}

impl<I2cBus> Pcf85063Rtc<I2cBus>
where
    I2cBus: I2c,
{
    /// Create an RTC adapter from an owned asynchronous I²C bus.
    pub fn new(i2c: I2cBus) -> Self {
        Self {
            driver: PCF85063::new(i2c),
        }
    }

    /// Return the owned I²C bus.
    pub fn release(self) -> I2cBus {
        self.driver.destroy()
    }
}

impl<I2cBus> Rtc for Pcf85063Rtc<I2cBus>
where
    I2cBus: I2c,
{
    type Error = Pcf85063RtcError<I2cBus::Error>;

    async fn read_datetime(&mut self) -> Result<LocalDateTime, Self::Error> {
        let seconds = self
            .driver
            .read_register(Register::SECONDS)
            .await
            .map_err(Pcf85063RtcError::Driver)?;
        if seconds & 0x80 != 0 {
            return Err(Pcf85063RtcError::OscillatorStopped);
        }

        let datetime = self
            .driver
            .get_datetime()
            .await
            .map_err(Pcf85063RtcError::Driver)?;
        Ok(LocalDateTime {
            year: u16::try_from(datetime.year()).map_err(|_| Pcf85063RtcError::InvalidDateTime)?,
            month: datetime.month() as u8,
            day: datetime.day(),
            hour: datetime.hour(),
            minute: datetime.minute(),
            second: datetime.second(),
        })
    }

    async fn set_datetime(&mut self, datetime: LocalDateTime) -> Result<(), Self::Error> {
        let datetime = datetime
            .to_primitive()
            .map_err(|_| Pcf85063RtcError::InvalidDateTime)?;
        self.driver
            .clear_register_bit_flag(Register::CONTROL_1, BitFlags::MODE_12_24)
            .await
            .map_err(Pcf85063RtcError::Driver)?;
        self.driver
            .start_clock()
            .await
            .map_err(Pcf85063RtcError::Driver)?;
        self.driver
            .set_datetime(&datetime)
            .await
            .map_err(Pcf85063RtcError::Driver)
    }

    async fn invalidate(&mut self) -> Result<(), Self::Error> {
        self.driver
            .write_register(Register::CONTROL_1, SOFTWARE_RESET)
            .await
            .map_err(Pcf85063RtcError::Driver)?;
        match self.read_datetime().await {
            Err(Pcf85063RtcError::OscillatorStopped) => Ok(()),
            Ok(_) => Err(Pcf85063RtcError::OscillatorStopped),
            Err(error) => Err(error),
        }
    }

    async fn configure_daily_alarm(&mut self) -> Result<(), Self::Error> {
        self.driver
            .disable_all_alarms()
            .await
            .map_err(Pcf85063RtcError::Driver)?;
        self.driver
            .clear_register_bit_flag(Register::CONTROL_1, BitFlags::CIE)
            .await
            .map_err(Pcf85063RtcError::Driver)?;
        self.driver
            .write_register(Register::TIMER_MODE, TIMER_DISABLED_LOW_POWER)
            .await
            .map_err(Pcf85063RtcError::Driver)?;
        self.driver
            .write_register(Register::CONTROL_2, CONTROL_2_IDLE)
            .await
            .map_err(Pcf85063RtcError::Driver)?;
        let alarm_time = Time::from_hms(7, 0, 0).map_err(|_| Pcf85063RtcError::InvalidDateTime)?;
        self.driver
            .set_alarm_time(alarm_time)
            .await
            .map_err(Pcf85063RtcError::Driver)?;
        self.driver
            .control_alarm_seconds(Control::On)
            .await
            .map_err(Pcf85063RtcError::Driver)?;
        self.driver
            .control_alarm_minutes(Control::On)
            .await
            .map_err(Pcf85063RtcError::Driver)?;
        self.driver
            .control_alarm_hours(Control::On)
            .await
            .map_err(Pcf85063RtcError::Driver)?;
        self.driver
            .write_register(Register::CONTROL_2, CONTROL_2_ALARM_ENABLED)
            .await
            .map_err(Pcf85063RtcError::Driver)
    }

    async fn alarm_pending(&mut self) -> Result<bool, Self::Error> {
        self.driver
            .get_alarm_flag()
            .await
            .map_err(Pcf85063RtcError::Driver)
    }

    async fn clear_alarm(&mut self) -> Result<(), Self::Error> {
        self.driver
            .clear_register_bit_flag(Register::CONTROL_2, BitFlags::AF)
            .await
            .map_err(Pcf85063RtcError::Driver)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{
        BitFlags, CONTROL_2_ALARM_ENABLED, CONTROL_2_IDLE, Pcf85063Rtc, Pcf85063RtcError, Register,
        SOFTWARE_RESET, TIMER_DISABLED_LOW_POWER,
    };
    use crate::{
        LocalDateTime, Rtc,
        test_i2c::{RecordingI2c, block_on_ready},
    };

    const DEVICE_ADDRESS: u8 = 0x51;
    const LEAP_DAY: LocalDateTime = LocalDateTime {
        year: 2024,
        month: 2,
        day: 29,
        hour: 23,
        minute: 58,
        second: 59,
    };

    #[test]
    fn datetime_lifecycle_forces_24_hour_running_mode_and_round_trips() {
        let mut i2c = RecordingI2c::new();
        i2c.set_register(Register::CONTROL_1, BitFlags::MODE_12_24 | BitFlags::STOP);
        let mut rtc = Pcf85063Rtc::new(i2c);

        block_on_ready(rtc.set_datetime(LEAP_DAY)).unwrap();
        assert_eq!(block_on_ready(rtc.read_datetime()).unwrap(), LEAP_DAY);

        let i2c = rtc.release();
        assert_eq!(i2c.register(Register::CONTROL_1), 0);
        assert_eq!(
            i2c.attempted_writes,
            std::vec![
                (DEVICE_ADDRESS, std::vec![Register::CONTROL_1]),
                (
                    DEVICE_ADDRESS,
                    std::vec![Register::CONTROL_1, BitFlags::STOP]
                ),
                (DEVICE_ADDRESS, std::vec![Register::CONTROL_1]),
                (DEVICE_ADDRESS, std::vec![Register::CONTROL_1, 0x00]),
                (
                    DEVICE_ADDRESS,
                    std::vec![Register::SECONDS, 0x59, 0x58, 0x23, 0x29, 0x04, 0x02, 0x24,],
                ),
                (DEVICE_ADDRESS, std::vec![Register::SECONDS]),
                (DEVICE_ADDRESS, std::vec![Register::SECONDS]),
            ]
        );
    }

    #[test]
    fn oscillator_stop_short_circuits_before_calendar_read() {
        let mut i2c = RecordingI2c::new();
        i2c.set_register(Register::SECONDS, 0x80);
        let mut rtc = Pcf85063Rtc::new(i2c);

        let result = block_on_ready(rtc.read_datetime());

        assert!(matches!(result, Err(Pcf85063RtcError::OscillatorStopped)));
        let i2c = rtc.release();
        assert_eq!(i2c.attempts(), 1);
        assert_eq!(
            i2c.attempted_writes,
            std::vec![(DEVICE_ADDRESS, std::vec![Register::SECONDS])]
        );
    }

    #[test]
    fn daily_alarm_lifecycle_clears_other_sources_and_preserves_enable() {
        let mut i2c = RecordingI2c::new();
        i2c.set_register(Register::CONTROL_1, BitFlags::CIE);
        i2c.set_register(
            Register::CONTROL_2,
            BitFlags::AIE | BitFlags::AF | BitFlags::MI | BitFlags::HMI | BitFlags::TF,
        );
        let mut rtc = Pcf85063Rtc::new(i2c);

        block_on_ready(rtc.configure_daily_alarm()).unwrap();
        assert!(!block_on_ready(rtc.alarm_pending()).unwrap());

        let mut i2c = rtc.release();
        assert_eq!(i2c.register(Register::CONTROL_1), 0);
        assert_eq!(TIMER_DISABLED_LOW_POWER, 0x18);
        assert_eq!(CONTROL_2_IDLE, 0x07);
        assert_eq!(CONTROL_2_ALARM_ENABLED, 0x87);
        assert_eq!(i2c.register(Register::TIMER_MODE), 0x18);
        assert_eq!(i2c.register(Register::CONTROL_2), 0x87);
        assert_eq!(i2c.register(Register::SECOND_ALARM), 0x00);
        assert_eq!(i2c.register(Register::MINUTE_ALARM), 0x00);
        assert_eq!(i2c.register(Register::HOUR_ALARM), 0x07);
        assert_eq!(i2c.register(Register::DAY_ALARM), BitFlags::AE);
        assert_eq!(i2c.register(Register::WEEKDAY_ALARM), BitFlags::AE);
        assert_eq!(
            i2c.register_writes,
            std::vec![
                (DEVICE_ADDRESS, Register::SECOND_ALARM, BitFlags::AE),
                (DEVICE_ADDRESS, Register::MINUTE_ALARM, BitFlags::AE),
                (DEVICE_ADDRESS, Register::HOUR_ALARM, BitFlags::AE),
                (DEVICE_ADDRESS, Register::DAY_ALARM, BitFlags::AE),
                (DEVICE_ADDRESS, Register::WEEKDAY_ALARM, BitFlags::AE),
                (DEVICE_ADDRESS, Register::CONTROL_1, 0x00),
                (
                    DEVICE_ADDRESS,
                    Register::TIMER_MODE,
                    TIMER_DISABLED_LOW_POWER,
                ),
                (DEVICE_ADDRESS, Register::CONTROL_2, CONTROL_2_IDLE),
                (DEVICE_ADDRESS, Register::SECOND_ALARM, BitFlags::AE),
                (DEVICE_ADDRESS, Register::MINUTE_ALARM, BitFlags::AE),
                (DEVICE_ADDRESS, Register::HOUR_ALARM, 0x87),
                (DEVICE_ADDRESS, Register::SECOND_ALARM, 0x00),
                (DEVICE_ADDRESS, Register::MINUTE_ALARM, 0x00),
                (DEVICE_ADDRESS, Register::HOUR_ALARM, 0x07),
                (DEVICE_ADDRESS, Register::CONTROL_2, CONTROL_2_ALARM_ENABLED,),
            ]
        );

        i2c.set_register(Register::CONTROL_2, CONTROL_2_ALARM_ENABLED | BitFlags::AF);
        let mut rtc = Pcf85063Rtc::new(i2c);
        assert!(block_on_ready(rtc.alarm_pending()).unwrap());
        block_on_ready(rtc.clear_alarm()).unwrap();
        assert!(!block_on_ready(rtc.alarm_pending()).unwrap());

        let i2c = rtc.release();
        assert_eq!(i2c.register(Register::CONTROL_2), CONTROL_2_ALARM_ENABLED);
        assert_eq!(
            i2c.register_writes.last(),
            Some(&(DEVICE_ADDRESS, Register::CONTROL_2, CONTROL_2_ALARM_ENABLED))
        );
    }

    #[test]
    fn software_reset_invalidates_time_and_verifies_the_oscillator_stop_flag() {
        let mut i2c = RecordingI2c::new();
        i2c.set_register(Register::SECONDS, 0x80);
        let mut rtc = Pcf85063Rtc::new(i2c);

        block_on_ready(rtc.invalidate()).unwrap();

        let i2c = rtc.release();
        assert_eq!(i2c.register(Register::CONTROL_1), SOFTWARE_RESET);
        assert_eq!(i2c.register(Register::SECONDS), 0x80);
        assert_eq!(
            i2c.attempted_writes,
            std::vec![
                (
                    DEVICE_ADDRESS,
                    std::vec![Register::CONTROL_1, SOFTWARE_RESET]
                ),
                (DEVICE_ADDRESS, std::vec![Register::SECONDS]),
            ]
        );
    }
}
