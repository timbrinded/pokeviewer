//! Bounded ESP32-S3 USB Serial/JTAG transport adapter.

use esp_hal::{Blocking, peripherals::USB_DEVICE, usb_serial_jtag::UsbSerialJtag};
use pokeviewer_core::{FrameAccumulator, FrameError};

use crate::{Rtc, handle_protocol_request};

const MAX_RX_BYTES_PER_POLL: usize = 64;

/// USB protocol polling failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsbProtocolError {
    /// An incoming frame failed validation.
    InvalidFrame(FrameError),
    /// USB transmit failed.
    Transport,
}

/// Owned CDC-ACM transport with a fixed incremental decoder.
pub struct UsbProtocolTransport {
    usb: UsbSerialJtag<'static, Blocking>,
    decoder: FrameAccumulator,
}

impl UsbProtocolTransport {
    /// Claim the ESP32-S3 hardwired USB Serial/JTAG peripheral.
    #[must_use]
    pub fn new(peripheral: USB_DEVICE<'static>) -> Self {
        Self {
            usb: UsbSerialJtag::new(peripheral),
            decoder: FrameAccumulator::new(),
        }
    }

    /// Drain at most one USB packet and execute every complete request in it.
    ///
    /// The fixed poll bound prevents a noisy host from monopolizing wake time.
    ///
    /// # Errors
    ///
    /// Returns a bounded framing or transport failure.
    pub async fn poll<R>(
        &mut self,
        rtc: &mut R,
        diagnostic_flags: u16,
    ) -> Result<usize, UsbProtocolError>
    where
        R: Rtc,
    {
        let mut handled = 0;
        for _ in 0..MAX_RX_BYTES_PER_POLL {
            let Ok(byte) = self.usb.read_byte() else {
                break;
            };
            let Some(frame) = self.decoder.push(byte) else {
                continue;
            };
            let request = frame.map_err(UsbProtocolError::InvalidFrame)?;
            let response = handle_protocol_request(rtc, request, diagnostic_flags)
                .await
                .map_err(UsbProtocolError::InvalidFrame)?;
            self.usb
                .write(response.encode().as_bytes())
                .map_err(|_| UsbProtocolError::Transport)?;
            handled += 1;
        }
        Ok(handled)
    }

    /// Discard a partial request after a provisioning timeout.
    pub fn reset_partial_frame(&mut self) {
        self.decoder.reset();
    }
}
