//! AI assist submodule (Cmd-K + failed-command explainer).

pub const MODULE_NAME: &str = "ai";

pub mod runner;
pub use runner::{
    AiAvailability, AiBackend, AiContext, AiIntent, AiRequest, AiSuggestion, UnavailableBackend,
};
