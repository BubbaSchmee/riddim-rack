//! Drag-to-patch, the sources strip, ring depth editing, the knob menu and the MATRIX page
//! (SPEC v2 Track 5, `docs/standards/mod-matrix.md`). A child of [`super`] (the editor), so it
//! shares its private helpers (`kn`, `drive`, `set_hover`, ...).
//!
//! Per frame [`begin_frame`] snapshots the matrix and the live routes into an [`MView`] kept in
//! egui memory; every compact knob asks it what to draw ([`MView::knob`]: stacked range arcs, the
//! live dot, the patch-drag target look) and [`end_frame`] finishes what the frame started (the
//! drop, the drag ghost, the knob menu, the MORE popover). All state lives in egui memory, so the
//! plugin, the previews and the headless tests run the same code.

use super::*;
use crate::matrix::{self, fixed_routes, gate_open, live_route, Edit, params_info, EditError, FixedRoute, Matrix, MatrixEditor, Source, MAX_SLOTS};
use rr_ui::theme::{SRC_AMP, SRC_BUS, SRC_CTRL, SRC_M, SRC_NOTE};

// ---- sources: look -----------------------------------------------------------------------------

/// How a source is drawn: glyph on the disc, colour (by family, `rr_ui::theme`), chip text.
#[derive(Clone, Copy)]
pub(super) struct SrcStyle {
    pub glyph: &'static str,
    pub color: Color32,
    pub name: &'static str,
}

pub(super) fn src_style(s: Source) -> SrcStyle {
    use Source::*;
    let (glyph, color, name) = match s {
        ModA => ("A", SRC_A, "Mod A"),
        ModB => ("B", SRC_B, "Mod B"),
        FilterEnv => ("E", SRC_E, "Filt Env"),
        AmpEnvA => ("a", SRC_AMP, "Amp A"),
        AmpEnvB => ("b", SRC_AMP, "Amp B"),
        Velocity => ("V", SRC_NOTE, "Velocity"),
        Keytrack => ("K", SRC_NOTE, "Keytrack"),
        Random => ("R", SRC_NOTE, "Random"),
        PolyAftertouch => ("P", SRC_NOTE, "Poly AT"),
        ModWheel => ("W", SRC_CTRL, "Wheel"),
        ChannelAftertouch => ("C", SRC_CTRL, "Chan AT"),
        Macro1 => ("1", SRC_M, "Macro 1"),
        Macro2 => ("2", SRC_M, "Macro 2"),
        Macro3 => ("3", SRC_M, "Macro 3"),
        Macro4 => ("4", SRC_M, "Macro 4"),
        BusCv1 => ("1", SRC_BUS, "Bus 1"),
        BusCv2 => ("2", SRC_BUS, "Bus 2"),
        BusCv3 => ("3", SRC_BUS, "Bus 3"),
        // Track 7 engine wave: placeholder looks, the UI wave styles and places them.
        ModC => ("C", SRC_A, "Mod C"),
        ModD => ("D", SRC_B, "Mod D"),
        ModEnv => ("M", SRC_E, "Mod Env"),
    };
    SrcStyle { glyph, color, name }
}

/// The five chips of the first row, in order.
const PRIMARY: [Source; 5] = [Source::ModA, Source::ModB, Source::FilterEnv, Source::AmpEnvA, Source::AmpEnvB];
/// Behind the MORE chip.
const MORE: [Source; 9] =
    [Source::Velocity, Source::Keytrack, Source::Random, Source::PolyAftertouch, Source::ModWheel, Source::ChannelAftertouch, Source::BusCv1, Source::BusCv2, Source::BusCv3];
const MACROS: [Source; 4] = [Source::Macro1, Source::Macro2, Source::Macro3, Source::Macro4];

fn macro_index(s: Source) -> Option<usize> {
    MACROS.iter().position(|m| *m == s)
}

/// Chip text of a source: the macros show their (editable) label.
fn label_of(m: &Matrix, s: Source) -> String {
    macro_index(s).map_or_else(|| src_style(s).name.to_string(), |k| m.macros[k].clone())
}

/// The label first, Live's host name after it ("Growl \u{b7} Macro 1"); a macro still called "Macro 1" says it once.
fn label_full(m: &Matrix, s: Source) -> String {
    match macro_index(s) {
        Some(k) if !m.macros[k].eq_ignore_ascii_case(&format!("Macro {}", k + 1)) => format!("{} \u{b7} Macro {}", m.macros[k], k + 1),
        _ => label_of(m, s),
    }
}

/// Matrix source cell: a renamed macro carries its slot, "Growl (M1)".
fn cell_label(m: &Matrix, s: Source) -> String {
    match macro_index(s) {
        Some(k) if !m.macros[k].eq_ignore_ascii_case(&format!("Macro {}", k + 1)) => format!("{} (M{})", m.macros[k], k + 1),
        _ => label_of(m, s),
    }
}

fn pidx(id: &str) -> Option<usize> {
    static M: OnceLock<HashMap<&'static str, usize>> = OnceLock::new();
    M.get_or_init(|| params_info().iter().enumerate().map(|(i, p)| (p.id.as_str(), i)).collect()).get(id).copied()
}

// ---- the per-frame view ------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Slot(usize),
    Fixed(&'static FixedRoute),
}

/// One source moving one target: a matrix slot, or a built-in route whose amount is non-zero and
/// whose gate is open.
#[derive(Clone, Copy)]
pub(super) struct Contrib {
    pub src: Source,
    pub kind: Kind,
    /// Slot amount (-1..1) or the route param's plain value.
    pub amount: f32,
    pub on: bool,
    pub bipolar: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum ArcEdit {
    Slot(usize),
    /// The built-in route's param id.
    Route(&'static str),
}

/// What a knob draws for the matrix (see [`MView::knob`]).
#[derive(Clone, Default)]
pub(super) struct KnobMod {
    pub ranges: [Option<RingRange>; RING_MAX],
    pub edits: [Option<ArcEdit>; RING_MAX],
    pub arc_src: [Option<Source>; RING_MAX],
    pub live: Option<(f32, Color32)>,
    pub live_glyph: &'static str,
    /// Bit `1 << Source as u32` for every source that moves this knob.
    pub srcs: u32,
    /// A valid drop target.
    pub target: bool,
    pub modulated: bool,
}

/// The matrix and the live routes as one frame sees them.
pub(super) struct MView {
    pub matrix: Matrix,
    by_target: HashMap<String, Vec<Contrib>>,
    live: Vec<(usize, f32)>,
    sounding: bool,
    /// Source chip under the pointer (last frame), when no patch drag runs.
    pub hover_src: Option<Source>,
    /// The source being dragged.
    pub drag: Option<Source>,
    pub time: f32,
    pub record: bool,
}

pub(super) fn mview_id() -> Id {
    Id::new("gts-mview")
}

fn drag_id() -> Id {
    Id::new("gts-patch-drag")
}

fn hot_id() -> Id {
    Id::new("gts-patch-hot")
}

fn notice_id() -> Id {
    Id::new("gts-notice")
}

/// The source being dragged for patching, if any.
pub(super) fn drag_src(ctx: &egui::Context) -> Option<Source> {
    ctx.data(|d| d.get_temp::<Option<Source>>(drag_id())).flatten()
}

fn start_drag(ctx: &egui::Context, s: Source) {
    ctx.data_mut(|d| d.insert_temp(drag_id(), Some(s)));
}

/// Previews and benchmarks: a patch drag of `s` that no pointer release ends (see `editor_preview`).
pub fn debug_hold_drag(ctx: &egui::Context, s: Source) {
    ctx.data_mut(|d| {
        d.insert_temp(drag_id(), Some(s));
        d.insert_temp(Id::new("gts-drag-hold"), true);
    });
}

/// A line for the help bar, over whatever is hovered, for 3 seconds.
pub(super) fn notice(ctx: &egui::Context, msg: &str) {
    let until = ctx.input(|i| i.time) + 3.0;
    ctx.data_mut(|d| d.insert_temp(notice_id(), (msg.to_string(), until)));
}

pub fn current_notice(ctx: &egui::Context) -> Option<String> {
    let now = ctx.input(|i| i.time);
    ctx.data(|d| d.get_temp::<(String, f64)>(notice_id())).filter(|(_, until)| *until > now).map(|(m, _)| m)
}

impl MView {
    fn new(c: &dyn Ctl, f: &VizFrame, hover_src: Option<Source>, drag: Option<Source>, time: f32, record: bool) -> Self {
        let matrix = c.matrix().map(|m| m.get()).unwrap_or_default();
        let mut by_target: HashMap<String, Vec<Contrib>> = HashMap::new();
        for r in fixed_routes() {
            if !gate_open(r, c) {
                continue;
            }
            let amount = pidx(r.route).map_or(0.0, |i| params_info()[i].plain(c.get01(r.route)));
            if amount != 0.0 {
                by_target.entry(r.target.to_string()).or_default().push(Contrib { src: r.source, kind: Kind::Fixed(r), amount, on: true, bipolar: r.source.bipolar() });
            }
        }
        for (i, s) in matrix.slots.iter().enumerate() {
            by_target.entry(s.target.clone()).or_default().push(Contrib { src: s.source, kind: Kind::Slot(i), amount: s.amount, on: s.enabled, bipolar: s.bipolar });
        }
        let live = (0..f.live_len()).map(|k| f.live_entry(k)).collect();
        Self { matrix, by_target, live, sounding: f.newest().is_some(), hover_src, drag, time, record }
    }

    pub fn contribs(&self, target: &str) -> &[Contrib] {
        self.by_target.get(target).map_or(&[], Vec::as_slice)
    }

    /// Routes `src` moves right now (slots, and built-in routes with a non-zero amount).
    pub fn routes_of(&self, src: Source) -> usize {
        self.by_target.values().flatten().filter(|c| c.src == src).count()
    }

    /// Everything knob `id` draws for the matrix: one stacked range arc per source (the built-in
    /// routes keep the unit-aware arcs of [`ring_ranges`]), the live dot and which sources move it.
    pub fn knob(&self, c: &dyn Ctl, id: &str) -> KnobMod {
        let mut m = KnobMod::default();
        let Some(pi) = pidx(id) else { return m };
        m.target = params_info()[pi].target;
        if !m.target {
            return m;
        }
        let base = c.get01(id);
        let mut legacy = if MOD_IDS.contains(&id) { ring_ranges(c, id) } else { [None, None] };
        let mut n = 0;
        let mut push = |m: &mut KnobMod, rg: RingRange, ed: Option<ArcEdit>, src: Option<Source>| {
            if n < RING_MAX {
                (m.ranges[n], m.edits[n], m.arc_src[n]) = (Some(rg), ed, src);
                n += 1;
            }
        };
        for ct in self.contribs(id) {
            m.srcs |= 1 << ct.src as u32;
            m.modulated = true;
            let color = src_style(ct.src).color;
            match ct.kind {
                Kind::Fixed(r) => {
                    let hit = legacy.iter().position(|l| l.is_some_and(|l| l.color == color)).and_then(|p| legacy[p].take());
                    let rg = hit.or_else(|| fixed_arc(c, id, r, ct.amount, base).map(|(lo, hi)| RingRange { lo, hi, color, dim: false }));
                    if let Some(rg) = rg {
                        push(&mut m, rg, Some(ArcEdit::Route(r.route)), Some(ct.src));
                    }
                }
                Kind::Slot(i) => {
                    let (lo, hi) = slot_span(base, ct.amount, ct.bipolar);
                    push(&mut m, RingRange { lo, hi, color, dim: !ct.on }, Some(ArcEdit::Slot(i)), Some(ct.src));
                }
            }
        }
        for l in legacy.into_iter().flatten() {
            if l.color == SRC_E {
                m.srcs |= 1 << Source::FilterEnv as u32;
                m.modulated = true;
            }
            push(&mut m, l, None, None);
        }
        // Live dot: the post-modulation value of the newest voice, in the colour of the source
        // that moves the knob most.
        if self.sounding {
            let gated = (id.starts_with("oscb") && c.get01("oscb_on") <= 0.5) || (id.starts_with("filt") && bypass(c));
            let v = self.live.iter().find(|e| e.0 == pi).map(|e| e.1);
            if let (Some(v), false) = (v, gated) {
                if (v - base).abs() > 0.004 {
                    let span = |k: usize| m.ranges[k].map_or(0.0, |r| r.hi - r.lo);
                    let top = (0..RING_MAX).filter(|&k| m.ranges[k].is_some_and(|r| !r.dim)).max_by(|&a, &b| span(a).total_cmp(&span(b)));
                    let src = top.and_then(|k| m.arc_src[k]).or_else(|| self.contribs(id).iter().find(|c| c.on).map(|c| c.src));
                    let (col, glyph) = src.map_or((SRC_B, "B"), |s| (src_style(s).color, src_style(s).glyph));
                    m.live = Some((v.clamp(0.0, 1.0), top.and_then(|k| m.ranges[k]).map_or(col, |r| r.color)));
                    m.live_glyph = glyph;
                }
            }
        }
        m
    }

    /// The ring `src` would add at +50 % if dropped on `id` now.
    pub fn preview(&self, c: &dyn Ctl, id: &str, src: Source) -> Option<RingRange> {
        let base = c.get01(id);
        let color = src_style(src).color;
        match matrix::live_route(src, id, c) {
            Some(r) => fixed_arc(c, id, r, 0.5, base).map(|(lo, hi)| RingRange { lo, hi, color, dim: false }),
            None => {
                let (lo, hi) = slot_span(base, 0.5, src.bipolar());
                Some(RingRange { lo, hi, color, dim: false })
            }
        }
    }
}

/// Normalized span a slot sweeps the knob across: both ways around the base when bipolar, one way
/// (up for a positive amount) when unipolar.
fn slot_span(base: f32, amount: f32, bipolar: bool) -> (f32, f32) {
    let (lo, hi) = if bipolar {
        (base - amount.abs(), base + amount.abs())
    } else if amount >= 0.0 {
        (base, base + amount)
    } else {
        (base + amount, base)
    };
    (lo.clamp(0.0, 1.0), hi.clamp(0.0, 1.0))
}

/// Normalized span of a built-in route other than the Mod A / Mod B / envelope ones
/// [`ring_ranges`] draws, from its amount in the route's own unit; `None` where the effect is not a
/// simple sweep (velocity to level, attack and depth).
fn fixed_arc(c: &dyn Ctl, id: &str, r: &FixedRoute, amt: f32, base: f32) -> Option<(f32, f32)> {
    let plain = c.plain(id, base);
    let n = |p: f32| c.norm(id, p).clamp(0.0, 1.0);
    let one_way = |a: f32, b: f32| (n(a.min(b)), n(a.max(b)));
    match r.route {
        "wheel_index" => Some(one_way(plain, plain + 127.0 * amt)),
        "wheel_shift" | "vel_shift" => Some(one_way(plain, plain + 24.0 * amt)),
        "wheel_cutoff" => Some(one_way(plain, plain * 2f32.powf(5.0 * amt))),
        "wheel_mod" => Some(one_way(plain, plain * (1.0 + amt))),
        "cv1_amount" | "cv2_amount" | "cv3_amount" => Some(((base - amt.abs()).clamp(0.0, 1.0), (base + amt.abs()).clamp(0.0, 1.0))),
        _ => None,
    }
}

// ---- test hooks --------------------------------------------------------------------------------

fn hooks_id() -> Id {
    Id::new("gts-test-hooks")
}

/// Headless tests: record where every knob and chip is drawn (see [`knob_rects`], [`chip_rects`],
/// [`menu_rects`], [`matrix_rows`]).
pub fn enable_test_hooks(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(hooks_id(), true));
}

/// Cell rect of every knob drawn in the last frame, by param id.
pub fn knob_rects(ctx: &egui::Context) -> Vec<(String, Rect)> {
    ctx.data(|d| d.get_temp::<Vec<(String, Rect)>>(Id::new("gts-hook-knobs"))).unwrap_or_default()
}

/// Rect of every source chip (and macro handle) drawn in the last frame, by source.
pub fn chip_rects(ctx: &egui::Context) -> Vec<(Source, Rect)> {
    ctx.data(|d| d.get_temp::<Vec<(Source, Rect)>>(Id::new("gts-hook-chips"))).unwrap_or_default()
}

/// The open knob menu's buttons: (action, routing index in the menu, rect).
pub fn menu_rects(ctx: &egui::Context) -> Vec<(&'static str, usize, Rect)> {
    ctx.data(|d| d.get_temp::<Vec<(&'static str, usize, Rect)>>(Id::new("gts-hook-menu"))).unwrap_or_default()
}

/// One filled slot row of the MATRIX page as drawn: (source, target, amount, bipolar, enabled).
pub type SlotRow = (Source, String, f32, bool, bool);

/// The slot rows the MATRIX page drew in the last frame, and how many built-in rows it listed.
pub fn matrix_rows(ctx: &egui::Context) -> (Vec<SlotRow>, usize) {
    ctx.data(|d| d.get_temp::<(Vec<SlotRow>, usize)>(Id::new("gts-hook-rows"))).unwrap_or_default()
}

fn hook_push<T: Clone + Send + Sync + 'static>(ui: &Ui, name: &str, item: T) {
    ui.data_mut(|d| d.get_temp_mut_or_default::<Vec<T>>(Id::new(name)).push(item));
}

// ---- frame bracket -----------------------------------------------------------------------------

/// Start of `draw`: the frame's [`MView`], and a clean slate for the hot target and the hooks.
pub(super) fn begin_frame(ctx: &egui::Context, c: &dyn Ctl, f: &VizFrame, hover_src: Option<Source>, now: f64) {
    let drag = drag_src(ctx);
    let record = ctx.data(|d| d.get_temp::<bool>(hooks_id())).unwrap_or(false);
    let mv = MView::new(c, f, if drag.is_some() { None } else { hover_src }, drag, now as f32, record);
    ctx.data_mut(|d| {
        d.insert_temp(mview_id(), Arc::new(mv));
        d.remove_temp::<String>(hot_id());
        if record {
            d.remove_temp::<Vec<(String, Rect)>>(Id::new("gts-hook-knobs"));
            d.remove_temp::<Vec<(Source, Rect)>>(Id::new("gts-hook-chips"));
            d.remove_temp::<Vec<(&'static str, usize, Rect)>>(Id::new("gts-hook-menu"));
        }
    });
}

pub(super) fn mview(ctx: &egui::Context) -> Option<Arc<MView>> {
    ctx.data(|d| d.get_temp::<Arc<MView>>(mview_id()))
}

/// What `kn` does for a compact knob with the matrix: fills its style, returns what it needs after.
pub(super) fn decorate(ctx: &egui::Context, c: &dyn Ctl, id: &str, style: &mut KnobStyle) -> Option<(Arc<MView>, KnobMod, bool)> {
    let mv = mview(ctx)?;
    let m = mv.knob(c, id);
    style.live = m.live;
    style.ranges = m.ranges;
    let mut dim = false;
    if let Some(src) = mv.drag {
        if m.target {
            let hot = ctx.data(|d| d.get_temp::<bool>(Id::new(("gts-hot", id)))).unwrap_or(false);
            style.target = Some(TargetFx { color: src_style(src).color, hot, t: if rr_ui::motion::reduced_motion(ctx) { 0.0 } else { mv.time } });
            if hot {
                if let Some(pr) = mv.preview(c, id, src) {
                    // The preview replaces this source's ring, or takes the next free one.
                    let k = (0..RING_MAX).find(|&k| m.arc_src[k] == Some(src)).or_else(|| style.ranges.iter().position(Option::is_none));
                    if let Some(k) = k {
                        style.ranges[k] = Some(pr);
                    }
                }
            }
        } else {
            dim = true;
        }
    } else if let Some(src) = mv.hover_src {
        if m.srcs & (1 << src as u32) != 0 {
            style.ring = Some(src_style(src).color);
        } else {
            dim = true;
        }
    }
    Some((mv, m, dim))
}

/// After `knob_fn`: the drop test, the test hook, ring edits and the knob menu.
pub(super) fn after_knob(ui: &mut Ui, c: &dyn Ctl, id: &str, r: &KnobResponse, mv: &MView, m: &KnobMod) {
    if mv.record {
        if let Some(rect) = r.rect {
            hook_push(ui, "gts-hook-knobs", (id.to_string(), rect));
        }
    }
    if mv.drag.is_some() {
        let hot = m.target && r.pointer_over;
        ui.data_mut(|d| {
            d.insert_temp(Id::new(("gts-hot", id)), hot);
            if hot {
                d.insert_temp(hot_id(), id.to_string());
            }
        });
    }
    if let Some(ev) = r.ring {
        edit_ring(c, mv, m, ev);
    }
    if let (Some(pos), true) = (r.secondary, m.modulated) {
        let now = ui.input(|i| i.time);
        ui.data_mut(|d| d.insert_temp(Id::new("gts-kmenu"), KMenu { id: id.to_string(), pos, t0: now }));
    }
    if let Some(k) = r.ring_hover {
        if let (Some(src), Some(ed)) = (m.arc_src[k], m.edits[k]) {
            let value = match ed {
                ArcEdit::Slot(i) => format!("{:+.0} %", mv.matrix.slots.get(i).map_or(0.0, |s| s.amount) * 100.0),
                ArcEdit::Route(p) => c.readout(p),
            };
            set_hover(ui, &format!("{} \u{203a} {}", label_full(&mv.matrix, src), c.name(id)).to_uppercase(), &value, None, crate::help::ui_help("ring").unwrap_or(""));
        }
    }
}

/// A press-drag on a ring band changes that source's amount: a slot's through the matrix, a
/// built-in route's through its param (one host gesture per drag).
fn edit_ring(c: &dyn Ctl, mv: &MView, m: &KnobMod, ev: RingEdit) {
    let (Some(ed), Some(shared)) = (m.edits.get(ev.k).copied().flatten(), c.matrix()) else { return };
    let med = MatrixEditor { shared, ctl: c };
    let step = |dy_up: f32, fine: bool, range: f32| dy_up / DRAG_PX * range * if fine { FINE } else { 1.0 };
    match (ed, ev.phase) {
        (ArcEdit::Slot(i), RingPhase::Drag { dy_up, fine }) => {
            let cur = mv.matrix.slots.get(i).map_or(0.0, |s| s.amount);
            med.set_amount(i, cur + step(dy_up, fine, 2.0));
        }
        (ArcEdit::Slot(i), RingPhase::Reset) => med.set_amount(i, 0.0),
        (ArcEdit::Route(p), RingPhase::Start) => c.begin(p),
        (ArcEdit::Route(p), RingPhase::Drag { dy_up, fine }) => c.set01(p, c.get01(p) + step(dy_up, fine, 1.0)),
        (ArcEdit::Route(p), RingPhase::End) => c.end(p),
        (ArcEdit::Route(p), RingPhase::Reset) => {
            let z = pidx(p).map_or(0.0, |i| params_info()[i].norm(0.0));
            one_shot(c, p, z);
        }
        _ => {}
    }
}

/// End of `draw`: the drop (or the cancel), the drag ghost and its help text, the knob menu.
pub(super) fn end_frame(ui: &mut Ui, c: &dyn Ctl, bv: &mut BusView, at: &dyn Fn(f32, f32, f32, f32) -> Rect) {
    let ctx = ui.ctx().clone();
    let Some(mv) = mview(&ctx) else { return };
    if let Some(s) = more_popover(ui, &mv, bv, at(0.0, 0.0, 0.0, 0.0).min) {
        // Light the knobs a MORE chip moves, like the primary chips (read next frame).
        ctx.data_mut(|d| d.insert_temp(chip_id(), Some(s)));
    }
    knob_menu(ui, c, &mv);
    let Some(src) = mv.drag else { return };
    let hot = ctx.data(|d| d.get_temp::<String>(hot_id()));
    let st = src_style(src);
    let name = label_of(&mv.matrix, src);
    match &hot {
        Some(t) => set_hover(ui, "PATCH", &format!("{name} \u{203a} {}", c.name(t)), None, crate::help::ui_help("patch:hot").unwrap_or("")),
        None => set_hover(ui, "PATCH", &name, None, crate::help::ui_help("patch:idle").unwrap_or("")),
    }
    ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
    let hold = ctx.data(|d| d.get_temp::<bool>(Id::new("gts-drag-hold"))).unwrap_or(false);
    let (released, esc, pos) = ctx.input(|i| (!i.pointer.primary_down() && !hold, i.key_pressed(egui::Key::Escape), i.pointer.latest_pos()));
    if let Some(pos) = pos {
        let lp = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, Id::new("gts-ghost")));
        let w = chip_width(&lp, &name, false);
        let rect = Rect::from_min_size(pos2(pos.x - w / 2.0, pos.y - 34.0), vec2(w, 20.0));
        lp.rect_filled(rect.translate(vec2(0.0, 6.0)).expand(2.0), 12.0, Color32::BLACK.gamma_multiply(0.35));
        lp.rect_filled(rect, 10.0, T.inset);
        paint_chip(&lp, rect, st.glyph, &name, st.color, None, 1.0);
        lp.rect_stroke(rect, 10.0, rr_ui::paint::st(1.5, st.color.gamma_multiply(0.9)), StrokeKind::Inside);
    }
    if released || esc {
        if let (false, Some(t), Some(shared)) = (esc, hot, c.matrix()) {
            match (MatrixEditor { shared, ctl: c }).add_or_edit(src, &t, 0.5) {
                Err(EditError::Full) => notice(&ctx, &format!("The matrix is full: {MAX_SLOTS} routings. Remove one on the MATRIX page first.")),
                Err(EditError::NotATarget) | Ok(_) => {}
            }
        }
        ctx.data_mut(|d| d.insert_temp(drag_id(), None::<Source>));
    }
}

// ---- sources strip -----------------------------------------------------------------------------

/// The two strip rows under the hero (MOTION.md 2.1): SOURCES (Mod A, Mod B, Filter Env, Amp A,
/// Amp B) and MACROS (four labelled knobs, each with a drag handle) with the MORE chip for the rest.
/// Returns the chip under the pointer (display only; the ring on its targets is drawn next frame).
pub(super) fn sources_strip(ui: &mut Ui, c: &dyn Ctl, f: &VizFrame, bv: &mut BusView, at: &dyn Fn(f32, f32, f32, f32) -> Rect) -> Option<Source> {
    let Some(mv) = mview(ui.ctx()) else { return None };
    let newest = f.newest();
    let act = |k: usize| newest.map(|v| f.voice(v, k).abs());
    let mut hovered = None;
    rr_ui::motion::with_page_fx(ui, 296.0, |ui| {
        let (row1, row2) = (at(44.0, 296.0, 568.0, 20.0), at(44.0, 318.0, 568.0, 26.0));
        let p = ui.painter().clone();
        rr_ui::paint::tracked(&p, pos2(row1.left() + 8.0, row1.center().y), Align2::LEFT_CENTER, "SOURCES", font::label(11.0), T.title, 1.4);
        rr_ui::paint::tracked(&p, pos2(row2.left() + 8.0, row2.center().y), Align2::LEFT_CENTER, "MACROS", font::label(11.0), T.title, 1.4);
        let mut x = row1.left() + 76.0;
        // The five chips share what the row has left; the gap shrinks before a chip would leave the hero's width.
        let widths = PRIMARY.map(|s| chip_width(&p, src_style(s).name, true));
        let gap = ((row1.right() - x - widths.iter().sum::<f32>()) / (PRIMARY.len() - 1) as f32).clamp(2.0, 5.0);
        for (src, w) in PRIMARY.into_iter().zip(widths) {
            let a = match src {
                Source::ModA => act(vf::OUT_A),
                Source::ModB => act(vf::OUT_B),
                Source::FilterEnv => act(vf::FENV),
                Source::AmpEnvA => act(vf::AMP_A),
                _ => act(vf::AMP_B),
            };
            chip(ui, &mv, src, Rect::from_min_size(pos2(x, row1.top()), vec2(w, 20.0)), Some(a.unwrap_or(0.0)), &mut hovered);
            x += w + gap;
        }
        let mut x = row2.left() + 76.0;
        for (k, src) in MACROS.into_iter().enumerate() {
            macro_pill(ui, c, &mv, k, src, Rect::from_min_size(pos2(x, row2.top()), vec2(100.0, 26.0)), &mut hovered);
            x += 106.0;
        }
        more_chip(ui, &mv, Rect::from_min_size(pos2(x, row2.top() + 2.0), vec2(row2.right() - x, 20.0)), bv);
    });
    hovered
}

/// One draggable chip: starts a patch drag, reports hover.
fn chip(ui: &mut Ui, mv: &MView, src: Source, rect: Rect, activity: Option<f32>, hovered: &mut Option<Source>) {
    let st = src_style(src);
    let name = label_of(&mv.matrix, src);
    let r = source_chip(ui, rect, st.glyph, &name, st.color, activity);
    if mv.record {
        hook_push(ui, "gts-hook-chips", (src, rect));
    }
    if r.drag_started() {
        start_drag(ui.ctx(), src);
    }
    if r.hovered() && mv.drag.is_none() {
        *hovered = Some(src);
        let n = mv.routes_of(src);
        let value = if n == 0 { "no routings".to_string() } else { format!("{n} routing{}", if n == 1 { "" } else { "s" }) };
        set_hover(ui, &name.to_uppercase(), &value, None, crate::help::ui_help(&format!("src:{}", src.name())).unwrap_or(""));
    }
}

/// A macro: handle disc (drag to patch), label (double-click to rename), knob.
fn macro_pill(ui: &mut Ui, c: &dyn Ctl, mv: &MView, k: usize, src: Source, rect: Rect, hovered: &mut Option<Source>) {
    let id = format!("macro{}", k + 1);
    let p = ui.painter().clone();
    let cy = rect.center().y;
    let handle = Rect::from_center_size(pos2(rect.left() + 13.0, cy), vec2(20.0, 20.0));
    let dial = Rect::from_center_size(pos2(rect.right() - 15.0, cy), vec2(24.0, 24.0));
    let label = Rect::from_min_max(pos2(rect.left() + 26.0, rect.top()), pos2(dial.left() - 3.0, rect.bottom()));
    let hr = ui.interact(handle, Id::new(("gts-macro-handle", k)), Sense::drag());
    let lr = ui.interact(label, Id::new(("gts-macro-label", k)), Sense::click());
    let v = c.get01(&id);
    let hot_h = rr_ui::motion::tween(ui.ctx(), Id::new(("gts-macro-hot", k)), hr.hovered() || hr.dragged(), 0.14);
    p.rect_filled(rect, 13.0, T.inset);
    p.rect_stroke(rect, 13.0, rr_ui::paint::hair(&p, SRC_M.gamma_multiply(0.35 + 0.45 * hot_h)), StrokeKind::Inside);
    p.circle_filled(handle.center(), 7.0, SRC_M);
    p.text(handle.center(), Align2::CENTER_CENTER, src_style(src).glyph, font::strong(10.0), Color32::from_rgb(0x10, 0x14, 0x16));
    let dr = mini_dial(ui, dial, &id, v, c.default01(&id), SRC_M);
    if hr.hovered() || hr.dragged() {
        ui.ctx().set_cursor_icon(if hr.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
    }
    if mv.record {
        hook_push(ui, "gts-hook-chips", (src, handle));
    }
    if hr.drag_started() {
        start_drag(ui.ctx(), src);
    }
    // Label: editable in place.
    let edit_id = Id::new("gts-macro-edit");
    let editing = ui.data(|d| d.get_temp::<(usize, String)>(edit_id)).filter(|e| e.0 == k);
    let name = &mv.matrix.macros[k];
    if let Some((_, mut buf)) = editing {
        let te = ui.put(label, egui::TextEdit::singleline(&mut buf).font(font::strong(11.0)).frame(false).margin(egui::Margin::ZERO).desired_width(label.width()).vertical_align(egui::Align::Center));
        if !te.has_focus() && !te.lost_focus() {
            te.request_focus();
            // Start with the old name selected so typing replaces it.
            if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), te.id) {
                st.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(buf.chars().count()))));
                st.store(ui.ctx(), te.id);
            }
        }
        if buf.chars().count() > 14 {
            buf = buf.chars().take(14).collect();
        }
        let (esc, enter) = ui.input(|i| (i.key_pressed(egui::Key::Escape), i.key_pressed(egui::Key::Enter)));
        if esc {
            ui.data_mut(|d| d.remove_temp::<(usize, String)>(edit_id));
        } else if te.lost_focus() || enter {
            if let Some(shared) = c.matrix() {
                let t = buf.trim();
                (MatrixEditor { shared, ctl: c }).set_macro_label(k, if t.is_empty() { format!("Macro {}", k + 1) } else { t.to_string() }.as_str());
            }
            ui.data_mut(|d| d.remove_temp::<(usize, String)>(edit_id));
        } else {
            ui.data_mut(|d| d.insert_temp(edit_id, (k, buf)));
        }
        p.rect_stroke(label.shrink2(vec2(-2.0, 1.0)), 3.0, rr_ui::paint::hair(&p, T.a(0.55)), StrokeKind::Inside);
    } else {
        if lr.double_clicked() {
            ui.data_mut(|d| d.insert_temp(edit_id, (k, name.clone())));
        }
        let showing_value = dr.hovered || dr.pointer_over && ui.input(|i| i.pointer.primary_down());
        if showing_value {
            p.text(pos2(label.left(), cy), Align2::LEFT_CENTER, c.readout_at(&id, dr.changed.unwrap_or(v)), font::mono(11.0), T.accent);
        } else {
            let text = elide(&p, &name.to_uppercase(), font::strong(11.0), label.width() - 2.0);
            rr_ui::paint::tracked(&p, pos2(label.left(), cy), Align2::LEFT_CENTER, &text, font::strong(11.0), T.text, 0.5);
        }
    }
    drive(ui, c, &id, dr, false);
    if mv.drag.is_none() {
        if hr.hovered() {
            *hovered = Some(src);
            let n = mv.routes_of(src);
            let value = if n == 0 { "no routings".to_string() } else { format!("{n} routing{}", if n == 1 { "" } else { "s" }) };
            set_hover(ui, &label_full(&mv.matrix, src).to_uppercase(), &value, None, crate::help::ui_help(&format!("src:{}", src.name())).unwrap_or(""));
        } else if lr.hovered() {
            set_hover(ui, &label_full(&mv.matrix, src).to_uppercase(), "double-click to rename", None, crate::help::ui_help("macro:label").unwrap_or(""));
        } else if dr.hovered {
            let value = c.readout_at(&id, v);
            ui.data_mut(|d| d.insert_temp(hover_id(), Hover { name: label_full(&mv.matrix, src).to_uppercase(), id: id.clone(), value, ..Default::default() }));
        }
    }
}

/// The MORE chip: opens the popover with the other nine sources; dots show which are in use.
fn more_chip(ui: &mut Ui, mv: &MView, rect: Rect, bv: &mut BusView) {
    let resp = ui.interact(rect, Id::new("gts-more-chip"), Sense::click());
    let p = ui.painter().clone();
    rr_ui::paint::inset_box(&p, rect, resp.hovered() || bv.more_open);
    if bv.more_open {
        p.rect_stroke(rect, 4.0, rr_ui::paint::hair(&p, T.a(0.55)), StrokeKind::Inside);
    }
    let col = if bv.more_open { T.accent } else { T.muted };
    rr_ui::paint::caps(&p, pos2(rect.left() + 8.0, rect.center().y), Align2::LEFT_CENTER, "More", 11.0, col);
    // One dot per family of hidden sources that is in use.
    let mut x = rect.right() - 8.0;
    for s in MORE.into_iter().filter(|s| mv.routes_of(*s) > 0).map(|s| src_style(s).color).collect::<Vec<_>>().into_iter().rev().take(3) {
        p.circle_filled(pos2(x, rect.center().y), 2.5, s);
        x -= 7.0;
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        set_hover(ui, "MORE", "9 sources", None, crate::help::ui_help("more").unwrap_or(""));
    }
    if resp.clicked() {
        bv.more_open = !bv.more_open;
    }
}

/// The nine less common sources as chips in a layer above the strip (150 ms fade, 6 px rise). It
/// closes on Esc, a click elsewhere, or as soon as one chip is picked up for a drag.
fn more_popover(ui: &mut Ui, mv: &MView, bv: &mut BusView, origin: Pos2) -> Option<Source> {
    let ctx = ui.ctx().clone();
    let k = rr_ui::motion::tween(&ctx, Id::new("gts-more-a"), bv.more_open, if bv.more_open { 0.15 } else { 0.09 });
    if k <= 0.0 {
        return None;
    }
    let tile = vec2(104.0, 20.0);
    let size = vec2(3.0 * tile.x + 2.0 * 6.0 + 16.0, 3.0 * tile.y + 2.0 * 6.0 + 16.0);
    let chip_row = Rect::from_min_size(origin + vec2(44.0, 320.0), vec2(568.0, 20.0));
    let pos = pos2(chip_row.right() - size.x, chip_row.top() - 30.0 - size.y + 6.0 * (1.0 - k));
    let open = bv.more_open;
    let mut picked = false;
    let mut hovered = None;
    egui::Area::new(Id::new("gts-more-pop")).order(egui::Order::Foreground).fixed_pos(pos).show(&ctx, |ui| {
        ui.multiply_opacity(k);
        let r = Rect::from_min_size(pos, size);
        ui.painter().rect_filled(r.translate(vec2(0.0, 10.0)).expand(4.0), 10.0, Color32::BLACK.gamma_multiply(0.35));
        ui.painter().rect_filled(r.translate(vec2(0.0, 2.0)), 6.0, Color32::BLACK.gamma_multiply(0.5));
        ui.painter().rect_filled(r, 6.0, T.well);
        ui.painter().rect_stroke(r, 6.0, rr_ui::paint::hair(ui.painter(), T.lo()), StrokeKind::Inside);
        for (i, src) in MORE.into_iter().enumerate() {
            let at = r.min + vec2(8.0 + (i % 3) as f32 * (tile.x + 6.0), 8.0 + (i / 3) as f32 * (tile.y + 6.0));
            let rect = Rect::from_min_size(at, tile);
            let st = src_style(src);
            let bar = match src {
                Source::BusCv1 | Source::BusCv2 | Source::BusCv3 => None,
                _ => None,
            };
            let resp = source_chip(ui, rect, st.glyph, st.name, st.color, bar);
            // Routes in use: a count badge at the chip's right end.
            let n = mv.routes_of(src);
            if n > 0 {
                ui.painter().text(pos2(rect.right() - 9.0, rect.center().y), Align2::CENTER_CENTER, n.to_string(), font::mono(10.0), st.color);
            }
            if mv.record {
                hook_push(ui, "gts-hook-chips", (src, rect));
            }
            if resp.drag_started() && open {
                start_drag(ui.ctx(), src);
                picked = true;
            }
            if resp.hovered() && open {
                hovered = Some(src);
            }
        }
    });
    if let Some(src) = hovered {
        let n = mv.routes_of(src);
        let value = if n == 0 { "no routings".to_string() } else { format!("{n} routing{}", if n == 1 { "" } else { "s" }) };
        set_hover(ui, &src_style(src).name.to_uppercase(), &value, None, crate::help::ui_help(&format!("src:{}", src.name())).unwrap_or(""));
    }
    if picked {
        bv.more_open = false;
    }
    if bv.more_open {
        let (esc, click) = ctx.input(|i| (i.key_pressed(egui::Key::Escape), i.pointer.primary_pressed().then(|| i.pointer.interact_pos()).flatten()));
        let chip = Rect::from_min_size(pos2(origin.x + 44.0 + 76.0 + 4.0 * 106.0, origin.y + 320.0), vec2(68.0, 20.0));
        let outside = click.is_some_and(|p| !Rect::from_min_size(pos, size).contains(p) && !chip.contains(p));
        if esc || outside {
            bv.more_open = false;
        }
    }
    hovered
}

// ---- knob menu ---------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct KMenu {
    id: String,
    pos: Pos2,
    /// Egui time of the right-click that opened it.
    t0: f64,
}

const MENU_W: f32 = 280.0;

/// A small caps button; `lit` draws it in `col`.
fn menu_button(ui: &mut Ui, rect: Rect, id: Id, label: &str, lit: bool, col: Color32, help: &'static str) -> bool {
    let resp = ui.interact(rect, id, Sense::click());
    let p = ui.painter();
    rr_ui::paint::inset_box(p, rect, resp.hovered());
    if lit {
        p.rect_filled(rect, 4.0, col.gamma_multiply(0.18));
        p.rect_stroke(rect, 4.0, rr_ui::paint::hair(p, col.gamma_multiply(0.6)), StrokeKind::Inside);
    }
    rr_ui::paint::caps(p, rect.center(), Align2::CENTER_CENTER, label, 11.0, if lit { col } else if resp.hovered() { T.text } else { T.muted });
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        set_hover(ui, "KNOB MENU", label, None, help);
    }
    resp.clicked()
}

/// Right-click menu of a modulated knob: per routing Bipolar / Unipolar, Invert, Bypass and Remove,
/// then Remove all modulation. Built-in routes offer Invert (where their range goes negative) and
/// Remove (their amount knob to 0).
fn knob_menu(ui: &mut Ui, c: &dyn Ctl, mv: &MView) {
    let ctx = ui.ctx().clone();
    let Some(menu) = ctx.data(|d| d.get_temp::<KMenu>(Id::new("gts-kmenu"))) else { return };
    let cs = mv.contribs(&menu.id);
    if cs.is_empty() {
        ctx.data_mut(|d| d.remove_temp::<KMenu>(Id::new("gts-kmenu")));
        return;
    }
    let size = vec2(MENU_W, 8.0 + cs.len() as f32 * 48.0 + 30.0 + 4.0);
    let screen = ctx.screen_rect();
    let pos = pos2(menu.pos.x.min(screen.right() - size.x - 4.0).max(screen.left() + 4.0), menu.pos.y.min(screen.bottom() - size.y - 4.0).max(screen.top() + 4.0));
    let mut act: Option<(&'static str, usize)> = None;
    egui::Area::new(Id::new("gts-kmenu-area")).order(egui::Order::Foreground).fixed_pos(pos).show(&ctx, |ui| {
        let r = Rect::from_min_size(pos, size);
        ui.painter().rect_filled(r.translate(vec2(0.0, 10.0)).expand(4.0), 10.0, Color32::BLACK.gamma_multiply(0.35));
        ui.painter().rect_filled(r.translate(vec2(0.0, 2.0)), 6.0, Color32::BLACK.gamma_multiply(0.5));
        ui.painter().rect_filled(r, 6.0, T.inset);
        ui.painter().rect_stroke(r, 6.0, rr_ui::paint::hair(ui.painter(), T.lo()), StrokeKind::Inside);
        let mut y = r.top() + 8.0;
        for (k, ct) in cs.iter().enumerate() {
            let st = src_style(ct.src);
            let head = Rect::from_min_size(pos2(r.left() + 10.0, y), vec2(MENU_W - 20.0, 20.0));
            ui.painter().circle_filled(pos2(head.left() + 7.0, head.center().y), 7.0, st.color);
            ui.painter().text(pos2(head.left() + 7.0, head.center().y), Align2::CENTER_CENTER, st.glyph, font::strong(10.0), Color32::from_rgb(0x10, 0x14, 0x16));
            let name = label_full(&mv.matrix, ct.src).to_uppercase();
            let amount = match ct.kind {
                Kind::Slot(_) => format!("{:+.0} %", ct.amount * 100.0),
                Kind::Fixed(f) => format!("{} (built in)", c.readout(f.route)),
            };
            let tr = rr_ui::paint::tracked(ui.painter(), pos2(head.left() + 20.0, head.center().y), Align2::LEFT_CENTER, &name, font::strong(11.0), if ct.on { T.text } else { T.muted }, 0.5);
            ui.painter().text(pos2(tr.right() + 8.0, head.center().y), Align2::LEFT_CENTER, amount, font::mono(11.0), T.muted);
            y += 22.0;
            let row = Rect::from_min_size(pos2(r.left() + 10.0, y), vec2(MENU_W - 20.0, 22.0));
            let btn = |i: usize, n: usize| {
                let w = (row.width() - 6.0 * (n - 1) as f32) / n as f32;
                Rect::from_min_size(pos2(row.left() + i as f32 * (w + 6.0), row.top()), vec2(w, row.height()))
            };
            match ct.kind {
                Kind::Slot(_) => {
                    for (i, (action, label, lit, help)) in [
                        ("polarity", if ct.bipolar { "Bipolar" } else { "Unipolar" }, ct.bipolar, "menu:pol"),
                        ("invert", "Invert", ct.amount < 0.0, "menu:invert"),
                        ("bypass", "Bypass", !ct.on, "menu:bypass"),
                        ("remove", "Remove", false, "menu:remove"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let rect = btn(i, 4);
                        if mv.record {
                            hook_push(ui, "gts-hook-menu", (action, k, rect));
                        }
                        if menu_button(ui, rect, Id::new(("gts-kmenu-btn", k, action)), label, lit, st.color, crate::help::ui_help(help).unwrap_or("")) {
                            act = Some((action, k));
                        }
                    }
                }
                Kind::Fixed(f) => {
                    let can_invert = pidx(f.route).is_some_and(|i| params_info()[i].plain(0.0) < 0.0);
                    let items: Vec<(&'static str, &'static str, bool, &'static str)> =
                        if can_invert { vec![("invert", "Invert", ct.amount < 0.0, "menu:invert"), ("remove", "Remove", false, "menu:remove")] } else { vec![("remove", "Remove", false, "menu:remove")] };
                    for (i, (action, label, lit, help)) in items.iter().enumerate() {
                        let rect = btn(i, items.len());
                        if mv.record {
                            hook_push(ui, "gts-hook-menu", (*action, k, rect));
                        }
                        if menu_button(ui, rect, Id::new(("gts-kmenu-btn", k, *action)), label, *lit, st.color, crate::help::ui_help(help).unwrap_or("")) {
                            act = Some((action, k));
                        }
                    }
                }
            }
            y += 26.0;
        }
        let all = Rect::from_min_size(pos2(r.left() + 10.0, y), vec2(MENU_W - 20.0, 24.0));
        if mv.record {
            hook_push(ui, "gts-hook-menu", ("all", 0usize, all));
        }
        if menu_button(ui, all, Id::new("gts-kmenu-all"), "Remove all (incl. built-in)", false, T.accent, crate::help::ui_help("menu:all").unwrap_or("")) {
            act = Some(("all", 0));
        }
    });
    let now = ctx.input(|i| i.time);
    let click = ctx.input(|i| (i.pointer.primary_pressed() || i.pointer.secondary_pressed()).then(|| i.pointer.interact_pos()).flatten());
    let outside = click.is_some_and(|p| !Rect::from_min_size(pos, size).contains(p)) && menu.t0 != now;
    let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let mut close = outside || esc;
    if let (Some((action, k)), Some(shared)) = (act, c.matrix()) {
        let ed = MatrixEditor { shared, ctl: c };
        let ct = cs[k];
        match (action, ct.kind) {
            ("polarity", Kind::Slot(i)) => ed.set_bipolar(i, !ct.bipolar),
            ("invert", Kind::Slot(i)) => ed.set_amount(i, -ct.amount),
            ("bypass", Kind::Slot(i)) => ed.set_enabled(i, !ct.on),
            ("remove", Kind::Slot(i)) => {
                ed.remove(i);
                close = cs.len() == 1;
            }
            ("invert", Kind::Fixed(f)) => {
                if let Some(i) = pidx(f.route) {
                    one_shot(c, f.route, params_info()[i].norm(-ct.amount));
                }
            }
            ("remove", Kind::Fixed(f)) => {
                if let Some(i) = pidx(f.route) {
                    one_shot(c, f.route, params_info()[i].norm(0.0));
                }
                close = cs.len() == 1;
            }
            ("all", _) => {
                ed.remove_all_for_target(&menu.id);
                close = true;
            }
            _ => {}
        }
    }
    if close {
        ctx.data_mut(|d| d.remove_temp::<KMenu>(Id::new("gts-kmenu")));
    }
}

// ---- MATRIX page -------------------------------------------------------------------------------

/// Section of a target for the picker, in panel order.
const SECTIONS: [&str; 21] = [
    "Output", "Osc A", "Osc B", "Sub", "Mod A", "Mod B", "Filter A", "Filter B", "Filter Env", "Shaper", "Voice", "Wheel", "Velocity", "Distortion", "Multiband", "EQ", "Chorus", "Dimension", "Delay", "Reverb",
    "Limiter",
];

/// Which [`SECTIONS`] entry holds target `id` (the last one, Limiter, is also the fallback).
pub fn section_of(id: &str) -> usize {
    let pre = |p: &str| id.starts_with(p);
    match id {
        "volume" | "spread" => 0,
        "mono_xover" => 3,
        "porta" | "bend_range" => 10,
        _ if pre("osca_") => 1,
        _ if pre("oscb_") => 2,
        _ if pre("sub_") || pre("noise_") => 3, // Track 7: noise shares the Sub section
        _ if pre("moda_") => 4,
        _ if pre("modb_") => 5,
        _ if pre("filta_") => 6,
        _ if pre("filtb_") => 7,
        _ if pre("fenv_") => 8,
        _ if pre("shaper_") => 9,
        _ if pre("wheel_") => 11,
        _ if pre("vel_") => 12,
        _ if pre("dist_") => 13,
        _ if pre("mb_") => 14,
        _ if pre("eq_") => 15,
        _ if pre("cho_") => 16,
        _ if pre("dim_") => 17,
        _ if pre("dly_") => 18,
        _ if pre("rev_") => 19,
        _ => 20,
    }
}

pub fn section_name(i: usize) -> &'static str {
    SECTIONS[i.min(SECTIONS.len() - 1)]
}

/// One row of the target picker.
enum PickRow {
    Header(&'static str),
    Item(&'static str, &'static str),
}

fn pick_rows() -> &'static [PickRow] {
    static R: OnceLock<Vec<PickRow>> = OnceLock::new();
    R.get_or_init(|| {
        let mut v = vec![];
        for (s, name) in SECTIONS.iter().enumerate() {
            v.push(PickRow::Header(name));
            v.extend(params_info().iter().filter(|p| p.target && section_of(&p.id) == s).map(|p| PickRow::Item(p.id.as_str(), p.name.as_str())));
        }
        v
    })
}

/// A target button with a scrolling list grouped by section. Returns the picked param id.
fn target_picker(ui: &mut Ui, rect: Rect, key: Id, cur: Option<&str>, help: &'static str) -> Option<String> {
    let resp = ui.interact(rect, key, Sense::click());
    let popup_id = key.with("popup");
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    let open = ui.memory(|m| m.is_popup_open(popup_id));
    let p = ui.painter().clone();
    rr_ui::paint::inset_box(&p, rect, resp.hovered() || open);
    let name = cur.and_then(pidx).map(|i| params_info()[i].name.as_str());
    let f = font::label(13.0);
    let (txt, col) = match name {
        Some(n) => (elide(&p, n, f.clone(), rect.width() - 28.0), T.text),
        None => ("Target\u{2026}".to_string(), T.muted),
    };
    p.text(pos2(rect.left() + 8.0, rect.center().y), Align2::LEFT_CENTER, txt, f.clone(), col);
    let s = 1.0 - 2.0 * rr_ui::motion::tween(ui.ctx(), popup_id.with("chev"), open, 0.18);
    let (cx, cy) = (rect.right() - 11.5, rect.center().y - 1.0);
    p.add(egui::Shape::line(vec![pos2(cx - 3.5, cy - 1.5 * s), pos2(cx, cy + 2.0 * s), pos2(cx + 3.5, cy - 1.5 * s)], rr_ui::paint::st(1.5, T.muted)));
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        set_hover(ui, "TARGET", name.unwrap_or("none"), None, help);
    }
    let mut picked = None;
    let fade = rr_ui::motion::tween(ui.ctx(), popup_id.with("fade"), open, if open { 0.13 } else { 0.09 });
    egui::popup::popup_below_widget(ui, popup_id, &resp, egui::popup::PopupCloseBehavior::CloseOnClickOutside, |ui| {
        ui.multiply_opacity(fade);
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.set_min_width(rect.width().max(216.0));
        let rows = pick_rows();
        egui::ScrollArea::vertical().max_height(360.0).auto_shrink([false, true]).show_rows(ui, CONTROL_H, rows.len(), |ui, range| {
            for row in &rows[range] {
                let (r, rr) = ui.allocate_exact_size(vec2(ui.available_width(), CONTROL_H), Sense::click());
                match row {
                    PickRow::Header(h) => {
                        rr_ui::paint::tracked(ui.painter(), pos2(r.left() + 8.0, r.center().y + 2.0), Align2::LEFT_CENTER, &h.to_uppercase(), font::strong(11.0), T.title, 1.2);
                    }
                    PickRow::Item(id, name) => {
                        if rr.hovered() {
                            ui.painter().rect_filled(r, 3.0, T.a(0.16));
                        }
                        let on = cur == Some(*id);
                        ui.painter().text(pos2(r.left() + 18.0, r.center().y), Align2::LEFT_CENTER, name, f.clone(), if on { T.accent } else { T.text });
                        if rr.clicked() {
                            picked = Some(id.to_string());
                            ui.memory_mut(|m| m.close_popup());
                        }
                    }
                }
            }
        });
    });
    picked
}

/// Built-in routes as MATRIX rows: (route param, source, target text; `""` = per CV target).
const FIXED_ROWS: [(&str, Source, &str); 21] = [
    ("moda_pitch", Source::ModA, "Osc Pitch"),
    ("moda_index", Source::ModA, "Osc Index"),
    ("moda_shift", Source::ModA, "Osc Shift"),
    ("modb_motion", Source::ModB, "Osc Motion"),
    ("modb_vol", Source::ModB, "Osc Vol"),
    ("modb_cutoff", Source::ModB, "Filter Freq"),
    ("modb_moda", Source::ModB, "Mod A depth"),
    ("wheel_index", Source::ModWheel, "Osc Index"),
    ("wheel_shift", Source::ModWheel, "Osc Shift"),
    ("wheel_cutoff", Source::ModWheel, "Filter Freq"),
    ("wheel_mod", Source::ModWheel, "Mod A depth"),
    ("vel_vol_a", Source::Velocity, "Osc A Vol"),
    ("vel_vol_b", Source::Velocity, "Osc B Vol"),
    ("vel_fenv", Source::Velocity, "Filter Env Amt"),
    ("vel_att", Source::Velocity, "Osc Attack"),
    ("vel_shift", Source::Velocity, "Osc Shift"),
    ("vel_mod", Source::Velocity, "Mod A depth"),
    ("fenv_amt", Source::FilterEnv, "Filter Freq"),
    ("cv1_amount", Source::BusCv1, ""),
    ("cv2_amount", Source::BusCv2, ""),
    ("cv3_amount", Source::BusCv3, ""),
];

/// Whether a built-in row currently reaches its target (any of its gates is open).
fn route_active(route: &str, c: &dyn Ctl) -> bool {
    route == "moda_pitch" || fixed_routes().iter().any(|r| r.route == route && gate_open(r, c))
}

fn fixed_row(ui: &mut Ui, c: &dyn Ctl, rect: Rect, route: &'static str, src: Source) {
    let st = src_style(src);
    let active = route_active(route, c);
    let p = ui.painter().clone();
    let cy = rect.center().y;
    let k = rr_ui::motion::tween(ui.ctx(), Id::new(("gts-fixed-dim", route)), !active, 0.18);
    let a = 1.0 - (1.0 - INACTIVE) * k;
    p.circle_filled(pos2(rect.left() + 7.0, cy), 7.0, st.color.gamma_multiply(a));
    p.text(pos2(rect.left() + 7.0, cy), Align2::CENTER_CENTER, st.glyph, font::strong(10.0), Color32::from_rgb(0x10, 0x14, 0x16));
    rr_ui::paint::tracked(&p, pos2(rect.left() + 20.0, cy), Align2::LEFT_CENTER, &st.name.to_uppercase(), font::strong(11.0), T.text.gamma_multiply(a), 0.5);
    let target = FIXED_ROWS.iter().find(|r| r.0 == route).map(|r| r.2).unwrap_or("");
    let target = if target.is_empty() {
        let n = route.as_bytes()[2] - b'0';
        let t = c.readout(&format!("cv{n}_target"));
        if t == "Off" { "RR-BUS (no target)".to_string() } else { t }
    } else {
        target.to_string()
    };
    let tx = rect.left() + 86.0;
    p.text(pos2(tx, cy), Align2::LEFT_CENTER, elide(&p, &target, font::label(12.0), 92.0), font::label(12.0), T.muted.gamma_multiply(a));
    let bar = Rect::from_min_size(pos2(rect.right() - 98.0, rect.top()), vec2(98.0, rect.height()));
    let v = c.get01(route);
    let bipolar = pidx(route).is_some_and(|i| params_info()[i].plain(0.0) < 0.0);
    let r = amount_bar(ui, bar, route, v, c.default01(route), bipolar, st.color, &c.readout_at(route, c.get01(route)), true);
    drive(ui, c, route, r, false);
    if r.hovered || ui.rect_contains_pointer(Rect::from_min_max(rect.min, pos2(bar.left(), rect.bottom()))) {
        ui.data_mut(|d| d.insert_temp(hover_id(), Hover { name: format!("{} \u{203a} {}", st.name, target).to_uppercase(), id: route.to_string(), value: c.readout(route), ..Default::default() }));
    }
}

/// Which row of a slot list is the add row, and the half-picked pair it holds.
fn draft_id() -> Id {
    Id::new("gts-matrix-draft")
}

const SLOT_COL_W: f32 = 436.0;

/// MATRIX page (MOTION.md 2.3): built-in routes with their knobs, then the 32 slots in two columns:
/// number (click to bypass), source, polarity, amount, target, remove. The first empty row adds.
pub(super) fn matrix_body(ui: &mut Ui, c: &dyn Ctl, at: &dyn Fn(f32, f32, f32, f32) -> Rect) {
    let Some(mv) = mview(ui.ctx()) else { return };
    let Some(shared) = c.matrix() else {
        group_at(ui, at(44.0, 80.0, 912.0, 120.0), "Matrix", |_| {}, |ui| caption(ui, "No modulation matrix is connected to this editor."));
        return;
    };
    let ed = MatrixEditor { shared, ctl: c };
    // ---- built-in routes
    group_at(ui, at(44.0, 80.0, 912.0, 226.0), "Built-in routes", |ui| caption(ui, "Their amounts are the matching knobs on the SYNTH page and the PATCH popover"), |ui| {
        ui.spacing_mut().item_spacing = vec2(24.0, 0.0);
        ui.horizontal_top(|ui| {
            for col in 0..3 {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for row in 0..7 {
                        let (route, src, _) = FIXED_ROWS[col * 7 + row];
                        let (rect, _) = ui.allocate_exact_size(vec2(280.0, 24.0), Sense::hover());
                        fixed_row(ui, c, rect, route, src);
                    }
                });
            }
        });
    });
    // ---- slots
    let n = mv.matrix.slots.len();
    // Source names as the user sees them: a renamed macro shows its label.
    let labels: Vec<String> = Source::ALL.iter().map(|s| cell_label(&mv.matrix, *s)).collect();
    let names: Vec<&str> = labels.iter().map(String::as_str).collect();
    let mut report: Vec<SlotRow> = vec![];
    group_at(ui, at(44.0, 314.0, 912.0, 462.0), "Routings", |ui| caption(ui, &format!("{n} of {MAX_SLOTS} slots in use")), |ui| {
        ui.spacing_mut().item_spacing = vec2(16.0, 0.0);
        ui.horizontal_top(|ui| {
            for col in 0..2 {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let (h, _) = ui.allocate_exact_size(vec2(SLOT_COL_W, 12.0), Sense::hover());
                    for (o, t) in [(34.0, "Source"), (138.0, "Pol"), (184.0, "Amount"), (272.0, "Target")] {
                        rr_ui::paint::caps(ui.painter(), pos2(h.left() + o, h.center().y), Align2::LEFT_CENTER, t, 10.0, T.muted);
                    }
                    for row in 0..16 {
                        let i = col * 16 + row;
                        let (rect, _) = ui.allocate_exact_size(vec2(SLOT_COL_W, 24.0), Sense::hover());
                        slot_row(ui, &ed, &mv, &names, rect, i);
                        if let Some(s) = mv.matrix.slots.get(i) {
                            report.push((s.source, s.target.clone(), s.amount, s.bipolar, s.enabled));
                        }
                    }
                });
            }
        });
    });
    if mv.record {
        ui.data_mut(|d| d.insert_temp(Id::new("gts-hook-rows"), (report, FIXED_ROWS.len())));
    }
}

/// One slot row (`i` < slots) or an empty one (`i` == slots: the add row; later ones are faint).
fn slot_row(ui: &mut Ui, ed: &MatrixEditor, mv: &MView, names: &[&str], rect: Rect, i: usize) {
    let slot = mv.matrix.slots.get(i);
    let adding = i == mv.matrix.slots.len();
    let p = ui.painter().clone();
    let cy = rect.center().y;
    let x = |o: f32| rect.left() + o;
    let (num, swatch, src_r, pol, amt, tgt, del) = (
        Rect::from_min_size(pos2(x(0.0), rect.top()), vec2(24.0, 24.0)),
        Rect::from_min_size(pos2(x(28.0), cy - 7.0), vec2(3.0, 14.0)),
        Rect::from_min_size(pos2(x(34.0), rect.top()), vec2(100.0, 24.0)),
        Rect::from_min_size(pos2(x(138.0), rect.top()), vec2(42.0, 24.0)),
        Rect::from_min_size(pos2(x(184.0), rect.top()), vec2(84.0, 24.0)),
        Rect::from_min_size(pos2(x(272.0), rect.top()), vec2(140.0, 24.0)),
        Rect::from_min_size(pos2(x(416.0), rect.top()), vec2(20.0, 24.0)),
    );
    let faint = if slot.is_none() && !adding { 0.35 } else { 1.0 };
    // Number: click to bypass (the add row shows a plus).
    let num_resp = ui.interact(num, Id::new(("gts-mx-num", i)), Sense::click());
    rr_ui::paint::inset_box(&p, num, num_resp.hovered() && slot.is_some());
    let (num_col, label) = match slot {
        Some(s) => (if s.enabled { src_style(s.source).color } else { T.muted }, (i + 1).to_string()),
        None => (T.muted.gamma_multiply(faint), if adding { "+".to_string() } else { (i + 1).to_string() }),
    };
    p.text(num.center(), Align2::CENTER_CENTER, label, font::mono(11.0), num_col);
    if let Some(s) = slot {
        if num_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            set_hover(ui, &format!("ROUTING {}", i + 1), if s.enabled { "on" } else { "bypassed" }, None, crate::help::ui_help("matrix:num").unwrap_or(""));
        }
        if num_resp.clicked() {
            ed.set_enabled(i, !s.enabled);
        }
    }
    if slot.is_none() && !adding {
        return;
    }
    let draft = ui.data(|d| d.get_temp::<(Option<Source>, Option<String>)>(draft_id())).unwrap_or_default();
    let (cur_src, cur_tgt): (Option<Source>, Option<String>) = match slot {
        Some(s) => (Some(s.source), Some(s.target.clone())),
        None => draft.clone(),
    };
    let on = slot.map_or(true, |s| s.enabled);
    let a = if on { 1.0 } else { 0.5 };
    if let Some(s) = cur_src {
        p.rect_filled(swatch, 1.5, src_style(s).color.gamma_multiply(a));
    }
    // Source combo (clamped to its column by a child ui of that width).
    let mut picked_src = None;
    ui.scope_builder(UiBuilder::new().max_rect(src_r), |ui| {
        let shown = cur_src.map_or("Source\u{2026}", |s| names[s as usize]);
        let sel = cur_src.map_or(usize::MAX, |s| s as usize);
        picked_src = combo_stacked_as(ui, "", sel, names, shown);
        if ui.rect_contains_pointer(src_r) {
            set_hover(ui, "SOURCE", shown, None, crate::help::ui_help("matrix:src").unwrap_or(""));
        }
    });
    // Polarity.
    if let Some(s) = slot {
        let r = ui.interact(pol, Id::new(("gts-mx-pol", i)), Sense::click());
        rr_ui::paint::inset_box(&p, pol, r.hovered());
        rr_ui::paint::caps(&p, pol.center(), Align2::CENTER_CENTER, if s.bipolar { "Bi" } else { "Uni" }, 11.0, if s.bipolar { T.accent } else { T.muted });
        if r.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            set_hover(ui, "POLARITY", if s.bipolar { "bipolar" } else { "unipolar" }, None, crate::help::ui_help("matrix:pol").unwrap_or(""));
        }
        if r.clicked() {
            ed.set_bipolar(i, !s.bipolar);
        }
    }
    // Amount.
    {
        let (v, text, col) = match slot {
            Some(s) => ((s.amount + 1.0) / 2.0, format!("{:+.0} %", s.amount * 100.0), src_style(s.source).color),
            None => (0.75, "+50 %".to_string(), T.muted),
        };
        let r = amount_bar(ui, amt, &format!("slot{i}"), v, 0.5, true, col, &text, slot.is_some());
        if let (Some(nv), Some(_)) = (r.changed, slot) {
            ed.set_amount(i, nv * 2.0 - 1.0);
        }
        if r.hovered {
            set_hover(ui, &format!("ROUTING {} AMOUNT", i + 1), &text, None, crate::help::ui_help("matrix:amt").unwrap_or(""));
        }
    }
    // Target.
    let picked_tgt = target_picker(ui, tgt, Id::new(("gts-mx-target", i)), cur_tgt.as_deref(), crate::help::ui_help("matrix:target").unwrap_or(""));
    // Remove.
    if slot.is_some() {
        let r = ui.interact(del, Id::new(("gts-mx-del", i)), Sense::click());
        rr_ui::paint::inset_box(&p, del, r.hovered());
        let (cx, cy) = (del.center().x, del.center().y);
        let col = if r.hovered() { T.text } else { T.muted };
        p.line_segment([pos2(cx - 3.0, cy - 3.0), pos2(cx + 3.0, cy + 3.0)], rr_ui::paint::st(1.5, col));
        p.line_segment([pos2(cx - 3.0, cy + 3.0), pos2(cx + 3.0, cy - 3.0)], rr_ui::paint::st(1.5, col));
        if r.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            set_hover(ui, "REMOVE", "", None, crate::help::ui_help("matrix:remove").unwrap_or(""));
        }
        if r.clicked() {
            ed.remove(i);
        }
    } else if ui.rect_contains_pointer(rect) {
        set_hover(ui, "NEW ROUTING", "", None, crate::help::ui_help("matrix:empty").unwrap_or(""));
    }
    // Apply a pick. An existing slot changes in place (never into a duplicate of another slot's
    // pair); the add row keeps a draft until it has both halves, then adds at +50 %.
    if picked_src.is_none() && picked_tgt.is_none() {
        return;
    }
    let new_src = picked_src.and_then(Source::from_index).or(cur_src);
    let new_tgt = picked_tgt.or(cur_tgt);
    match (slot, new_src, new_tgt) {
        (Some(_), Some(s), Some(t)) => {
            if live_route(s, &t, ed.ctl).is_some() {
                notice(ui.ctx(), "That pair is a built-in route: edit its amount above.");
            } else if mv.matrix.find(s, &t).is_some_and(|j| j != i) {
                notice(ui.ctx(), "That source already moves that target in another routing.");
            } else {
                ed.shared.edit(|m| {
                    let Some(sl) = m.slots.get_mut(i) else { return };
                    if sl.source != s {
                        sl.bipolar = s.bipolar();
                    }
                    (sl.source, sl.target) = (s, t);
                });
            }
        }
        (None, Some(s), Some(t)) => {
            match ed.add_or_edit(s, &t, 0.5) {
                Err(EditError::Full) => notice(ui.ctx(), "The matrix is full: remove a routing first."),
                Ok(Edit::Fixed(_)) => notice(ui.ctx(), "That pair is a built-in route: edit its amount in the built-in rows above."),
                _ => {}
            }
            ui.data_mut(|d| d.remove_temp::<(Option<Source>, Option<String>)>(draft_id()));
        }
        (None, s, t) => ui.data_mut(|d| d.insert_temp(draft_id(), (s, t))),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every target belongs to a named section, and only the limiter's own ids fall to the last.
    #[test]
    fn every_target_has_a_section_and_the_picker_lists_it_once() {
        let mut seen = vec![];
        for id in matrix::target_ids() {
            let s = section_of(id);
            // Track 7 engine wave: Mod C / D, Mod Env and Pitch All have no section yet (UI wave binds it).
            let pending = ["modc_", "modd_", "menv_", "pitch_all", "phs_", "flg_"].iter().any(|p| id.starts_with(p));
            assert!(s < 20 || id.starts_with("lim_") || pending, "{id} fell through to {}", section_name(s));
            seen.push(id);
        }
        let listed: Vec<&str> = pick_rows().iter().filter_map(|r| if let PickRow::Item(id, _) = r { Some(*id) } else { None }).collect();
        assert_eq!(listed.len(), seen.len(), "the picker lists every target");
        assert_eq!(listed.iter().collect::<std::collections::BTreeSet<_>>().len(), listed.len(), "each once");
    }

    /// A source is told apart by colour family and glyph: no two sources share both.
    #[test]
    fn sources_have_distinct_look_within_a_family() {
        for a in Source::ALL {
            for b in Source::ALL {
                if a != b {
                    let (sa, sb) = (src_style(a), src_style(b));
                    assert!(sa.color != sb.color || sa.glyph != sb.glyph || sa.name != sb.name, "{a:?} and {b:?} look the same");
                    if sa.color == sb.color {
                        assert!(sa.glyph != sb.glyph || sa.name != sb.name, "{a:?} / {b:?}: same colour needs another glyph or name");
                    }
                }
            }
        }
        // Chips are 20 px tall and hold a one-character glyph.
        assert!(Source::ALL.iter().all(|s| src_style(*s).glyph.chars().count() == 1));
    }

    #[test]
    fn every_source_is_reachable_from_the_strip_and_every_chip_has_help() {
        let mut all: Vec<Source> = PRIMARY.iter().chain(MORE.iter()).chain(MACROS.iter()).copied().collect();
        all.sort_by_key(|s| *s as usize);
        // Track 7 engine wave: Mod C / Mod D / Mod Env chips are not placed yet (UI wave binds it).
        let placed: Vec<Source> = Source::ALL.into_iter().filter(|s| !matches!(s, Source::ModC | Source::ModD | Source::ModEnv)).collect();
        assert_eq!(all, placed, "PRIMARY + MORE + MACROS cover every source once");
        for s in Source::ALL {
            assert!(crate::help::ui_help(&format!("src:{}", s.name())).is_some(), "no chip help for {}", s.name());
        }
    }

    #[test]
    fn slot_spans_follow_polarity_and_sign() {
        let near = |got: (f32, f32), want: (f32, f32)| assert!((got.0 - want.0).abs() < 1e-6 && (got.1 - want.1).abs() < 1e-6, "{got:?} vs {want:?}");
        near(slot_span(0.5, 0.3, true), (0.2, 0.8));
        near(slot_span(0.5, -0.3, true), (0.2, 0.8));
        near(slot_span(0.5, 0.3, false), (0.5, 0.8));
        near(slot_span(0.5, -0.3, false), (0.2, 0.5));
        near(slot_span(0.9, 0.5, false), (0.9, 1.0)); // clamped to the knob's range
    }

    #[test]
    fn the_built_in_rows_cover_every_route_param_once() {
        let mut routes: Vec<&str> = fixed_routes().iter().map(|r| r.route).collect();
        routes.push("moda_pitch");
        routes.sort_unstable();
        routes.dedup();
        let mut rows: Vec<&str> = FIXED_ROWS.iter().map(|r| r.0).collect();
        rows.sort_unstable();
        assert_eq!(rows, routes);
    }
}
