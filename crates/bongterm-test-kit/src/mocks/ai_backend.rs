//! Scripted mock for `bongterm_devassist::ai::AiBackend`.

use bongterm_devassist::DevassistError;
use bongterm_devassist::ai::{AiAvailability, AiBackend, AiRequest, AiSuggestion};

/// A scripted AI backend for tests. Returns a fixed suggestion or unavailable.
pub struct MockAiBackend {
    availability: AiAvailability,
    suggestion: Option<AiSuggestion>,
}

impl MockAiBackend {
    /// Available backend that returns a scripted suggestion.
    #[must_use]
    pub fn available(suggestion: AiSuggestion) -> Self {
        Self {
            availability: AiAvailability::Available {
                version: "mock-1.0".to_string(),
            },
            suggestion: Some(suggestion),
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
        }
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
