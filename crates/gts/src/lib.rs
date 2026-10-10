//! GTS, a two-oscillator graintable synth. See SPEC.md at the repo root for the contract.

pub mod dsp;
mod editor;
pub mod params;

use nih_plug::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use params::GtsParams;

pub struct Gts {
    params: Arc<GtsParams>,
    engine: dsp::Engine,
    /// Osc A's current grain index (0..127), for the GUI readout.
    current_index: Arc<AtomicU32>,
}

impl Default for Gts {
    fn default() -> Self {
        Self {
            params: Arc::new(GtsParams::default()),
            engine: dsp::Engine::new(),
            current_index: Arc::new(AtomicU32::new(0)),
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

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.current_index.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        nih_log!(
            "initialize: sample_rate={} max_buffer_size={}",
            buffer_config.sample_rate,
            buffer_config.max_buffer_size
        );
        self.engine.set_sample_rate(buffer_config.sample_rate as f64);
        self.engine.log_notes = std::env::var_os("NIH_LOG").is_some();
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();
        for (i, mut frame) in buffer.iter_samples().enumerate() {
            while let Some(event) = next_event {
                if event.timing() > i as u32 {
                    break;
                }
                match event {
                    NoteEvent::NoteOn { note, velocity, .. } => {
                        self.engine.note_on(note, velocity);
                        if self.engine.log_notes {
                            nih_log!(
                                "note-on: note={} velocity={:.2} held={}",
                                note,
                                velocity,
                                self.engine.held_count()
                            );
                        }
                    }
                    NoteEvent::NoteOff { note, .. } => self.engine.note_off(note),
                    // TODO(voice): MidiPitchBend { value 0..1, 0.5 = centre }, MidiCC { cc 1 = wheel }.
                    _ => {}
                }
                next_event = context.next_event();
            }

            let gain = util::db_to_gain(self.params.volume.smoothed.next());
            let s = self.engine.next_sample() * gain;
            for sample in frame.iter_mut() {
                *sample = s;
            }
        }

        self.current_index
            .store(self.params.osca.index.value() as u32, Ordering::Relaxed);
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
