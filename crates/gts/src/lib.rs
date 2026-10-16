//! GTS, a two-oscillator graintable synth. See SPEC.md at the repo root for the contract.

pub mod bus;
pub mod dsp;
pub mod editor;
pub mod params;
pub mod preset;

use nih_plug::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use params::GtsParams;

/// Samples between CV-in re-applications inside a block.
const CV_CHUNK: usize = 32;

pub struct Gts {
    params: Arc<GtsParams>,
    engine: dsp::Engine,
    /// Osc A's current grain index (0..127), for the GUI readout.
    current_index: Arc<AtomicU32>,
    /// Limiter gain reduction and output peak since the editor last looked.
    meters: Arc<editor::Meters>,
    /// Last tempo written to the log (a ramp logs every 0.5 BPM, not every block).
    logged_tempo: f64,
    /// RR-BUS publisher + CV-in slots.
    bus: bus::DeviceBus,
    sample_rate: f32,
    /// Last oversampling value read from the persisted setting.
    oversampling: usize,
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
        Self {
            params: Arc::new(GtsParams::default()),
            engine: dsp::Engine::new(),
            current_index: Arc::new(AtomicU32::new(0)),
            meters: Arc::new(editor::Meters::default()),
            logged_tempo: dsp::Transport::default().tempo,
            bus: bus::DeviceBus::default(),
            sample_rate: 44100.0,
            oversampling: 2,
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
    type BackgroundTask = ();

    fn task_executor(&mut self) -> TaskExecutor<Self> {
        // The one background task: refresh the bus bindings (see bus::BusLink::refresh).
        let (link, params) = (self.bus.link.clone(), self.params.clone());
        Box::new(move |()| link.refresh(&params))
    }

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.current_index.clone(), self.meters.clone(), self.bus.link.clone(), self.engine.tables_handle())
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
            nih_log!("initialize: graintables generated in {:.2} ms", t.as_secs_f64() * 1000.0);
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
        let n = buffer.samples();
        self.bus.exchange(self.engine.bus_signals());
        if self.bus.refresh_due(n, self.sample_rate) {
            // Rate-limited here: a reclaimed-and-reused record otherwise only recovers on reset().
            if self.bus.needs_register() {
                util::permit_alloc(|| self.bus.ensure_registered(&self.params));
            }
            context.execute_background(());
        }
        // CV targets are re-applied every CV_CHUNK samples so the 5 ms smoothing reaches the
        // cutoff/index inside large blocks instead of stepping at block edges.
        let cv_on = self.params.cv_slots().iter().any(|(t, _)| *t != params::CvTarget::Off);
        let base = dsp::Patch::from_params(&self.params);
        let mut patch = base;
        let first = if cv_on { n.min(CV_CHUNK) } else { n };
        bus::apply_cv(&mut patch, &self.params, self.bus.advance(first, self.sample_rate));
        self.engine.set_patch(&patch);

        let mut next_event = context.next_event();
        let mut peak = 0.0f32;
        for (i, frame) in buffer.iter_samples().enumerate() {
            if cv_on && i > 0 && i % CV_CHUNK == 0 {
                let mut patch = base;
                bus::apply_cv(&mut patch, &self.params, self.bus.advance((n - i).min(CV_CHUNK), self.sample_rate));
                self.engine.set_cv_patch(&patch);
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
            peak = peak.max(l.abs()).max(r.abs());
            write_frame(frame, (l, r));
        }

        // Idle (no voice), the display follows the Osc A Index knob instead of the last note's grain.
        let index = if self.engine.active_voices() == 0 {
            self.params.osca.index.value().round() as u32
        } else {
            self.engine.display_index()
        };
        self.current_index.store(index, Ordering::Relaxed);
        // Non-negative f32 bits order like the values, so fetch_max keeps the peak until the editor swaps it out.
        self.meters.gr_db.fetch_max(self.engine.limiter_gr_db().to_bits(), Ordering::Relaxed);
        self.meters.peak.fetch_max(peak.to_bits(), Ordering::Relaxed);
        ProcessStatus::Normal
    }
}

impl ClapPlugin for Gts {
    const CLAP_ID: &'static str = "com.github.bubbaschmee.riddim-rack.gts";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A two-oscillator graintable synthesizer");
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
