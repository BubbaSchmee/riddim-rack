//! Renders the GTS-like demo panel (1000x760) or, with `vss`, the VSS-like panel (1064x560) in
//! an eframe window, saves a screenshot to `docs/design/` on the first stable frame and exits.
//! Pass `live` to keep the window open and play with the widgets.
//!
//!   cargo run -p rr-ui --example preview            -> docs/design/preview-rr-ui.png
//!   cargo run -p rr-ui --example preview -- vss     -> docs/design/preview-rr-ui-vss.png

use eframe::egui::{self, vec2, Context, Ui, ViewportCommand};
use rr_ui::theme::{metrics::*, T};
use rr_ui::widgets::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// PATCH column width and the VSS window width it needs (760 + column + gap).
const PATCH_W_VSS: f32 = 292.0;
const VSS_W: f32 = 760.0 + PATCH_W_VSS + 12.0;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Gts,
    Vss,
}

struct Demo {
    mode: Mode,
    live: bool,
    frame: u32,
    asked: bool,
    st: RefCell<State>,
}

/// Widget state, behind a RefCell so header and body closures can both use it.
struct State {
    vals: HashMap<String, f32>,
    on: HashMap<String, bool>,
    sel: HashMap<String, usize>,
    steps: f32,
    cells: [u8; 16],
    patch_label: String,
    patch_src: [Option<usize>; 3],
    patch_tgt: [usize; 3],
    patch_amt: [f32; 3],
}

fn additive(duty: f32, n: usize, amp: f32) -> Vec<f32> {
    let ys: Vec<f32> = (0..=n)
        .map(|i| {
            let x = i as f32 / n as f32;
            (1..=24).map(|h| {
                let h = h as f32;
                (2.0 / (h * std::f32::consts::PI)) * (h * std::f32::consts::PI * duty).sin() * (std::f32::consts::TAU * h * (x - duty / 2.0)).cos()
            }).sum()
        })
        .collect();
    let mx = ys.iter().fold(0.0f32, |m, y| m.max(y.abs()));
    ys.iter().map(|y| y / mx * amp).collect()
}

impl Demo {
    fn new(mode: Mode, live: bool) -> Self {
        Self {
            mode,
            live,
            frame: 0,
            asked: false,
            st: RefCell::new(State {
                vals: HashMap::new(),
                on: HashMap::new(),
                sel: HashMap::new(),
                steps: 12.0 / 16.0,
                cells: [0, 0, 3, 3, 1, 1, 2, 5, 4, 4, 2, 0, 3, 3, 1, 5],
                patch_label: "Lead bass".into(),
                patch_src: [Some(1), Some(0), None],
                patch_tgt: [0, 1, 2],
                patch_amt: [0.78, 0.35, 0.5],
            }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn kn(&self, ui: &mut Ui, key: &str, label: &str, init: f32, def: f32, style: KnobStyle, fmt: &dyn Fn(f32) -> String) {
        let v = *self.st.borrow_mut().vals.entry(key.to_string()).or_insert(init);
        if let Some(n) = knob_styled(ui, label, v, def, &fmt(v), style).changed {
            self.st.borrow_mut().vals.insert(key.to_string(), n);
        }
    }
    fn tg(&self, ui: &mut Ui, key: &str, label: &str, init: bool) {
        let v = *self.st.borrow_mut().on.entry(key.to_string()).or_insert(init);
        if let Some(n) = led_toggle(ui, label, v) {
            self.st.borrow_mut().on.insert(key.to_string(), n);
        }
    }
    fn cb(&self, ui: &mut Ui, key: &str, label: &str, items: &[&str], init: usize) {
        let v = *self.st.borrow_mut().sel.entry(key.to_string()).or_insert(init);
        if let Some(n) = combo(ui, label, v, items) {
            self.st.borrow_mut().sel.insert(key.to_string(), n);
        }
    }

    /// PATCH strip with demo data: row 2 follows a live source, row 3 a lost one, row 4 none.
    fn patch(&self, ui: &mut Ui, targets: &[&str]) {
        let sources: Vec<String> = ["GTS 1 · Mod A", "VSS · Step Gate", "GTS 2 · Mod B"].iter().map(|s| s.to_string()).collect();
        let mut label = self.st.borrow().patch_label.clone();
        let (src, tgt, amt) = {
            let s = self.st.borrow();
            (s.patch_src, s.patch_tgt, s.patch_amt)
        };
        let act = [0.92, 0.35, 0.0];
        let lost = [None, None, Some("GTS 3 · Mod A")];
        let mut rows: Vec<PatchRow> = (0..3)
            .map(|i| PatchRow { sources: &sources, source_idx: src[i], lost_name: lost[i], targets, target_idx: tgt[i], amount01: amt[i], activity01: act[i] })
            .collect();
        let r = patch_strip(ui, &mut label, &mut rows);
        let mut s = self.st.borrow_mut();
        s.patch_label = label;
        for (i, e) in r.rows.iter().enumerate() {
            if let Some(v) = e.source_changed {
                s.patch_src[i] = v;
            }
            if let Some(v) = e.target_changed {
                s.patch_tgt[i] = v;
            }
            if let Some(v) = e.amount.changed {
                s.patch_amt[i] = v;
            }
        }
    }

    // --- GTS -----------------------------------------------------------------------------

    fn gts(&self, hui: &mut Ui, bui: &mut Ui) {
        let sig = KnobStyle::default();
        let bip = KnobStyle::default().bipolar();
        let int = |lo: f32, hi: f32| move |v: f32| format!("{}", (lo + v * (hi - lo)).round() as i32);
        let sgn = |lo: f32, hi: f32| move |v: f32| format!("{:+.2}", lo + v * (hi - lo));
        let hz = |v: f32| format!("{:.1} Hz", 0.1 * 400f32.powf(v));
        let ms = |v: f32| format!("{:.0} ms", 1.0 + 4000.0 * v * v);
        let two = |v: f32| format!("{v:.2}");

        // Header: display
        let wave = additive(0.27, 240, 1.0);
        let g1 = additive(0.42, 240, 0.9);
        let g2 = additive(0.12, 240, 0.9);
        hui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            let pw = 300.0;
            ui.allocate_ui(vec2(ui.available_width() - pw - GAP, DISPLAY_H), |ui| {
                waveform_display_ex(ui, &wave, &[&g1, &g2], 64.0 / 127.0, "Osc A  /  Square Morph", "Index 64");
            });
            ui.allocate_ui(vec2(pw, PATCH_H), |ui| self.patch(ui, &["Osc A Index", "Osc B Index", "Filter A Freq", "Filter B Freq", "Mod A Rate", "Volume"]));
        });

        bui.spacing_mut().item_spacing = vec2(GAP, GAP);
        let (h1, h2, h3, h4) = (184.0, 141.0, 111.0, 111.0);
        let tables = ["Square Morph", "Saw Stack", "Pulse Sweep", "Formant"];

        // Row 1: oscillators
        bui.horizontal_top(|ui| {
            for (n, key_on, tbl, on) in [("Osc A", "oa_on", "oa_t", true), ("Osc B", "ob_on", "ob_t", false)] {
                let id = if n == "Osc A" { "oa" } else { "ob" };
                let k = |s: &str| format!("{id}_{s}");
                let me = self;
                group_ext(ui, vec2(450.0, h1), n, |ui| {
                    me.tg(ui, key_on, "", on);
                    me.cb(ui, tbl, "", &tables, if on { 0 } else { 1 });
                }, |ui| {
                    knob_row(ui, |ui| {
                        me.kn(ui, &k("oct"), "Oct", 0.5, 0.5, bip, &int(-4.0, 4.0));
                        me.kn(ui, &k("semi"), "Semi", 0.5, 0.5, bip, &int(-12.0, 12.0));
                        me.kn(ui, &k("cent"), "Cent", 0.5, 0.5, bip, &int(-50.0, 50.0));
                        me.kn(ui, &k("idx"), "Index", 64.0 / 127.0, 0.0, sig, &int(0.0, 127.0));
                        me.kn(ui, &k("mot"), "Motion", 0.5, 0.5, bip, &sgn(-1.0, 1.0));
                        me.kn(ui, &k("sh"), "Shift", 0.5, 0.5, bip, &|v| format!("{:+} st", (v * 48.0 - 24.0).round() as i32).replace("+0", "0"));
                        me.kn(ui, &k("vol"), "Vol", 0.8, 0.8, sig, &two);
                    });
                    knob_row(ui, |ui| {
                        me.kn(ui, &k("a"), "Att", 0.05, 0.0, sig, &ms);
                        me.kn(ui, &k("d"), "Dec", 0.38, 0.3, sig, &ms);
                        me.kn(ui, &k("s"), "Sus", 1.0, 1.0, sig, &two);
                        me.kn(ui, &k("r"), "Rel", 0.12, 0.1, sig, &ms);
                    });
                });
            }
        });

        // Row 2: modulators + shaper
        bui.horizontal_top(|ui| {
            let me = self;
            group_ext(ui, vec2(316.0, h2), "Mod A", |ui| me.cb(ui, "ma_c", "", &["Decay", "Sine", "Ramp"], 0), |ui| {
                strip(ui, |ui| {
                    me.tg(ui, "ma_sync", "Sync", true);
                    me.cb(ui, "ma_div", "", &["1/8", "1/4", "1/16"], 0);
                    me.tg(ui, "ma_1s", "1-shot", true);
                    me.cb(ui, "ma_tg", "", &["Both", "A", "B"], 0);
                });
                knob_row(ui, |ui| {
                    me.kn(ui, "ma_rate", "Rate", 0.5, 0.5, sig, &hz);
                    me.kn(ui, "ma_pitch", "Pitch", 0.5, 0.5, bip, &sgn(-1.0, 1.0));
                    me.kn(ui, "ma_idx", "Index", 0.75, 0.5, bip, &sgn(-1.0, 1.0));
                    me.kn(ui, "ma_sh", "Shift", 0.5, 0.5, bip, &sgn(-1.0, 1.0));
                });
            });
            group_ext(ui, vec2(356.0, h2), "Mod B", |ui| me.cb(ui, "mb_c", "", &["Sine", "Decay", "Ramp"], 0), |ui| {
                strip(ui, |ui| {
                    me.tg(ui, "mb_sync", "Sync", false);
                    me.cb(ui, "mb_div", "", &["1/8", "1/4", "1/16"], 0);
                    me.tg(ui, "mb_1s", "1-shot", false);
                    me.cb(ui, "mb_tg", "", &["Both", "A", "B"], 0);
                });
                knob_row(ui, |ui| {
                    let c = KnobStyle::default().cell(52.0);
                    me.kn(ui, "mb_rate", "Rate", 0.5, 0.5, c, &hz);
                    for (k, l) in [("mb_mot", "Motion"), ("mb_vol", "Vol"), ("mb_cut", "Cutoff"), ("mb_ma", "Mod A")] {
                        me.kn(ui, k, l, 0.5, 0.5, c.bipolar(), &sgn(-1.0, 1.0));
                    }
                });
            });
            group_ext(ui, vec2(216.0, h2), "Shaper", |ui| me.tg(ui, "sh_on", "", false), |ui| {
                strip(ui, |ui| me.cb(ui, "sh_mode", "", &["Saturate", "Fold", "Quantize"], 0));
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    me.kn(ui, "sh_amt", "Amt", 0.3, 0.0, sig, &two);
                    let curve: Vec<f32> = (0..=32).map(|i| { let x = i as f32 / 32.0; ((x * 3.0 - 1.5).tanh() / 1.5f32.tanh() * 0.5 + 0.5).clamp(0.0, 1.0) }).collect();
                    ui.add_space(0.0);
                    ui.vertical(|ui| {
                        ui.add_space(KNOB_LABEL_H + (KNOB_D - 48.0) / 2.0);
                        curve_preview(ui, vec2(64.0, 48.0), &curve);
                    });
                });
            });
        });

        // Row 3: filters
        bui.horizontal_top(|ui| {
            let me = self;
            for (n, kt, ke, kf, kr, kk, on) in [("Filter A", "fa_t", "fa_e", "fa_f", "fa_r", "fa_k", true), ("Filter B", "fb_t", "fb_e", "fb_f", "fb_r", "fb_k", true)] {
                group_ext(ui, vec2(300.0, h3), n, |ui| me.tg(ui, ke, "Env", on), |ui| {
                    knob_row(ui, |ui| {
                        let (ty, f, r) = if n == "Filter A" { (4usize, 0.62, 0.6) } else { (0, 0.72, 0.1) };
                        me.cb(ui, kt, "Type", &["LP12", "LP24", "HP12", "BP", "Comb -", "Comb +", "AM"], ty);
                        me.kn(ui, kf, "Freq", f, 0.5, sig, &|v| { let hz = 20.0 * 1000f32.powf(v); if hz >= 1000.0 { format!("{:.1} kHz", hz / 1000.0) } else { format!("{hz:.0} Hz") } });
                        me.kn(ui, kr, "Res", r, 0.0, sig, &two);
                        me.kn(ui, kk, "Kbd", 0.5, 0.0, sig, &two);
                    });
                });
            }
            group_ext(ui, vec2(288.0, h3), "Filter Env", |ui| me.tg(ui, "fe_inv", "Inv", false), |ui| {
                knob_row(ui, |ui| {
                    let c = KnobStyle::default().cell(52.0);
                    me.kn(ui, "fe_a", "Att", 0.05, 0.0, c, &ms);
                    me.kn(ui, "fe_d", "Dec", 0.35, 0.3, c, &ms);
                    me.kn(ui, "fe_s", "Sus", 0.2, 0.5, c, &two);
                    me.kn(ui, "fe_r", "Rel", 0.16, 0.1, c, &ms);
                    me.kn(ui, "fe_amt", "Amt", 0.5, 0.0, c, &two);
                });
            });
        });

        // Row 4: voice, wheel, velocity, output
        bui.horizontal_top(|ui| {
            let me = self;
            let c = KnobStyle::default().cell(44.0);
            let cv = KnobStyle::default().cell(50.0);
            group_ext(ui, vec2(258.0, h4), "Voice", |ui| me.tg(ui, "v_leg", "Legato", true), |ui| {
                knob_row(ui, |ui| {
                    me.cb(ui, "v_route", "Routing", &["Parallel", "Series", "Split"], 0);
                    me.kn(ui, "v_poly", "Poly", 0.0, 0.0, cv, &|v| format!("{}", 1 + (v * 7.0).round() as i32));
                    me.kn(ui, "v_porta", "Porta", 0.04, 0.0, cv, &|v| format!("{:.0} ms", v * 2000.0));
                    me.kn(ui, "v_bend", "Bend", 0.5, 0.5, cv, &|v| format!("{} st", (v * 24.0).round() as i32));
                });
            });
            group_ext(ui, vec2(200.0, h4), "Wheel", |_| {}, |ui| {
                knob_row(ui, |ui| {
                    for (k, l) in [("w_idx", "Index"), ("w_sh", "Shift"), ("w_cut", "Cutoff"), ("w_mod", "Mod")] {
                        me.kn(ui, k, l, 0.5, 0.5, c.bipolar(), &two);
                    }
                });
            });
            group_ext(ui, vec2(290.0, h4), "Velocity", |_| {}, |ui| {
                knob_row(ui, |ui| {
                    for (k, l) in [("l_a", "Vol A"), ("l_b", "Vol B"), ("l_fe", "F.Env"), ("l_att", "Att"), ("l_sh", "Shift"), ("l_mod", "Mod")] {
                        me.kn(ui, k, l, 0.5, 0.5, c.bipolar(), &two);
                    }
                });
            });
            group_ext(ui, vec2(128.0, h4), "Output", |_| {}, |ui| {
                knob_row(ui, |ui| {
                    me.kn(ui, "o_vol", "Volume", 0.6, 0.6, KnobStyle::default().cell(52.0), &|v| format!("{:.1} dB", -60.0 + v * 60.0));
                    me.kn(ui, "o_spread", "Spread", 0.0, 0.0, KnobStyle::default().cell(52.0), &two);
                });
            });
        });
    }

    // --- VSS -----------------------------------------------------------------------------

    fn vss(&self, hui: &mut Ui, bui: &mut Ui) {
        let lg = KnobStyle::default().size(KNOB_D_LG).cell(50.0);
        let two = |v: f32| format!("{v:.2}");
        let me = self;
        hui.spacing_mut().item_spacing = vec2(GAP, GAP);
        hui.horizontal_top(|ui| {
            group_ext(ui, vec2(376.0, 124.0), "Voice", |_| {}, |ui| {
                knob_row(ui, |ui| {
                    me.kn(ui, "formant", "Formant", 0.5, 0.5, lg.bipolar(), &|v| format!("{:+.1} st", (v - 0.5) * 24.0));
                    me.kn(ui, "reson", "Reson.", 0.45, 0.5, lg, &two);
                    me.kn(ui, "drive", "Drive", 0.2, 0.0, lg, &two);
                    me.kn(ui, "glide", "Glide", 0.15, 0.0, lg, &|v| format!("{:.0} ms", v * 120.0));
                    me.kn(ui, "swing", "Swing", 0.3, 0.0, lg, &two);
                    me.kn(ui, "mix", "Mix", 1.0, 1.0, lg, &|v| format!("{:.0} %", v * 100.0));
                    me.kn(ui, "out", "Output", 0.8, 0.8, lg, &|v| format!("{:.1} dB", -24.0 + v * 24.0));
                });
            });
            group_ext(ui, vec2(100.0, 124.0), "Sequence", |_| {}, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                {
                    let cur = me.st.borrow().sel.get("rate").copied().unwrap_or(3);
                    if let Some(n) = combo_stacked(ui, "Rate", cur, &["1/4", "1/8", "1/8T", "1/16", "1/16T"]) {
                        me.st.borrow_mut().sel.insert("rate".into(), n);
                    }
                }
                let steps = me.st.borrow().steps;
                let r = hslider_resp(ui, 76.0, "Steps", steps, &format!("{}", 1 + (steps * 15.0).round() as i32));
                if let Some(v) = r.changed {
                    me.st.borrow_mut().steps = (v * 15.0).round() / 15.0;
                }
            });
            ui.allocate_ui(vec2(PATCH_W_VSS, PATCH_H), |ui| me.patch(ui, &["Formant", "Reson.", "Drive", "Glide", "Swing", "Mix"]));
        });

        let steps = 1 + (me.st.borrow().steps * 15.0).round() as usize;
        let rows = ["A", "E", "I", "O", "U", "Hold"];
        let cur = 5usize;
        let body_h = bui.max_rect().height();
        let wide = bui.max_rect().width();
        let cells = me.st.borrow().cells;
        group_ext(bui, vec2(wide, body_h), "Vowel pattern", |ui| caption(ui, &format!("Step {} \u{b7} {}", cur + 1, rows[cells[cur] as usize])), |ui| {
            let h = body_h - GROUP_CHROME_H;
            let hold = Some(5);
            if let Some((c, r)) = step_grid_ex(ui, &rows, 16, steps, Some(cur), &cells, hold, Some(h)) {
                me.st.borrow_mut().cells[c] = r as u8;
            }
        });
    }
}

impl eframe::App for Demo {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let (w, h, name) = match self.mode {
            Mode::Gts => (1000.0, 760.0, "preview-rr-ui.png"),
            Mode::Vss => (VSS_W, 560.0, "preview-rr-ui-vss.png"),
        };
        if self.frame == 0 {
            // Render at 1 px per point regardless of OS scaling so the PNG is 1:1.
            let native = ctx.native_pixels_per_point().unwrap_or(1.0);
            ctx.set_pixels_per_point(1.0);
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(w, h) / native));
        }
        self.frame += 1;

        let mode = self.mode;
        let dispatch = |me: &Self, ui: &mut Ui| {
            let plate = if mode == Mode::Gts { vec2(232.0, 124.0) } else { vec2(168.0, 124.0) };
            let (name, desc, status) = if mode == Mode::Gts {
                ("GTS", "Grain Table Synth", "Two-osc grain table \u{b7} v1.0")
            } else {
                ("VSS", "Vowel Step Sequencer", "16-step formant \u{b7} v1.0")
            };
            rr_ui::panel::rack_panel(ui, name, desc, status, plate, |hui, bui| {
                if mode == Mode::Gts { me.gts(hui, bui) } else { me.vss(hui, bui) }
            });
        };
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| dispatch(&*self, ui));

        let size = ctx.screen_rect().size();
        let ready = (size.x - w).abs() < 1.0 && (size.y - h).abs() < 1.0 && self.frame >= 3;
        if self.live {
            return;
        }
        if ready && !self.asked {
            self.asked = true;
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
        }
        if self.frame > 120 && !self.asked {
            eprintln!("window never reached {w}x{h} (is {size:?}); capturing anyway");
            self.asked = true;
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
        }
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/design").join(name);
            let file = std::io::BufWriter::new(std::fs::File::create(&path).expect("create png"));
            let mut enc = png::Encoder::new(file, img.size[0] as u32, img.size[1] as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(img.as_raw()).unwrap();
            println!("wrote {} ({}x{})", path.display(), img.size[0], img.size[1]);
            ctx.send_viewport_cmd(ViewportCommand::Close);
        } else {
            ctx.request_repaint();
        }
    }
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    let mode = if args.iter().any(|a| a == "vss") { Mode::Vss } else { Mode::Gts };
    let live = args.iter().any(|a| a == "live");
    let (w, h) = if mode == Mode::Gts { (1000.0, 760.0) } else { (VSS_W, 560.0) };
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([w, h]).with_resizable(false).with_title("rr-ui preview"),
        ..Default::default()
    };
    let _ = T; // tokens are exercised through the widgets
    eframe::run_native("rr-ui preview", opts, Box::new(move |cc| {
        // Fonts apply on the next pass, so install them at creation, as a plugin editor would.
        rr_ui::theme::apply(&cc.egui_ctx);
        Ok(Box::new(Demo::new(mode, live)))
    }))
}
