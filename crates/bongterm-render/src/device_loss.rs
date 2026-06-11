//! Device-loss recovery policy for renderer backends.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceRemovedReason {
    Removed,
    Reset,
    Hung,
    DriverInternalError,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    RecreateDevice,
    SoftwareFallback,
}

pub struct DeviceLossRecovery {
    recent: VecDeque<Instant>,
    window: Duration,
    fallback_after: usize,
}

impl DeviceLossRecovery {
    #[must_use]
    pub fn new(window: Duration, fallback_after: usize) -> Self {
        Self {
            recent: VecDeque::new(),
            window,
            fallback_after,
        }
    }

    #[must_use]
    pub fn record_loss(&mut self, now: Instant, _reason: DeviceRemovedReason) -> RecoveryAction {
        self.recent.push_back(now);
        while self
            .recent
            .front()
            .is_some_and(|seen| now.duration_since(*seen) > self.window)
        {
            self.recent.pop_front();
        }
        if self.recent.len() >= self.fallback_after {
            RecoveryAction::SoftwareFallback
        } else {
            RecoveryAction::RecreateDevice
        }
    }
}

impl Default for DeviceLossRecovery {
    fn default() -> Self {
        Self::new(Duration::from_mins(1), 3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_two_device_losses_recreate_device() {
        let mut recovery = DeviceLossRecovery::default();
        let now = Instant::now();
        assert_eq!(
            recovery.record_loss(now, DeviceRemovedReason::Removed),
            RecoveryAction::RecreateDevice
        );
        assert_eq!(
            recovery.record_loss(now + Duration::from_secs(10), DeviceRemovedReason::Reset),
            RecoveryAction::RecreateDevice
        );
    }

    #[test]
    fn three_losses_in_window_trigger_software_fallback() {
        let mut recovery = DeviceLossRecovery::default();
        let now = Instant::now();
        let _ = recovery.record_loss(now, DeviceRemovedReason::Removed);
        let _ = recovery.record_loss(now + Duration::from_secs(10), DeviceRemovedReason::Reset);
        assert_eq!(
            recovery.record_loss(now + Duration::from_secs(20), DeviceRemovedReason::Hung),
            RecoveryAction::SoftwareFallback
        );
    }

    #[test]
    fn old_losses_age_out_of_window() {
        let mut recovery = DeviceLossRecovery::default();
        let now = Instant::now();
        let _ = recovery.record_loss(now, DeviceRemovedReason::Removed);
        let _ = recovery.record_loss(now + Duration::from_secs(61), DeviceRemovedReason::Reset);
        assert_eq!(
            recovery.record_loss(
                now + Duration::from_secs(62),
                DeviceRemovedReason::DriverInternalError,
            ),
            RecoveryAction::RecreateDevice
        );
    }
}
