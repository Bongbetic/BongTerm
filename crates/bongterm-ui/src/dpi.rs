//! Per-monitor DPI state and scale math.

pub trait DpiProvider {
    fn dpi_for_window(&self) -> u32;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DpiState {
    dpi: u32,
}

impl DpiState {
    pub const DEFAULT_DPI: u32 = 96;

    #[must_use]
    pub const fn new(dpi: u32) -> Self {
        Self {
            dpi: if dpi == 0 { Self::DEFAULT_DPI } else { dpi },
        }
    }

    #[must_use]
    pub const fn dpi(self) -> u32 {
        self.dpi
    }

    #[must_use]
    pub fn scale(self) -> f64 {
        f64::from(self.dpi) / f64::from(Self::DEFAULT_DPI)
    }

    pub fn update_from(&mut self, provider: &impl DpiProvider) -> bool {
        let next = provider.dpi_for_window().max(1);
        if next == self.dpi {
            return false;
        }
        self.dpi = next;
        true
    }
}

pub struct FixedDpiProvider(pub u32);

impl DpiProvider for FixedDpiProvider {
    fn dpi_for_window(&self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_tracks_dpi_ratio() {
        assert!((DpiState::new(144).scale() - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn update_reports_only_real_changes() {
        let mut dpi = DpiState::new(96);
        assert!(!dpi.update_from(&FixedDpiProvider(96)));
        assert!(dpi.update_from(&FixedDpiProvider(192)));
        assert_eq!(dpi.dpi(), 192);
    }
}
