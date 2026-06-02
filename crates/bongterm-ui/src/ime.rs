//! IME composition state and preedit positioning.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImeEvent {
    Opened,
    Preedit(String),
    Commit(String),
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImeState {
    open: bool,
    preedit: String,
    commits: Vec<String>,
}

impl ImeState {
    pub fn apply(&mut self, event: ImeEvent) {
        match event {
            ImeEvent::Opened => {
                self.open = true;
                self.preedit.clear();
            }
            ImeEvent::Preedit(text) => {
                if self.open {
                    self.preedit = text;
                }
            }
            ImeEvent::Commit(text) => {
                if self.open {
                    self.commits.push(text);
                    self.preedit.clear();
                }
            }
            ImeEvent::Closed => {
                self.open = false;
                self.preedit.clear();
            }
        }
    }

    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.open
    }

    #[must_use]
    pub fn preedit(&self) -> &str {
        &self.preedit
    }

    #[must_use]
    pub fn commits(&self) -> &[String] {
        &self.commits
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompositionWindow {
    pub x: f32,
    pub y: f32,
}

impl CompositionWindow {
    #[must_use]
    pub fn at_cell(col: u16, row: u16, cell_w: f32, cell_h: f32, scale: f32) -> Self {
        Self {
            x: f32::from(col) * cell_w * scale,
            y: (f32::from(row) + 1.0) * cell_h * scale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ime_tracks_preedit_commit_and_close() {
        let mut ime = ImeState::default();
        ime.apply(ImeEvent::Opened);
        ime.apply(ImeEvent::Preedit("ni".to_string()));
        assert!(ime.is_open());
        assert_eq!(ime.preedit(), "ni");
        ime.apply(ImeEvent::Commit("你".to_string()));
        assert_eq!(ime.preedit(), "");
        assert_eq!(ime.commits(), &["你".to_string()]);
        ime.apply(ImeEvent::Closed);
        assert!(!ime.is_open());
    }

    #[test]
    fn composition_window_uses_cell_metrics_and_dpi_scale() {
        let pos = CompositionWindow::at_cell(10, 2, 8.0, 16.0, 1.5);
        assert!((pos.x - 120.0).abs() < f32::EPSILON);
        assert!((pos.y - 72.0).abs() < f32::EPSILON);
    }
}
