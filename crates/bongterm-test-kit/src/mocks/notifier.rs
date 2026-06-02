//! Recording mock for `bongterm_devassist::jobs::Notifier`.

use bongterm_devassist::jobs::{Notifier, Toast};
use std::sync::Mutex;

/// Records every toast for assertions.
pub struct MockNotifier {
    toasts: Mutex<Vec<Toast>>,
}

impl MockNotifier {
    #[must_use]
    pub fn new() -> Self {
        Self {
            toasts: Mutex::new(Vec::new()),
        }
    }

    /// Snapshot of recorded toasts.
    #[must_use]
    pub fn toasts(&self) -> Vec<Toast> {
        self.toasts.lock().unwrap().clone()
    }
}

impl Default for MockNotifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Notifier for MockNotifier {
    fn notify(&self, toast: &Toast) {
        self.toasts.lock().unwrap().push(toast.clone());
    }
}
