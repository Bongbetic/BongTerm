//! Scripted mock for `bongterm_devassist::ai::AiBackend`.

use bongterm_devassist::DevassistError;
use bongterm_devassist::ai::{AiAvailability, AiBackend, AiRequest, AiSuggestion};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A scripted AI backend for tests. Returns a fixed suggestion or unavailable.
pub struct MockAiBackend {
    availability: AiAvailability,
    suggestion: Option<AiSuggestion>,
    run_count: Arc<AtomicUsize>,
}

impl Clone for MockAiBackend {
    fn clone(&self) -> Self {
        Self {
            availability: self.availability.clone(),
            suggestion: self.suggestion.clone(),
            run_count: Arc::clone(&self.run_count),
        }
    }
}

impl MockAiBackend {
    /// Available backend with a custom full suggestion.
    #[must_use]
    pub fn available(suggestion: AiSuggestion) -> Self {
        Self {
            availability: AiAvailability::Available {
                version: "mock-1.0".to_string(),
            },
            suggestion: Some(suggestion),
            run_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Available backend that returns a fixed single suggestion.
    #[must_use]
    pub fn with_suggestion(cmd: impl Into<String>) -> Self {
        Self {
            availability: AiAvailability::Available {
                version: "mock-1.0".to_string(),
            },
            suggestion: Some(AiSuggestion {
                command: cmd.into(),
                explanation: String::new(),
            }),
            run_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Unavailable backend.
    #[must_use]
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            availability: AiAvailability::Unavailable {
                reason: reason.into(),
            },
            suggestion: None,
            run_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Number of execute calls recorded by the mock.
    #[must_use]
    pub fn run_count(&self) -> usize {
        self.run_count.load(Ordering::Relaxed)
    }
}

impl AiBackend for MockAiBackend {
    fn availability(&self) -> AiAvailability {
        self.availability.clone()
    }

    fn suggest(&self, _request: &AiRequest) -> Result<AiSuggestion, DevassistError> {
        match &self.suggestion {
            Some(suggestion) => Ok(suggestion.clone()),
            None => Err(DevassistError::Unavailable("mock unavailable".to_string())),
        }
    }
}
