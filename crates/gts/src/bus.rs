//! GTS on the RR-BUS (SPEC.md "RR-BUS — inter-device modulation"): publishes Mod A, Mod B, Gate,
//! Filter Env and Amp Env, and applies three CV-in slots to the patch.
//!
//! Threads: [`DeviceBus::connect`] / [`DeviceBus::ensure_registered`] run in `initialize` (and in
//! `reset` only after the host paused long enough for the record to be reclaimed);
//! [`DeviceBus::process_block`] and [`apply_cv`] run in `process`. The editor talks to the shared
//! [`BusLink`]: it swaps `Subscription`s under `subs`, which the audio thread only `try_lock`s
//! (keeping the last values on contention), so the audio thread never blocks and never drops one.

use crate::dsp::Patch;
use crate::params::{CvTarget, GtsParams};
use nih_plug::prelude::{Enum, Param};
use rr_bus::{Bus, Instance, Kind, Source, Subscription};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};

/// Published signals, in [`crate::dsp::Engine::bus_signals`] order.
pub const SIGNALS: [(&str, Kind); 5] = [
    ("Mod A", Kind::Bipolar),
    ("Mod B", Kind::Bipolar),
    ("Gate", Kind::Gate),
    ("Filter Env", Kind::Unipolar),
    ("Amp Env", Kind::Unipolar),
];

/// CV-in smoothing time constant (seconds).
const SMOOTH_S: f32 = 0.005;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// State shared by the plugin and its editor.
#[derive(Default)]
pub struct BusLink {
    bus: Mutex<Option<Bus>>,
    instance: Mutex<Option<Arc<Instance>>>,
    /// CV-in bindings. GUI: `lock`; audio: `try_lock` only.
    subs: Mutex<[Option<Subscription>; 3]>,
    /// |last read value| per slot, f32 bits, for the activity LEDs.
    activity: [AtomicU32; 3],
}

impl BusLink {
    /// GUI thread: the session bus (disabled until `initialize` ran).
    pub fn bus(&self) -> Bus {
        lock(&self.bus).clone().unwrap_or_else(Bus::disabled)
    }

    /// GUI thread: rebinds slot `i` to a persisted `"<label> · <name>"` (empty = unbind). The
    /// old subscription is dropped here, after the lock is released.
    pub fn set_source(&self, i: usize, persisted: &str) {
        let new = (!persisted.is_empty()).then(|| Subscription::bind_persisted(&self.bus(), persisted));
        let old = std::mem::replace(&mut lock(&self.subs)[i], new);
        drop(old);
    }

    /// GUI thread, ~4x/s: reconnects lost bindings by label + name.
    pub fn rebind_lost(&self) {
        for s in lock(&self.subs).iter().flatten() {
            s.rebind_if_lost();
        }
    }

    /// GUI thread.
    pub fn set_label(&self, label: &str) {
        if let Some(i) = lock(&self.instance).as_ref() {
            i.set_label(label);
        }
    }

    pub fn activity(&self, i: usize) -> f32 {
        f32::from_bits(self.activity[i].load(Ordering::Relaxed))
    }
}

/// Owned by the plugin.
pub struct DeviceBus {
    pub link: Arc<BusLink>,
    /// Audio-side copy of the bus, so `reset` can check it without a lock.
    bus: Bus,
    rt: Option<(Arc<Instance>, [Source; 5])>,
    raw: [f32; 3],
    smooth: [f32; 3],
}

impl Default for DeviceBus {
    fn default() -> Self {
        Self { link: Arc::default(), bus: Bus::disabled(), rt: None, raw: [0.0; 3], smooth: [0.0; 3] }
    }
}

impl DeviceBus {
    /// `initialize` (also re-run by the host wrapper after a state load): opens the bus `name` once,
    /// registers if needed, binds the persisted sources and applies the persisted label.
    pub fn connect(&mut self, params: &GtsParams, name: &str) {
        if !self.bus.is_enabled() {
            self.bus = Bus::open_named(name);
            *lock(&self.link.bus) = Some(self.bus.clone());
        }
        self.ensure_registered(params);
        if let Some((inst, _)) = &self.rt {
            let label = params.instance_label.read().unwrap_or_else(PoisonError::into_inner).clone();
            if !label.is_empty() {
                inst.set_label(&label);
            }
        }
        let sources = params.cv_sources.read().unwrap_or_else(PoisonError::into_inner).clone();
        for (i, s) in sources.iter().enumerate() {
            self.link.set_source(i, s);
        }
    }

    /// Registers (again) when there is no live instance record, e.g. after a host paused
    /// `process` for longer than the bus's stale timeout and another device reclaimed it.
    /// Allocates; cheap no-op when already registered or the bus is disabled.
    pub fn ensure_registered(&mut self, params: &GtsParams) {
        if !self.bus.is_enabled() || self.rt.as_ref().is_some_and(|(i, _)| i.is_enabled()) {
            return;
        }
        self.rt = None; // free the old slots first
        let inst = Arc::new(self.bus.register("GTS"));
        let sources = SIGNALS.map(|(name, kind)| inst.publish(name, kind));
        {
            let mut label = params.instance_label.write().unwrap_or_else(PoisonError::into_inner);
            if label.is_empty() {
                *label = inst.label();
            } else {
                inst.set_label(&label);
            }
        }
        *lock(&self.link.instance) = Some(inst.clone());
        self.rt = Some((inst, sources));
    }

    /// True when the instance record is gone (bus enabled but not registered).
    pub fn needs_register(&self) -> bool {
        self.bus.is_enabled() && !self.rt.as_ref().is_some_and(|(i, _)| i.is_enabled())
    }

    /// Audio thread, once per block of `n` samples: heartbeat, publish `signals`, read the three
    /// CV-ins and return them smoothed (5 ms one-pole). No locks waited on, no allocation.
    pub fn process_block(&mut self, signals: [f32; 5], n: usize, sr: f32) -> [f32; 3] {
        if let Some((inst, sources)) = &self.rt {
            inst.heartbeat();
            for (s, v) in sources.iter().zip(signals) {
                s.set(v);
            }
        }
        let subs = match self.link.subs.try_lock() {
            Ok(g) => Some(g),
            Err(TryLockError::Poisoned(e)) => Some(e.into_inner()),
            Err(TryLockError::WouldBlock) => None, // GUI is swapping: keep the last values
        };
        if let Some(subs) = subs {
            for (raw, s) in self.raw.iter_mut().zip(subs.iter()) {
                *raw = s.as_ref().and_then(Subscription::read).unwrap_or(0.0);
            }
        }
        let k = (-(n as f32) / (SMOOTH_S * sr)).exp();
        for i in 0..3 {
            self.smooth[i] = self.raw[i] + (self.smooth[i] - self.raw[i]) * k;
            self.link.activity[i].store(self.raw[i].abs().min(1.0).to_bits(), Ordering::Relaxed);
        }
        self.smooth
    }
}

/// Adds `amount × cv` per slot to the target's normalized value (clamped 0..1) and writes the
/// plain result into `patch`. Host parameter values are never touched.
pub fn apply_cv(patch: &mut Patch, params: &GtsParams, cv: [f32; 3]) {
    let mut off = [0f32; 9];
    for ((target, amount), v) in params.cv_slots().into_iter().zip(cv) {
        off[target.to_index()] += amount * v;
    }
    let m = |p: &nih_plug::prelude::FloatParam, o: f32| p.preview_plain((p.modulated_normalized_value() + o).clamp(0.0, 1.0));
    for (i, &o) in off.iter().enumerate().skip(1) {
        if o == 0.0 {
            continue;
        }
        match CvTarget::from_index(i) {
            CvTarget::Off => {}
            CvTarget::OscAIndex => patch.osc[0].index = m(&params.osca.index, o),
            CvTarget::OscAShift => patch.osc[0].shift = m(&params.osca.shift, o),
            CvTarget::OscBIndex => patch.osc[1].index = m(&params.oscb.index, o),
            CvTarget::OscBShift => patch.osc[1].shift = m(&params.oscb.shift, o),
            CvTarget::FilterAFreq => patch.filt[0].freq = m(&params.filta.freq, o),
            CvTarget::FilterBFreq => patch.filt[1].freq = m(&params.filtb.freq, o),
            // ponytail: "Mod A Amount" drives Mod A's Index amount, the wobble depth of the
            // default patch; a joint scale of all three Mod A amounts has no single param.
            CvTarget::ModAAmount => patch.moda_index = m(&params.moda_amt.index, o),
            CvTarget::Volume => patch.volume_db = m(&params.volume, o),
        }
    }
}
