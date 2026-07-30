#![forbid(unsafe_code)]
#![doc = "Linux USB provisioning and diagnostics command-line interface."]

use std::{
    io::{self, Read, Write},
    thread,
    time::{Duration, Instant},
};

use jiff::Zoned;
use pokeviewer_core::{
    CAP_ENTER_STORAGE, Command, FrameAccumulator, FrameKind, LocalDateTime, ProtocolFrame, Status,
    decode_datetime, encode_datetime,
};
use serial2::SerialPort;

const BAUD_RATE: u32 = 115_200;
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);
const STARTUP_RESPONSE_TIMEOUT: Duration = Duration::from_secs(6);
const STARTUP_RETRY_TIMEOUT: Duration = Duration::from_millis(500);
const PARENT_COMMAND_RESPONSE_TIMEOUT: Duration = Duration::from_secs(12);
const DEVICE_WAIT_TIMEOUT: Duration = Duration::from_mins(1);
const DEVICE_WAIT_INTERVAL: Duration = Duration::from_millis(250);
const RESPONSE_TIMEOUT_ERROR: &str = "timed out waiting for device response";

#[derive(Debug, Default, PartialEq, Eq)]
struct Options {
    device: Option<String>,
    datetime: Option<String>,
    now: bool,
    wait_for_device: bool,
    confirm_time_loss: bool,
}

#[derive(Clone, Copy)]
struct Handshake {
    firmware: [u8; 3],
    capabilities: u8,
}

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

    let options = parse_options(&arguments)?;
    validate_options(command, &options)?;
    let device = options.device.as_deref().ok_or_else(usage)?;
    let mut port = open_device(device, options.wait_for_device)?;
    let handshake = start_session(&mut port, options.wait_for_device)?;
    if command == "info" {
        return Ok(format!(
            "protocol=1 firmware={}.{}.{} capabilities=0x{:02x}",
            handshake.firmware[0],
            handshake.firmware[1],
            handshake.firmware[2],
            handshake.capabilities,
        ));
    }

    let (request_command, payload) = match command {
        "get-rtc" => (Command::ReadRtc, None),
        "diagnostics" => (Command::Diagnostics, None),
        "set-rtc" => {
            let datetime = if options.now {
                local_now()?
            } else {
                parse_datetime(options.datetime.as_deref().ok_or_else(usage)?)?
            };
            (Command::SetRtc, Some(encode_datetime(datetime)))
        }
        "enter-storage" => {
            if handshake.capabilities & CAP_ENTER_STORAGE == 0 {
                return Err("device firmware does not support storage mode".to_owned());
            }
            (Command::EnterStorage, None)
        }
        _ => return Err(usage()),
    };
    let response = exchange_command(
        &mut port,
        options.wait_for_device,
        request_command,
        payload.as_ref().map_or(&[], <[u8; 7]>::as_slice),
    )?;
    format_command_response(response, 2)
}

fn parse_options(arguments: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--device" => {
                index += 1;
                let value = arguments.get(index).ok_or_else(usage)?;
                if options.device.replace(value.clone()).is_some() {
                    return Err(usage());
                }
            }
            "--datetime" => {
                index += 1;
                let value = arguments.get(index).ok_or_else(usage)?;
                if options.datetime.replace(value.clone()).is_some() {
                    return Err(usage());
                }
            }
            "--now" if !options.now => options.now = true,
            "--wait-for-device" if !options.wait_for_device => options.wait_for_device = true,
            "--confirm-time-loss" if !options.confirm_time_loss => {
                options.confirm_time_loss = true;
            }
            _ => return Err(usage()),
        }
        index += 1;
    }
    Ok(options)
}

fn validate_options(command: &str, options: &Options) -> Result<(), String> {
    if options.device.is_none() {
        return Err(usage());
    }
    match command {
        "info" | "get-rtc" | "diagnostics"
            if options.datetime.is_none() && !options.now && !options.confirm_time_loss =>
        {
            Ok(())
        }
        "set-rtc" if options.datetime.is_some() != options.now && !options.confirm_time_loss => {
            Ok(())
        }
        "enter-storage"
            if options.datetime.is_none() && !options.now && options.confirm_time_loss =>
        {
            Ok(())
        }
        _ => Err(usage()),
    }
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

fn open_device(device: &str, wait_for_device: bool) -> Result<SerialPort, String> {
    let deadline = Instant::now() + DEVICE_WAIT_TIMEOUT;
    loop {
        match SerialPort::open(device, BAUD_RATE) {
            Ok(port) => return Ok(port),
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                return Err("permission denied for selected serial device".to_owned());
            }
            Err(error)
                if wait_for_device
                    && error.kind() == io::ErrorKind::NotFound
                    && Instant::now() < deadline =>
            {
                thread::sleep(DEVICE_WAIT_INTERVAL);
            }
            Err(error)
                if wait_for_device
                    && error.kind() == io::ErrorKind::NotFound
                    && Instant::now() >= deadline =>
            {
                return Err("timed out waiting for selected serial device".to_owned());
            }
            Err(_) => return Err("failed to open selected serial device".to_owned()),
        }
    }
}

fn start_session(port: &mut SerialPort, allow_startup_delay: bool) -> Result<Handshake, String> {
    port.discard_input_buffer()
        .map_err(|_| "failed to clear stale serial input".to_owned())?;
    let timeout = if allow_startup_delay {
        STARTUP_RESPONSE_TIMEOUT
    } else {
        RESPONSE_TIMEOUT
    };
    let response = if allow_startup_delay {
        exchange_until_ready(port, 1, Command::Handshake, &[], timeout)?
    } else {
        configure_port(port, timeout)?;
        exchange(port, 1, Command::Handshake, &[], timeout)?
    };
    parse_handshake(&response)
}

fn exchange_until_ready(
    port: &mut SerialPort,
    request_id: u16,
    command: Command,
    payload: &[u8],
    timeout: Duration,
) -> Result<ProtocolFrame, String> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(RESPONSE_TIMEOUT_ERROR.to_owned());
        }
        let attempt_timeout = remaining.min(STARTUP_RETRY_TIMEOUT);
        configure_port(port, attempt_timeout)?;
        match exchange(port, request_id, command, payload, attempt_timeout) {
            Ok(response) => return Ok(response),
            Err(error) if error == RESPONSE_TIMEOUT_ERROR && Instant::now() < deadline => {}
            Err(error) => return Err(error),
        }
    }
}

fn exchange_command(
    port: &mut SerialPort,
    allow_startup_delay: bool,
    command: Command,
    payload: &[u8],
) -> Result<ProtocolFrame, String> {
    let timeout = if allow_startup_delay {
        PARENT_COMMAND_RESPONSE_TIMEOUT
    } else {
        RESPONSE_TIMEOUT
    };
    configure_port(port, timeout)?;
    exchange(port, 2, command, payload, timeout)
}

fn configure_port(port: &mut SerialPort, read_timeout: Duration) -> Result<(), String> {
    port.set_read_timeout(read_timeout)
        .map_err(|_| "failed to configure serial read timeout".to_owned())?;
    port.set_write_timeout(RESPONSE_TIMEOUT)
        .map_err(|_| "failed to configure serial write timeout".to_owned())
}

fn exchange(
    port: &mut SerialPort,
    request_id: u16,
    command: Command,
    payload: &[u8],
    response_timeout: Duration,
) -> Result<ProtocolFrame, String> {
    let request = ProtocolFrame::new(request_id, FrameKind::Request, command, payload)
        .map_err(|error| format!("invalid request: {error:?}"))?;
    port.write_all(request.encode().as_bytes())
        .map_err(|_| "failed to write request".to_owned())?;
    port.flush()
        .map_err(|_| "failed to flush request".to_owned())?;
    read_matching_response(port, request_id, command, response_timeout)
}

fn read_matching_response(
    reader: &mut impl Read,
    request_id: u16,
    command: Command,
    timeout: Duration,
) -> Result<ProtocolFrame, String> {
    let deadline = Instant::now() + timeout;
    loop {
        let response = read_response_until(reader, deadline)?;
        if response.kind == FrameKind::Response
            && response.request_id == request_id
            && response.command == command
        {
            return Ok(response);
        }
    }
}

#[cfg(test)]
fn read_response(reader: &mut impl Read, timeout: Duration) -> Result<ProtocolFrame, String> {
    read_response_until(reader, Instant::now() + timeout)
}

fn read_response_until(reader: &mut impl Read, deadline: Instant) -> Result<ProtocolFrame, String> {
    let mut decoder = FrameAccumulator::new();
    let mut byte = [0];
    loop {
        if Instant::now() >= deadline {
            return Err(RESPONSE_TIMEOUT_ERROR.to_owned());
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
        RESPONSE_TIMEOUT_ERROR.to_owned()
    } else {
        "failed to read device response".to_owned()
    }
}

fn parse_handshake(response: &ProtocolFrame) -> Result<Handshake, String> {
    let payload = successful_payload(response)?;
    if payload.len() != 4 {
        return Err("device returned an invalid handshake".to_owned());
    }
    Ok(Handshake {
        firmware: [payload[0], payload[1], payload[2]],
        capabilities: payload[3],
    })
}

fn format_command_response(response: ProtocolFrame, request_id: u16) -> Result<String, String> {
    if response.request_id != request_id {
        return Err("device returned an unrelated response".to_owned());
    }
    let command = response.command;
    let payload = successful_payload(&response)?;
    match command {
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
        Command::EnterStorage if payload.is_empty() => {
            Ok("storage mode accepted; RTC time was cleared".to_owned())
        }
        Command::Handshake | Command::EnterStorage => {
            Err("device returned an invalid command response".to_owned())
        }
    }
}

fn successful_payload(response: &ProtocolFrame) -> Result<&[u8], String> {
    let payload = response.payload();
    let Some((&status, payload)) = payload.split_first() else {
        return Err("device returned an empty response".to_owned());
    };
    if status != Status::Ok as u8 {
        return Err(format!("device rejected command with status {status}"));
    }
    Ok(payload)
}

fn local_now() -> Result<LocalDateTime, String> {
    let now = Zoned::now();
    LocalDateTime {
        year: u16::try_from(now.year()).map_err(|_| "local year is outside the RTC range")?,
        month: u8::try_from(now.month()).map_err(|_| "local month is outside the RTC range")?,
        day: u8::try_from(now.day()).map_err(|_| "local day is outside the RTC range")?,
        hour: u8::try_from(now.hour()).map_err(|_| "local hour is outside the RTC range")?,
        minute: u8::try_from(now.minute()).map_err(|_| "local minute is outside the RTC range")?,
        second: u8::try_from(now.second()).map_err(|_| "local second is outside the RTC range")?,
    }
    .validate()
    .map_err(|_| "local time is outside the RTC calendar range".to_owned())
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

fn usage() -> String {
    "usage: pokeviewerctl --version | list | <info|get-rtc|diagnostics> --device PATH [--wait-for-device] | set-rtc --device PATH <--now|--datetime YYYY-MM-DDTHH:MM:SS> [--wait-for-device] | enter-storage --device PATH --confirm-time-loss [--wait-for-device]".to_owned()
}

#[cfg(test)]
mod tests {
    use std::{
        io::{self, Cursor, Read},
        thread,
        time::Duration,
    };

    use pokeviewer_core::{
        CAPABILITIES, Command, FIRMWARE_VERSION, FrameKind, ProtocolFrame, Status,
    };
    use serial2::SerialPort;

    use super::{
        Options, RESPONSE_TIMEOUT, exchange_command, format_command_response, parse_datetime,
        parse_handshake, parse_options, read_matching_response, read_response, run, start_session,
        validate_options,
    };

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
            "pokeviewerctl 1.1.0"
        );
    }

    #[test]
    fn strict_datetime_parser_rejects_invalid_calendar_fields() {
        assert_eq!(super::format_datetime(parse_datetime(NOW).unwrap()), NOW);
        assert!(parse_datetime("2025-02-29T12:00:00").is_err());
        assert!(parse_datetime("2026-07-27 19:05:09").is_err());
    }

    #[test]
    fn command_options_are_explicit_and_mutually_exclusive() {
        let explicit = vec![
            "set-rtc".to_owned(),
            "--device".to_owned(),
            "/dev/ttyACM0".to_owned(),
            "--datetime".to_owned(),
            NOW.to_owned(),
            "--wait-for-device".to_owned(),
        ];
        let options = parse_options(&explicit).unwrap();
        assert_eq!(
            options,
            Options {
                device: Some("/dev/ttyACM0".to_owned()),
                datetime: Some(NOW.to_owned()),
                wait_for_device: true,
                ..Options::default()
            }
        );
        validate_options("set-rtc", &options).unwrap();

        let both = Options {
            device: Some("/dev/ttyACM0".to_owned()),
            datetime: Some(NOW.to_owned()),
            now: true,
            ..Options::default()
        };
        assert!(validate_options("set-rtc", &both).is_err());
        assert!(
            validate_options(
                "enter-storage",
                &Options {
                    device: Some("/dev/ttyACM0".to_owned()),
                    ..Options::default()
                }
            )
            .is_err()
        );
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
                CAPABILITIES,
            ],
        )
        .unwrap()
        .encode();
        let mut bytes = b"boot log\n".to_vec();
        bytes.extend_from_slice(frame.as_bytes());
        let response = read_response(&mut Cursor::new(bytes), RESPONSE_TIMEOUT).unwrap();
        let handshake = parse_handshake(&response).unwrap();
        assert_eq!(handshake.firmware, FIRMWARE_VERSION);
        assert_eq!(handshake.capabilities, CAPABILITIES);
    }

    #[test]
    fn parent_startup_allows_firmware_hold_before_handshake() {
        let (mut host, mut device) = SerialPort::pair().unwrap();
        device.set_read_timeout(Duration::from_secs(1)).unwrap();
        device.set_write_timeout(Duration::from_secs(1)).unwrap();

        let responder = thread::spawn(move || {
            let early_request = read_response(&mut device, RESPONSE_TIMEOUT).unwrap();
            assert_eq!(early_request.kind, FrameKind::Request);
            assert_eq!(early_request.command, Command::Handshake);
            thread::sleep(Duration::from_millis(2_200));

            let request = read_response(&mut device, RESPONSE_TIMEOUT).unwrap();
            assert_eq!(request.kind, FrameKind::Request);
            assert_eq!(request.command, Command::Handshake);
            let response = ProtocolFrame::new(
                request.request_id,
                FrameKind::Response,
                request.command,
                &[
                    Status::Ok as u8,
                    FIRMWARE_VERSION[0],
                    FIRMWARE_VERSION[1],
                    FIRMWARE_VERSION[2],
                    CAPABILITIES,
                ],
            )
            .unwrap()
            .encode();
            device.write_all(response.as_bytes()).unwrap();
        });

        let result = start_session(&mut host, true);
        responder.join().unwrap();
        let handshake = result.unwrap();
        assert_eq!(handshake.firmware, FIRMWARE_VERSION);
        assert_eq!(handshake.capabilities, CAPABILITIES);
    }

    #[test]
    fn parent_command_allows_setup_screen_refresh_before_response() {
        let (mut host, mut device) = SerialPort::pair().unwrap();
        device.set_read_timeout(Duration::from_secs(1)).unwrap();
        device.set_write_timeout(Duration::from_secs(1)).unwrap();

        let responder = thread::spawn(move || {
            let handshake = read_response(&mut device, RESPONSE_TIMEOUT).unwrap();
            let handshake_response = ProtocolFrame::new(
                handshake.request_id,
                FrameKind::Response,
                handshake.command,
                &[
                    Status::Ok as u8,
                    FIRMWARE_VERSION[0],
                    FIRMWARE_VERSION[1],
                    FIRMWARE_VERSION[2],
                    CAPABILITIES,
                ],
            )
            .unwrap()
            .encode();
            device.write_all(handshake_response.as_bytes()).unwrap();

            let command = read_response(&mut device, RESPONSE_TIMEOUT).unwrap();
            assert_eq!(command.command, Command::Diagnostics);
            thread::sleep(Duration::from_millis(2_200));
            let command_response = ProtocolFrame::new(
                command.request_id,
                FrameKind::Response,
                command.command,
                &[Status::Ok as u8, 0x20, 0x00],
            )
            .unwrap()
            .encode();
            device.write_all(command_response.as_bytes()).unwrap();
        });

        start_session(&mut host, true).unwrap();
        let result = exchange_command(&mut host, true, Command::Diagnostics, &[]);
        responder.join().unwrap();
        let response = result.unwrap();
        assert_eq!(response.command, Command::Diagnostics);
    }

    #[test]
    fn matching_response_skips_a_stale_frame() {
        let stale = ProtocolFrame::new(
            1,
            FrameKind::Response,
            Command::Handshake,
            &[
                Status::Ok as u8,
                FIRMWARE_VERSION[0],
                FIRMWARE_VERSION[1],
                FIRMWARE_VERSION[2],
                CAPABILITIES,
            ],
        )
        .unwrap()
        .encode();
        let expected = ProtocolFrame::new(
            2,
            FrameKind::Response,
            Command::Diagnostics,
            &[Status::Ok as u8, 0x20, 0x00],
        )
        .unwrap()
        .encode();
        let mut bytes = stale.as_bytes().to_vec();
        bytes.extend_from_slice(expected.as_bytes());

        let response = read_matching_response(
            &mut Cursor::new(bytes),
            2,
            Command::Diagnostics,
            RESPONSE_TIMEOUT,
        )
        .unwrap();

        assert_eq!(response.request_id, 2);
        assert_eq!(response.command, Command::Diagnostics);
    }

    #[test]
    fn storage_response_is_bounded() {
        let response = ProtocolFrame::new(
            2,
            FrameKind::Response,
            Command::EnterStorage,
            &[Status::Ok as u8],
        )
        .unwrap();
        assert_eq!(
            format_command_response(response, 2).unwrap(),
            "storage mode accepted; RTC time was cleared"
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
            read_response(&mut TimeoutReader, RESPONSE_TIMEOUT).unwrap_err(),
            "timed out waiting for device response"
        );
    }
}
