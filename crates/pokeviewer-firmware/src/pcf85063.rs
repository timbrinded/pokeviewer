//! PCF85063A implementation of the project RTC contract.

use embedded_hal_async::i2c::I2c;
use pcf85063a::{BitFlags, Control, Error, PCF85063, Register};
use time::Time;

use crate::{LocalDateTime, Rtc};

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
            .set_datetime(&datetime)
            .await
            .map_err(Pcf85063RtcError::Driver)
    }

    async fn configure_daily_alarm(&mut self) -> Result<(), Self::Error> {
        self.driver
            .disable_all_alarms()
            .await
            .map_err(Pcf85063RtcError::Driver)?;
        self.driver
            .clear_alarm_flag()
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
            .control_alarm_interrupt(Control::On)
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
