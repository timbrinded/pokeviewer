//! Bounded command handling independent of the USB transport.

use pokeviewer_core::{
    CAPABILITIES, Command, FIRMWARE_VERSION, FrameError, FrameKind, ProtocolFrame, Status,
    decode_datetime, encode_datetime,
};

use crate::Rtc;

/// Runtime work requested by a successfully handled protocol command.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProtocolAction {
    /// Continue the current session.
    #[default]
    None,
    /// The RTC was set successfully.
    RtcSet,
    /// A validated storage-mode request is ready for runtime execution.
    EnterStorage,
}

/// Response and deferred runtime action for one request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolOutcome {
    /// Response that must be sent before any deferred action.
    pub response: ProtocolFrame,
    /// Runtime action to execute after the response is sent.
    pub action: ProtocolAction,
}

/// Validate and execute one protocol request.
///
/// Calendar and compatibility validation occurs before any RTC mutation.
///
/// # Errors
///
/// Returns [`FrameError`] only if the bounded response cannot be constructed.
pub async fn handle_protocol_request<R>(
    rtc: &mut R,
    request: ProtocolFrame,
    diagnostic_flags: u16,
    allow_storage: bool,
) -> Result<ProtocolOutcome, FrameError>
where
    R: Rtc,
{
    if request.kind != FrameKind::Request {
        return outcome(
            request,
            &[Status::InvalidRequest as u8],
            ProtocolAction::None,
        );
    }
    if request.command != Command::SetRtc && !request.payload().is_empty() {
        return outcome(
            request,
            &[Status::InvalidRequest as u8],
            ProtocolAction::None,
        );
    }
    match request.command {
        Command::Handshake => outcome(
            request,
            &[
                Status::Ok as u8,
                FIRMWARE_VERSION[0],
                FIRMWARE_VERSION[1],
                FIRMWARE_VERSION[2],
                CAPABILITIES,
            ],
            ProtocolAction::None,
        ),
        Command::ReadRtc => match rtc.read_datetime().await {
            Ok(datetime) => datetime_outcome(request, Status::Ok, datetime, ProtocolAction::None),
            Err(_) => outcome(request, &[Status::DeviceError as u8], ProtocolAction::None),
        },
        Command::SetRtc => {
            let Ok(datetime) = decode_datetime(request.payload()) else {
                return outcome(
                    request,
                    &[Status::InvalidRequest as u8],
                    ProtocolAction::None,
                );
            };
            if rtc.set_datetime(datetime).await.is_err() {
                return outcome(request, &[Status::DeviceError as u8], ProtocolAction::None);
            }
            match rtc.read_datetime().await {
                Ok(readback) => {
                    datetime_outcome(request, Status::Ok, readback, ProtocolAction::RtcSet)
                }
                Err(_) => outcome(request, &[Status::DeviceError as u8], ProtocolAction::None),
            }
        }
        Command::Diagnostics => {
            let flags = diagnostic_flags.to_le_bytes();
            outcome(
                request,
                &[Status::Ok as u8, flags[0], flags[1]],
                ProtocolAction::None,
            )
        }
        Command::EnterStorage => {
            if allow_storage {
                outcome(request, &[Status::Ok as u8], ProtocolAction::EnterStorage)
            } else {
                outcome(
                    request,
                    &[Status::InvalidRequest as u8],
                    ProtocolAction::None,
                )
            }
        }
    }
}

fn datetime_outcome(
    request: ProtocolFrame,
    status: Status,
    datetime: pokeviewer_core::LocalDateTime,
    action: ProtocolAction,
) -> Result<ProtocolOutcome, FrameError> {
    let encoded = encode_datetime(datetime);
    let mut payload = [0; 8];
    payload[0] = status as u8;
    payload[1..].copy_from_slice(&encoded);
    outcome(request, &payload, action)
}

fn outcome(
    request: ProtocolFrame,
    payload: &[u8],
    action: ProtocolAction,
) -> Result<ProtocolOutcome, FrameError> {
    Ok(ProtocolOutcome {
        response: response(request, payload)?,
        action,
    })
}

fn response(request: ProtocolFrame, payload: &[u8]) -> Result<ProtocolFrame, FrameError> {
    ProtocolFrame::new(
        request.request_id,
        FrameKind::Response,
        request.command,
        payload,
    )
}

#[cfg(test)]
mod tests {
    use core::{
        future::Future,
        pin::pin,
        task::{Context, Poll, Waker},
    };

    use pokeviewer_core::{
        Command, FrameKind, LocalDateTime, ProtocolFrame, Status, decode_datetime, encode_datetime,
    };

    use super::{ProtocolAction, handle_protocol_request};
    use crate::{FakeRtc, Rtc};

    const NOW: LocalDateTime = LocalDateTime {
        year: 2026,
        month: 7,
        day: 27,
        hour: 19,
        minute: 0,
        second: 0,
    };

    fn block_on_ready<Output>(future: impl Future<Output = Output>) -> Output {
        let mut context = Context::from_waker(Waker::noop());
        let mut future = pin!(future);
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("fake RTC unexpectedly yielded"),
        }
    }

    #[test]
    fn invalid_datetime_never_changes_rtc() {
        let mut rtc = FakeRtc::new(NOW).unwrap();
        let mut invalid = encode_datetime(NOW);
        invalid[2] = 13;
        let request = ProtocolFrame::new(1, FrameKind::Request, Command::SetRtc, &invalid).unwrap();

        let outcome = block_on_ready(handle_protocol_request(&mut rtc, request, 0, false)).unwrap();
        let response = outcome.response;
        assert_eq!(response.payload(), &[Status::InvalidRequest as u8]);
        assert_eq!(outcome.action, ProtocolAction::None);
        assert_eq!(block_on_ready(rtc.read_datetime()), Ok(NOW));
    }

    #[test]
    fn set_response_contains_matching_readback() {
        let mut rtc = FakeRtc::new(NOW).unwrap();
        let later = LocalDateTime { day: 28, ..NOW };
        let request = ProtocolFrame::new(
            42,
            FrameKind::Request,
            Command::SetRtc,
            &encode_datetime(later),
        )
        .unwrap();

        let outcome = block_on_ready(handle_protocol_request(&mut rtc, request, 0, false)).unwrap();
        let response = outcome.response;
        assert_eq!(response.request_id, 42);
        assert_eq!(response.kind, FrameKind::Response);
        assert_eq!(response.payload()[0], Status::Ok as u8);
        assert_eq!(decode_datetime(&response.payload()[1..]), Ok(later));
        assert_eq!(outcome.action, ProtocolAction::RtcSet);
    }

    #[test]
    fn diagnostics_are_bounded_and_little_endian() {
        let mut rtc = FakeRtc::new(NOW).unwrap();
        let request = ProtocolFrame::new(3, FrameKind::Request, Command::Diagnostics, &[]).unwrap();

        let response = block_on_ready(handle_protocol_request(&mut rtc, request, 0x1234, false))
            .unwrap()
            .response;
        assert_eq!(response.payload(), &[Status::Ok as u8, 0x34, 0x12]);
    }

    #[test]
    fn storage_mode_requires_an_authorized_parent_session() {
        let mut rtc = FakeRtc::new(NOW).unwrap();
        let request =
            ProtocolFrame::new(4, FrameKind::Request, Command::EnterStorage, &[]).unwrap();

        let denied = block_on_ready(handle_protocol_request(&mut rtc, request, 0, false)).unwrap();
        assert_eq!(denied.response.payload(), &[Status::InvalidRequest as u8]);
        assert_eq!(denied.action, ProtocolAction::None);

        let allowed = block_on_ready(handle_protocol_request(&mut rtc, request, 0, true)).unwrap();
        assert_eq!(allowed.response.payload(), &[Status::Ok as u8]);
        assert_eq!(allowed.action, ProtocolAction::EnterStorage);
    }
}
