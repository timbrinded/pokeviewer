//! Project-owned boundary around the PCF85063A driver.

/// RTC adapter whose public surface is independent of the selected driver.
pub struct Pcf85063Rtc<I2c> {
    driver: pcf85063a::PCF85063<I2c>,
}

impl<I2c> Pcf85063Rtc<I2c>
where
    I2c: embedded_hal_async::i2c::I2c,
{
    /// Wrap an I²C bus using the exact-pinned PCF85063A driver.
    pub fn new(i2c: I2c) -> Self {
        Self {
            driver: pcf85063a::PCF85063::new(i2c),
        }
    }

    /// Return the owned I²C bus.
    pub fn release(self) -> I2c {
        self.driver.destroy()
    }
}
