//! Bounded ESP32-S3 USB Serial/JTAG transport adapter.

use esp_hal::{Blocking, peripherals::USB_DEVICE, usb::usb_serial_jtag::UsbSerialJtag};
use pokeviewer_core::{FrameAccumulator, FrameError};

use crate::{ProtocolAction, Rtc, handle_protocol_request};

const MAX_RX_BYTES_PER_POLL: usize = 64;

/// USB protocol polling failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsbProtocolError {
    /// An incoming frame failed validation.
    InvalidFrame(FrameError),
    /// USB transmit failed.
    Transport,
}

/// Results from one bounded transport poll.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UsbPoll {
    /// Complete requests handled in this poll.
    pub handled: usize,
    /// Deferred runtime action, if a command requested one.
    pub action: ProtocolAction,
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
        allow_storage: bool,
    ) -> Result<UsbPoll, UsbProtocolError>
    where
        R: Rtc,
    {
        let mut result = UsbPoll::default();
        for _ in 0..MAX_RX_BYTES_PER_POLL {
            let Ok(byte) = self.usb.read_byte() else {
                break;
            };
            let Some(frame) = self.decoder.push(byte) else {
                continue;
            };
            let request = frame.map_err(UsbProtocolError::InvalidFrame)?;
            let outcome = handle_protocol_request(rtc, request, diagnostic_flags, allow_storage)
                .await
                .map_err(UsbProtocolError::InvalidFrame)?;
            self.usb
                .write(outcome.response.encode().as_bytes())
                .map_err(|_| UsbProtocolError::Transport)?;
            result.handled += 1;
            if outcome.action != ProtocolAction::None {
                result.action = outcome.action;
                break;
            }
        }
        Ok(result)
    }

    /// Discard a partial request after a provisioning timeout.
    pub fn reset_partial_frame(&mut self) {
        self.decoder.reset();
    }
}
