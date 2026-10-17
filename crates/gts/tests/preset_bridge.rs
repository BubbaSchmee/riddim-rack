//! The editor's Preset combo: loading an embedded preset through the gesture API reproduces the
//! preset's Patch, Init restores the defaults, and routing / label / Quality are never touched.
//!
//! nih-plug has no public plain-value setter outside a host, so the fake `Ctl` records what the
//! host would receive (like the editor preview): reading the recorded values back through the
//! params' own ranges gives the Patch the engine would run (`Patch::from_params` of the host's
//! params after the same gestures).
use gts::dsp::Patch;
use gts::editor::{apply_preset, Ctl, ParamTable};
use gts::params::GtsParams;
use gts::preset::{factory, Preset};
use nih_plug::prelude::{Params, ParamPtr};
use serde_json::{json, Map, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

struct Fake {
    params: Arc<GtsParams>,
    table: ParamTable,
    /// id -> last normalized value set, and every id a gesture touched.
    vals: RefCell<BTreeMap<String, f32>>,
    gestures: RefCell<Vec<String>>,
}

impl Ctl for Fake {
    fn get01(&self, id: &str) -> f32 {
        self.vals.borrow().get(id).copied().unwrap_or_else(|| self.table.get01(id))
    }
    fn default01(&self, id: &str) -> f32 {
        self.table.default01(id)
    }
    fn readout(&self, id: &str) -> String {
        self.table.readout_at(id, self.get01(id))
    }
    fn steps(&self, id: &str) -> Option<usize> {
        self.table.steps(id)
    }
    fn begin(&self, id: &str) {
        self.gestures.borrow_mut().push(id.to_string());
    }
    fn set01(&self, id: &str, v: f32) {
        self.vals.borrow_mut().insert(id.to_string(), v.clamp(0.0, 1.0));
    }
    fn end(&self, _: &str) {}
}

impl Fake {
    fn new() -> Self {
        let params = Arc::new(GtsParams::default());
        Self { table: ParamTable::new(params.clone()), params, vals: Default::default(), gestures: Default::default() }
    }

    fn sound_ids(&self) -> Vec<String> {
        self.params.param_map().into_iter().map(|(id, _, _)| id).filter(|id| !id.starts_with("cv")).collect()
    }

    /// The Patch the engine would run after the recorded gestures.
    fn patch(&self) -> Patch {
        let mut params = Map::new();
        for id in self.sound_ids() {
            let (v, ptr) = (self.get01(&id), self.table.ptr(&id));
            // SAFETY: the table keeps the params alive.
            let value = unsafe {
                match ptr {
                    ParamPtr::BoolParam(_) => Value::from(v > 0.5),
                    ParamPtr::EnumParam(_) => Value::from(ptr.normalized_value_to_string(v, false)),
                    _ => json!(ptr.preview_plain(v)),
                }
            };
            params.insert(id, value);
        }
        let file = json!({ "format_version": 1, "product": "GTS", "params": params });
        let (p, warnings) = Preset::from_json(&file.to_string()).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        p.patch
    }
}

/// Every sound param of `a` equals `b`'s: exact for ints / bools / enums, within 1e-4 for floats
/// (the param stores a normalized value, so plain -> normalized -> plain drifts by an ulp or two).
fn assert_same(a: &Patch, b: &Patch, what: &str) {
    let json = |p: &Patch| serde_json::from_str::<Value>(&Preset { patch: *p, ..Preset::default() }.to_json()).unwrap();
    let (a, b) = (json(a), json(b));
    let (a, b) = (a["params"].as_object().unwrap(), b["params"].as_object().unwrap());
    assert_eq!(a.len(), b.len());
    for (id, x) in a {
        let y = &b[id];
        match (x.as_f64(), y.as_f64()) {
            (Some(x), Some(y)) if x.fract() != 0.0 || y.fract() != 0.0 => {
                assert!((x - y).abs() <= 1e-4 * y.abs().max(1.0), "{what}: {id} {x} != {y}")
            }
            _ => assert_eq!(x, y, "{what}: {id}"),
        }
    }
}

#[test]
fn factory_presets_load_through_the_gesture_api() {
    let names: Vec<&str> = factory().iter().map(|p| p.name.as_str()).collect();
    // Bank order: category folder, then name.
    assert_eq!(names, ["Growl Hound", "Reese Drift", "Sub Weight", "Talk Box", "Wobble Shot", "Screech Lead"]);
    let fake = Fake::new();
    for p in factory() {
        apply_preset(&fake.table, &fake, Some(p));
        assert_same(&fake.patch(), &p.patch, &p.name);
    }
}

#[test]
fn init_restores_defaults_and_routing_is_untouched() {
    let fake = Fake::new();
    *fake.params.instance_label.write().unwrap() = "Lead bus".into();
    fake.params.cv_sources.write().unwrap()[0] = "VSS 1 · Step Gate".into();
    fake.params.oversampling.store(4, Ordering::Relaxed);

    for p in factory().iter().map(Some).chain([None]) {
        apply_preset(&fake.table, &fake, p);
        assert_eq!(*fake.params.instance_label.read().unwrap(), "Lead bus");
        assert_eq!(fake.params.cv_sources.read().unwrap()[0], "VSS 1 · Step Gate");
        assert_eq!(fake.params.oversampling.load(Ordering::Relaxed), 4);
    }
    // No gesture ever touched a CV slot, and the 214 sound params (180 + 34 Track 7) were each driven.
    let touched = fake.gestures.borrow();
    assert!(!touched.iter().any(|id| id.starts_with("cv")), "a CV slot was written");
    assert_eq!(touched.len(), 7 * 214);
    // The last load was Init: every sound param sits at its default, the Patch is the default Patch.
    for id in fake.sound_ids() {
        assert_eq!(fake.get01(&id), fake.default01(&id), "{id}");
    }
    assert_same(&fake.patch(), &Patch::default(), "Init");
}
