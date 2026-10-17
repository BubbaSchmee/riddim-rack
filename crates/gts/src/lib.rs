//! GTS, a two-oscillator grain table synth. See SPEC.md at the repo root for the contract.

pub mod bus;
pub mod dsp;
pub mod editor;
pub mod fxview;
pub mod help;
pub mod matrix;
pub mod params;
pub mod preset;
pub mod viz;

use nih_plug::prelude::*;
use std::any::Any;
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use params::GtsParams;

/// Samples between CV-in re-applications inside a block.
const CV_CHUNK: usize = 32;

pub struct Gts {
    params: Arc<GtsParams>,
    engine: dsp::Engine,
    /// Limiter gain reduction and output peak since the editor last looked.
    meters: Arc<editor::Meters>,
    /// Scope ring + per-block voice / modulation record for the editor (MOTION.md section 8).
    viz: viz::VizWriter,
    viz_shared: Arc<viz::Viz>,
    scope: rr_ui::ring::RingReader,
    /// Last tempo written to the log (a ramp logs every 0.5 BPM, not every block).
    logged_tempo: f64,
    /// RR-BUS publisher + CV-in slots.
    bus: bus::DeviceBus,
    sample_rate: f32,
    /// Last oversampling value read from the persisted setting.
    oversampling: usize,
    /// Matrix snapshot version applied last, and the record buffer it is read into.
    matrix_seen: u32,
    matrix_rec: [f32; matrix::ENC_LEN],
}

/// Writes one engine frame to the plugin's output channels: channel 0 = left, the rest = right.
#[doc(hidden)]
pub fn write_frame(mut frame: nih_plug::buffer::ChannelSamples, (l, r): (f32, f32)) {
    for (ch, sample) in frame.iter_mut().enumerate() {
        *sample = if ch == 0 { l } else { r };
    }
}

impl Default for Gts {
    fn default() -> Self {
        let (viz, viz_shared, scope) = viz::VizWriter::new();
        Self {
            params: Arc::new(GtsParams::default()),
            engine: dsp::Engine::new(),
            meters: Arc::new(editor::Meters::default()),
            viz,
            viz_shared,
            scope,
            logged_tempo: dsp::Transport::default().tempo,
            bus: bus::DeviceBus::default(),
            sample_rate: 44100.0,
            oversampling: 2,
            matrix_seen: u32::MAX,
            matrix_rec: [0.0; matrix::ENC_LEN],
        }
    }
}

impl Plugin for Gts {
    const NAME: &'static str = "GTS";
    const VENDOR: &'static str = "Riddim Rack Open Source";
    const URL: &'static str = "https://github.com/BubbaSchmee/riddim-rack";
    const EMAIL: &'static str = "https://github.com/BubbaSchmee/riddim-rack/issues";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    // SPEC needs pitch bend and CC1; those only arrive with MidiCCs (Basic has neither).
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = Task;

    fn task_executor(&mut self) -> TaskExecutor<Self> {
        let (link, params) = (self.bus.link.clone(), self.params.clone());
        Box::new(move |task| match task {
            Task::BusRefresh => link.refresh(&params),
            // Still the host's value (a GUI edit or a state load may have moved on since).
            Task::MatrixRev(rev) => {
                if params.matrix.rev.value() == rev {
                    params.matrix.restore(rev);
                }
            }
        })
    }

    fn params(&self) -> Arc<dyn Params> {
        Arc::new(HostParams(self.params.clone()))
    }

    /// nih-plug restores only the fields a state contains, so a pre-matrix state (or one whose
    /// matrix does not parse) would keep the instance's current matrix and macros. Such states
    /// load an empty matrix and zero macros instead (SPEC v2 Track 5: older states sound identical).
    ///
    /// A matrix that parses as JSON but not as a whole (a newer version's source name in one
    /// slot) keeps every slot that parses and the macro labels.
    fn filter_state(state: &mut PluginState) {
        let ok = state.fields.get("matrix").is_some_and(|s| serde_json::from_str::<matrix::Matrix>(s).is_ok());
        if !ok {
            let v: serde_json::Value = state.fields.get("matrix").and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();
            let mut m = matrix::Matrix::default();
            if let Some(slots) = v.get("slots").and_then(|s| s.as_array()) {
                m.slots = slots.iter().filter_map(|s| serde_json::from_value(s.clone()).ok()).collect();
            }
            if let Some(labels) = v.get("macros").and_then(|l| serde_json::from_value(l.clone()).ok()) {
                m.macros = labels;
            }
            state.fields.insert("matrix".into(), serde_json::to_string(&m).expect("a matrix serialises"));
        }
        for k in 1..=matrix::MACROS {
            state.params.entry(format!("macro{k}")).or_insert(nih_plug::wrapper::state::ParamValue::F32(0.0));
        }
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let inner = editor::create(
            self.params.clone(),
            editor::Feeds { meters: self.meters.clone(), viz: self.viz_shared.clone(), scope: self.scope.clone() },
            self.bus.link.clone(),
            self.engine.tables_handle(),
        )?;
        Some(Box::new(MatrixHostHook { inner, params: self.params.clone() }))
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        nih_log!(
            "initialize: sample_rate={} max_buffer_size={}",
            buffer_config.sample_rate,
            buffer_config.max_buffer_size
        );
        if let Some(t) = self.engine.initialize(buffer_config.sample_rate) {
            nih_log!("initialize: grain tables generated in {:.2} ms", t.as_secs_f64() * 1000.0);
        }
        self.engine.log_notes = std::env::var_os("NIH_LOG").is_some();
        // Quality setting (host state, SPEC v2 1.2) and the latency it implies.
        self.oversampling = self.params.oversampling.load(Ordering::Relaxed);
        self.engine.set_oversampling(self.oversampling);
        context.set_latency_samples(self.engine.latency() as u32);
        nih_log!("initialize: oversampling {}x, reported latency {} samples", self.engine.oversampling(), self.engine.latency());
        self.sample_rate = buffer_config.sample_rate;
        // The bus mapping is opened here, never in process.
        self.bus.connect(&self.params, rr_bus::DEFAULT_NAME);
        // Debug builds only: GTS_DEBUG_FULL_MATRIX=1 fills all 32 slots (every source, voice and
        // global targets) so a pluginval run exercises the matrix under assert_process_allocs.
        #[cfg(debug_assertions)]
        if std::env::var_os("GTS_DEBUG_FULL_MATRIX").is_some() {
            let slots = matrix::target_ids()
                .step_by(4)
                .zip(matrix::Source::ALL.iter().cycle())
                .take(matrix::MAX_SLOTS)
                .enumerate()
                .map(|(k, (t, &s))| matrix::Slot::new(s, t, if k % 2 == 0 { 0.3 } else { -0.2 }))
                .collect();
            self.params.matrix.replace(matrix::Matrix { slots, ..Default::default() });
            nih_log!("initialize: GTS_DEBUG_FULL_MATRIX, {} slots", self.params.matrix.get().slots.len());
        }
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
        // A host that paused process() past the bus timeout lost its record. Rare, and reset
        // may run under the audio-thread allocation guard, hence permit_alloc.
        if self.bus.needs_register() {
            util::permit_alloc(|| self.bus.ensure_registered(&self.params));
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let t = context.transport();
        let transport = dsp::Transport {
            tempo: t.tempo.unwrap_or(120.0),
            playing: t.playing,
            pos_beats: t.pos_beats().unwrap_or(0.0),
        };
        let prev = self.engine.transport;
        // Rare events; logging formats a String, hence permit_alloc.
        if transport.playing != prev.playing {
            util::permit_alloc(|| nih_log!("transport: {}", if transport.playing { "play" } else { "stop" }));
        }
        if (transport.tempo - self.logged_tempo).abs() > 0.5 {
            self.logged_tempo = transport.tempo;
            util::permit_alloc(|| nih_log!("transport: tempo {:.2} bpm", transport.tempo));
        }
        self.engine.transport = transport;
        // Quality change from the editor: applied between blocks, never inside note processing.
        let os = self.params.oversampling.load(Ordering::Relaxed);
        if os != self.oversampling {
            self.oversampling = os;
            let latency = self.engine.latency();
            self.engine.set_oversampling(os);
            if self.engine.latency() != latency {
                context.set_latency_samples(self.engine.latency() as u32);
            }
        }
        // Host undo / redo of a matrix edit: the snapshot is restored off the audio thread.
        if let Some(rev) = self.params.matrix.host_moved(self.params.matrix.rev.value()) {
            context.execute_background(Task::MatrixRev(rev));
        }
        // Matrix edits from the editor, a preset or a state load: lock-free snapshot, read once per block.
        if self.params.matrix.poll(&mut self.matrix_seen, &mut self.matrix_rec) {
            self.engine.set_matrix(&self.matrix_rec);
        }
        let n = buffer.samples();
        self.bus.exchange(self.engine.bus_signals());
        if self.bus.refresh_due(n, self.sample_rate) {
            // Rate-limited here: a reclaimed-and-reused record otherwise only recovers on reset().
            if self.bus.needs_register() {
                util::permit_alloc(|| self.bus.ensure_registered(&self.params));
            }
            context.execute_background(Task::BusRefresh);
        }
        // CV targets are re-applied every CV_CHUNK samples so the 5 ms smoothing reaches the
        // cutoff/index inside large blocks instead of stepping at block edges. With the matrix
        // on, the CV-ins also advance per chunk (they are matrix sources too).
        let cv_on = self.params.cv_slots().iter().any(|(t, _)| *t != params::CvTarget::Off);
        let chunked = cv_on || self.engine.matrix_on();
        let base = dsp::Patch::from_params(&self.params);
        let mut patch = base;
        let first = if chunked { n.min(CV_CHUNK) } else { n };
        let cv = self.bus.advance(first, self.sample_rate);
        self.engine.set_bus_cv(cv);
        bus::apply_cv(&mut patch, &self.params, cv);
        self.engine.set_patch(&patch);

        let mut next_event = context.next_event();
        let mut peak = [0.0f32; 2];
        for (i, frame) in buffer.iter_samples().enumerate() {
            if chunked && i > 0 && i % CV_CHUNK == 0 {
                let cv = self.bus.advance((n - i).min(CV_CHUNK), self.sample_rate);
                self.engine.set_bus_cv(cv);
                if cv_on {
                    let mut patch = base;
                    bus::apply_cv(&mut patch, &self.params, cv);
                    self.engine.set_cv_patch(&patch);
                }
            }
            while let Some(event) = next_event {
                if event.timing() > i as u32 {
                    break;
                }
                self.engine.handle_event(&event);
                if let NoteEvent::NoteOn { note, velocity, .. } = event {
                    if self.engine.log_notes {
                        let voices = self.engine.active_voices();
                        util::permit_alloc(|| {
                            nih_log!("note-on: note={} velocity={:.2} voices={}", note, velocity, voices)
                        });
                    }
                }
                next_event = context.next_event();
            }

            let (l, r) = self.engine.next_frame();
            peak = [peak[0].max(l.abs()), peak[1].max(r.abs())];
            self.viz.push(l, r);
            write_frame(frame, (l, r));
        }

        // Non-negative f32 bits order like the values, so fetch_max keeps the peak until the editor swaps it out.
        self.meters.gr_db.fetch_max(self.engine.limiter_gr_db().to_bits(), Ordering::Relaxed);
        for (m, p) in self.meters.peak.iter().zip(peak) {
            m.fetch_max(p.to_bits(), Ordering::Relaxed);
        }
        let fx = self.engine.fx_telemetry();
        self.viz.publish(&self.engine, &self.params, self.engine.patch(), self.sample_rate, &fx);
        ProcessStatus::Normal
    }
}

/// Background work requested from `process`.
pub enum Task {
    /// Refresh the bus bindings (see bus::BusLink::refresh).
    BusRefresh,
    /// The host moved `matrix_rev` to this rev (its undo / redo): restore that matrix.
    MatrixRev(i32),
}

/// What the host sees: every `GtsParams` param plus `matrix_rev` ([`matrix::MatrixShared::rev`]),
/// kept out of `GtsParams::param_map` so the editor, help and preset tables stay sound params only.
struct HostParams(Arc<GtsParams>);

// SAFETY: the pointers point into the `GtsParams` this keeps alive, as the derived impl's do.
unsafe impl Params for HostParams {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        let mut v = self.0.param_map();
        v.push(("matrix_rev".into(), self.0.matrix.rev.as_ptr(), String::new()));
        v
    }

    fn serialize_fields(&self) -> BTreeMap<String, String> {
        self.0.serialize_fields()
    }

    fn deserialize_fields(&self, serialized: &BTreeMap<String, String>) {
        self.0.deserialize_fields(serialized)
    }
}

/// The editor, plus its host context handed to the matrix while it is open (GUI matrix edits
/// move `matrix_rev` through it).
struct MatrixHostHook {
    inner: Box<dyn Editor>,
    params: Arc<GtsParams>,
}

/// Detaches the context when the editor closes (no plugin -> context -> plugin cycle).
struct Detach(#[allow(dead_code)] Box<dyn Any + Send>, Arc<GtsParams>);

impl Drop for Detach {
    fn drop(&mut self) {
        self.1.matrix.attach(None);
    }
}

impl Editor for MatrixHostHook {
    fn spawn(&self, parent: ParentWindowHandle, context: Arc<dyn GuiContext>) -> Box<dyn Any + Send> {
        self.params.matrix.attach(Some(context.clone()));
        Box::new(Detach(self.inner.spawn(parent, context), self.params.clone()))
    }
    fn size(&self) -> (u32, u32) {
        self.inner.size()
    }
    fn set_scale_factor(&self, factor: f32) -> bool {
        self.inner.set_scale_factor(factor)
    }
    fn param_value_changed(&self, id: &str, normalized_value: f32) {
        self.inner.param_value_changed(id, normalized_value)
    }
    fn param_modulation_changed(&self, id: &str, modulation_offset: f32) {
        self.inner.param_modulation_changed(id, modulation_offset)
    }
    fn param_values_changed(&self) {
        self.inner.param_values_changed()
    }
}

impl ClapPlugin for Gts {
    const CLAP_ID: &'static str = "com.github.bubbaschmee.riddim-rack.gts";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A two-oscillator grain table synthesizer");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::Instrument, ClapFeature::Synthesizer, ClapFeature::Stereo];
}

impl Vst3Plugin for Gts {
    const VST3_CLASS_ID: [u8; 16] = *b"RiddimRackGTS!!!";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(Gts);
nih_export_vst3!(Gts);
