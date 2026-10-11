//! Riddim Rack shared egui look. Depends only on egui so both plugin editors (through their
//! param-setter adapters) and the preview example can use it. See `docs/design/DESIGN.md`.
//!
//! Every widget takes plain values and reports changes through its return value; nothing here
//! knows about plugin parameters.

#![allow(clippy::too_many_arguments)]

pub mod fonts;
pub mod paint;
pub mod panel;
pub mod theme;
pub mod widgets;

pub use theme::{Theme, T};
