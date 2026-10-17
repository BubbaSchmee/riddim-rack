//! Modulation range arcs (MOTION.md 5.2): computed from the knob's base value and its sources'
//! amounts, in the param's own units, one arc per source with the source colour.
use gts::editor::{filter_db, ring_ranges, Ctl, ParamTable};
use gts::params::GtsParams;
use rr_ui::theme::{SRC_A, SRC_B, SRC_E};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// Default params plus overrides by plain value.
struct Ov {
    table: ParamTable,
    vals: RefCell<HashMap<String, f32>>,
}

impl Ov {
    fn new() -> Self {
        Self { table: ParamTable::new(Arc::new(GtsParams::default())), vals: Default::default() }
    }
    fn set(&self, id: &str, plain: f32) {
        let n = self.table.norm(id, plain);
        self.vals.borrow_mut().insert(id.into(), n);
    }
}

impl Ctl for Ov {
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
    fn begin(&self, _: &str) {}
    fn set01(&self, _: &str, _: f32) {}
    fn end(&self, _: &str) {}
    fn plain(&self, id: &str, v01: f32) -> f32 {
        self.table.plain(id, v01)
    }
    fn norm(&self, id: &str, plain: f32) -> f32 {
        self.table.norm(id, plain)
    }
}

fn near(a: f32, b: f32, tol: f32) {
    assert!((a - b).abs() <= tol, "{a} vs {b}");
}

#[test]
fn no_amounts_no_rings() {
    let c = Ov::new();
    // The init patch has both filters following the envelope: that is a ring. Without it, none.
    for id in ["filta_env", "filtb_env"] {
        c.vals.borrow_mut().insert(id.into(), 0.0);
    }
    for id in gts::viz::MOD_IDS {
        assert_eq!(ring_ranges(&c, id), [None, None], "{id}");
    }
}

#[test]
fn mod_a_index_range_is_a_one_way_span_for_decay_and_two_way_for_sine() {
    let c = Ov::new();
    c.set("moda_index", 0.5); // +-63.5 grains at full depth; the init Mod A curve is Decay (unipolar)
    let r = ring_ranges(&c, "osca_index")[0].expect("range");
    assert_eq!(r.color, SRC_A);
    near(r.lo, 40.0 / 127.0, 1e-3);
    near(r.hi, 103.5 / 127.0, 1e-3);
    // A bipolar curve swings both ways around the base.
    c.vals.borrow_mut().insert("moda_curve".into(), 0.0);
    let r = ring_ranges(&c, "osca_index")[0].expect("range");
    // 40 - 63.5 clamps at 0.
    assert_eq!(r.lo, 0.0);
    near(r.hi, 103.5 / 127.0, 1e-3);
}

#[test]
fn mod_a_target_and_osc_b_state_gate_the_rings() {
    let c = Ov::new();
    c.set("moda_index", 0.5);
    // Osc B is off in the init patch: no ring on its Index.
    assert!(ring_ranges(&c, "oscb_index")[0].is_none());
    c.vals.borrow_mut().insert("oscb_on".into(), 1.0);
    assert!(ring_ranges(&c, "oscb_index")[0].is_some());
    // Target A only: Osc B drops out again.
    c.vals.borrow_mut().insert("moda_target".into(), 0.0);
    assert!(ring_ranges(&c, "osca_index")[0].is_some());
    assert!(ring_ranges(&c, "oscb_index")[0].is_none());
}

#[test]
fn filter_freq_ring_has_env_and_mod_b_ranges_in_octaves() {
    let c = Ov::new();
    // Init: Filter A follows the envelope (Env on, Amt 0.4 = +3.2 octaves), Mod B cutoff is 0.
    let [env, b] = ring_ranges(&c, "filta_freq");
    let env = env.expect("env range");
    assert_eq!(env.color, SRC_E);
    assert!(b.is_none());
    let (lo_hz, hi_hz) = (c.plain("filta_freq", env.lo), c.plain("filta_freq", env.hi));
    near(lo_hz, 2000.0, 20.0);
    near(hi_hz / 2000.0, 2f32.powf(3.2), 0.4);
    // Inverted envelope: the span runs downward.
    c.set("fenv_inv", 1.0);
    let env = ring_ranges(&c, "filta_freq")[0].unwrap();
    assert!(c.plain("filta_freq", env.hi) <= 2001.0 && c.plain("filta_freq", env.lo) < 400.0);
    // Mod B cutoff +0.5 on a sine: +-2.5 octaves around the base, in the Mod B colour.
    c.set("modb_cutoff", 0.5);
    let b = ring_ranges(&c, "filta_freq")[1].expect("mod b range");
    assert_eq!(b.color, SRC_B);
    near((c.plain("filta_freq", b.lo) / 2000.0).log2(), -2.5, 0.05);
    near((c.plain("filta_freq", b.hi) / 2000.0).log2(), 2.5, 0.1);
    // Filter B's Env is on in the init patch too, but its range needs Env on and Amt: switch it off.
    c.vals.borrow_mut().insert("filtb_env".into(), 0.0);
    assert!(ring_ranges(&c, "filtb_freq")[0].is_none());
}

#[test]
fn mod_b_scales_the_mod_a_amount_knobs() {
    let c = Ov::new();
    c.set("moda_pitch", 0.5);
    assert!(ring_ranges(&c, "moda_pitch")[0].is_none());
    c.set("modb_moda", 0.5);
    let r = ring_ranges(&c, "moda_pitch")[0].expect("range");
    assert_eq!(r.color, SRC_B);
    // Sine: the amount swings 0.5 +- 0.25.
    near(c.plain("moda_pitch", r.lo), 0.25, 0.01);
    near(c.plain("moda_pitch", r.hi), 0.75, 0.01);
}

#[test]
fn filter_curves_have_the_expected_shapes() {
    // LP12: unity at DC, -3 dB-ish at the cutoff with res 0, a peak with res up, steep above.
    assert!(filter_db(0, 20.0, 1000.0, 0.0).abs() < 0.2);
    assert!(filter_db(0, 1000.0, 1000.0, 0.8) > 6.0);
    assert!(filter_db(0, 8000.0, 1000.0, 0.0) < -30.0);
    // BP12 peaks at the cutoff and falls both sides.
    assert!(filter_db(1, 1000.0, 1000.0, 0.5) > filter_db(1, 125.0, 1000.0, 0.5) + 10.0);
    assert!(filter_db(1, 1000.0, 1000.0, 0.5) > filter_db(1, 8000.0, 1000.0, 0.5) + 10.0);
    // Combs: teeth at multiples of the cutoff, notches (feedforward) or valleys (feedback) between.
    assert!(filter_db(2, 2000.0, 1000.0, 1.0) > filter_db(2, 1500.0, 1000.0, 1.0) + 6.0);
    assert!(filter_db(3, 2000.0, 1000.0, 0.8) > filter_db(3, 1500.0, 1000.0, 0.8) + 6.0);
    // AM: flat; res 0 is a clean pass.
    assert!(filter_db(4, 100.0, 1000.0, 0.0).abs() < 1e-3 && filter_db(4, 5000.0, 1000.0, 0.0).abs() < 1e-3);
}
