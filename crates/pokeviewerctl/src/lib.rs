#![forbid(unsafe_code)]
#![doc = "Linux USB provisioning and diagnostics command-line interface."]

use std::{
    io::{self, Read},
    time::{Duration, Instant},
};

use pokeviewer_core::{
    Command, FrameAccumulator, FrameKind, LocalDateTime, ProtocolFrame, Status, decode_datetime,
    encode_datetime,
};
use serial2::SerialPort;

const BAUD_RATE: u32 = 115_200;
const TIMEOUT: Duration = Duration::from_secs(2);

/// Parse CLI arguments and execute one explicit command.
///
/// # Errors
///
/// Returns a privacy-bounded human-readable error suitable for stderr.
pub fn run(arguments: impl IntoIterator<Item = String>) -> Result<String, String> {
    let arguments: Vec<_> = arguments.into_iter().collect();
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(usage());
    };
    if command == "--version" {
        if arguments.len() != 1 {
            return Err(usage());
        }
        return Ok(format!("pokeviewerctl {}", env!("CARGO_PKG_VERSION")));
    }
    if command == "list" {
        if arguments.len() != 1 {
            return Err(usage());
        }
        return list_devices();
    }
    let device = option(&arguments, "--device").ok_or_else(usage)?;
    let request = match command {
        "info" => request(Command::Handshake, &[])?,
        "get-rtc" => request(Command::ReadRtc, &[])?,
        "diagnostics" => request(Command::Diagnostics, &[])?,
        "set-rtc" => {
            let value = option(&arguments, "--datetime").ok_or_else(usage)?;
            let datetime = parse_datetime(value)?;
            request(Command::SetRtc, &encode_datetime(datetime))?
        }
        _ => return Err(usage()),
    };
    let response = exchange(device, request)?;
    format_response(response)
}

fn list_devices() -> Result<String, String> {
    let ports = SerialPort::available_ports()
        .map_err(|_| "failed to enumerate serial devices".to_owned())?;
    if ports.is_empty() {
        Ok("no serial devices found".to_owned())
    } else {
        Ok(ports
            .iter()
            .map(|path| path.to_string_lossy())
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

fn exchange(device: &str, request: ProtocolFrame) -> Result<ProtocolFrame, String> {
    let mut port = SerialPort::open(device, BAUD_RATE)
        .map_err(|_| "failed to open selected serial device".to_owned())?;
    port.set_read_timeout(TIMEOUT)
        .map_err(|_| "failed to configure serial read timeout".to_owned())?;
    port.set_write_timeout(TIMEOUT)
        .map_err(|_| "failed to configure serial write timeout".to_owned())?;
    port.write_all(request.encode().as_bytes())
        .map_err(|_| "failed to write request".to_owned())?;
    port.flush()
        .map_err(|_| "failed to flush request".to_owned())?;
    read_response(&mut port)
}

fn read_response(reader: &mut impl Read) -> Result<ProtocolFrame, String> {
    let mut decoder = FrameAccumulator::new();
    let mut byte = [0];
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if Instant::now() >= deadline {
            return Err("timed out waiting for device response".to_owned());
        }
        reader
            .read_exact(&mut byte)
            .map_err(|error| read_error(&error))?;
        if let Some(result) = decoder.push(byte[0]) {
            return result.map_err(|error| format!("invalid device response: {error:?}"));
        }
    }
}

fn read_error(error: &io::Error) -> String {
    if matches!(
        error.kind(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
    ) {
        "timed out waiting for device response".to_owned()
    } else {
        "failed to read device response".to_owned()
    }
}

fn request(command: Command, payload: &[u8]) -> Result<ProtocolFrame, String> {
    ProtocolFrame::new(1, FrameKind::Request, command, payload)
        .map_err(|error| format!("invalid request: {error:?}"))
}

fn format_response(response: ProtocolFrame) -> Result<String, String> {
    if response.kind != FrameKind::Response || response.request_id != 1 {
        return Err("device returned an unrelated response".to_owned());
    }
    let Some((&status, payload)) = response.payload().split_first() else {
        return Err("device returned an empty response".to_owned());
    };
    if status != Status::Ok as u8 {
        return Err(format!("device rejected command with status {status}"));
    }
    match response.command {
        Command::Handshake => {
            if payload.len() != 4 {
                return Err("device returned an invalid handshake".to_owned());
            }
            Ok(format!(
                "protocol=1 firmware={}.{}.{} capabilities=0x{:02x}",
                payload[0], payload[1], payload[2], payload[3]
            ))
        }
        Command::ReadRtc | Command::SetRtc => {
            let datetime = decode_datetime(payload)
                .map_err(|_| "device returned invalid RTC fields".to_owned())?;
            Ok(format_datetime(datetime))
        }
        Command::Diagnostics => {
            if payload.len() != 2 {
                return Err("device returned invalid diagnostics".to_owned());
            }
            Ok(format!(
                "diagnostic_flags=0x{:04x}",
                u16::from_le_bytes([payload[0], payload[1]])
            ))
        }
    }
}

fn parse_datetime(value: &str) -> Result<LocalDateTime, String> {
    let bytes = value.as_bytes();
    if bytes.len() != 19
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return Err("datetime must use YYYY-MM-DDTHH:MM:SS".to_owned());
    }
    let datetime = LocalDateTime {
        year: decimal(&bytes[0..4])?,
        month: u8::try_from(decimal(&bytes[5..7])?)
            .map_err(|_| "datetime field is out of range")?,
        day: u8::try_from(decimal(&bytes[8..10])?).map_err(|_| "datetime field is out of range")?,
        hour: u8::try_from(decimal(&bytes[11..13])?)
            .map_err(|_| "datetime field is out of range")?,
        minute: u8::try_from(decimal(&bytes[14..16])?)
            .map_err(|_| "datetime field is out of range")?,
        second: u8::try_from(decimal(&bytes[17..19])?)
            .map_err(|_| "datetime field is out of range")?,
    };
    datetime
        .validate()
        .map_err(|_| "datetime is outside the RTC calendar range".to_owned())
}

fn decimal(bytes: &[u8]) -> Result<u16, String> {
    bytes.iter().try_fold(0_u16, |value, byte| {
        if byte.is_ascii_digit() {
            Ok(value * 10 + u16::from(byte - b'0'))
        } else {
            Err("datetime contains a non-digit".to_owned())
        }
    })
}

fn format_datetime(value: LocalDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        value.year, value.month, value.day, value.hour, value.minute, value.second
    )
}

fn option<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    arguments
        .windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}

fn usage() -> String {
    "usage: pokeviewerctl --version | list | <info|get-rtc|diagnostics> --device PATH | set-rtc --device PATH --datetime YYYY-MM-DDTHH:MM:SS".to_owned()
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor, Read};

    use pokeviewer_core::{Command, FIRMWARE_VERSION, FrameKind, ProtocolFrame, Status};

    use super::{format_response, parse_datetime, read_response, run};

    const NOW: &str = "2026-07-27T19:05:09";

    #[test]
    fn package_and_protocol_versions_cannot_drift() {
        assert_eq!(
            env!("CARGO_PKG_VERSION"),
            format!(
                "{}.{}.{}",
                FIRMWARE_VERSION[0], FIRMWARE_VERSION[1], FIRMWARE_VERSION[2]
            )
        );
        assert_eq!(
            run(["--version".to_owned()]).unwrap(),
            "pokeviewerctl 1.0.0"
        );
    }

    #[test]
    fn strict_datetime_parser_rejects_invalid_calendar_fields() {
        assert_eq!(super::format_datetime(parse_datetime(NOW).unwrap()), NOW);
        assert!(parse_datetime("2025-02-29T12:00:00").is_err());
        assert!(parse_datetime("2026-07-27 19:05:09").is_err());
    }

    #[test]
    fn response_reader_skips_noise_and_validates_frame() {
        let frame = ProtocolFrame::new(
            1,
            FrameKind::Response,
            Command::Handshake,
            &[
                Status::Ok as u8,
                FIRMWARE_VERSION[0],
                FIRMWARE_VERSION[1],
                FIRMWARE_VERSION[2],
                0x0f,
            ],
        )
        .unwrap()
        .encode();
        let mut bytes = b"boot log\n".to_vec();
        bytes.extend_from_slice(frame.as_bytes());
        let response = read_response(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(
            format_response(response).unwrap(),
            "protocol=1 firmware=1.0.0 capabilities=0x0f"
        );
    }

    struct TimeoutReader;

    impl Read for TimeoutReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::TimedOut, "test timeout"))
        }
    }

    #[test]
    fn transport_timeout_is_clear_and_private() {
        assert_eq!(
            read_response(&mut TimeoutReader).unwrap_err(),
            "timed out waiting for device response"
        );
    }
}
