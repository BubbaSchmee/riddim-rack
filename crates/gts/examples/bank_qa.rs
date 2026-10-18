//! Factory-bank QA tool: the per-family QA table and the bank-level checks of
//! `docs/design/bank-recipes.md` section A, run against preset JSON files.
//!
//! ```text
//! bank_qa [--only <name>] [--out <dir>] [--name <stem>] [--no-macros] [--bright-amp] [--pair-min <dB>] [path...]
//! ```
//!
//! `--bright-amp` judges the amplitude-weighted centroid instead of the sheet's power-weighted Brightness
//! (the report always shows both; the baseline report explains why). `--no-macros` skips the macro sweeps.
//! `--pair-min` sets the distinctiveness threshold (log-mel distance, dB; default 3.3, see `PAIR_MIN_DEFAULT`).
//!
//! `path` is a preset file or a directory (searched recursively); no path = `crates/gts/presets/factory`.
//! Per preset: the family comes from the category and tags (tags riddim, wobble/wub, tearout/growl,
//! formant/yoi, reese/neuro, sub, lead/screech, pluck/stab, fx/riser, seq; the tag `qa_family=<family>`
//! wins). Each preset is rendered at 150 and 140 BPM through the real engine (the preset's matrix,
//! custom LFO shapes and FX order applied) with the family's test notes (see `Ctx::plan`), and every metric of the table is computed: peak at velocity 100 and 127, RMS / LUFS against
//! the family band, the category median and the bank median, low-band correlation and stereo-vs-mono
//! difference, sub share / THD / sub integrity across the cycle, Brightness, Move centroid depth and
//! period (autocorrelation against the preset's own LFO division), Width, alias floor on a frozen
//! modulation render, macro sweeps (5 steps), pitch dives, reese beat rate, FX trends, seq slots, and
//! pairwise log-mel distinctiveness inside a family.
//!
//! Reading of the sheet where it is ambiguous (each choice is also in the report): Move period = the
//! autocorrelation peak within +-5 % of the expected lag when it is at least 85 % of the largest peak
//! (the largest alone is often a multiple); alias floor = the sheet's literal +-1 % rule, so nothing above
//! about 50 f0 is judged, with the harmonic grid dropped to f0 / 2 when the sub sounds below the played
//! pitch; FX "width rises / falls" = stereo width, i.e. the L/R correlation moves the other way.
//!
//! Output: `<out>/<name>.md` and `<out>/<name>.json` (default `docs/audits/bank-qa/bank-qa.*`), one
//! pass / fail per check with value, target and margin. Exit code 1 on any failure.
//!
//! Tags that steer the tool (case-insensitive, `=` or `:`):
//! `qa_family=<riddim|wobble|tearout|formant|reese|sub|lead|pluck|fx|seq|init>`,
//! `qa_period=<division|seconds|none>` (expected Move period; `none` for per-note presets),
//! `qa_dive=<st>[@<ms>]` (expected pitch dive), `qa_decay=<ms>` (pluck: 20 dB fall time),
//! `qa_beat=<Hz at F1>` (reese beat rate), `qa_fxkind=<riser|downlifter|impact|sweep>`,
//! `qa_macro<1-4>=<C|D|L|W>:<threshold>` or `Tatt|Tfall|Trel:<ms>` (pin the stated macro check),
//! `qa_<check id>=<lo>..<hi>` (replace the numeric target of one check, for the per-preset targets).
use gts::dsp::tables::Fft;
use gts::dsp::{Engine, Patch, Transport};
use gts::matrix::{Matrix, Source};
use gts::params::{Curve, Division, Trigger};
use gts::preset::Preset;
use nih_plug::prelude::{Enum, NoteEvent};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

#[path = "render_preset.rs"]
#[allow(dead_code)]
mod rp;
#[path = "support/qa_dsp.rs"]
mod q;
use q::{db10, mean, mean_sq, MOVE_HOP, SR};

const F1: u8 = 29;
const E1: u8 = 28;
const G1: u8 = 31;
const AS1: u8 = 34;
const C2: u8 = 36;
const C3: u8 = 48;
const E3: u8 = 52;
const F3: u8 = 53;
const GS3: u8 = 56;
const AS3: u8 = 58;
const C4: u8 = 60;
const G4: u8 = 67;
const VEL100: f32 = 100.0 / 127.0;
const VEL127: f32 = 1.0;
const TEMPOS: [u32; 2] = [150, 140];
/// Excluded from loudness and spectral stats (the attack).
const SKIP_S: f64 = 0.05;
/// `--bright-amp`: judge the amplitude-weighted centroid instead of the sheet's power-weighted one.
// Calibration 2026-10-18: amplitude-weighted is the judged figure (a -6..0 dB sub drags the power
// centroid below every bass band); `--bright-power` judges the power-weighted one instead.
static BRIGHT_AMP: AtomicBool = AtomicBool::new(true);
/// Default distinctiveness threshold, dB RMS distance between log-mel descriptors. Calibrated on the six
/// existing presets: every pair of them is a different design and sits at 4.19 dB or more (median 9.8 dB);
/// copies of them with one to three parameters changed (a division, a cutoff, a drive, the sub level, the
/// unison count) sit at 0.1 to 2.5 dB. 3.3 dB is the midpoint of 2.5 and 4.19.
const PAIR_MIN_DEFAULT: f64 = 3.3;

fn hz(note: u8) -> f64 {
    440.0 * 2f64.powf((note as f64 - 69.0) / 12.0)
}
fn smp(t: f64) -> usize {
    (t * SR).round().max(0.0) as usize
}

// ---- families ----------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Family {
    Riddim,
    Wobble,
    Tearout,
    Formant,
    Reese,
    Sub,
    Lead,
    Pluck,
    Fx,
    Seq,
    Init,
}

const FAMILIES: [(Family, &str); 11] = [
    (Family::Riddim, "riddim"),
    (Family::Wobble, "wobble"),
    (Family::Tearout, "tearout"),
    (Family::Formant, "formant"),
    (Family::Reese, "reese"),
    (Family::Sub, "sub"),
    (Family::Lead, "lead"),
    (Family::Pluck, "pluck"),
    (Family::Fx, "fx"),
    (Family::Seq, "seq"),
    (Family::Init, "init"),
];

impl Family {
    fn name(self) -> &'static str {
        FAMILIES.iter().find(|f| f.0 == self).unwrap().1
    }
    fn is_bass(self) -> bool {
        matches!(self, Family::Riddim | Family::Wobble | Family::Tearout | Family::Formant | Family::Reese | Family::Sub)
    }
}

fn norm_tags(p: &Preset) -> Vec<String> {
    p.tags.iter().map(|t| t.trim().to_lowercase()).collect()
}

/// `key=value` or `key:value` among the tags.
fn tag_value(tags: &[String], key: &str) -> Option<String> {
    tags.iter().find_map(|t| t.strip_prefix(key).and_then(|r| r.strip_prefix('=').or_else(|| r.strip_prefix(':'))).map(|v| v.trim().to_string()))
}

fn bass_family(words: &[&str]) -> Option<Family> {
    const ORDER: [(&[&str], Family); 6] = [
        (&["tearout", "growl"], Family::Tearout),
        (&["formant", "yoi", "talking"], Family::Formant),
        (&["reese", "neuro"], Family::Reese),
        (&["sub"], Family::Sub),
        (&["wobble", "wub"], Family::Wobble),
        (&["riddim"], Family::Riddim),
    ];
    ORDER.iter().find(|(ws, _)| ws.iter().any(|w| words.contains(w))).map(|x| x.1)
}

/// (family, how it was decided).
fn family_of(p: &Preset) -> (Family, String) {
    let tags = norm_tags(p);
    if let Some(v) = tag_value(&tags, "qa_family") {
        if let Some(f) = FAMILIES.iter().find(|f| f.1 == v) {
            return (f.0, "qa_family tag".into());
        }
    }
    let cat = p.category.to_lowercase();
    let (main, sub) = cat.split_once('/').unwrap_or((cat.as_str(), ""));
    let (main, sub) = (main.trim(), sub.trim());
    match main {
        "lead" => return (Family::Lead, "category".into()),
        "pluck" => return (Family::Pluck, "category".into()),
        "fx" => return (Family::Fx, "category".into()),
        "seq" => return (Family::Seq, "category".into()),
        "init" => return (Family::Init, "category".into()),
        _ => {}
    }
    let words: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
    if let Some(f) = bass_family(&words) {
        return (f, "tags".into());
    }
    if let Some(f) = bass_family(&[sub]) {
        return (f, "category sub-tag".into());
    }
    (Family::Riddim, "default (no family tag)".into())
}

// ---- checks ------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Status {
    Pass,
    Fail,
    Info,
    Skip,
}

impl Status {
    fn name(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Fail => "fail",
            Status::Info => "info",
            Status::Skip => "skip",
        }
    }
}

#[derive(Clone, Debug)]
struct Chk {
    id: String,
    group: String,
    label: String,
    tempo: Option<u32>,
    value: Option<f64>,
    lo: Option<f64>,
    hi: Option<f64>,
    unit: String,
    status: Status,
    margin: Option<f64>,
    detail: String,
}

type Bounds = (Option<f64>, Option<f64>);

/// Status and margin of `v` against the bounds (margin = distance to the nearest bound, negative = outside).
fn judge(v: f64, lo: Option<f64>, hi: Option<f64>) -> (Status, Option<f64>) {
    if !v.is_finite() {
        return (Status::Fail, None);
    }
    if lo.is_none() && hi.is_none() {
        return (Status::Info, None);
    }
    let m = [lo.map(|l| v - l), hi.map(|h| h - v)].into_iter().flatten().fold(f64::MAX, f64::min);
    (if m >= -1e-9 { Status::Pass } else { Status::Fail }, Some(m))
}

/// The worst of several per-note values against the bounds (least margin), and the list of all.
fn worst(vals: &[(String, f64)], lo: Option<f64>, hi: Option<f64>) -> (f64, String) {
    let detail = vals.iter().map(|(n, v)| format!("{n} {}", fmt(*v))).collect::<Vec<_>>().join(", ");
    let key = |v: f64| if v.is_finite() { judge(v, lo, hi).1.unwrap_or(0.0) } else { f64::MIN };
    let w = vals.iter().map(|x| x.1).min_by(|a, b| key(*a).total_cmp(&key(*b))).unwrap_or(f64::NAN);
    (w, detail)
}

fn fmt(v: f64) -> String {
    if !v.is_finite() {
        "n/a".into()
    } else if v.abs() >= 100.0 {
        format!("{v:.1}")
    } else if v.abs() >= 1.0 {
        format!("{v:.2}")
    } else {
        format!("{v:.3}")
    }
}

/// `lo..hi` (either side may be empty).
fn parse_bounds(s: &str) -> Option<Bounds> {
    let (a, b) = s.split_once("..")?;
    let f = |x: &str| if x.trim().is_empty() { Some(None) } else { x.trim().parse::<f64>().ok().map(Some) };
    Some((f(a)?, f(b)?))
}

const RESERVED: [&str; 6] = ["qa_family", "qa_period", "qa_dive", "qa_decay", "qa_beat", "qa_fxkind"];

fn parse_overrides(tags: &[String]) -> HashMap<String, Bounds> {
    let mut m = HashMap::new();
    for t in tags {
        let Some(rest) = t.strip_prefix("qa_") else { continue };
        if RESERVED.iter().any(|r| t.starts_with(r)) || rest.starts_with("macro") {
            continue;
        }
        let Some((k, v)) = rest.split_once('=').or_else(|| rest.split_once(':')) else { continue };
        if let Some(b) = parse_bounds(v) {
            m.insert(k.to_string(), b);
        }
    }
    m
}

// ---- LFO analysis ------------------------------------------------------------------------------

/// Cycle length in beats of a division (the table of bank-recipes.md section A).
fn div_beats(d: Division) -> f64 {
    use Division::*;
    match d {
        Bars4 => 16.0,
        Bars2 => 8.0,
        Bar1 => 4.0,
        Half => 2.0,
        HalfT => 4.0 / 3.0,
        Quarter => 1.0,
        QuarterT => 2.0 / 3.0,
        Eighth => 0.5,
        EighthT => 1.0 / 3.0,
        Sixteenth => 0.25,
        SixteenthT => 1.0 / 6.0,
        ThirtySecond => 0.125,
        ThirtySecondT => 1.0 / 12.0,
        HalfD => 3.0,
        QuarterD => 1.5,
        EighthD => 0.75,
        SixteenthD => 0.375,
    }
}

fn parse_division(s: &str) -> Option<Division> {
    let s = s.trim().to_lowercase();
    Division::variants().iter().position(|v| v.to_lowercase() == s).map(Division::from_index)
}

fn lfo_source(k: usize) -> Source {
    [Source::ModA, Source::ModB, Source::ModC, Source::ModD][k]
}

fn lfo_of(p: &Preset, k: usize) -> (&gts::dsp::ModPatch, Trigger) {
    let m = [&p.patch.moda, &p.patch.modb, &p.patch.modc, &p.patch.modd][k];
    (m, p.patch.lfo_ext[k].trig)
}

/// Total routed depth of LFO `k`: matrix amounts plus the fixed routes of Mod A / Mod B.
fn lfo_weight(p: &Preset, k: usize) -> f64 {
    let m: f64 = p.matrix.iter().filter(|s| s.enabled && s.source == lfo_source(k)).map(|s| s.amount.abs() as f64).sum();
    let pa = &p.patch;
    let fixed = match k {
        0 => pa.moda_pitch.abs() + pa.moda_index.abs() + pa.moda_shift.abs(),
        1 => pa.modb_motion.abs() + pa.modb_vol.abs() + pa.modb_cutoff.abs(),
        _ => 0.0,
    };
    m + fixed as f64
}

fn lfo_periodic(p: &Preset, k: usize) -> bool {
    let (m, trig) = lfo_of(p, k);
    !m.oneshot && trig != Trigger::Envelope
}

#[derive(Clone, Debug)]
struct Dom {
    idx: usize,
    curve: Curve,
    sync: bool,
    div: Division,
    rate: f32,
    weight: f64,
    depth: f64,
    steps: Option<Vec<f32>>,
}

impl Dom {
    fn of(p: &Preset, k: usize) -> Dom {
        let (m, _) = lfo_of(p, k);
        let sh = &p.lfo_shapes.0[k];
        let steps = (m.curve == Curve::Custom && sh.mode == gts::dsp::shape::Mode::Steps).then(|| sh.steps.clone());
        Dom { idx: k, curve: m.curve, sync: m.sync, div: m.div, rate: m.rate, weight: lfo_weight(p, k), depth: f64::NAN, steps }
    }
    fn period(&self, tempo: f64) -> f64 {
        if self.sync {
            div_beats(self.div) * 60.0 / tempo
        } else {
            1.0 / self.rate.max(1e-3) as f64
        }
    }
    fn random(&self) -> bool {
        matches!(self.curve, Curve::SampleHold | Curve::SmoothRandom)
    }
    fn name(&self) -> String {
        let r = if self.sync { self.div.to_string_name() } else { format!("{} Hz", self.rate) };
        format!("Mod {} {:?} {}", ["A", "B", "C", "D"][self.idx], self.curve, r)
    }
}

trait DivName {
    fn to_string_name(&self) -> String;
}
impl DivName for Division {
    fn to_string_name(&self) -> String {
        Division::variants()[self.to_index()].to_string()
    }
}

#[derive(Clone, Debug)]
enum PeriodOv {
    None,
    Secs(f64),
    Div(Division),
}

enum Expect {
    Periodic(f64, String),
    Aperiodic(String),
    NoLfo,
    Disabled,
}

// ---- rendering ---------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Strip {
    No,
    All,
    /// Everything but this LFO.
    Keep(usize),
}

#[derive(Clone, Copy)]
struct Mode {
    strip: Strip,
    dry: bool,
    sub_only: bool,
    /// Sub oscillator off (the oscillators alone, for the reese beat).
    no_sub: bool,
}

impl Mode {
    const LIVE: Mode = Mode { strip: Strip::No, dry: false, sub_only: false, no_sub: false };
    const FROZEN: Mode = Mode { strip: Strip::All, dry: false, sub_only: false, no_sub: false };
    const FROZEN_DRY: Mode = Mode { strip: Strip::All, dry: true, sub_only: false, no_sub: false };
    const FROZEN_DRY_NO_SUB: Mode = Mode { strip: Strip::All, dry: true, sub_only: false, no_sub: true };
    const SUB_ONLY: Mode = Mode { strip: Strip::All, dry: true, sub_only: true, no_sub: false };
}

#[derive(Clone)]
struct Ev {
    at: f64,
    dur: f64,
    note: u8,
    vel: f32,
}

struct Job {
    tempo: u32,
    notes: Vec<Ev>,
    len: f64,
    mode: Mode,
    macros: Option<[f32; 4]>,
}

/// The patch and matrix of a job: macro override, frozen modulation, dry FX, sub only.
fn prepare(p: &Preset, j: &Job) -> (Patch, Matrix) {
    let mut patch = p.patch;
    let mut m = p.matrix_state();
    if let Some(mc) = j.macros {
        patch.macros = mc;
    }
    if j.mode.strip != Strip::No {
        let keep = match j.mode.strip {
            Strip::Keep(k) => Some(k),
            _ => None,
        };
        m.slots.retain(|s| match s.source {
            Source::ModA => keep == Some(0),
            Source::ModB => keep == Some(1),
            Source::ModC => keep == Some(2),
            Source::ModD => keep == Some(3),
            Source::ModEnv => false,
            _ => true,
        });
        if keep != Some(0) {
            (patch.moda_pitch, patch.moda_index, patch.moda_shift) = (0.0, 0.0, 0.0);
        }
        if keep != Some(1) {
            (patch.modb_motion, patch.modb_vol, patch.modb_cutoff, patch.modb_moda) = (0.0, 0.0, 0.0, 0.0);
        }
    }
    if j.mode.dry {
        let f = &mut patch.fx;
        f.dist.on = false;
        f.mb.on = false;
        f.eq.on = false;
        f.cho.on = false;
        f.dim.on = false;
        f.dly.on = false;
        f.rev.on = false;
        f.phs.on = false;
        f.flg.on = false;
    }
    if j.mode.no_sub {
        patch.sub_on = false;
    }
    if j.mode.sub_only {
        for o in &mut patch.osc {
            o.on = false;
        }
        patch.noise_level_db = gts::dsp::noise::NOISE_OFF_DB;
        m.slots.retain(|s| s.target.starts_with("sub_") || s.target == "volume");
    }
    (patch, m)
}

fn render(p: &Preset, j: &Job) -> Vec<[f32; 2]> {
    let (patch, matrix) = prepare(p, j);
    let mut ev: Vec<(usize, u8, bool, f32)> = vec![];
    for e in &j.notes {
        ev.push((smp(e.at), e.note, true, e.vel));
        ev.push((smp(e.at + e.dur), e.note, false, 0.0));
    }
    ev.sort_by_key(|&(t, _, on, _)| (t, on));
    let mut e = Engine::new();
    e.initialize(SR as f32);
    e.transport = Transport { tempo: j.tempo as f64, playing: true, pos_beats: 0.0 };
    e.set_matrix(&matrix.encode());
    e.set_lfo_shapes(&p.lfo_shapes.encode());
    e.set_fx_order(p.fx_order);
    let len = smp(j.len);
    let mut frames = Vec::with_capacity(len);
    let mut next = 0;
    for i in 0..len {
        if i % 512 == 0 {
            e.set_patch(&patch);
        }
        while next < ev.len() && ev[next].0 <= i {
            let (_, note, on, vel) = ev[next];
            e.handle_event(&if on {
                NoteEvent::NoteOn { timing: 0, voice_id: None, channel: 0, note, velocity: vel }
            } else {
                NoteEvent::NoteOff { timing: 0, voice_id: None, channel: 0, note, velocity: 0.0 }
            });
            next += 1;
        }
        let (l, r) = e.next_frame();
        frames.push([l, r]);
    }
    frames
}

/// One held note and the analysis window inside it.
struct Hold {
    fr: Vec<[f32; 2]>,
    note: u8,
    a: usize,
    b: usize,
    off: usize,
}

impl Hold {
    fn lr(&self) -> (Vec<f64>, Vec<f64>) {
        let b = self.b.min(self.fr.len());
        let a = self.a.min(b);
        (self.fr[a..b].iter().map(|f| f[0] as f64).collect(), self.fr[a..b].iter().map(|f| f[1] as f64).collect())
    }
    fn peak(&self) -> f64 {
        peak_of(&self.fr)
    }
}

fn peak_of(fr: &[[f32; 2]]) -> f64 {
    fr.iter().flatten().fold(0f64, |m, v| m.max(v.abs() as f64))
}
fn finite_of(fr: &[[f32; 2]]) -> bool {
    fr.iter().flatten().all(|v| v.is_finite())
}

#[derive(Clone, Copy)]
struct Plan {
    hold: f64,
    tail: f64,
    a: f64,
    b: f64,
}

type Phrase = &'static [(f32, f32, u8)];
#[rustfmt::skip]
const BASS: Phrase = &[
    (0.0, 0.6, F1), (1.0, 0.6, F1), (3.0, 1.7, F1), (6.0, 0.6, G1), (7.0, 0.5, F1),
    (8.0, 0.6, AS1), (9.0, 0.6, AS1), (11.0, 0.9, G1), (14.0, 0.6, F1), (15.0, 0.5, F1),
    (16.0, 0.6, F1), (17.0, 0.6, F1), (18.0, 0.6, G1), (19.0, 1.7, F1), (22.0, 0.6, G1), (23.0, 0.5, F1),
    (24.0, 7.9, F1),
];
#[rustfmt::skip]
const LEAD: Phrase = &[
    (0.0, 0.9, F3), (1.0, 0.9, GS3), (2.0, 1.8, AS3), (4.0, 0.9, C4), (5.0, 0.9, AS3), (6.0, 1.8, GS3),
    (8.0, 0.9, F3), (10.0, 0.9, F3), (11.0, 0.9, GS3), (12.0, 3.8, AS3),
    (16.0, 0.9, F3), (17.0, 0.9, GS3), (18.0, 1.8, AS3), (20.0, 0.9, C4), (21.0, 0.9, AS3), (22.0, 1.8, GS3),
    (24.0, 7.9, F3),
];

/// The 4-bar pattern of the family: call-and-response bass, lead phrase, or four chord hits (Cm, Fm,
/// Gm, Cm voiced from C3); none for the held-note families.
fn pattern_notes(fam: Family, tempo: u32, vel: f32) -> Vec<Ev> {
    let eighth = 30.0 / tempo as f64;
    let phrase = |ph: Phrase| ph.iter().map(|&(s, l, n)| Ev { at: s as f64 * eighth, dur: l as f64 * eighth, note: n, vel }).collect::<Vec<_>>();
    match fam {
        f if f.is_bass() => phrase(BASS),
        Family::Lead => phrase(LEAD),
        Family::Pluck => {
            let bar = 240.0 / tempo as f64;
            let chords: [[u8; 3]; 4] = [[48, 51, 55], [53, 56, 60], [55, 58, 62], [48, 51, 55]];
            chords
                .iter()
                .enumerate()
                .flat_map(|(i, c)| c.iter().map(move |&n| Ev { at: i as f64 * bar, dur: 0.4 * bar, note: n, vel }).collect::<Vec<_>>())
                .collect()
        }
        _ => vec![],
    }
}

// ---- per-hold metrics --------------------------------------------------------------------------

fn lufs_of(l: &[f64], r: &[f64]) -> f64 {
    if l.len() >= 19200 {
        rp::lufs(l, r)
    } else {
        let (kl, kr) = (rp::k_weight(l), rp::k_weight(r));
        -0.691 + db10(mean_sq(&kl) + mean_sq(&kr))
    }
}

struct M {
    note: String,
    pow: f64,
    rms_db: f64,
    lufs: f64,
    win_peak: f64,
    /// Brightness as judged, and the other weighting (evidence).
    bright: f64,
    bright_alt: f64,
    cent: Vec<f64>,
    depth: f64,
    low_corr: f64,
    low_sm: f64,
    mono_diff: f64,
    width: f64,
    sub_var: f64,
}

fn analyze(h: &Hold, wfc: f64, fft: &Fft, label: &str) -> M {
    let (l, r) = h.lr();
    let (mid, _) = q::mid_side(&l, &r);
    let pow = 0.5 * (mean_sq(&l) + mean_sq(&r));
    let cent = if mid.len() >= 1024 { q::move_centroid(fft, &mid) } else { vec![] };
    let (low_corr, low_sm, mono_diff) = q::low_band(&l, &r, 120.0);
    let sub_var = {
        let sub = q::lr4(&mid, 90.0, false);
        let env = q::rms_env_db(sub.get(2400..).unwrap_or(&[]), 4400, 960);
        if env.is_empty() {
            f64::NAN
        } else {
            env.iter().cloned().fold(f64::MIN, f64::max) - env.iter().cloned().fold(f64::MAX, f64::min)
        }
    };
    M {
        note: label.to_string(),
        pow,
        rms_db: db10(pow),
        lufs: lufs_of(&l, &r),
        win_peak: l.iter().chain(&r).fold(0f64, |m, v| m.max(v.abs())),
        bright: if mid.len() < 4096 { f64::NAN } else if BRIGHT_AMP.load(Ordering::Relaxed) { q::brightness_amp(fft, &mid) } else { q::brightness(fft, &mid) },
        bright_alt: if mid.len() < 4096 { f64::NAN } else if BRIGHT_AMP.load(Ordering::Relaxed) { q::brightness(fft, &mid) } else { q::brightness_amp(fft, &mid) },
        depth: if cent.len() > 3 { q::depth_oct(&cent) } else { f64::NAN },
        cent,
        low_corr,
        low_sm,
        mono_diff,
        width: q::width_corr(&l, &r, wfc),
        sub_var,
    }
}

struct PeriodM {
    err_pct: f64,
    peak: f64,
    measured: f64,
}

/// The Move period against `expected_s`: the autocorrelation peak within +-5 % of the expected lag
/// counts when it is at least 85 % of the largest peak (else the largest peak's lag is reported, a
/// miss).
fn period_of(cent: &[f64], expected_s: f64) -> Option<PeriodM> {
    let rate = SR / MOVE_HOP as f64;
    let lc: Vec<f64> = cent.iter().map(|v| v.max(1.0).log2()).collect();
    let r = q::acf(&q::detrend(&lc), ((1.25 * expected_s).max(1.0) * rate) as usize + 2);
    let lo = (0.03 * rate) as usize;
    let all = q::peaks(&r, lo, r.len().saturating_sub(2));
    let maxv = all.iter().map(|p| p.val).fold(0.0, f64::max);
    let near = q::peak_near(&r, expected_s * rate, 0.05);
    let use_near = near.filter(|n| n.val >= 0.85 * maxv);
    let pick = use_near.or_else(|| q::dominant_peak(&r, lo, r.len().saturating_sub(2)))?;
    let measured = pick.lag / rate;
    Some(PeriodM { err_pct: 100.0 * (measured / expected_s - 1.0), peak: near.map_or(pick.val, |n| n.val), measured })
}

/// Largest autocorrelation peak beyond 30 ms (the not-periodic test).
fn max_peak(cent: &[f64]) -> f64 {
    let rate = SR / MOVE_HOP as f64;
    let lc: Vec<f64> = cent.iter().map(|v| v.max(1.0).log2()).collect();
    let r = q::acf(&q::detrend(&lc), cent.len() / 2);
    q::peaks(&r, (0.03 * rate) as usize, r.len().saturating_sub(2)).iter().map(|p| p.val).fold(0.0, f64::max)
}

// ---- the context -------------------------------------------------------------------------------

struct Ctx<'a> {
    p: &'a Preset,
    fam: Family,
    tags: Vec<String>,
    fft: &'a Fft,
    ov: HashMap<String, Bounds>,
    dom: Option<Dom>,
    per_ov: Option<PeriodOv>,
    checks: Vec<Chk>,
    notes: Vec<String>,
    raw: serde_json::Map<String, Value>,
}

impl<'a> Ctx<'a> {
    fn has(&self, t: &str) -> bool {
        self.tags.iter().any(|x| x == t)
    }

    #[allow(clippy::too_many_arguments)]
    fn put(&mut self, id: &str, group: &str, label: &str, tempo: Option<u32>, v: f64, lo: Option<f64>, hi: Option<f64>, unit: &str, detail: impl Into<String>) {
        let (lo, hi) = self.ov.get(id).copied().unwrap_or((lo, hi));
        let (status, margin) = judge(v, lo, hi);
        self.checks.push(Chk {
            id: id.into(),
            group: group.into(),
            label: label.into(),
            tempo,
            value: v.is_finite().then_some(v),
            lo,
            hi,
            unit: unit.into(),
            status,
            margin,
            detail: detail.into(),
        });
    }

    /// Per-note values: the worst one is judged.
    #[allow(clippy::too_many_arguments)]
    fn put_notes(&mut self, id: &str, group: &str, label: &str, tempo: Option<u32>, vals: &[(String, f64)], lo: Option<f64>, hi: Option<f64>, unit: &str) {
        let (lo, hi) = self.ov.get(id).copied().unwrap_or((lo, hi));
        let (v, d) = worst(vals, lo, hi);
        self.put(id, group, label, tempo, v, lo, hi, unit, d);
    }

    fn skip(&mut self, id: &str, group: &str, label: &str, tempo: Option<u32>, why: &str) {
        self.checks.push(Chk {
            id: id.into(),
            group: group.into(),
            label: label.into(),
            tempo,
            value: None,
            lo: None,
            hi: None,
            unit: String::new(),
            status: Status::Skip,
            margin: None,
            detail: why.into(),
        });
    }

    fn expected(&self, tempo: u32) -> Expect {
        let t = tempo as f64;
        match &self.per_ov {
            Some(PeriodOv::None) => return Expect::Disabled,
            Some(PeriodOv::Secs(s)) => return Expect::Periodic(*s, format!("qa_period {s} s")),
            Some(PeriodOv::Div(d)) => return Expect::Periodic(div_beats(*d) * 60.0 / t, format!("qa_period {}", d.to_string_name())),
            None => {}
        }
        match &self.dom {
            None => Expect::NoLfo,
            Some(d) if d.random() => Expect::Aperiodic(d.name()),
            Some(d) => Expect::Periodic(d.period(t), d.name()),
        }
    }

    fn period_s(&self, tempo: u32) -> Option<f64> {
        match self.expected(tempo) {
            Expect::Periodic(p, _) => Some(p),
            _ => None,
        }
    }

    fn plan(&self, tempo: u32) -> Plan {
        let bar = 240.0 / tempo as f64;
        let long = |n: f64| {
            let hold = (n * bar).max(2.4 * self.period_s(tempo).unwrap_or(0.0)).min(12.0);
            Plan { hold, tail: 0.5, a: SKIP_S, b: hold }
        };
        match self.fam {
            Family::Pluck => Plan { hold: 0.5, tail: 1.5, a: SKIP_S, b: 0.3 },
            Family::Fx => Plan { hold: 4.0 * bar, tail: 1.0, a: SKIP_S, b: 4.0 * bar },
            Family::Seq => long(4.0),
            Family::Init => Plan { hold: 1.0, tail: 0.6, a: SKIP_S, b: 1.0 },
            _ => long(2.0),
        }
    }

    fn width_fc(&self) -> f64 {
        if self.fam == Family::Lead {
            300.0
        } else {
            200.0
        }
    }

    fn test_notes(&self) -> Vec<(u8, &'static str)> {
        match self.fam {
            f if f.is_bass() => vec![(F1, "F1"), (G1, "G1"), (AS1, "A#1")],
            Family::Lead => vec![(C3, "C3"), (E3, "E3"), (G4, "G4")],
            Family::Pluck => vec![if self.has("sub") { (C2, "C2") } else { (C3, "C3") }],
            Family::Fx => vec![(C3, "C3")],
            Family::Seq => vec![(F1, "F1"), (AS1, "A#1")],
            _ => vec![(C2, "C2")],
        }
    }

    fn hold(&self, tempo: u32, note: u8, vel: f32, pl: Plan, mode: Mode, macros: Option<[f32; 4]>) -> Hold {
        let job = Job { tempo, notes: vec![Ev { at: 0.0, dur: pl.hold, note, vel }], len: pl.hold + pl.tail, mode, macros };
        let fr = render(self.p, &job);
        let n = fr.len();
        Hold { fr, note, a: smp(pl.a).min(n), b: smp(pl.b).min(n), off: smp(pl.hold).min(n) }
    }

    fn pattern(&self, tempo: u32, vel: f32) -> Option<Vec<[f32; 2]>> {
        let notes = pattern_notes(self.fam, tempo, vel);
        if notes.is_empty() {
            return None;
        }
        let len = 4.0 * 240.0 / tempo as f64 + 0.5;
        Some(render(self.p, &Job { tempo, notes, len, mode: Mode::LIVE, macros: None }))
    }

    fn category_median(&self) -> Option<f64> {
        let c = self.p.category.to_lowercase();
        match c.split('/').next().unwrap_or("").trim() {
            "bass" => Some(-18.0),
            "lead" => Some(-20.0),
            "pluck" => Some(-21.0),
            "seq" => Some(-18.0),
            "fx" => Some(-22.0),
            _ => None,
        }
    }
}

// ---- dominant LFO ------------------------------------------------------------------------------

/// The most-modulating periodic LFO: with several routed, each is rendered alone (everything else
/// frozen) and the one with the deepest Move centroid movement wins; Seq presets take the step LFO.
fn find_dominant(p: &Preset, fam: Family, fft: &Fft) -> Option<Dom> {
    let mut c: Vec<Dom> = (0..4).filter(|&k| lfo_weight(p, k) > 0.0 && lfo_periodic(p, k)).map(|k| Dom::of(p, k)).collect();
    let by_weight = |c: &mut Vec<Dom>| {
        c.sort_by(|a, b| b.weight.total_cmp(&a.weight));
        c.first().cloned()
    };
    match c.len() {
        0 => None,
        1 => c.pop(),
        _ if fam == Family::Seq => {
            c.sort_by(|a, b| (b.steps.is_some(), b.weight).partial_cmp(&(a.steps.is_some(), a.weight)).unwrap());
            c.first().cloned()
        }
        _ => {
            let note = if fam == Family::Lead { C3 } else { F1 };
            for d in &mut c {
                let job = Job {
                    tempo: 150,
                    notes: vec![Ev { at: 0.0, dur: 2.0, note, vel: VEL100 }],
                    len: 2.0,
                    mode: Mode { strip: Strip::Keep(d.idx), ..Mode::LIVE },
                    macros: None,
                };
                let fr = render(p, &job);
                let mid: Vec<f64> = fr[smp(SKIP_S)..].iter().map(|f| 0.5 * (f[0] + f[1]) as f64).collect();
                d.depth = q::depth_oct(&q::move_centroid(fft, &mid));
            }
            let best = c.iter().map(|d| d.depth).fold(0.0, f64::max);
            if best < 0.02 {
                return by_weight(&mut c);
            }
            c.into_iter().max_by(|a, b| a.depth.total_cmp(&b.depth))
        }
    }
}

// ---- spec table (bank-recipes.md A.1) ----------------------------------------------------------

struct Spec {
    rms: (f64, f64),
    lufs: (f64, f64),
    crest: (f64, f64),
    low_corr: Option<f64>,
    bright: (f64, f64),
    width: (f64, f64),
    alias: f64,
}

// Calibration 2026-10-18: hold-window RMS/LUFS of sub-heavy families shifted +1.5 dB (the sheet's bands
// came from SQ-37, measured on a pattern with rests); Init brightness band moved to the amplitude reading.
fn spec(c: &Ctx) -> Spec {
    let s = |rms, lufs, crest, low_corr, bright, width, alias| Spec { rms, lufs, crest, low_corr, bright, width, alias };
    let f = &c.fam;
    match f {
        Family::Riddim => s((-18.5, -14.5), (-19.5, -13.5), (6., 13.), Some(0.99), (250., 1500.), (0.6, 1.0), -50.),
        Family::Wobble => s((-18.5, -14.5), (-19.5, -13.5), (5., 11.), Some(0.99), (180., 1600.), (0.7, 1.0), -55.),
        Family::Tearout => s((-18.5, -14.5), (-18.5, -12.5), (5., 10.), Some(0.95), (600., 3500.), (0.2, 0.8), -50.),
        Family::Formant => s((-18.5, -14.5), (-19.5, -13.5), (6., 12.), Some(0.99), (350., 2200.), (0.85, 1.0), -55.),
        Family::Reese => s((-18.5, -14.5), (-19.5, -13.5), (5., 10.), Some(if c.has("wide") { 0.95 } else { 0.97 }), (180., 2200.), (0.2, 0.9), -50.),
        Family::Sub if pure_sub(c.p) => s((-18.5, -14.5), (-19.5, -13.5), (3., 7.), Some(0.995), (55., 160.), (0.99, 1.0), -70.),
        Family::Sub => s((-18.5, -14.5), (-19.5, -13.5), (3., 7.), Some(0.995), (55., 160.), (0.95, 1.0), -60.),
        Family::Lead => {
            let w = if c.has("clean") { (0.6, 0.95) } else { (0.1, 0.7) };
            s((-22., -18.), (-22., -16.), (6., 13.), None, (1500., 4500.), w, if c.has("square") { -60. } else { -50. })
        }
        Family::Pluck => s((-24., -18.), (-24., -18.), (10., 20.), Some(0.97), (600., 3500.), (0.3, 1.0), -50.),
        Family::Fx => s((-26., -20.), (-26., -20.), (7., 16.), Some(0.9), (800., 6000.), (0.0, 0.7), -45.),
        Family::Seq => s((-18.5, -14.5), (-19.5, -13.5), (6., 13.), Some(0.99), (250., 1400.), (0.85, 1.0), -55.),
        Family::Init => s((-99., 0.), (-99., 0.), (0., 99.), Some(0.99), (300., 1000.), (0.95, 1.0), -70.),
    }
}

/// The pure-sine sub: no distortion, no sub drive, sine sub, a plain Basic table.
fn pure_sub(p: &Preset) -> bool {
    let pa = &p.patch;
    !pa.fx.dist.on && pa.sub_drive == 0.0 && pa.sub_shape == gts::params::SubShape::Sine && pa.noise_level_db <= -59.0
}

// ---- shared check groups -----------------------------------------------------------------------

fn lin(db: f64) -> f64 {
    10f64.powf(db / 10.0)
}

fn mean_db(v: &[f64]) -> f64 {
    db10(mean(&v.iter().map(|d| lin(*d)).collect::<Vec<_>>()))
}

/// Level, loudness, crest and stereo checks shared by every family, from the per-note metrics.
fn level_checks(c: &mut Ctx, tempo: u32, ms: &[M], sp: &Spec) {
    let t = Some(tempo);
    let rms = db10(mean(&ms.iter().map(|m| m.pow).collect::<Vec<_>>()));
    let lufs = mean_db(&ms.iter().map(|m| m.lufs).collect::<Vec<_>>());
    let peak = ms.iter().map(|m| m.win_peak).fold(0.0, f64::max);
    let crest = db10(peak * peak) - rms;
    if c.fam != Family::Init {
        c.put("rms", "level", "RMS over the hold, dBFS", t, rms, Some(sp.rms.0), Some(sp.rms.1), "dBFS", format!("per note: {}", per_note(ms, |m| m.rms_db)));
        c.put("lufs", "level", "LUFS-I over the hold", t, lufs, Some(sp.lufs.0), Some(sp.lufs.1), "LUFS", format!("per note: {}", per_note(ms, |m| m.lufs)));
        c.put("crest", "level", "Crest (peak - RMS)", t, crest, Some(sp.crest.0), Some(sp.crest.1), "dB", "");
        if let Some(med) = c.category_median() {
            c.put("rms_cat_median", "level", "RMS within 2 dB of the category median", t, rms - med, Some(-2.0), Some(2.0), "dB", format!("category median {med} dBFS"));
        }
    }
    c.raw.insert(format!("rms_db@{tempo}"), json!(rms));
    c.raw.insert(format!("lufs@{tempo}"), json!(lufs));
}

fn per_note(ms: &[M], f: impl Fn(&M) -> f64) -> String {
    ms.iter().map(|m| format!("{} {}", m.note, fmt(f(m)))).collect::<Vec<_>>().join(", ")
}

fn vals(ms: &[M], f: impl Fn(&M) -> f64) -> Vec<(String, f64)> {
    ms.iter().map(|m| (m.note.clone(), f(m))).collect()
}

/// Low end, Brightness, Width, Move depth and period shared by the held-note families.
fn stereo_spectral_checks(c: &mut Ctx, tempo: u32, ms: &[M], sp: &Spec) {
    let t = Some(tempo);
    if let Some(lc) = sp.low_corr {
        c.put_notes("low_corr", "low end", "Low-band (<120 Hz) L/R correlation", t, &vals(ms, |m| m.low_corr), Some(lc), None, "");
    }
    if c.fam.is_bass() {
        // Judged for Riddim (the sheet's row); Info for the other bass families.
        let hi = (c.fam == Family::Riddim).then_some(-30.0);
        c.put_notes("low_sm", "low end", "Low-band side/mid, dB", t, &vals(ms, |m| m.low_sm), None, hi, "dB");
    }
    if c.fam.is_bass() {
        c.put_notes("mono_diff", "low end", "Mono-sum vs stereo energy below 120 Hz", t, &vals(ms, |m| m.mono_diff), Some(-1.0), Some(1.0), "dB");
    }
    let (b0, b1) = sp.bright;
    c.put_notes("bright", "spectrum", "Brightness (power centroid)", t, &vals(ms, |m| m.bright), Some(b0), Some(b1), "Hz");
    if let Some(ch) = c.checks.last_mut() {
        let other = if BRIGHT_AMP.load(Ordering::Relaxed) { "power" } else { "amplitude" };
        ch.detail += &format!("; {other}-weighted centroid for comparison: {}", per_note(ms, |m| m.bright_alt));
    }
    let (w0, w1) = sp.width;
    let fc = c.width_fc();
    c.put_notes("width", "stereo", &format!("Width: L/R correlation above {fc} Hz"), t, &vals(ms, |m| m.width), Some(w0), Some(w1), "");
}

fn depth_period_checks(c: &mut Ctx, tempo: u32, ms: &[M], min_depth: Option<f64>) {
    let t = Some(tempo);
    let exp = c.expected(tempo);
    let short = matches!(&exp, Expect::Periodic(p, _) if *p < 0.08);
    if let Some(md) = min_depth {
        let md = if short { md.min(0.4) } else { md };
        c.put_notes("depth", "movement", "Move depth (p95-p5 of log2 centroid)", t, &vals(ms, |m| m.depth), Some(md), None, "oct");
    }
    match exp {
        Expect::Disabled => c.skip("period_err", "movement", "Move period", t, "qa_period=none"),
        Expect::NoLfo => c.put("period_err", "movement", "Move period", t, f64::NAN, Some(-5.0), Some(5.0), "%", "no periodic LFO is routed (use the tag qa_period=none for per-note presets)"),
        Expect::Aperiodic(src) => {
            let v = vals(ms, |m| max_peak(&m.cent));
            c.put_notes("aperiodic", "movement", &format!("Not periodic: largest ACF peak beyond 30 ms ({src})"), t, &v, None, Some(0.4), "");
        }
        Expect::Periodic(p, src) => {
            let thr = if p < 0.08 { 0.4 } else { 0.5 };
            let r: Vec<Option<PeriodM>> = ms.iter().map(|m| period_of(&m.cent, p)).collect();
            let errs: Vec<(String, f64)> = ms.iter().zip(&r).map(|(m, r)| (m.note.clone(), r.as_ref().map_or(f64::NAN, |r| r.err_pct))).collect();
            let peaks: Vec<(String, f64)> = ms.iter().zip(&r).map(|(m, r)| (m.note.clone(), r.as_ref().map_or(f64::NAN, |r| r.peak))).collect();
            let meas = r.iter().flatten().map(|r| r.measured).fold(f64::NAN, |a, b| if a.is_nan() { b } else { a.min(b) });
            c.put_notes("period_err", "movement", &format!("Move period vs {src} = {:.4} s (error)", p), t, &errs, Some(-5.0), Some(5.0), "%");
            if let Some(ch) = c.checks.last_mut() {
                ch.detail += &format!(" (measured {meas:.4} s)");
            }
            c.put_notes("period_peak", "movement", "Move period ACF peak height", t, &peaks, Some(thr), None, "");
        }
    }
}

// ---- alias floor, frozen render ----------------------------------------------------------------

fn unison_spread_cents(p: &Preset) -> f64 {
    let pa = &p.patch;
    let uni = pa.osc.iter().filter(|o| o.on && o.unison > 1).map(|o| o.detune as f64).fold(0.0, f64::max);
    let both = pa.osc[0].on && pa.osc[1].on;
    let off = if both { ((pa.osc[0].cent + 100.0 * pa.osc[0].semi as f32) - (pa.osc[1].cent + 100.0 * pa.osc[1].semi as f32)).abs() as f64 } else { 0.0 };
    uni + if off < 100.0 { off } else { 0.0 }
}

/// The pitch of the harmonic grid: the played note, or the sub-octave below it when a layer (the sub
/// oscillator follows oscillator A, `sub_oct` below it) sounds lower.
fn grid_base(p: &Preset, note: u8) -> f64 {
    let f0 = hz(note);
    let pa = &p.patch;
    let freq = |o: &gts::dsp::OscPatch| f0 * 2f64.powf(o.oct as f64 + o.semi as f64 / 12.0 + o.cent as f64 / 1200.0);
    let mut lowest = pa.osc.iter().filter(|o| o.on).map(freq).fold(f64::MAX, f64::min);
    if pa.sub_on {
        let mul = match pa.sub_oct {
            o if o <= -2 => 0.25,
            -1 => 0.5,
            _ => 1.0,
        };
        lowest = lowest.min(freq(&pa.osc[0]) * mul);
    }
    if lowest < 0.9 * f0 {
        f0 * 2f64.powi(-((f0 / lowest).log2().round() as i32))
    } else {
        f0
    }
}

fn detail_dry() -> &'static str {
    "oscillators, filters and shaper only"
}

fn alias_window(fam: Family, decaying: bool) -> (f64, usize) {
    match fam {
        _ if decaying => (0.01, 16384),
        Family::Pluck => (0.01, 16384),
        Family::Init => (0.2, 32768),
        _ => (0.5, 65536),
    }
}

fn alias_checks(c: &mut Ctx, sp: &Spec, live150: &[M]) {
    let (note, label) = c.test_notes()[0];
    let tempo = 150;
    let pl = c.plan(tempo);
    let fz = c.hold(tempo, note, VEL100, pl, Mode::FROZEN, None);
    let (l, r) = fz.lr();
    let frozen_rms = db10(0.5 * (mean_sq(&l) + mean_sq(&r)));
    let live_rms = live150.iter().find(|m| m.note == label).map_or(f64::NAN, |m| m.rms_db);
    let (start, n) = alias_window(c.fam, c.fam == Family::Fx && fx_kind(c) == FxKind::Impact);
    let a = smp(start);
    if fz.fr.len() < a + n {
        c.skip("alias", "alias", "Alias floor", Some(tempo), "hold too short for the alias window");
        return;
    }
    let mono: Vec<f64> = fz.fr[a..a + n].iter().map(|f| 0.5 * (f[0] + f[1]) as f64).collect();
    let (p, df) = q::bh_power(c.fft, &mono);
    let alias = q::alias_floor_db(&p, df, grid_base(c.p, note), unison_spread_cents(c.p));
    let noisy = c.p.patch.noise_level_db > -59.0 && c.fam == Family::Fx;
    let detail = format!("{label} frozen render, grid {:.2} Hz, spread {:.1} ct", grid_base(c.p, note), unison_spread_cents(c.p));
    if noisy {
        c.put("alias", "alias", "Alias floor (frozen)", Some(tempo), alias, None, None, "dB", format!("{detail}; noise layer on, inharmonic by construction (Info)"));
    } else {
        c.put("alias", "alias", "Alias floor (frozen modulation)", Some(tempo), alias, None, Some(sp.alias), "dB", detail);
    }
    c.raw.insert("alias_db".into(), json!(alias));
    // The same with the FX chain off: separates the engine's aliasing from FX modulation sidebands.
    if !noisy {
        let dry = c.hold(tempo, note, VEL100, pl, Mode::FROZEN_DRY, None);
        let mono: Vec<f64> = dry.fr[a..a + n].iter().map(|f| 0.5 * (f[0] + f[1]) as f64).collect();
        let (p, df) = q::bh_power(c.fft, &mono);
        let ad = q::alias_floor_db(&p, df, grid_base(c.p, note), unison_spread_cents(c.p));
        c.put("alias_dry", "alias", "Alias floor, frozen and FX off (Info)", Some(tempo), ad, None, None, "dB", detail_dry());
    }
    c.put("frozen_valid", "alias", "Frozen render within 1 dB RMS of live", Some(tempo), frozen_rms - live_rms, Some(-1.0), Some(1.0), "dB", format!("frozen {frozen_rms:.2} dBFS, live {live_rms:.2} dBFS"));
    if c.fam == Family::Sub {
        let sh = q::band_fraction(&p, df, 150.0);
        let thd = q::thd_percent(&p, df, hz(note), 150.0);
        c.put("sub_share", "low end", "Energy below 150 Hz (share)", Some(tempo), 100.0 * sh, Some(if pure_sub(c.p) { 99.0 } else { 90.0 }), None, "%", "F1, frozen");
        if pure_sub(c.p) {
            c.put("thd", "low end", "THD below 150 Hz", Some(tempo), thd, None, Some(0.3), "%", "F1, frozen");
        } else {
            c.put("thd", "low end", "THD below 150 Hz", Some(tempo), thd, None, None, "%", "Info; use qa_thd for a target");
        }
    }
}

// ---- macro sweeps ------------------------------------------------------------------------------

const MACRO_STEPS: [f32; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];

fn macro_checks(c: &mut Ctx) {
    let (note, _) = c.test_notes()[0];
    let tempo = 150;
    let pl = c.plan(tempo);
    let rest = c.p.patch.macros;
    let labels = c.p.matrix_state().macros;
    let wfc = c.width_fc();
    for k in 0..4 {
        let id = format!("macro{}", k + 1);
        let group = "macros";
        let label = format!("Macro {} \"{}\" sweep", k + 1, labels[k]);
        if c.fam == Family::Init {
            c.skip(&id, group, &label, None, "Init macros are unassigned");
            continue;
        }
        let spec = tag_value(&c.tags, &format!("qa_macro{}", k + 1));
        let (kind, thr) = match spec.as_deref().and_then(|s| s.split_once(':')) {
            Some((a, b)) => (a.to_lowercase(), b.parse::<f64>().ok()),
            None => (String::new(), None),
        };
        let timing = kind.starts_with('t');
        // (name, unit, threshold, monotonic tolerance, series)
        let mut metrics: Vec<(&str, &str, f64, f64, Vec<f64>)> = vec![];
        if timing {
            let mut vals = vec![];
            for &m in &MACRO_STEPS {
                let mut mm = rest;
                mm[k] = m;
                let h = c.hold(tempo, note, VEL100, Plan { hold: 1.0, tail: 2.0, a: 0.0, b: 1.0 }, Mode::LIVE, Some(mm));
                let mono: Vec<f64> = h.fr.iter().map(|f| 0.5 * (f[0] + f[1]) as f64).collect();
                let (win, hop_s) = (96, 96.0 / SR);
                let env = q::rms_env_db(&mono, win, win);
                let amp: Vec<f64> = env.iter().map(|d| 10f64.powf(d / 20.0)).collect();
                let ip = amp.iter().enumerate().take(smp(1.0) / win).fold((0, 0.0), |b, (i, v)| if *v > b.1 { (i, *v) } else { b }).0;
                let ms = match kind.as_str() {
                    "tatt" => q::rise_time(&amp[..smp(0.5) / win], hop_s, 0.9),
                    "tfall" => q::fall_time(&env, hop_s, ip, 20.0),
                    _ => q::fall_time(&env, hop_s, (smp(1.0) / win).saturating_sub(5), 40.0),
                };
                vals.push(ms.map_or(f64::NAN, |s| s * 1000.0));
            }
            metrics.push(("T", "ms", thr.unwrap_or(10.0), 5.0, vals));
        } else {
            let (mut cs, mut ds, mut ls, mut ws) = (vec![], vec![], vec![], vec![]);
            for &m in &MACRO_STEPS {
                let mut mm = rest;
                mm[k] = m;
                let h = c.hold(tempo, note, VEL100, pl, Mode::LIVE, Some(mm));
                let a = analyze(&h, wfc, c.fft, "");
                cs.push(a.bright.max(1.0).log2());
                ds.push(a.depth);
                ls.push(a.lufs);
                ws.push(a.width);
            }
            let want = kind.chars().next();
            let all: [(&str, &str, f64, f64, Vec<f64>); 4] =
                [("C", "oct", 0.3, 0.05, cs), ("D", "oct", 0.3, 0.05, ds), ("L", "LU", 1.0, 0.2, ls), ("W", "corr", 0.1, 0.02, ws)];
            for m in all {
                if want.is_none() || want.is_some_and(|w| m.0.to_lowercase().starts_with(w)) {
                    let mut m = m;
                    if let (Some(t), true) = (thr, want.is_some()) {
                        m.2 = t;
                    }
                    metrics.push(m);
                }
            }
        }
        let range = |v: &[f64]| v.iter().cloned().fold(f64::MIN, f64::max) - v.iter().cloned().fold(f64::MAX, f64::min);
        let eval: Vec<(String, f64, f64, bool, &str)> = metrics.iter().map(|(n, u, t, tol, v)| (n.to_string(), range(v), *t, q::monotonic(v, *tol), *u)).collect();
        let detail = eval
            .iter()
            .zip(&metrics)
            .map(|((n, r, t, mono, u), m)| format!("{n} {} {u} (min {t}){} [{}]", fmt(*r), if *mono { "" } else { " non-monotonic" }, m.4.iter().map(|x| fmt(*x)).collect::<Vec<_>>().join(" ")))
            .collect::<Vec<_>>()
            .join("; ");
        // Best passing metric (audible and monotonic), else the closest one.
        let good = eval.iter().filter(|e| e.1.is_finite() && e.1 >= e.2 && e.3).max_by(|a, b| (a.1 / a.2).total_cmp(&(b.1 / b.2)));
        let pick = good.or_else(|| eval.iter().filter(|e| e.1.is_finite()).max_by(|a, b| (a.1 / a.2).total_cmp(&(b.1 / b.2))));
        let Some(e) = pick else {
            c.put(&id, group, &label, None, f64::NAN, None, None, "", detail);
            continue;
        };
        let status = if good.is_some() { Status::Pass } else { Status::Fail };
        let why = if good.is_some() { String::new() } else { "; no metric is both audible and monotonic".to_string() };
        c.checks.push(Chk {
            id,
            group: group.into(),
            label,
            tempo: None,
            value: Some(e.1),
            lo: Some(e.2),
            hi: None,
            unit: format!("{} {}", e.0, e.4),
            status,
            margin: Some(e.1 - e.2),
            detail: detail + &why,
        });
    }
}

// ---- family flows ------------------------------------------------------------------------------

/// Peak checks, NaN check, E1 floor; returns per-tempo held-note metrics at 150 for the alias check.
fn held_flow(c: &mut Ctx) -> Vec<M> {
    let sp = spec(c);
    let notes = c.test_notes();
    let wfc = c.width_fc();
    let mut at150 = vec![];
    for &tempo in &TEMPOS {
        let pl = c.plan(tempo);
        let mut ms = vec![];
        let mut peak100 = 0f64;
        let mut finite = true;
        for &(n, label) in &notes {
            let h = c.hold(tempo, n, VEL100, pl, Mode::LIVE, None);
            peak100 = peak100.max(h.peak());
            finite &= finite_of(&h.fr);
            ms.push(analyze(&h, wfc, c.fft, label));
        }
        if let Some(pat) = c.pattern(tempo, VEL100) {
            peak100 = peak100.max(peak_of(&pat));
            finite &= finite_of(&pat);
        }
        let t = Some(tempo);
        c.put("nan", "level", "No NaN or inf in the output", t, if finite { 0.0 } else { 1.0 }, None, Some(0.0), "", "");
        if c.fam != Family::Init {
            c.put("peak100", "level", "Peak at velocity 100, dBFS", t, q::db20(peak100), None, Some(-3.0), "dBFS", "holds and the 4-bar pattern, after the limiter");
        }
        level_checks(c, tempo, &ms, &sp);
        stereo_spectral_checks(c, tempo, &ms, &sp);
        match c.fam {
            Family::Seq | Family::Init | Family::Pluck | Family::Fx => {}
            Family::Lead => {
                // Energy below 150 Hz against the 300 Hz - 3 kHz band.
                let v: Vec<(String, f64)> = notes
                    .iter()
                    .map(|&(n, label)| {
                        let h = c.hold(tempo, n, VEL100, pl, Mode::LIVE, None);
                        let (l, r) = h.lr();
                        let mono = q::mid_side(&l, &r).0;
                        let n2 = 65536.min(mono.len().next_power_of_two() / 2);
                        let (p, df) = q::bh_power(c.fft, &mono[..n2]);
                        (label.to_string(), q::band_db(&p, df, 20.0, 150.0) - q::band_db(&p, df, 300.0, 3000.0))
                    })
                    .collect();
                c.put_notes("low_vs_mid", "low end", "Energy below 150 Hz vs the 300 Hz - 3 kHz band", t, &v, None, Some(-20.0), "dB");
            }
            _ => {
                let md = match c.fam {
                    Family::Riddim => Some(0.6),
                    Family::Wobble => Some(0.8),
                    Family::Tearout => Some(1.0),
                    Family::Formant => Some(1.0),
                    Family::Reese => Some(if c.has("neuro") { 0.8 } else { 0.4 }),
                    Family::Sub => None,
                    _ => None,
                };
                if c.fam == Family::Sub {
                    let lim = if pure_sub(c.p) { 0.1 } else { 0.25 };
                    c.put_notes("depth", "movement", "Static: Move depth", t, &vals(&ms, |m| m.depth), None, Some(lim), "oct");
                } else {
                    depth_period_checks(c, tempo, &ms, md);
                }
                if c.fam != Family::Sub {
                    // Judged for Wobble (the sheet's row); Info for the other bass families.
                    let lim = (c.fam == Family::Wobble).then(|| if c.dom.as_ref().is_some_and(|d| d.curve == Curve::Square) { 2.0 } else { 1.5 });
                    c.put_notes("sub_var", "low end", "Sub (<90 Hz) RMS variation over the cycle", t, &vals(&ms, |m| m.sub_var), None, lim, "dB");
                }
                if matches!(c.fam, Family::Riddim | Family::Wobble | Family::Sub) {
                    let h = c.hold(tempo, E1, VEL100, c.plan(tempo), Mode::LIVE, None);
                    let m = analyze(&h, wfc, c.fft, "E1");
                    let base = db10(mean(&ms.iter().map(|m| m.pow).collect::<Vec<_>>()));
                    c.put("e1_floor", "low end", "E1 floor: RMS within 3 dB of F1/G1/A#1 (tool-defined)", t, m.rms_db - base, Some(-3.0), Some(3.0), "dB", format!("E1 {:.2} dBFS vs {:.2}", m.rms_db, base));
                }
            }
        }
        if tempo == 150 {
            at150 = ms;
        }
    }
    // Velocity 127 peak (150 BPM): the pattern and the first and last test notes.
    let pl = c.plan(150);
    let mut peak127 = 0f64;
    if let Some(pat) = c.pattern(150, VEL127) {
        peak127 = peak_of(&pat);
    }
    let mut ends = vec![notes[0].0, notes[notes.len() - 1].0];
    ends.dedup();
    for n in ends {
        peak127 = peak127.max(c.hold(150, n, VEL127, pl, Mode::LIVE, None).peak());
    }
    if c.fam == Family::Init {
        c.put("peak100", "level", "Peak at velocity 100, dBFS (Init band -12..-6)", Some(150), q::db20(c.hold(150, notes[0].0, VEL100, pl, Mode::LIVE, None).peak()), Some(-12.0), Some(-6.0), "dBFS", "");
    }
    c.put("peak127", "level", "Peak at velocity 127, dBFS", Some(150), q::db20(peak127), None, Some(-3.0), "dBFS", "");
    at150
}

/// Reese: beat rate of the fundamental and beating depth on the frozen dry render (sub off, so the
/// detuned oscillators are measured alone). Expected rate: `f0 (2^(span/1200) - 1)` for two voices;
/// with `n` equally spread unison voices the envelope repeats at the voice spacing, span / (n - 1).
fn reese_checks(c: &mut Ctx) {
    let pa = &c.p.patch;
    let osc = pa.osc.iter().filter(|o| o.on && o.unison > 1).max_by(|a, b| a.detune.total_cmp(&b.detune));
    let ratio = |cents: f64| 2f64.powf(cents / 1200.0) - 2f64.powf(-cents / 1200.0);
    let (rate_per_hz, src) = if let Some(v) = tag_value(&c.tags, "qa_beat").and_then(|v| v.parse::<f64>().ok()) {
        (v / hz(F1), "qa_beat tag".to_string())
    } else if let Some(o) = osc {
        let n = o.unison.max(2) as f64;
        (ratio(o.detune as f64) / (n - 1.0), format!("{} unison voices, outer +-{} ct", o.unison, o.detune))
    } else {
        let d = unison_spread_cents(c.p);
        (2f64.powf(d / 1200.0) - 1.0, format!("oscillator offset {d:.1} ct"))
    };
    let pl = c.plan(150);
    let mut errs = vec![];
    let mut depth = vec![];
    let mut detail = vec![];
    for (n, label) in [(F1, "F1"), (AS1, "A#1")] {
        let h = c.hold(150, n, VEL100, pl, Mode::FROZEN_DRY_NO_SUB, None);
        let (l, r) = h.lr();
        let mono = q::mid_side(&l, &r).0;
        let f0 = hz(n);
        let band = q::bandpass(&mono, 0.9 * f0, 1.1 * f0);
        let env = q::rms_env_db(&band[smp(0.2).min(band.len())..], 960, 480);
        let (rate, series) = (SR / 480.0, env.iter().map(|d| lin(*d).sqrt()).collect::<Vec<_>>());
        let r = q::acf(&q::detrend(&series), series.len() / 2);
        let want = rate_per_hz * f0;
        let beat = q::dominant_peak(&r, (rate / (5.0 * want.max(0.2))) as usize, r.len().saturating_sub(2)).map(|p| rate / p.lag);
        errs.push((label.to_string(), beat.map_or(f64::NAN, |b| 100.0 * (b / want - 1.0))));
        detail.push(format!("{label}: measured {} Hz, expected {want:.3} Hz", beat.map_or("none".into(), |b| format!("{b:.3}"))));
        let hi = q::bandpass(&mono, 100.0, 300.0);
        let e2 = q::rms_env_db(&hi[smp(0.2).min(hi.len())..], 960, 480);
        depth.push((label.to_string(), q::percentile(&e2, 0.95) - q::percentile(&e2, 0.05)));
    }
    c.put_notes("beat_err", "movement", &format!("Reese beat rate at f0 vs expected ({src}), error"), Some(150), &errs, Some(-10.0), Some(10.0), "%");
    if let Some(ch) = c.checks.last_mut() {
        ch.detail = detail.join("; ");
    }
    let lo = if c.has("neuro") { None } else { Some(3.0) };
    c.put_notes("beat_depth", "movement", "Beating depth, 100-300 Hz envelope (p95-p5)", Some(150), &depth, lo, None, "dB");
}

/// Tearout: the 40-90 Hz band of the full render against the sub oscillator alone.
fn sub_band_check(c: &mut Ctx) {
    let pl = c.plan(150);
    let band = |h: &Hold| {
        let (l, r) = h.lr();
        let (b0, b1) = (q::bandpass(&l, 40.0, 90.0), q::bandpass(&r, 40.0, 90.0));
        db10(0.5 * (mean_sq(&b0) + mean_sq(&b1)))
    };
    let full = band(&c.hold(150, F1, VEL100, pl, Mode::LIVE, None));
    let sub = band(&c.hold(150, F1, VEL100, pl, Mode::SUB_ONLY, None));
    c.put("sub_band", "low end", "Clean sub: 40-90 Hz RMS vs the sub oscillator alone", Some(150), full - sub, Some(-2.0), Some(2.0), "dB", format!("full {full:.2} dB, sub only {sub:.2} dB"));
}

/// Pitch dive after note-on (tags dive, impact).
fn dive_checks(c: &mut Ctx) {
    let (note, label) = c.test_notes()[0];
    let f0 = hz(note);
    let h = c.hold(150, note, VEL100, Plan { hold: 1.2, tail: 0.4, a: 0.0, b: 1.2 }, Mode::LIVE, None);
    let mono: Vec<f64> = h.fr.iter().map(|f| 0.5 * (f[0] + f[1]) as f64).collect();
    let tr = q::pitch_track(&mono, smp(0.005), 240, 190, f0 / 4.0, f0 * 16.0);
    if tr.len() < 20 {
        c.skip("dive_st", "pitch", "Pitch dive", None, "pitch track failed");
        return;
    }
    let (spec_st, ms) = match tag_value(&c.tags, "qa_dive") {
        Some(v) => {
            let (a, b) = v.split_once('@').map_or((v.as_str(), None), |(a, b)| (a, Some(b)));
            (a.parse::<f64>().ok(), b.and_then(|b| b.parse::<f64>().ok()).unwrap_or(90.0))
        }
        None => (None, 90.0),
    };
    let t0 = 0.015;
    let (a, b) = (q::track_at(&tr, t0).unwrap(), q::track_at(&tr, t0 + ms / 1000.0).unwrap());
    let st = 12.0 * (a / b).log2();
    let late: Vec<f64> = tr.iter().filter(|p| p.0 >= 0.5).map(|p| p.1).collect();
    let total = 12.0 * (a / q::percentile(&late, 0.5)).log2();
    let conf = mean(&tr.iter().map(|p| p.2).collect::<Vec<_>>());
    let detail = format!("{label}: {a:.1} Hz at 15 ms -> {b:.1} Hz at {:.0} ms; settles at {:.1} Hz ({total:.1} st total); mean confidence {conf:.2}", t0 * 1000.0 + ms, q::percentile(&late, 0.5));
    match spec_st {
        Some(e) => c.put("dive_st", "pitch", &format!("Pitch dive over {ms:.0} ms (expected {e} st +-1)"), None, st, Some(e - 1.0), Some(e + 1.0), "st", detail),
        None => c.put("dive_st", "pitch", "Pitch dive total, st (tool-defined: falls at least 3 st)", None, total, Some(3.0), None, "st", detail),
    }
}

fn pitch_trend_info(c: &mut Ctx, hold: &Hold) {
    let f0 = hz(hold.note);
    let mono: Vec<f64> = hold.fr.iter().map(|f| 0.5 * (f[0] + f[1]) as f64).collect();
    let a = q::yin(&mono, smp(0.3), f0 / 2.0, f0 * 8.0);
    let b = q::yin(&mono, hold.off.saturating_sub(smp(0.6)), f0 / 2.0, f0 * 8.0);
    if let (Some(a), Some(b)) = (a, b) {
        c.put("pitch_trend", "pitch", "Pitch trend start -> end of the hold (Info)", None, 12.0 * (b.0 / a.0).log2(), None, None, "st", format!("{:.1} Hz -> {:.1} Hz, confidence {:.2} / {:.2}", a.0, b.0, a.1, b.1));
    }
}

fn lead_like_extras(c: &mut Ctx) {
    if ["dive", "impact"].iter().any(|t| c.has(t)) || tag_value(&c.tags, "qa_dive").is_some() {
        dive_checks(c);
    }
}

// ---- pluck -------------------------------------------------------------------------------------

fn pluck_flow(c: &mut Ctx) -> Vec<M> {
    let sp = spec(c);
    let (note, label) = c.test_notes()[0];
    let mut at150 = vec![];
    for &tempo in &TEMPOS {
        let pl = c.plan(tempo);
        let h = c.hold(tempo, note, VEL100, pl, Mode::LIVE, None);
        let m = analyze(&h, 200.0, c.fft, label);
        let mut peak100 = h.peak();
        let mut finite = finite_of(&h.fr);
        if let Some(pat) = c.pattern(tempo, VEL100) {
            peak100 = peak100.max(peak_of(&pat));
            finite &= finite_of(&pat);
        }
        let t = Some(tempo);
        c.put("nan", "level", "No NaN or inf in the output", t, if finite { 0.0 } else { 1.0 }, None, Some(0.0), "", "");
        c.put("peak100", "level", "Peak at velocity 100, dBFS", t, q::db20(peak100), None, Some(-3.0), "dBFS", "");
        let ms = vec![m];
        level_checks(c, tempo, &ms, &sp);
        stereo_spectral_checks(c, tempo, &ms, &sp);
        // Decay and centroid fall on the full note (not the 50-300 ms window).
        let mono: Vec<f64> = h.fr.iter().map(|f| 0.5 * (f[0] + f[1]) as f64).collect();
        let env = q::rms_env_db(&mono, 240, 240);
        let hop_s = 240.0 / SR;
        let ip = env.iter().enumerate().take(smp(0.3) / 240).fold((0, f64::MIN), |b, (i, v)| if *v > b.1 { (i, *v) } else { b }).0;
        let t20 = q::fall_time(&env, hop_s, ip, 20.0).map(|s| s * 1000.0);
        match tag_value(&c.tags, "qa_decay").and_then(|v| v.parse::<f64>().ok()) {
            Some(d) => c.put("t20", "movement", &format!("Time to -20 dB (expected {d} ms +-30 %)"), t, t20.unwrap_or(f64::NAN), Some(0.7 * d), Some(1.3 * d), "ms", ""),
            None => c.put("t20", "movement", "Time to -20 dB from the peak (tool-defined: at most 500 ms)", t, t20.unwrap_or(f64::NAN), None, Some(500.0), "ms", ""),
        }
        let cs = q::move_centroid(c.fft, &mono);
        let at = |sec: f64| {
            let i = ((sec * SR - 512.0) / MOVE_HOP as f64).round().max(0.0) as usize;
            let w = &cs[i.saturating_sub(1)..(i + 2).min(cs.len())];
            mean(w)
        };
        let fall = (at(0.010) / at(0.150)).log2();
        c.put("cent_fall", "movement", "Centroid fall from 10 ms to 150 ms", t, fall, Some(1.0), None, "oct", "");
        if tempo == 150 {
            at150 = ms;
        }
    }
    let pl = c.plan(150);
    let mut peak127 = c.hold(150, note, VEL127, pl, Mode::LIVE, None).peak();
    if let Some(pat) = c.pattern(150, VEL127) {
        peak127 = peak127.max(peak_of(&pat));
    }
    c.put("peak127", "level", "Peak at velocity 127, dBFS", Some(150), q::db20(peak127), None, Some(-3.0), "dBFS", "");
    lead_like_extras(c);
    at150
}

// ---- FX ----------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
enum FxKind {
    Riser,
    Downlifter,
    Impact,
    Sweep,
}

fn fx_kind(c: &Ctx) -> FxKind {
    let by = |w: &str| match w {
        "riser" => Some(FxKind::Riser),
        "downlifter" => Some(FxKind::Downlifter),
        "impact" => Some(FxKind::Impact),
        "sweep" => Some(FxKind::Sweep),
        _ => None,
    };
    if let Some(k) = tag_value(&c.tags, "qa_fxkind").and_then(|v| by(&v)) {
        return k;
    }
    c.tags.iter().find_map(|t| by(t)).unwrap_or(FxKind::Sweep)
}

fn fx_flow(c: &mut Ctx) -> Vec<M> {
    let kind = fx_kind(c);
    let mut sp = spec(c);
    match kind {
        FxKind::Riser => sp.bright = (1500., 6000.),
        FxKind::Downlifter => sp.bright = (800., 5000.),
        FxKind::Impact => {
            sp.bright = (60., 400.);
            sp.width = (0.5, 1.0);
            sp.low_corr = Some(0.95);
        }
        FxKind::Sweep => sp.width = (0.0, 0.6),
    }
    let (note, label) = c.test_notes()[0];
    let mut at150 = vec![];
    for &tempo in &TEMPOS {
        let mut pl = c.plan(tempo);
        let bar = 240.0 / tempo as f64;
        // Level window per kind: riser = last bar, downlifter = first bar, impact = first 800 ms.
        let win = match kind {
            FxKind::Riser => (pl.hold - bar, pl.hold),
            FxKind::Downlifter => (SKIP_S, bar),
            FxKind::Impact => (SKIP_S, 0.8),
            FxKind::Sweep => (SKIP_S, pl.hold),
        };
        let h = c.hold(tempo, note, VEL100, pl, Mode::LIVE, None);
        let whole = analyze(&h, 200.0, c.fft, label);
        pl.a = win.0;
        pl.b = win.1;
        let hw = Hold { fr: Vec::new(), note, a: smp(win.0), b: smp(win.1).min(h.fr.len()), off: h.off };
        let hw = Hold { fr: h.fr.clone(), ..hw };
        let m = analyze(&hw, 200.0, c.fft, label);
        let t = Some(tempo);
        let finite = finite_of(&h.fr);
        c.put("nan", "level", "No NaN or inf in the output", t, if finite { 0.0 } else { 1.0 }, None, Some(0.0), "", "");
        c.put("peak100", "level", "Peak at velocity 100, dBFS", t, q::db20(h.peak()), None, Some(-3.0), "dBFS", "");
        let ms = vec![M { bright: whole.bright, ..m }];
        level_checks(c, tempo, &ms, &sp);
        if kind == FxKind::Impact {
            c.put("low_corr", "low end", "Low-band L/R correlation", t, ms[0].low_corr, sp.low_corr, None, "", "");
        }
        let (l, r) = h.lr();
        let low_peak = {
            let (a, b) = (q::lr4(&l, 150.0, false), q::lr4(&r, 150.0, false));
            let env_l = q::rms_env_db(&a, 2400, 1200);
            let env_r = q::rms_env_db(&b, 2400, 1200);
            let pk = |e: &[f64]| e.iter().cloned().fold(f64::MIN, f64::max);
            pk(&env_l).max(pk(&env_r))
        };
        let total_peak = q::db20(l.iter().chain(&r).fold(0f64, |m, v| m.max(v.abs())));
        if kind != FxKind::Impact {
            let rel = low_peak + 3.0 - total_peak;
            c.put("low_energy", "low end", "Energy below 150 Hz re peak (50 ms RMS)", t, rel, None, Some(-30.0), "dB", "");
            if ms[0].low_corr.is_finite() && rel > -50.0 && sp.low_corr.is_some() {
                c.put("low_corr", "low end", "Low-band L/R correlation (there is low energy)", t, ms[0].low_corr, sp.low_corr, None, "", "");
            }
        }
        c.put("bright", "spectrum", "Brightness (power centroid, whole hold)", t, whole.bright, Some(sp.bright.0), Some(sp.bright.1), "Hz", "");
        c.put("width", "stereo", "Width: L/R correlation above 200 Hz (level window)", t, ms[0].width, Some(sp.width.0), Some(sp.width.1), "", "");
        // Trends over the hold.
        let mono: Vec<f64> = h.fr[..h.off].iter().map(|f| 0.5 * (f[0] + f[1]) as f64).collect();
        let cs: Vec<f64> = q::move_centroid(c.fft, &mono).iter().map(|v| v.log2()).collect();
        let rate = SR / MOVE_HOP as f64;
        let n10 = (cs.len() / 10).max(1);
        let rise = mean(&cs[cs.len() - n10..]) - mean(&cs[..n10]);
        let bins: Vec<f64> = cs.chunks((0.25 * rate) as usize).filter(|b| b.len() > 5).map(mean).collect();
        let frac_up = bins.windows(2).filter(|w| w[1] >= w[0] - 1e-3).count() as f64 / (bins.len().max(2) - 1) as f64;
        let frac_dn = bins.windows(2).filter(|w| w[1] <= w[0] + 1e-3).count() as f64 / (bins.len().max(2) - 1) as f64;
        let first = Hold { fr: h.fr.clone(), note, a: smp(SKIP_S), b: smp(bar), off: h.off };
        let last = Hold { fr: h.fr.clone(), note, a: smp(pl.hold - bar), b: smp(pl.hold), off: h.off };
        let (m1, m2) = (analyze(&first, 200.0, c.fft, ""), analyze(&last, 200.0, c.fft, ""));
        let level_tr = m2.rms_db - m1.rms_db;
        let width_tr = m2.width - m1.width;
        match kind {
            FxKind::Riser => {
                c.put("fx_cent_trend", "movement", "Centroid rise, first to last 10 % of the hold", t, rise, Some(2.0), None, "oct", "");
                c.put("fx_monotone", "movement", "Share of 250 ms centroid steps rising", t, frac_up, Some(0.9), None, "", "");
                c.put("fx_level_trend", "movement", "Level rise, last bar minus first bar", t, level_tr, Some(3.0), Some(8.0), "dB", "");
                c.put("fx_width_trend", "stereo", "Widening: correlation change last bar vs first (must fall by 0.2; the sheet says \"width rises\")", t, width_tr, None, Some(-0.2), "", "");
            }
            FxKind::Downlifter => {
                c.put("fx_cent_trend", "movement", "Centroid rise, first to last 10 % of the hold (must fall 2 oct)", t, rise, None, Some(-2.0), "oct", "");
                c.put("fx_monotone", "movement", "Share of 250 ms centroid steps falling", t, frac_dn, Some(0.9), None, "", "");
                c.put("fx_level_trend", "movement", "Level change, last bar minus first bar (falls 6..14 dB)", t, level_tr, Some(-14.0), Some(-6.0), "dB", "");
                c.put("fx_width_trend", "stereo", "Narrowing: correlation change last bar vs first (must rise by 0.15)", t, width_tr, Some(0.15), None, "", "");
            }
            FxKind::Sweep => {
                let ph = Hold { fr: h.fr.clone(), ..first };
                let all = analyze(&Hold { a: smp(SKIP_S), b: h.off, ..ph }, 200.0, c.fft, label);
                depth_period_checks(c, tempo, &[all], Some(1.5));
            }
            FxKind::Impact => {
                let env = q::rms_env_db(&mono, 480, 480);
                let ip = env.iter().enumerate().take(smp(0.3) / 480).fold((0, f64::MIN), |b, (i, v)| if *v > b.1 { (i, *v) } else { b }).0;
                let t20 = q::fall_time(&env, 480.0 / SR, ip, 20.0);
                c.put("fx_fall20", "movement", "Level falls 20 dB within 1.5 s", t, t20.unwrap_or(f64::NAN), None, Some(1.5), "s", "");
            }
        }
        if tempo == 150 {
            if matches!(kind, FxKind::Riser | FxKind::Downlifter) {
                pitch_trend_info(c, &h);
            }
            at150 = ms;
        }
    }
    let pl = c.plan(150);
    c.put("peak127", "level", "Peak at velocity 127, dBFS", Some(150), q::db20(c.hold(150, note, VEL127, pl, Mode::LIVE, None).peak()), None, Some(-3.0), "dBFS", "");
    if kind == FxKind::Impact || c.has("dive") || tag_value(&c.tags, "qa_dive").is_some() {
        dive_checks(c);
    }
    at150
}

// ---- seq ---------------------------------------------------------------------------------------

fn seq_checks(c: &mut Ctx, tempo: u32, h: &Hold) {
    let t = Some(tempo);
    let Some(dom) = c.dom.clone() else {
        c.put("seq_pattern", "movement", "Gate pattern", t, f64::NAN, None, None, "", "no periodic LFO routed");
        return;
    };
    let period = c.period_s(tempo).unwrap_or_else(|| dom.period(tempo as f64));
    let shape = &c.p.lfo_shapes.0[dom.idx];
    let levels: Vec<f64> = match &dom.steps {
        Some(s) => s.iter().map(|v| *v as f64).collect(),
        None => (0..16).map(|i| shape.eval((i as f64 + 0.5) / 16.0) as f64).collect(),
    };
    let n = levels.len();
    let mono: Vec<f64> = h.fr[..h.off].iter().map(|f| 0.5 * (f[0] + f[1]) as f64).collect();
    let slot = (period / n as f64 * SR) as usize;
    let slots = mono.len() / slot.max(1);
    let mut acc = vec![0.0; n];
    let mut cnt = vec![0usize; n];
    for i in 0..slots {
        acc[i % n] += mean_sq(&mono[i * slot..(i + 1) * slot]).sqrt();
        cnt[i % n] += 1;
    }
    let pat: Vec<f64> = acc.iter().zip(&cnt).map(|(a, n)| a / (*n).max(1) as f64).collect();
    let corr = q::pearson(&pat, &levels);
    c.put("seq_corr", "movement", "Slot RMS pattern correlation to the steps", t, corr, Some(0.9), None, "", format!("{n} slots of {:.4} s", slot as f64 / SR));
    let mx = levels.iter().cloned().fold(f64::MIN, f64::max);
    let mn = levels.iter().cloned().fold(f64::MAX, f64::min);
    let on: Vec<f64> = pat.iter().zip(&levels).filter(|(_, l)| **l >= 0.5 * mx).map(|(p, _)| q::db20(*p)).collect();
    let off: Vec<f64> = pat.iter().zip(&levels).filter(|(_, l)| **l <= mn + 0.05).map(|(p, _)| q::db20(*p)).collect();
    if !on.is_empty() && !off.is_empty() && mx - mn > 0.5 {
        c.put("seq_contrast", "movement", "Active slots above the slots at rest, dB", t, mean(&on) - mean(&off), Some(10.0), None, "dB", "");
    }
    let env = q::rms_env_db(&mono, 480, 240);
    let rate = SR / 240.0;
    let r = q::acf(&env, ((1.25 * period) * rate) as usize + 2);
    match q::peak_near(&r, period * rate, 0.05) {
        Some(p) => {
            c.put("period_err", "movement", &format!("Bar period of the gate pattern vs {:.4} s (error)", period), t, 100.0 * (p.lag / rate / period - 1.0), Some(-5.0), Some(5.0), "%", "");
            c.put("period_peak", "movement", "Bar period ACF peak height", t, p.val, Some(0.5), None, "", "");
        }
        None => c.put("period_err", "movement", "Bar period of the gate pattern", t, f64::NAN, Some(-5.0), Some(5.0), "%", "no autocorrelation peak near the expected lag"),
    }
}

fn seq_flow(c: &mut Ctx) -> Vec<M> {
    let sp = spec(c);
    let wfc = 200.0;
    let notes = c.test_notes();
    let mut at150 = vec![];
    for &tempo in &TEMPOS {
        let pl = c.plan(tempo);
        let mut ms = vec![];
        let mut peak100 = 0f64;
        let mut finite = true;
        let mut first: Option<Hold> = None;
        for &(n, label) in &notes {
            let h = c.hold(tempo, n, VEL100, pl, Mode::LIVE, None);
            peak100 = peak100.max(h.peak());
            finite &= finite_of(&h.fr);
            ms.push(analyze(&h, wfc, c.fft, label));
            if first.is_none() {
                first = Some(h);
            }
        }
        let t = Some(tempo);
        c.put("nan", "level", "No NaN or inf in the output", t, if finite { 0.0 } else { 1.0 }, None, Some(0.0), "", "");
        c.put("peak100", "level", "Peak at velocity 100, dBFS", t, q::db20(peak100), None, Some(-3.0), "dBFS", "");
        level_checks(c, tempo, &ms, &sp);
        stereo_spectral_checks(c, tempo, &ms, &sp);
        seq_checks(c, tempo, first.as_ref().unwrap());
        if tempo == 150 {
            at150 = ms;
        }
    }
    let pl = c.plan(150);
    let p127 = c.hold(150, F1, VEL127, pl, Mode::LIVE, None).peak();
    c.put("peak127", "level", "Peak at velocity 127, dBFS", Some(150), q::db20(p127), None, Some(-3.0), "dBFS", "");
    at150
}

// ---- init --------------------------------------------------------------------------------------

fn init_extras(c: &mut Ctx) {
    let pl = c.plan(150);
    let h = c.hold(150, C2, VEL100, pl, Mode::LIVE, None);
    c.put("init_ceiling", "level", "No sample above 0.95", Some(150), h.peak(), None, Some(0.95), "", "");
    let (l, r): (Vec<f32>, Vec<f32>) = h.fr.iter().map(|f| (f[0], f[1])).unzip();
    let steady = q::max_step(&l, &r, smp(0.3), smp(0.8));
    let on = q::max_step(&l, &r, 1, smp(0.005));
    let off = q::max_step(&l, &r, h.off, h.off + smp(0.005));
    c.put("init_click_on", "level", "Note-on step vs 2x steady step", Some(150), on / (2.0 * steady).max(1e-12), None, Some(1.0), "", "");
    c.put("init_click_off", "level", "Note-off step vs 2x steady step", Some(150), off / (2.0 * steady).max(1e-12), None, Some(1.0), "", "");
    c.notes.push("Init: the bit-identical comparison with the SPEC 1.12 reference render is not implemented (no reference file in the repo).".into());
}

// ---- metadata / bank-level checks --------------------------------------------------------------

const BRANDS: [&str; 24] = [
    "serum", "sylenth", "skrillex", "virtual riot", "snails", "excision", "subtronics", "noisia", "phase plant", "pigments", "zomboy", "datsik",
    "flux pavilion", "knife party", "rusko", "bassnectar", "kshmr", "omnisphere", "tb-303", "dx7", "minimoog", "ableton", "rekordbox", "massive x",
];

fn meta_checks(c: &mut Ctx, warnings: &[String]) {
    let p = c.p;
    let init = c.fam == Family::Init;
    let g = "metadata";
    c.put("meta_load", g, "Loads without warnings", None, warnings.len() as f64, None, Some(0.0), "", warnings.join("; "));
    c.put("meta_author", g, "Author is \"Riddim Rack\"", None, (p.author != "Riddim Rack") as u8 as f64, None, Some(0.0), "", format!("author {:?}", p.author));
    let m = p.matrix_state();
    let mut bad = vec![];
    for (k, label) in m.macros.iter().enumerate() {
        if init {
            break;
        }
        if label.trim().is_empty() || label.starts_with("Macro ") {
            bad.push(format!("macro {} has no purpose label", k + 1));
        }
        let src = Source::from_index(Source::Macro1 as usize + k).unwrap();
        if !m.slots.iter().any(|s| s.source == src && s.enabled && s.amount != 0.0) {
            bad.push(format!("macro {} ({label}) drives no matrix slot", k + 1));
        }
    }
    c.put("meta_macros", g, "Four labelled macros that each drive a slot", None, bad.len() as f64, None, Some(0.0), "", bad.join("; "));
    if c.fam.is_bass() {
        const OK: [&str; 15] = ["riddim", "tearout", "growl", "wobble", "wub", "reese", "neuro", "sub", "formant", "yoi", "talking", "hybrid", "metal", "square", "triplet"];
        let n = c.tags.iter().filter(|t| ["riddim", "tearout", "growl", "wobble", "wub", "reese", "neuro", "sub", "formant", "yoi", "talking"].contains(&t.as_str())).count();
        let _ = OK;
        c.put("meta_tags", g, "A riddim / dubstep family tag (Bass)", None, (n == 0) as u8 as f64, None, Some(0.0), "", format!("tags {:?}", p.tags));
    }
    let text = format!("{} {}", p.name, p.description).to_lowercase();
    let hit: Vec<&str> = BRANDS.iter().copied().filter(|b| text.contains(b)).collect();
    c.put("meta_names", g, "No artist or product name in name or description", None, hit.len() as f64, None, Some(0.0), "", hit.join(", "));
    c.put("meta_description", g, "Description of at least 80 characters", None, p.description.chars().count() as f64, Some(80.0), None, "chars", "");
    let fx = &p.patch.fx;
    let uses = fx.dist.on || fx.mb.on || fx.eq.on || fx.cho.on || fx.dim.on || fx.dly.on || fx.rev.on || fx.phs.on || fx.flg.on;
    c.raw.insert("uses_fx".into(), json!(uses));
    c.raw.insert("fx_reordered".into(), json!(p.fx_order != gts::dsp::fx::FxOrder::default()));
}

/// Two renders of the same preset match within -80 dB (skipped for random LFOs and random phase).
fn repeat_check(c: &mut Ctx) {
    let p = c.p;
    let random = (0..4).any(|k| matches!(lfo_of(p, k).0.curve, Curve::SampleHold | Curve::SmoothRandom) && lfo_weight(p, k) > 0.0)
        || p.patch.osc.iter().any(|o| o.on && o.rand_phase > 0.0 && !o.phase_reset);
    if random {
        c.skip("repeat", "metadata", "Repeatability (two renders, -80 dB)", None, "random LFO or free random phase: the engine seeds those from a global counter");
        return;
    }
    let (note, _) = c.test_notes()[0];
    let pl = Plan { hold: 0.8, tail: 0.2, a: 0.0, b: 0.8 };
    let (a, b) = (c.hold(150, note, VEL100, pl, Mode::LIVE, None), c.hold(150, note, VEL100, pl, Mode::LIVE, None));
    let (mut d, mut s) = (0.0, 0.0);
    for (x, y) in a.fr.iter().zip(&b.fr) {
        for ch in 0..2 {
            d += ((x[ch] - y[ch]) as f64).powi(2);
            s += (x[ch] as f64).powi(2);
        }
    }
    c.put("repeat", "metadata", "Repeatability: difference of two renders re signal", None, db10(d / s.max(1e-30)), None, Some(-80.0), "dB", "");
}

// ---- distinctiveness descriptor ----------------------------------------------------------------

/// Mean log-mel descriptor over two 2-bar holds at 150 BPM (the family's test notes).
fn descriptor(c: &Ctx) -> Vec<f64> {
    let notes = c.test_notes();
    let picks: Vec<u8> = match notes.len() {
        0 | 1 => notes.iter().map(|n| n.0).collect(),
        _ => vec![notes[0].0, notes[notes.len() - 1].0],
    };
    let bar = 240.0 / 150.0;
    let pl = match c.fam {
        Family::Pluck => Plan { hold: 0.5, tail: 1.0, a: 0.0, b: 1.2 },
        _ => Plan { hold: 2.0 * bar, tail: 0.3, a: SKIP_S, b: 2.0 * bar },
    };
    let ds: Vec<Vec<f64>> = picks
        .iter()
        .map(|&n| {
            let h = c.hold(150, n, VEL100, pl, Mode::LIVE, None);
            let (l, r) = h.lr();
            q::mel_descriptor(c.fft, &q::mid_side(&l, &r).0)
        })
        .collect();
    (0..ds[0].len()).map(|i| mean(&ds.iter().map(|d| d[i]).collect::<Vec<_>>())).collect()
}

// ---- driver ------------------------------------------------------------------------------------

struct Report {
    name: String,
    file: String,
    family: Family,
    how: String,
    category: String,
    tags: Vec<String>,
    checks: Vec<Chk>,
    notes: Vec<String>,
    raw: serde_json::Map<String, Value>,
    desc: Vec<f64>,
    error: Option<String>,
    secs: f64,
}

impl Report {
    fn fails(&self) -> usize {
        self.checks.iter().filter(|c| c.status == Status::Fail).count() + self.error.is_some() as usize
    }
}

struct Loaded {
    path: PathBuf,
    preset: Preset,
    warnings: Vec<String>,
    family: Family,
    how: String,
}

fn analyze_preset(l: &Loaded, fft: &Fft, macros: bool) -> Report {
    let t0 = std::time::Instant::now();
    let p = &l.preset;
    let tags = norm_tags(p);
    let per_ov = tag_value(&tags, "qa_period").map(|v| {
        if v == "none" {
            PeriodOv::None
        } else if let Some(d) = parse_division(&v) {
            PeriodOv::Div(d)
        } else {
            v.parse::<f64>().map_or(PeriodOv::None, PeriodOv::Secs)
        }
    });
    let mut c = Ctx {
        p,
        fam: l.family,
        ov: parse_overrides(&tags),
        tags,
        fft,
        dom: None,
        per_ov,
        checks: vec![],
        notes: vec![],
        raw: serde_json::Map::new(),
    };
    if matches!(c.fam, Family::Riddim | Family::Wobble | Family::Tearout | Family::Formant | Family::Reese | Family::Fx | Family::Seq | Family::Lead) {
        c.dom = find_dominant(p, c.fam, fft);
    }
    if let Some(d) = &c.dom {
        c.raw.insert("dominant_lfo".into(), json!({"lfo": d.name(), "weight": d.weight, "depth_oct": d.depth, "period_150": d.period(150.0), "period_140": d.period(140.0)}));
    }
    meta_checks(&mut c, &l.warnings);
    let sp = spec(&c);
    let live150 = match c.fam {
        Family::Pluck => pluck_flow(&mut c),
        Family::Fx => fx_flow(&mut c),
        Family::Seq => seq_flow(&mut c),
        _ => held_flow(&mut c),
    };
    if matches!(c.fam, Family::Lead) {
        lead_like_extras(&mut c);
    }
    if c.fam.is_bass() && ["dive", "impact"].iter().any(|t| c.has(t)) {
        lead_like_extras(&mut c);
    }
    alias_checks(&mut c, &sp, &live150);
    match c.fam {
        Family::Reese => reese_checks(&mut c),
        Family::Tearout => sub_band_check(&mut c),
        Family::Init => init_extras(&mut c),
        _ => {}
    }
    if macros {
        macro_checks(&mut c);
    }
    repeat_check(&mut c);
    let desc = descriptor(&c);
    Report {
        name: p.name.clone(),
        file: l.path.display().to_string(),
        family: l.family,
        how: l.how.clone(),
        category: p.category.clone(),
        tags: p.tags.clone(),
        checks: c.checks,
        notes: c.notes,
        raw: c.raw,
        desc,
        error: None,
        secs: t0.elapsed().as_secs_f64(),
    }
}

fn collect(paths: &[PathBuf]) -> Vec<PathBuf> {
    fn walk(p: &Path, out: &mut Vec<PathBuf>) {
        if p.is_dir() {
            let mut kids: Vec<PathBuf> = std::fs::read_dir(p).map(|d| d.flatten().map(|e| e.path()).collect()).unwrap_or_default();
            kids.sort();
            for k in kids {
                walk(&k, out);
            }
        } else if p.extension().is_some_and(|e| e == "json") {
            out.push(p.to_path_buf());
        }
    }
    let mut out = vec![];
    for p in paths {
        walk(p, &mut out);
    }
    out
}

struct Opts {
    paths: Vec<PathBuf>,
    only: Option<String>,
    out: PathBuf,
    name: String,
    macros: bool,
    pair_min: f64,
}

fn parse_args() -> Result<Opts, String> {
    let mut o = Opts {
        paths: vec![],
        only: None,
        out: Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/audits/bank-qa"),
        name: "bank-qa".into(),
        macros: true,
        pair_min: PAIR_MIN_DEFAULT,
    };
    let mut a = std::env::args().skip(1);
    while let Some(x) = a.next() {
        let mut val = |what: &str| a.next().ok_or(format!("{what} needs a value"));
        match x.as_str() {
            "--only" => o.only = Some(val("--only")?.to_lowercase()),
            "--out" => o.out = PathBuf::from(val("--out")?),
            "--name" => o.name = val("--name")?,
            "--pair-min" => o.pair_min = val("--pair-min")?.parse().map_err(|_| "--pair-min needs a number".to_string())?,
            "--no-macros" => o.macros = false,
            "--bright-amp" => BRIGHT_AMP.store(true, Ordering::Relaxed),
            "--bright-power" => BRIGHT_AMP.store(false, Ordering::Relaxed),
            "-h" | "--help" => return Err("usage: bank_qa [--only <name>] [--out <dir>] [--name <stem>] [--no-macros] [--bright-amp] [--pair-min <dB>] [preset.json|dir]...".into()),
            s if s.starts_with("--") => return Err(format!("unknown flag {s}")),
            s => o.paths.push(PathBuf::from(s)),
        }
    }
    if o.paths.is_empty() {
        o.paths.push(rp::candidates_dir());
    }
    Ok(o)
}

fn main() {
    let o = match parse_args() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let report_json = std::fs::canonicalize(o.out.join(format!("{}.json", o.name))).ok();
    let files: Vec<PathBuf> = collect(&o.paths).into_iter().filter(|f| std::fs::canonicalize(f).ok() != report_json || report_json.is_none()).collect();
    if files.is_empty() {
        eprintln!("no preset files found");
        std::process::exit(2);
    }
    let mut loaded = vec![];
    let mut report_errs = vec![];
    for f in &files {
        match Preset::load(f) {
            Ok((preset, warnings)) => {
                let (family, how) = family_of(&preset);
                loaded.push(Loaded { path: f.clone(), preset, warnings, family, how });
            }
            Err(e) => report_errs.push((f.clone(), e)),
        }
    }
    let hit = |l: &Loaded| {
        o.only.as_ref().map_or(true, |n| {
            l.preset.name.to_lowercase().contains(n) || l.path.file_stem().is_some_and(|s| s.to_string_lossy().to_lowercase().contains(n))
        })
    };
    let targets: Vec<usize> = (0..loaded.len()).filter(|&i| hit(&loaded[i])).collect();
    if targets.is_empty() {
        eprintln!("--only matched no preset");
        std::process::exit(2);
    }
    // Full analysis for targets; descriptor only for the other presets (distinctiveness peers).
    let results: Mutex<Vec<Option<Report>>> = Mutex::new((0..loaded.len()).map(|_| None).collect());
    let next = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(targets.len().max(1)).min(16);
    let order: Vec<usize> = (0..loaded.len()).collect();
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                let fft = Fft::new(65536);
                loop {
                    let k = next.fetch_add(1, Ordering::SeqCst);
                    let Some(&i) = order.get(k) else { break };
                    let l = &loaded[i];
                    let full = targets.contains(&i);
                    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        if full {
                            analyze_preset(l, &fft, o.macros)
                        } else {
                            let tags = norm_tags(&l.preset);
                            let c = Ctx { p: &l.preset, fam: l.family, ov: HashMap::new(), tags, fft: &fft, dom: None, per_ov: None, checks: vec![], notes: vec![], raw: Default::default() };
                            Report { name: l.preset.name.clone(), file: l.path.display().to_string(), family: l.family, how: l.how.clone(), category: l.preset.category.clone(), tags: l.preset.tags.clone(), checks: vec![], notes: vec![], raw: Default::default(), desc: descriptor(&c), error: None, secs: 0.0 }
                        }
                    }));
                    let rep = r.unwrap_or_else(|e| {
                        let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                        Report { name: l.preset.name.clone(), file: l.path.display().to_string(), family: l.family, how: l.how.clone(), category: l.preset.category.clone(), tags: l.preset.tags.clone(), checks: vec![], notes: vec![], raw: Default::default(), desc: vec![], error: Some(format!("panic: {msg}")), secs: 0.0 }
                    });
                    if full {
                        eprintln!("{} ({}): {} checks, {} fail, {:.1} s", rep.name, rep.family.name(), rep.checks.len(), rep.fails(), rep.secs);
                    }
                    results.lock().unwrap()[i] = Some(rep);
                }
            });
        }
    });
    let mut all: Vec<Report> = results.into_inner().unwrap().into_iter().map(|r| r.unwrap()).collect();
    let analysed: Vec<usize> = targets.clone();
    bank_checks(&mut all, &analysed, o.pair_min);
    let pairs = pair_table(&all);
    let (md, js, fails) = output(&o, &all, &analysed, &pairs, &report_errs);
    std::fs::create_dir_all(&o.out).expect("create output directory");
    let (mp, jp) = (o.out.join(format!("{}.md", o.name)), o.out.join(format!("{}.json", o.name)));
    std::fs::write(&mp, md).expect("write markdown");
    std::fs::write(&jp, serde_json::to_string_pretty(&js).unwrap() + "\n").expect("write json");
    println!("wrote {} and {}", mp.display(), jp.display());
    println!("{} preset(s), {} failed check(s)", analysed.len(), fails);
    if fails > 0 {
        std::process::exit(1);
    }
}

// ---- bank-level checks, pairs ------------------------------------------------------------------

/// RMS against the median of the same category in this run; distinctiveness inside a family.
fn bank_checks(all: &mut [Report], analysed: &[usize], pair_min: f64) {
    let rms150 = |r: &Report| r.raw.get("rms_db@150").and_then(|v| v.as_f64());
    let mut by_cat: HashMap<String, Vec<f64>> = HashMap::new();
    for r in all.iter() {
        if let Some(v) = rms150(r) {
            by_cat.entry(r.category.split('/').next().unwrap_or("").to_lowercase()).or_default().push(v);
        }
    }
    let descs: Vec<(usize, Family, Vec<f64>)> = all.iter().enumerate().map(|(i, r)| (i, r.family, r.desc.clone())).collect();
    for &i in analysed {
        let cat = all[i].category.split('/').next().unwrap_or("").to_lowercase();
        if let (Some(v), Some(peers)) = (rms150(&all[i]), by_cat.get(&cat)) {
            let mut s = peers.clone();
            s.sort_by(|a, b| a.total_cmp(b));
            let med = if s.len() % 2 == 1 { s[s.len() / 2] } else { 0.5 * (s[s.len() / 2 - 1] + s[s.len() / 2]) };
            let (lo, hi) = if s.len() >= 3 { (Some(-2.0), Some(2.0)) } else { (None, None) };
            let (status, margin) = judge(v - med, lo, hi);
            all[i].checks.push(Chk {
                id: "rms_bank_median".into(),
                group: "level".into(),
                label: "RMS against the median of its category in this run".into(),
                tempo: Some(150),
                value: Some(v - med),
                lo,
                hi,
                unit: "dB".into(),
                status,
                margin,
                detail: format!("{} preset(s) in the category, median {med:.2} dBFS{}", s.len(), if s.len() < 3 { " (Info: needs 3 or more)" } else { "" }),
            });
        }
        // Distinctiveness.
        let me = &descs[i];
        if me.2.is_empty() {
            continue;
        }
        let peers: Vec<(usize, f64)> = descs.iter().filter(|d| d.0 != i && d.1 == me.1 && !d.2.is_empty()).map(|d| (d.0, q::mel_distance(&me.2, &d.2))).collect();
        match peers.iter().cloned().min_by(|a, b| a.1.total_cmp(&b.1)) {
            Some((j, d)) => {
                let (status, margin) = judge(d, Some(pair_min), None);
                let c = Chk {
                    id: "pair_dist".into(),
                    group: "distinctiveness".into(),
                    label: format!("Log-mel distance to the nearest {} preset", me.1.name()),
                    tempo: None,
                    value: Some(d),
                    lo: Some(pair_min),
                    hi: None,
                    unit: "dB".into(),
                    status,
                    margin,
                    detail: format!("nearest: {}", all[j].name),
                };
                all[i].checks.push(c);
            }
            None => all[i].checks.push(Chk {
                id: "pair_dist".into(),
                group: "distinctiveness".into(),
                label: "Log-mel distance to the nearest preset of the family".into(),
                tempo: None,
                value: None,
                lo: None,
                hi: None,
                unit: String::new(),
                status: Status::Skip,
                margin: None,
                detail: "no other preset of this family in the run".into(),
            }),
        }
    }
}

/// (a, b, same family, distance) for every pair with a descriptor.
fn pair_table(all: &[Report]) -> Vec<(String, String, bool, f64)> {
    let mut v = vec![];
    for i in 0..all.len() {
        for j in i + 1..all.len() {
            if !all[i].desc.is_empty() && !all[j].desc.is_empty() {
                v.push((all[i].name.clone(), all[j].name.clone(), all[i].family == all[j].family, q::mel_distance(&all[i].desc, &all[j].desc)));
            }
        }
    }
    v.sort_by(|a, b| a.3.total_cmp(&b.3));
    v
}

// ---- output ------------------------------------------------------------------------------------

fn chk_json(c: &Chk) -> Value {
    json!({
        "id": c.id, "group": c.group, "label": c.label, "tempo": c.tempo, "value": c.value,
        "target": {"min": c.lo, "max": c.hi}, "unit": c.unit, "status": c.status.name(), "margin": c.margin, "detail": c.detail,
    })
}

fn target_text(c: &Chk) -> String {
    match (c.lo, c.hi) {
        (Some(a), Some(b)) => format!("{}..{}", fmt(a), fmt(b)),
        (Some(a), None) => format!(">= {}", fmt(a)),
        (None, Some(b)) => format!("<= {}", fmt(b)),
        _ => "-".into(),
    }
}

fn output(o: &Opts, all: &[Report], analysed: &[usize], pairs: &[(String, String, bool, f64)], errs: &[(PathBuf, String)]) -> (String, Value, usize) {
    let mut fails = errs.len();
    let mut md = String::new();
    md += "# Bank QA report\n\n";
    md += &format!(
        "Tool `bank_qa` (GTS), contract `docs/design/bank-recipes.md` section A. Renders at 150 and 140 BPM, 48 kHz, velocity 100 (127 for the peak check). \
         Tags that steer the tool are listed in the source header of `crates/gts/examples/bank_qa.rs`.\n\nInputs: {}\n\n",
        o.paths.iter().map(|p| format!("`{}`", p.display())).collect::<Vec<_>>().join(", ")
    );
    for (p, e) in errs {
        md += &format!("- LOAD ERROR `{}`: {e}\n", p.display());
    }
    md += "## Summary\n\n| Preset | Family | Checks | Fail | Info/Skip | Result |\n|---|---|---|---|---|---|\n";
    let mut preset_json = vec![];
    for &i in analysed {
        let r = &all[i];
        let f = r.fails();
        fails += f;
        let n = r.checks.iter().filter(|c| matches!(c.status, Status::Pass | Status::Fail)).count();
        let ni = r.checks.len() - n;
        md += &format!("| {} | {} ({}) | {} | {} | {} | {} |\n", r.name, r.family.name(), r.how, n, f, ni, if f == 0 { "PASS" } else { "FAIL" });
        preset_json.push(json!({
            "name": r.name, "file": r.file, "family": r.family.name(), "family_source": r.how, "category": r.category, "tags": r.tags,
            "pass": f == 0, "failed_checks": f, "checks": r.checks.iter().map(chk_json).collect::<Vec<_>>(),
            "notes": r.notes, "measurements": r.raw, "error": r.error, "seconds": r.secs,
        }));
    }
    for &i in analysed {
        let r = &all[i];
        md += &format!("\n## {} ({}, {})\n\n", r.name, r.family.name(), r.category);
        if let Some(e) = &r.error {
            md += &format!("ERROR: {e}\n");
            continue;
        }
        md += "| Check | Target | 150 BPM | 140 BPM | Margin | Result |\n|---|---|---|---|---|---|\n";
        let mut ids: Vec<&str> = vec![];
        for c in &r.checks {
            if !ids.contains(&c.id.as_str()) {
                ids.push(&c.id);
            }
        }
        let mut notes = vec![];
        for id in ids {
            let cs: Vec<&Chk> = r.checks.iter().filter(|c| c.id == id).collect();
            let first = cs[0];
            let cell = |t: u32| -> String {
                cs.iter().find(|c| c.tempo == Some(t)).map_or("".into(), |c| match c.status {
                    Status::Skip => "skip".into(),
                    _ => format!("{}{}", fmt(c.value.unwrap_or(f64::NAN)), if c.status == Status::Fail { " X" } else { "" }),
                })
            };
            let (a, b) = if first.tempo.is_some() {
                (cell(150), cell(140))
            } else {
                (cs.iter().map(|c| format!("{} {}", fmt(c.value.unwrap_or(f64::NAN)), c.unit)).collect::<Vec<_>>().join(", "), String::new())
            };
            let margin = cs.iter().filter_map(|c| c.margin).fold(f64::NAN, |m, v| if m.is_nan() { v } else { m.min(v) });
            let res = if cs.iter().any(|c| c.status == Status::Fail) {
                "FAIL"
            } else if cs.iter().all(|c| c.status == Status::Skip) {
                "skip"
            } else if cs.iter().all(|c| matches!(c.status, Status::Info | Status::Skip)) {
                "info"
            } else {
                "pass"
            };
            let unit = if first.unit.is_empty() || first.tempo.is_none() { String::new() } else { format!(" {}", first.unit) };
            md += &format!("| {} | {}{} | {} | {} | {} | {} |\n", first.label.replace('|', "/"), target_text(first), unit, a, b, if margin.is_nan() { "-".into() } else { format!("{margin:+.3}") }, res);
            for c in cs {
                if !c.detail.is_empty() && (c.status == Status::Fail || c.status == Status::Skip) {
                    notes.push(format!("- {}{}: {}", c.id, c.tempo.map_or(String::new(), |t| format!("@{t}")), c.detail));
                }
            }
        }
        if !notes.is_empty() {
            md += "\nDetails of failed or skipped checks:\n\n";
            md += &notes.join("\n");
            md += "\n";
        }
        for n in &r.notes {
            md += &format!("\nNote: {n}\n");
        }
    }
    md += &format!("\n## Distinctiveness (log-mel distance, dB; threshold {} dB)\n\nSame-family pairs are judged; other pairs are listed for calibration.\n\n| A | B | Same family | Distance |\n|---|---|---|---|\n", o.pair_min);
    for (a, b, same, d) in pairs.iter().take(60) {
        md += &format!("| {a} | {b} | {} | {d:.2} |\n", if *same { "yes" } else { "no" });
    }
    let n_fx = analysed.iter().filter(|&&i| all[i].raw.get("uses_fx").and_then(|v| v.as_bool()) == Some(true)).count();
    let n_re = analysed.iter().filter(|&&i| all[i].raw.get("fx_reordered").and_then(|v| v.as_bool()) == Some(true)).count();
    md += &format!(
        "\n## Bank summary\n\n{} preset(s) analysed; {} use the FX chain; {} use a reordered chain (the sheet asks 20 of 41 and 6 of 41 for the whole bank). \
         Failed checks: {fails}.\n",
        analysed.len(), n_fx, n_re
    );
    let js = json!({
        "tool": "bank_qa", "unix_time": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()),
        "tempos": TEMPOS, "pair_min_db": o.pair_min,
        "inputs": o.paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        "summary": {"presets": analysed.len(), "failed_checks": fails, "pass": fails == 0, "uses_fx": n_fx, "fx_reordered": n_re},
        "load_errors": errs.iter().map(|(p, e)| json!({"file": p.display().to_string(), "error": e})).collect::<Vec<_>>(),
        "presets": preset_json,
        "pairs": pairs.iter().map(|(a, b, s, d)| json!({"a": a, "b": b, "same_family": s, "distance_db": d})).collect::<Vec<_>>(),
    });
    (md, js, fails)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(cat: &str, tags: &[&str]) -> Preset {
        Preset { category: cat.into(), tags: tags.iter().map(|s| s.to_string()).collect(), ..Preset::default() }
    }

    #[test]
    fn families_from_category_and_tags() {
        let f = |c: &str, t: &[&str]| family_of(&preset(c, t)).0;
        assert_eq!(f("Bass", &["riddim", "metal", "mono"]), Family::Riddim);
        assert_eq!(f("Bass", &["wobble", "wub", "riddim", "mono"]), Family::Wobble);
        assert_eq!(f("Bass", &["tearout", "growl", "formant", "dirty"]), Family::Tearout);
        assert_eq!(f("Bass", &["formant", "talking", "pluck", "mono"]), Family::Formant);
        assert_eq!(f("Bass", &["reese", "mono", "wub"]), Family::Reese);
        assert_eq!(f("Bass", &["sub", "dirty", "hybrid"]), Family::Sub);
        assert_eq!(f("Bass/growl", &["metallic"]), Family::Tearout);
        assert_eq!(f("Lead", &["screech", "formant"]), Family::Lead);
        assert_eq!(f("Pluck", &["pluck", "sub", "impact"]), Family::Pluck);
        assert_eq!(f("FX", &["impact"]), Family::Fx);
        assert_eq!(f("Seq", &["riddim"]), Family::Seq);
        assert_eq!(f("Init", &[]), Family::Init);
        assert_eq!(f("Bass", &["qa_family=reese", "riddim"]), Family::Reese);
        assert_eq!(f("Bass", &["QA_FAMILY:Sub"]), Family::Sub, "case-insensitive, colon accepted");
    }

    #[test]
    fn division_periods_match_the_sheet() {
        let p = |d: &str, bpm: f64| div_beats(parse_division(d).unwrap()) * 60.0 / bpm;
        for (d, a, b) in [("1/4", 0.4, 0.4286), ("1/8T", 0.1333, 0.1429), ("1/4T", 0.2667, 0.2857), ("1/8D", 0.3, 0.3214), ("1/16", 0.1, 0.1071), ("1/16T", 0.0667, 0.0714), ("1/1", 1.6, 1.7143), ("2/1", 3.2, 3.4286), ("4/1", 6.4, 6.8571), ("1/2", 0.8, 0.8571)] {
            assert!((p(d, 150.0) - a).abs() < 5e-5 && (p(d, 140.0) - b).abs() < 5e-5, "{d}");
        }
        assert!(parse_division("1/8t").is_some() && parse_division("nope").is_none());
    }

    #[test]
    fn override_tags_and_bounds() {
        let tags: Vec<String> = ["qa_bright=280..900", "qa_alias=..-55", "qa_depth=0.8..", "qa_period=1/8t", "qa_macro1=c:0.5", "riddim"].iter().map(|s| s.to_string()).collect();
        let m = parse_overrides(&tags);
        assert_eq!(m["bright"], (Some(280.0), Some(900.0)));
        assert_eq!(m["alias"], (None, Some(-55.0)));
        assert_eq!(m["depth"], (Some(0.8), None));
        assert!(!m.contains_key("period") && !m.contains_key("macro1"));
        assert_eq!(tag_value(&tags, "qa_period").as_deref(), Some("1/8t"));
    }

    #[test]
    fn judge_margin_and_worst() {
        assert_eq!(judge(5.0, Some(1.0), Some(8.0)), (Status::Pass, Some(3.0)));
        let (s, m) = judge(9.0, Some(1.0), Some(8.0));
        assert!(s == Status::Fail && (m.unwrap() + 1.0).abs() < 1e-12);
        assert_eq!(judge(2.0, None, None).0, Status::Info);
        assert_eq!(judge(f64::NAN, Some(0.0), None).0, Status::Fail);
        let v = vec![("F1".to_string(), 0.99), ("G1".to_string(), 0.90), ("A#1".to_string(), 0.97)];
        assert_eq!(worst(&v, Some(0.95), None).0, 0.90, "least margin wins");
        assert_eq!(worst(&v, None, Some(0.95)).0, 0.99);
    }

    /// A centroid series that swings 1 octave every 0.2 s gives that period and about a 1 octave depth.
    #[test]
    fn period_of_a_synthetic_centroid() {
        let rate = SR / MOVE_HOP as f64;
        let c: Vec<f64> = (0..(3.4 * rate) as usize).map(|i| 800.0 * 2f64.powf(0.5 * (std::f64::consts::TAU * i as f64 / (0.2 * rate)).sin())).collect();
        let r = period_of(&c, 0.2).unwrap();
        assert!(r.err_pct.abs() < 2.0 && r.peak > 0.9, "{} {}", r.err_pct, r.peak);
        assert!((q::depth_oct(&c) - 0.9).abs() < 0.15);
        // Expecting 0.3 s from a 0.2 s series misses by more than 5 %.
        let r = period_of(&c, 0.3).unwrap();
        assert!(r.err_pct.abs() > 5.0 || r.peak < 0.5, "{} {}", r.err_pct, r.peak);
        assert!(max_peak(&c) > 0.9);
    }
}
