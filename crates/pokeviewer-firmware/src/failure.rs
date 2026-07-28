//! Bounded failure taxonomy shared by runtime policy and host tests.

use pokeviewer_core::{Framebuffer, RenderError, render_recovery_screen, render_setup_screen};

/// Stable adult-facing failure classes and wired diagnostic bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureKind {
    /// RTC is absent, stopped, unreadable, or invalid.
    InvalidRtc,
    /// Offline content is corrupt or incompatible.
    Content,
    /// Panel initialization or full refresh failed.
    Panel,
    /// The next daily RTC alarm could not be armed.
    Alarm,
    /// A wake source other than cold/reset or RTC EXT0 was observed.
    UnexpectedWake,
}

/// Terminal recovery action shown to an adult.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Set and verify the RTC through the wired utility.
    SetRtc,
    /// Reset or power-cycle after checking the named subsystem.
    Reset,
    /// Install a verified release image again.
    Reflash,
}

/// Complete bounded policy for one failure class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FailurePolicy {
    /// Stable display/log code.
    pub code: &'static str,
    /// Recovery action that does not require a child-facing menu.
    pub recovery: RecoveryAction,
    /// Maximum automatic hardware attempts in one wake.
    pub max_attempts: u8,
    /// Bit exposed by the wired diagnostics command while active.
    pub diagnostic_flag: u16,
}

impl FailureKind {
    /// Return the fixed policy for this class.
    #[must_use]
    pub const fn policy(self) -> FailurePolicy {
        match self {
            Self::InvalidRtc => FailurePolicy {
                code: "RTC",
                recovery: RecoveryAction::SetRtc,
                max_attempts: 0,
                diagnostic_flag: 1 << 0,
            },
            Self::Content => FailurePolicy {
                code: "PACK",
                recovery: RecoveryAction::Reflash,
                max_attempts: 1,
                diagnostic_flag: 1 << 1,
            },
            Self::Panel => FailurePolicy {
                code: "PANEL",
                recovery: RecoveryAction::Reset,
                max_attempts: 1,
                diagnostic_flag: 1 << 2,
            },
            Self::Alarm => FailurePolicy {
                code: "ALARM",
                recovery: RecoveryAction::Reset,
                max_attempts: 1,
                diagnostic_flag: 1 << 3,
            },
            Self::UnexpectedWake => FailurePolicy {
                code: "WAKE",
                recovery: RecoveryAction::Reset,
                max_attempts: 0,
                diagnostic_flag: 1 << 4,
            },
        }
    }
}

/// Render the stable screen for one classified failure.
///
/// # Errors
///
/// Returns [`RenderError`] only if a repository-owned fixed label violates the
/// shared renderer contract.
pub fn render_failure_screen(
    framebuffer: &mut Framebuffer,
    failure: FailureKind,
) -> Result<(), RenderError> {
    if failure == FailureKind::InvalidRtc {
        render_setup_screen(framebuffer);
        return Ok(());
    }
    let policy = failure.policy();
    render_recovery_screen(framebuffer, policy.code, action_label(policy.recovery))
}

const fn action_label(action: RecoveryAction) -> &'static str {
    match action {
        RecoveryAction::SetRtc => "SET RTC",
        RecoveryAction::Reset => "RESET",
        RecoveryAction::Reflash => "REFLASH",
    }
}

#[cfg(test)]
mod tests {
    use pokeviewer_core::Framebuffer;

    use super::{FailureKind, RecoveryAction, render_failure_screen};

    #[test]
    fn every_failure_has_a_bounded_unique_policy_and_screen() {
        let failures = [
            FailureKind::InvalidRtc,
            FailureKind::Content,
            FailureKind::Panel,
            FailureKind::Alarm,
            FailureKind::UnexpectedWake,
        ];
        let mut flags = 0;
        let mut hashes = [0; 5];
        for (index, failure) in failures.into_iter().enumerate() {
            let policy = failure.policy();
            assert!(policy.max_attempts <= 1);
            assert_eq!(flags & policy.diagnostic_flag, 0);
            flags |= policy.diagnostic_flag;
            let mut framebuffer = Framebuffer::default();
            render_failure_screen(&mut framebuffer, failure).unwrap();
            hashes[index] = framebuffer.crc32();
        }
        for (index, hash) in hashes.into_iter().enumerate() {
            assert!(!hashes[..index].contains(&hash));
        }
    }

    #[test]
    fn recovery_actions_are_explicit() {
        assert_eq!(
            FailureKind::InvalidRtc.policy().recovery,
            RecoveryAction::SetRtc
        );
        assert_eq!(
            FailureKind::Content.policy().recovery,
            RecoveryAction::Reflash
        );
        for failure in [
            FailureKind::Panel,
            FailureKind::Alarm,
            FailureKind::UnexpectedWake,
        ] {
            assert_eq!(failure.policy().recovery, RecoveryAction::Reset);
        }
    }
}
