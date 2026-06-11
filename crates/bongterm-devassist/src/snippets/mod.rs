//! Snippets submodule.

pub const MODULE_NAME: &str = "snippets";

pub mod model;
pub use model::{ParamPrompt, Snippet, SnippetLibrary, SnippetScope, SnippetStore};

pub mod render;
pub use render::render_snippet;
