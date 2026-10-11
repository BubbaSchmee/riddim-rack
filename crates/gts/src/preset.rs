//! GTS preset files (SPEC v2 2.3, format subset): JSON with metadata and every sound parameter as
//! the plain value its knob displays (f32 for floats, i32 for ints, bool, enum variant name).
//!
//! ```json
//! { "format_version": 1, "product": "GTS", "name": "...", "author": "...", "description": "...",
//!   "category": "Bass/growl", "tags": [], "macros": [{"label": "Growl"}], "matrix": [],
//!   "params": { "osca_table": "Square Morph", "osca_index": 40, ... } }
//! ```
//!
//! Loading applies the params to a [`Patch`] (the engine's plain-value snapshot), starting from the
//! defaults: missing ids keep their default; unknown ids, wrong types and out-of-range values
//! (clamped) are reported in a warning list. The RR-BUS CV slots (`cv{1,2,3}_*`) are project
//! routing, not sound: never written, silently ignored on load.
//! ponytail: loads into `Patch`, not `GtsParams` (nih-plug has no public plain-value setter
//! outside a host context); the browser wave pushes the loaded `Patch` into the host params via
//! `ParamSetter`.

use crate::dsp::{FilterPatch, ModPatch, OscPatch, Patch};
use crate::params::GtsParams;
use nih_plug::prelude::{Enum, ParamPtr, Params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};

pub const FORMAT_VERSION: u32 = 1;
pub const PRODUCT: &str = "GTS";

#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    pub name: String,
    pub author: String,
    pub description: String,
    /// "Bass/growl", "Lead", ... (SPEC v2 2.5 categories, sub-tag after the slash).
    pub category: String,
    pub tags: Vec<String>,
    /// Macro labels (assignments arrive with the matrix).
    pub macros: Vec<String>,
    pub patch: Patch,
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            name: "Init".into(),
            author: String::new(),
            description: String::new(),
            category: "Init".into(),
            tags: vec![],
            macros: vec![],
            patch: Patch::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Macro {
    label: String,
}

/// The file layout. Unknown top-level fields are ignored.
#[derive(Serialize, Deserialize)]
struct File {
    format_version: u32,
    product: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    macros: Vec<Macro>,
    #[serde(default)]
    matrix: Vec<Value>,
    #[serde(default)]
    params: Map<String, Value>,
}

/// A writable view of one `Patch` field.
enum Slot<'a> {
    F(&'a mut f32),
    I(&'a mut i32),
    /// An int param stored as `usize` (unison, poly).
    N(&'a mut usize),
    /// An int param stored as `f32` (bend range).
    IF(&'a mut f32),
    B(&'a mut bool),
    E(&'a mut dyn Choice),
}

/// An enum field, by variant display name.
trait Choice {
    #[cfg_attr(not(test), allow(dead_code))]
    fn names(&self) -> &'static [&'static str];
    fn name(&self) -> &'static str;
    fn set_name(&mut self, s: &str) -> bool;
}

impl<T: Enum + Copy> Choice for T {
    fn names(&self) -> &'static [&'static str] {
        T::variants()
    }
    fn name(&self) -> &'static str {
        T::variants()[self.to_index()]
    }
    fn set_name(&mut self, s: &str) -> bool {
        match T::variants().iter().position(|v| *v == s) {
            Some(i) => {
                *self = T::from_index(i);
                true
            }
            None => false,
        }
    }
}

fn prefixed<'a, const K: usize>(pre: &str, v: [(&str, Slot<'a>); K], out: &mut Vec<(String, Slot<'a>)>) {
    out.extend(v.into_iter().map(|(k, s)| (format!("{pre}_{k}"), s)));
}

fn osc_slots<'a>(pre: &str, o: &'a mut OscPatch, out: &mut Vec<(String, Slot<'a>)>) {
    use Slot::*;
    let OscPatch {
        on, table, oct, semi, cent, index, motion, shift, vol, att, dec, sus, rel, curve, unison, detune, det_curve,
        blend, width, rand_phase, phase_reset,
    } = o;
    #[rustfmt::skip]
    let v = [
        ("on", B(on)), ("table", E(table)), ("oct", I(oct)), ("semi", I(semi)), ("cent", F(cent)),
        ("index", F(index)), ("motion", F(motion)), ("shift", F(shift)), ("vol", F(vol)), ("att", F(att)),
        ("dec", F(dec)), ("sus", F(sus)), ("rel", F(rel)), ("curve", F(curve)), ("unison", N(unison)),
        ("detune", F(detune)), ("det_curve", F(det_curve)), ("blend", F(blend)), ("width", F(width)),
        ("rand_phase", F(rand_phase)), ("phase_reset", B(phase_reset)),
    ];
    prefixed(pre, v, out);
}

fn mod_slots<'a>(pre: &str, m: &'a mut ModPatch, out: &mut Vec<(String, Slot<'a>)>) {
    use Slot::*;
    let ModPatch { curve, rate, sync, div, oneshot, target } = m;
    let v = [("curve", E(curve)), ("rate", F(rate)), ("sync", B(sync)), ("div", E(div)), ("oneshot", B(oneshot)), ("target", E(target))];
    prefixed(pre, v, out);
}

fn filt_slots<'a>(pre: &str, f: &'a mut FilterPatch, out: &mut Vec<(String, Slot<'a>)>) {
    use Slot::*;
    let FilterPatch { ty, freq, res, kbd, env } = f;
    prefixed(pre, [("type", E(ty)), ("freq", F(freq)), ("res", F(res)), ("kbd", F(kbd)), ("env", B(env))], out);
}

/// Every preset parameter of `p` under its param id (all GtsParams ids except the CV slots).
fn slots(p: &mut Patch) -> Vec<(String, Slot<'_>)> {
    use Slot::*;
    let mut out = Vec::with_capacity(102);
    let Patch {
        osc: [osca, oscb], moda, modb, moda_pitch, moda_index, moda_shift, modb_motion, modb_vol, modb_cutoff,
        modb_moda, shaper_on, shaper_mode, shaper_amt, filt: [filta, filtb], fenv_att, fenv_dec, fenv_sus, fenv_rel,
        fenv_amt, fenv_inv, fenv_curve, sub_on, sub_level_db, sub_oct, mono_xover, routing, poly, legato, porta,
        bend_range, wheel_index, wheel_shift, wheel_cutoff, wheel_mod, vel_vol_a, vel_vol_b, vel_fenv, vel_att,
        vel_shift, vel_mod, volume_db, spread,
    } = p;
    osc_slots("osca", osca, &mut out);
    osc_slots("oscb", oscb, &mut out);
    mod_slots("moda", moda, &mut out);
    mod_slots("modb", modb, &mut out);
    filt_slots("filta", filta, &mut out);
    filt_slots("filtb", filtb, &mut out);
    #[rustfmt::skip]
    let rest: Vec<(&str, Slot<'_>)> = vec![
        ("moda_pitch", F(moda_pitch)), ("moda_index", F(moda_index)), ("moda_shift", F(moda_shift)),
        ("modb_motion", F(modb_motion)), ("modb_vol", F(modb_vol)), ("modb_cutoff", F(modb_cutoff)),
        ("modb_moda", F(modb_moda)), ("shaper_on", B(shaper_on)), ("shaper_mode", E(shaper_mode)),
        ("shaper_amt", F(shaper_amt)), ("fenv_att", F(fenv_att)), ("fenv_dec", F(fenv_dec)),
        ("fenv_sus", F(fenv_sus)), ("fenv_rel", F(fenv_rel)), ("fenv_amt", F(fenv_amt)), ("fenv_inv", B(fenv_inv)),
        ("fenv_curve", F(fenv_curve)), ("sub_on", B(sub_on)), ("sub_level", F(sub_level_db)), ("sub_oct", I(sub_oct)),
        ("mono_xover", F(mono_xover)), ("routing", E(routing)), ("poly", N(poly)), ("legato", B(legato)),
        ("porta", F(porta)), ("bend_range", IF(bend_range)), ("wheel_index", F(wheel_index)),
        ("wheel_shift", F(wheel_shift)), ("wheel_cutoff", F(wheel_cutoff)), ("wheel_mod", F(wheel_mod)),
        ("vel_vol_a", F(vel_vol_a)), ("vel_vol_b", F(vel_vol_b)), ("vel_fenv", F(vel_fenv)), ("vel_att", F(vel_att)),
        ("vel_shift", F(vel_shift)), ("vel_mod", F(vel_mod)), ("volume", F(volume_db)), ("spread", F(spread)),
    ];
    out.extend(rest.into_iter().map(|(k, s)| (k.to_string(), s)));
    out
}

/// (id, min, max) plain range of every GtsParams id, from the param definitions.
fn ranges() -> Vec<(String, f32, f32)> {
    let params = GtsParams::default();
    params
        .param_map()
        .into_iter()
        // SAFETY: `params` outlives every pointer use in this function.
        .map(|(id, ptr, _): (String, ParamPtr, String)| unsafe { (id, ptr.preview_plain(0.0), ptr.preview_plain(1.0)) })
        .collect()
}

/// f32 as its shortest round-trip decimal (what the knob shows), not the f64 widening.
fn num_f32(x: f32) -> Value {
    serde_json::from_str::<Number>(&x.to_string()).map(Value::Number).unwrap_or(Value::Null)
}

impl Preset {
    /// Pretty JSON of the whole preset (every sound parameter, sorted by id).
    pub fn to_json(&self) -> String {
        let mut patch = self.patch;
        let params = slots(&mut patch)
            .into_iter()
            .map(|(id, s)| {
                let v = match s {
                    Slot::F(x) => num_f32(*x),
                    Slot::I(x) => Value::from(*x),
                    Slot::N(x) => Value::from(*x as i32),
                    Slot::IF(x) => Value::from(x.round() as i32),
                    Slot::B(x) => Value::from(*x),
                    Slot::E(x) => Value::from(x.name()),
                };
                (id, v)
            })
            .collect();
        let file = File {
            format_version: FORMAT_VERSION,
            product: PRODUCT.into(),
            name: self.name.clone(),
            author: self.author.clone(),
            description: self.description.clone(),
            category: self.category.clone(),
            tags: self.tags.clone(),
            macros: self.macros.iter().map(|l| Macro { label: l.clone() }).collect(),
            matrix: vec![],
            params,
        };
        serde_json::to_string_pretty(&file).expect("a preset always serialises") + "\n"
    }

    /// Parses a preset. Hard errors: not JSON, wrong product, newer format. Everything else loads
    /// with a warning: unknown ids, wrong value types, out-of-range values (clamped), unknown enum
    /// names, a non-empty matrix (not supported yet).
    pub fn from_json(json: &str) -> Result<(Preset, Vec<String>), String> {
        let file: File = serde_json::from_str(json).map_err(|e| format!("not a GTS preset: {e}"))?;
        if file.product != PRODUCT {
            return Err(format!("preset is for \"{}\", not {PRODUCT}", file.product));
        }
        if file.format_version > FORMAT_VERSION {
            return Err(format!("preset format {} is newer than this GTS ({FORMAT_VERSION})", file.format_version));
        }
        let mut warnings = vec![];
        if !file.matrix.is_empty() {
            warnings.push(format!("matrix: {} slots ignored (no matrix in this version)", file.matrix.len()));
        }
        let ranges = ranges();
        let mut patch = Patch::default();
        let mut slots = slots(&mut patch);
        for (id, v) in &file.params {
            let Some((lo, hi)) = ranges.iter().find(|r| &r.0 == id).map(|r| (r.1, r.2)) else {
                warnings.push(format!("{id}: unknown parameter, ignored"));
                continue;
            };
            let Some((_, slot)) = slots.iter_mut().find(|(k, _)| k == id) else {
                continue; // CV routing slot
            };
            let mut num = |integral: bool| -> Option<f32> {
                let Some(x) = v.as_f64().map(|x| x as f32).filter(|x| x.is_finite()) else {
                    warnings.push(format!("{id}: expected a number, got {v}; default kept"));
                    return None;
                };
                let y = if integral { x.round() } else { x }.clamp(lo, hi);
                if y != x {
                    warnings.push(format!("{id}: {x} out of range {lo}..{hi}, set to {y}"));
                }
                Some(y)
            };
            match slot {
                Slot::F(x) => **x = num(false).unwrap_or(**x),
                Slot::IF(x) => **x = num(true).unwrap_or(**x),
                Slot::I(x) => **x = num(true).map_or(**x, |y| y as i32),
                Slot::N(x) => **x = num(true).map_or(**x, |y| y as usize),
                Slot::B(x) => match v.as_bool() {
                    Some(b) => **x = b,
                    None => warnings.push(format!("{id}: expected true/false, got {v}; default kept")),
                },
                Slot::E(x) => {
                    if !v.as_str().is_some_and(|s| x.set_name(s)) {
                        warnings.push(format!("{id}: unknown choice {v}; default kept"));
                    }
                }
            }
        }
        drop(slots);
        let preset = Preset {
            name: file.name,
            author: file.author,
            description: file.description,
            category: file.category,
            tags: file.tags,
            macros: file.macros.into_iter().map(|m| m.label).collect(),
            patch,
        };
        Ok((preset, warnings))
    }

    /// Reads and parses a preset file.
    pub fn load(path: &std::path::Path) -> Result<(Preset, Vec<String>), String> {
        let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::from_json(&s)
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.to_json())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The slot table covers exactly the GtsParams ids minus the CV routing slots.
    #[test]
    fn slots_cover_every_sound_param() {
        let mut p = Patch::default();
        let mut got: Vec<String> = slots(&mut p).into_iter().map(|(k, _)| k).collect();
        let mut want: Vec<String> = ranges().into_iter().map(|r| r.0).filter(|k| !k.starts_with("cv")).collect();
        got.sort();
        want.sort();
        assert_eq!(got, want);
        assert_eq!(got.len(), 102);
    }

    #[test]
    fn defaults_round_trip() {
        let p = Preset::default();
        let (q, w) = Preset::from_json(&p.to_json()).unwrap();
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(q, p);
        assert_eq!(q.patch, Patch::default());
    }

    /// Bit pattern of every slot, so -0.0 vs 0.0 or a 1-ulp drift fails.
    fn bits(p: &Patch) -> Vec<(String, u64)> {
        let mut p = *p;
        slots(&mut p)
            .into_iter()
            .map(|(k, s)| {
                let b = match s {
                    Slot::F(x) | Slot::IF(x) => x.to_bits() as u64,
                    Slot::I(x) => *x as u32 as u64,
                    Slot::N(x) => *x as u64,
                    Slot::B(x) => *x as u64,
                    Slot::E(x) => x.names().iter().position(|n| *n == x.name()).unwrap() as u64,
                };
                (k, b)
            })
            .collect()
    }

    /// Fully randomised valid presets round-trip bit-exactly.
    #[test]
    fn random_presets_round_trip_bit_exact() {
        let ranges = ranges();
        let mut rng = 0x1234_5678u32;
        let mut rand = || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            (rng >> 8) as f32 / (1u32 << 24) as f32
        };
        for _ in 0..1000 {
            let mut patch = Patch::default();
            for (id, s) in slots(&mut patch) {
                let (_, lo, hi) = ranges.iter().find(|r| r.0 == id).unwrap().clone();
                let r = rand();
                match s {
                    // Arbitrary f32 bit patterns in range, not round decimals.
                    Slot::F(x) => *x = (lo + (hi - lo) * r).clamp(lo, hi),
                    Slot::I(x) => *x = (lo + (hi - lo) * r).round() as i32,
                    Slot::N(x) => *x = (lo + (hi - lo) * r).round() as usize,
                    Slot::IF(x) => *x = (lo + (hi - lo) * r).round(),
                    Slot::B(x) => *x = r < 0.5,
                    Slot::E(x) => {
                        let names = x.names();
                        assert!(x.set_name(names[((r * names.len() as f32) as usize).min(names.len() - 1)]));
                    }
                }
            }
            let p = Preset { name: "R".into(), tags: vec!["t".into()], macros: vec!["M1".into()], patch, ..Preset::default() };
            let (q, w) = Preset::from_json(&p.to_json()).unwrap();
            assert!(w.is_empty(), "{w:?}");
            assert_eq!(bits(&q.patch), bits(&p.patch));
            assert_eq!(q, p);
        }
    }

    #[test]
    fn unknown_and_missing_ids_load_with_warnings() {
        let json = r#"{"format_version": 1, "product": "GTS", "name": "x", "extra_top": 5,
            "params": {"osca_index": 99.5, "not_a_param": 1, "filta_freq": 1e9, "osca_table": "Nope",
                       "cv1_target": "Off"}}"#;
        let (p, w) = Preset::from_json(json).unwrap();
        assert_eq!(p.patch.osc[0].index, 99.5);
        assert_eq!(p.patch.filt[0].freq, 20000.0);
        assert_eq!(p.patch.osc[0].table, Patch::default().osc[0].table);
        assert_eq!(p.patch.osc[1], Patch::default().osc[1]);
        assert_eq!(w.len(), 3, "{w:?}");
        assert!(Preset::from_json(r#"{"format_version": 1, "product": "VSS"}"#).is_err());
        assert!(Preset::from_json(r#"{"format_version": 2, "product": "GTS"}"#).is_err());
    }
}
