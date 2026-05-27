//! `bongterm-pty` — `ConPTY` host + reusable byte buffers.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

pub mod ring;
pub use ring::{Slab, SlabPool};

pub mod host;
pub use host::{ChildSpec, PtyChild, PtyHost, ScaffoldPtyHost};
