//! Bounded version-1 USB provisioning wire protocol.

use crate::LocalDateTime;

const MAGIC: &[u8; 4] = b"PKVW";
const HEADER_BYTES: usize = 10;
const CHECKSUM_BYTES: usize = 4;
const MAX_PAYLOAD_BYTES: usize = 16;
/// Maximum encoded protocol frame length.
pub const MAX_FRAME_BYTES: usize = HEADER_BYTES + MAX_PAYLOAD_BYTES + CHECKSUM_BYTES;
/// Supported USB protocol version.
pub const PROTOCOL_VERSION: u8 = 1;
/// Product version reported by v1 firmware over USB.
pub const FIRMWARE_VERSION: [u8; 3] = [1, 1, 0];
/// Firmware can negotiate protocol metadata.
pub const CAP_HANDSHAKE: u8 = 1 << 0;
/// Firmware can read the RTC.
pub const CAP_READ_RTC: u8 = 1 << 1;
/// Firmware can set the RTC.
pub const CAP_SET_RTC: u8 = 1 << 2;
/// Firmware can report diagnostics.
pub const CAP_DIAGNOSTICS: u8 = 1 << 3;
/// Firmware can invalidate the RTC and enter storage mode.
pub const CAP_ENTER_STORAGE: u8 = 1 << 4;
/// Capabilities implemented by this firmware version.
pub const CAPABILITIES: u8 =
    CAP_HANDSHAKE | CAP_READ_RTC | CAP_SET_RTC | CAP_DIAGNOSTICS | CAP_ENTER_STORAGE;

/// Direction encoded in a protocol frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
    /// Host-to-device request.
    Request = 0,
    /// Device-to-host response.
    Response = 1,
}

/// Stable command IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    /// Negotiate protocol and firmware capabilities.
    Handshake = 1,
    /// Read local RTC fields.
    ReadRtc = 2,
    /// Validate, set, and read back local RTC fields.
    SetRtc = 3,
    /// Read a bounded diagnostic bit field.
    Diagnostics = 4,
    /// Invalidate the RTC and enter no-wake storage mode.
    EnterStorage = 5,
}

/// Stable response status codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    /// Command completed.
    Ok = 0,
    /// Request payload or calendar fields are invalid.
    InvalidRequest = 1,
    /// Command is unsupported by this firmware.
    UnsupportedCommand = 2,
    /// Device-side operation failed.
    DeviceError = 3,
}

/// A validated, allocation-free protocol frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolFrame {
    /// Host-selected correlation identifier.
    pub request_id: u16,
    /// Request or response direction.
    pub kind: FrameKind,
    /// Command identifier.
    pub command: Command,
    payload: [u8; MAX_PAYLOAD_BYTES],
    payload_len: u8,
}

impl ProtocolFrame {
    /// Construct a validated frame from a bounded payload.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::PayloadTooLong`] for payloads over 16 bytes.
    pub fn new(
        request_id: u16,
        kind: FrameKind,
        command: Command,
        payload: &[u8],
    ) -> Result<Self, FrameError> {
        let payload_len = u8::try_from(payload.len()).map_err(|_| FrameError::PayloadTooLong)?;
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(FrameError::PayloadTooLong);
        }
        let mut bytes = [0; MAX_PAYLOAD_BYTES];
        bytes[..payload.len()].copy_from_slice(payload);
        Ok(Self {
            request_id,
            kind,
            command,
            payload: bytes,
            payload_len,
        })
    }

    /// Borrow the exact payload.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload[..usize::from(self.payload_len)]
    }

    /// Encode a complete checksummed frame.
    #[must_use]
    pub fn encode(&self) -> EncodedFrame {
        let payload_len = self.payload().len();
        let checksum_offset = HEADER_BYTES + payload_len;
        let mut bytes = [0; MAX_FRAME_BYTES];
        bytes[..4].copy_from_slice(MAGIC);
        bytes[4] = PROTOCOL_VERSION;
        bytes[5] = self.kind as u8;
        bytes[6] = self.command as u8;
        bytes[7] = self.payload_len;
        bytes[8..10].copy_from_slice(&self.request_id.to_le_bytes());
        bytes[HEADER_BYTES..checksum_offset].copy_from_slice(self.payload());
        let checksum = crc32fast::hash(&bytes[..checksum_offset]);
        bytes[checksum_offset..checksum_offset + CHECKSUM_BYTES]
            .copy_from_slice(&checksum.to_le_bytes());
        EncodedFrame {
            bytes,
            len: checksum_offset + CHECKSUM_BYTES,
        }
    }

    /// Decode and validate one exact frame.
    ///
    /// # Errors
    ///
    /// Returns a bounded [`FrameError`] for framing, compatibility, or checksum
    /// failures.
    pub fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        if bytes.len() < HEADER_BYTES + CHECKSUM_BYTES {
            return Err(FrameError::Truncated);
        }
        if bytes.get(..4) != Some(MAGIC) {
            return Err(FrameError::InvalidMagic);
        }
        if bytes[4] != PROTOCOL_VERSION {
            return Err(FrameError::UnsupportedVersion);
        }
        let payload_len = usize::from(bytes[7]);
        if payload_len > MAX_PAYLOAD_BYTES {
            return Err(FrameError::PayloadTooLong);
        }
        let expected_len = HEADER_BYTES + payload_len + CHECKSUM_BYTES;
        if bytes.len() != expected_len {
            return Err(FrameError::InvalidLength);
        }
        let expected_checksum = u32::from_le_bytes(
            bytes[expected_len - 4..]
                .try_into()
                .map_err(|_| FrameError::Truncated)?,
        );
        if crc32fast::hash(&bytes[..expected_len - 4]) != expected_checksum {
            return Err(FrameError::InvalidChecksum);
        }
        Self::new(
            u16::from_le_bytes([bytes[8], bytes[9]]),
            decode_kind(bytes[5])?,
            decode_command(bytes[6])?,
            &bytes[HEADER_BYTES..HEADER_BYTES + payload_len],
        )
    }
}

/// Owned encoded frame with a bounded valid prefix.
#[derive(Clone, Copy)]
pub struct EncodedFrame {
    bytes: [u8; MAX_FRAME_BYTES],
    len: usize,
}

impl EncodedFrame {
    /// Borrow the encoded frame prefix.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

/// Incremental bounded decoder for firmware RX FIFOs.
pub struct FrameAccumulator {
    bytes: [u8; MAX_FRAME_BYTES],
    len: usize,
    expected_len: Option<usize>,
}

impl FrameAccumulator {
    /// Create an empty decoder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: [0; MAX_FRAME_BYTES],
            len: 0,
            expected_len: None,
        }
    }

    /// Push one byte, returning one complete frame or bounded error.
    pub fn push(&mut self, byte: u8) -> Option<Result<ProtocolFrame, FrameError>> {
        if self.len < MAGIC.len() && byte != MAGIC[self.len] {
            self.len = usize::from(byte == MAGIC[0]);
            self.bytes[0] = byte;
            return None;
        }
        self.bytes[self.len] = byte;
        self.len += 1;
        if self.len == HEADER_BYTES {
            let payload_len = usize::from(self.bytes[7]);
            if payload_len > MAX_PAYLOAD_BYTES {
                self.reset();
                return Some(Err(FrameError::PayloadTooLong));
            }
            self.expected_len = Some(HEADER_BYTES + payload_len + CHECKSUM_BYTES);
        }
        if self.expected_len == Some(self.len) {
            let result = ProtocolFrame::decode(&self.bytes[..self.len]);
            self.reset();
            return Some(result);
        }
        None
    }

    /// Discard a partial frame after a transport timeout.
    pub fn reset(&mut self) {
        self.len = 0;
        self.expected_len = None;
    }
}

impl Default for FrameAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Bounded frame validation failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    /// Not enough bytes for a frame.
    Truncated,
    /// Magic prefix is absent.
    InvalidMagic,
    /// Protocol version is unsupported.
    UnsupportedVersion,
    /// Direction code is unsupported.
    InvalidKind,
    /// Command ID is unsupported.
    InvalidCommand,
    /// Payload exceeds the fixed bound.
    PayloadTooLong,
    /// Total length does not match the payload length.
    InvalidLength,
    /// CRC-32 does not match.
    InvalidChecksum,
    /// RTC payload is malformed or invalid.
    InvalidDateTime,
}

/// Encode seven explicit local-wall-clock fields.
#[must_use]
pub fn encode_datetime(datetime: LocalDateTime) -> [u8; 7] {
    let year = datetime.year.to_le_bytes();
    [
        year[0],
        year[1],
        datetime.month,
        datetime.day,
        datetime.hour,
        datetime.minute,
        datetime.second,
    ]
}

/// Decode and validate seven explicit local-wall-clock fields.
///
/// # Errors
///
/// Returns [`FrameError::InvalidDateTime`] for invalid length or calendar
/// fields.
pub fn decode_datetime(payload: &[u8]) -> Result<LocalDateTime, FrameError> {
    if payload.len() != 7 {
        return Err(FrameError::InvalidDateTime);
    }
    LocalDateTime {
        year: u16::from_le_bytes([payload[0], payload[1]]),
        month: payload[2],
        day: payload[3],
        hour: payload[4],
        minute: payload[5],
        second: payload[6],
    }
    .validate()
    .map_err(|_| FrameError::InvalidDateTime)
}

fn decode_kind(value: u8) -> Result<FrameKind, FrameError> {
    match value {
        0 => Ok(FrameKind::Request),
        1 => Ok(FrameKind::Response),
        _ => Err(FrameError::InvalidKind),
    }
}

fn decode_command(value: u8) -> Result<Command, FrameError> {
    match value {
        1 => Ok(Command::Handshake),
        2 => Ok(Command::ReadRtc),
        3 => Ok(Command::SetRtc),
        4 => Ok(Command::Diagnostics),
        5 => Ok(Command::EnterStorage),
        _ => Err(FrameError::InvalidCommand),
    }
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
