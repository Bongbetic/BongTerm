//! Snippets submodule.

pub const MODULE_NAME: &str = "snippets";

pub mod model;
pub use model::{Snippet, SnippetLibrary, SnippetScope};

pub mod render;
pub use render::render_snippet;
