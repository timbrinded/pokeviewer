//! Timeout guard for drivers that poll an active-high BUSY pin.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use embedded_hal::digital::{ErrorType, InputPin};

/// Shared state for one bounded BUSY pin.
pub(crate) struct BusyState {
    polls: AtomicU32,
    timed_out: AtomicBool,
}

impl BusyState {
    /// Create idle timeout state.
    pub(crate) const fn new() -> Self {
        Self {
            polls: AtomicU32::new(0),
            timed_out: AtomicBool::new(false),
        }
    }

    /// Start a new bounded wait.
    pub(crate) fn reset(&self) {
        self.polls.store(0, Ordering::Relaxed);
        self.timed_out.store(false, Ordering::Relaxed);
    }

    /// Report whether the most recent wait reached its limit.
    pub(crate) fn timed_out(&self) -> bool {
        self.timed_out.load(Ordering::Relaxed)
    }
}

/// BUSY-pin adapter that releases an upstream polling loop at a fixed limit.
pub(crate) struct BoundedBusy<'state, Pin> {
    pin: Pin,
    max_polls: u32,
    state: &'state BusyState,
}

impl<'state, Pin> BoundedBusy<'state, Pin> {
    /// Wrap an active-high BUSY pin.
    pub(crate) fn new(pin: Pin, max_polls: u32, state: &'state BusyState) -> Self {
        state.reset();
        Self {
            pin,
            max_polls,
            state,
        }
    }
}

impl<Pin> ErrorType for BoundedBusy<'_, Pin>
where
    Pin: ErrorType,
{
    type Error = Pin::Error;
}

impl<Pin> InputPin for BoundedBusy<'_, Pin>
where
    Pin: InputPin,
{
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        let is_high = self.pin.is_high()?;
        if !is_high {
            self.state.polls.store(0, Ordering::Relaxed);
            return Ok(false);
        }

        let polls = self.state.polls.fetch_add(1, Ordering::Relaxed) + 1;
        if polls >= self.max_polls {
            self.state.timed_out.store(true, Ordering::Relaxed);
            Ok(false)
        } else {
            Ok(true)
        }
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        self.is_high().map(|is_high| !is_high)
    }
}

#[cfg(test)]
mod tests {
    use core::convert::Infallible;
    use embedded_hal::digital::{ErrorType, InputPin};

    use super::{BoundedBusy, BusyState};

    struct Pin(bool);

    impl ErrorType for Pin {
        type Error = Infallible;
    }

    impl InputPin for Pin {
        fn is_high(&mut self) -> Result<bool, Self::Error> {
            Ok(self.0)
        }

        fn is_low(&mut self) -> Result<bool, Self::Error> {
            Ok(!self.0)
        }
    }

    #[test]
    fn stuck_busy_is_released_and_reported_at_limit() {
        let state = BusyState::new();
        let mut busy = BoundedBusy::new(Pin(true), 3, &state);

        assert!(busy.is_high().unwrap());
        assert!(busy.is_high().unwrap());
        assert!(!busy.is_high().unwrap());
        assert!(state.timed_out());
    }

    #[test]
    fn idle_pin_resets_the_wait_state() {
        let state = BusyState::new();
        let mut busy = BoundedBusy::new(Pin(false), 3, &state);

        assert!(!busy.is_high().unwrap());
        state.reset();
        assert!(!state.timed_out());
    }
}
