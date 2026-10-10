//! GUI (nih_plug_egui). Not started yet.

use nih_plug::prelude::*;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use crate::params::GtsParams;

pub fn create(_params: Arc<GtsParams>, _current_index: Arc<AtomicU32>) -> Option<Box<dyn Editor>> {
    // TODO: SPEC "GUI" (1000x620 egui window, rows under 960 px).
    None
}
