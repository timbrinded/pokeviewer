extern crate std;

use super::{
    Command, FrameAccumulator, FrameError, FrameKind, MAX_FRAME_BYTES, PROTOCOL_VERSION,
    ProtocolFrame, decode_datetime, encode_datetime,
};
use crate::LocalDateTime;

const NOW: LocalDateTime = LocalDateTime {
    year: 2026,
    month: 7,
    day: 27,
    hour: 18,
    minute: 5,
    second: 9,
};

#[test]
fn every_command_round_trips_with_request_id_and_payload() {
    for command in [
        Command::Handshake,
        Command::ReadRtc,
        Command::SetRtc,
        Command::Diagnostics,
    ] {
        let frame =
            ProtocolFrame::new(0x1234, FrameKind::Request, command, &encode_datetime(NOW)).unwrap();
        assert_eq!(
            ProtocolFrame::decode(frame.encode().as_bytes()).unwrap(),
            frame
        );
    }
}

#[test]
fn corruption_truncation_versions_and_lengths_are_rejected() {
    let encoded = ProtocolFrame::new(7, FrameKind::Request, Command::Handshake, &[])
        .unwrap()
        .encode();
    let mut corrupted = encoded.as_bytes().to_vec();
    corrupted[8] ^= 1;
    assert_eq!(
        ProtocolFrame::decode(&corrupted),
        Err(FrameError::InvalidChecksum)
    );
    assert_eq!(
        ProtocolFrame::decode(&encoded.as_bytes()[..8]),
        Err(FrameError::Truncated)
    );
    let mut version = encoded.as_bytes().to_vec();
    version[4] = PROTOCOL_VERSION + 1;
    assert_eq!(
        ProtocolFrame::decode(&version),
        Err(FrameError::UnsupportedVersion)
    );
    let mut length = encoded.as_bytes().to_vec();
    length[7] = 1;
    assert_eq!(
        ProtocolFrame::decode(&length),
        Err(FrameError::InvalidLength)
    );
    assert_eq!(
        ProtocolFrame::new(
            1,
            FrameKind::Request,
            Command::Handshake,
            &[0; MAX_FRAME_BYTES]
        ),
        Err(FrameError::PayloadTooLong)
    );
}

#[test]
fn calendar_payload_is_explicit_and_validated() {
    assert_eq!(decode_datetime(&encode_datetime(NOW)), Ok(NOW));
    let mut invalid = encode_datetime(NOW);
    invalid[2] = 13;
    assert_eq!(decode_datetime(&invalid), Err(FrameError::InvalidDateTime));
    assert_eq!(
        decode_datetime(&invalid[..6]),
        Err(FrameError::InvalidDateTime)
    );
}

#[test]
fn accumulator_handles_noise_partial_frames_and_timeout_reset() {
    let frame = ProtocolFrame::new(9, FrameKind::Request, Command::ReadRtc, &[])
        .unwrap()
        .encode();
    let mut decoder = FrameAccumulator::new();
    for byte in b"noise" {
        assert!(decoder.push(*byte).is_none());
    }
    let mut completed = None;
    for byte in frame.as_bytes() {
        completed = decoder.push(*byte).or(completed);
    }
    assert_eq!(completed.unwrap().unwrap().request_id, 9);

    for byte in &frame.as_bytes()[..6] {
        assert!(decoder.push(*byte).is_none());
    }
    decoder.reset();
    for byte in frame.as_bytes() {
        completed = decoder.push(*byte).or(completed);
    }
    assert!(completed.is_some());
}
