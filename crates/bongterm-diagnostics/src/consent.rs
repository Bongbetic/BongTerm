//! Telemetry consent state. Default is off.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TelemetryConsent {
    #[default]
    Off,
    OptedIn,
}

impl TelemetryConsent {
    #[must_use]
    pub const fn can_send(self) -> bool {
        matches!(self, Self::OptedIn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_is_off_by_default() {
        assert!(!TelemetryConsent::default().can_send());
    }
}
