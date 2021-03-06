/*
    This project is more meant to suit my personal bass needs, since all synths I've tried fall short in one way or another.
    Goal for now is 4 wavetable oscillators that can FM each other however you want them to
    TODO:
    Glide
    Get FM going
    use f64 where it makes sense (pitch and possibly interp coefficients)
    More wavetables (this might require an installer to do properly :/// )
    Turn off processing for inactive oscillators (some magic with the for loop?)

    Fix the elusive glitch where more than one tone makes the entire daw crap out

    Optimisation. look into doing simd on the oscillators sometime
    Licensing. Look into MIT and copyleft
    https://docs.rs/basic_dsp/0.2.0/basic_dsp/

*/
/*
    GUI Research:
    figure out how to do a basic gui with vstgui
    figure out how to link it together with a basic rust vst through cxx

*/

//vst stuff
#[macro_use]
extern crate vst;

use vst::api::{Events, Supported};
use vst::buffer::AudioBuffer;
use vst::event::Event;
use vst::plugin::{CanDo, Category, Info, Plugin, PluginParameters};

extern crate wavetable;

use wavetable::voiceset::Parameters as WavetableParameters;
use wavetable::Synth as WavetableSynth;

struct Synth<'a> {
    synth: WavetableSynth<'a>,
    params: Arc<Parameters>,
}

struct Parameters {
    inner: Arc<WavetableParameters>,
}

impl<'a> Default for Synth<'a> {
    fn default() -> Synth<'a> {
        let synth = WavetableSynth::default();
        let params = Arc::new(Parameters {
            inner: Arc::clone(&synth.voices.params),
        });
        Synth { synth, params }
    }
}

use std::sync::Arc;
impl<'a> Plugin for Synth<'a> {
    // FIXME: needs to set filter sample rate too
    fn set_sample_rate(&mut self, rate: f32) {
        self.synth.sample_rate = rate;
    }
    fn get_info(&self) -> Info {
        Info {
            name: "BassMachine".to_string(),
            unique_id: 9265,
            inputs: 0,
            outputs: 2,
            category: Category::Synth,
            parameters: 19,
            ..Default::default()
        }
    }
    fn process_events(&mut self, events: &Events) {
        for event in events.events() {
            match event {
                Event::Midi(ev) => self.synth.process_midi_event(ev.data),
                // More events can be handled here.
                _ => (),
            }
        }
    }

    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        // let samples = buffer.samples();
        let (_, outputs) = buffer.split();
        if outputs.len() < 2 {
            return;
        }
        let (mut l, mut r) = outputs.split_at_mut(1);

        let stereo_out = l[0].iter_mut().zip(r[0].iter_mut());
        for (left_out, right_out) in stereo_out {
            let out = self.synth.voices.step_one();
            *left_out = out[0];
            *right_out = out[1];
        }
        // using the plugin-agnostic process didn't work for splitting up
        // self.synth.process(samples, &mut l, &mut r);
        // self.synth.process(samples, &mut outputs);
    }
    fn can_do(&self, can_do: CanDo) -> Supported {
        match can_do {
            CanDo::ReceiveMidiEvent => Supported::Yes,
            _ => Supported::Maybe,
        }
    }
    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        Arc::clone(&self.params) as Arc<dyn PluginParameters>
    }
}
/*
    Parameters that could be added easily, but won't be until there's a gui:
    vol env parameters
    envelope slopes
    Osc panning
    Osc on/off switches
    filter on/off
    Unison blend
    Gain-compensation on transistor filter
    Parameters that could be added with some effort when there's a gui:
    more waveshaping parameters in the style of sytrus
    spectral crossfading for wavetables with few waves
*/
impl PluginParameters for Parameters {
    fn get_parameter(&self, index: i32) -> f32 {
        match index {
            0 => self.inner.pos[0].get() as f32 / (self.inner.wave_number1 as f32 - 1.),
            1 => self.inner.vol[0].get(),
            2 => self.inner.detune[0].get() * 25. - 24.5,
            3 => self.inner.octave[0].get() as f32 / 4. + 0.5,
            4 => self.inner.pos[1].get() as f32 / (self.inner.wave_number1 as f32 - 1.),
            5 => self.inner.vol[1].get(),
            6 => self.inner.detune[1].get() * 25. - 24.5,
            7 => self.inner.octave[1].get() as f32 / 4. + 0.5,
            8 => self.inner.filter_params[0].get_cutoff() * 4.,
            9 => self.inner.filter_params[0].res.get() / 4.,
            10 => (self.inner.filter_params[0].poles.get()) as f32 / 3.,
            11 => self.inner.filter_params[0].drive.get() / 5.,
            12 => self.inner.modenv_params.attack_time.get() as f32 / 176400.,
            13 => self.inner.modenv_params.decay_time.get() as f32 / 176400.,
            14 => self.inner.modenv_params.sustain.get(),
            15 => self.inner.modenv_params.release_time.get() as f32 / 176400.,
            16 => self.inner.cutoff_amount.get(),
            17 => self.inner.g_uvoices.get() as f32 / 7.,
            18 => self.inner.pitch_offs_val.get(),
            _ => 0.0,
        }
    }
    fn set_parameter(&self, index: i32, value: f32) {
        match index {
            0 => self.inner.pos[0]
                .set(((value * (self.inner.wave_number1 - 1) as f32).round()) as usize),
            1 => self.inner.vol[0].set(value),
            // FIXME: make some proper detune formulas. They're just eyeballed for now.
            2 => self.inner.detune[0].set(0.98 + value * 0.04),
            3 => self.inner.octave[0].set((((value - 0.5) * 3.).round()) as i8),
            4 => self.inner.pos[1]
                .set(((value * (self.inner.wave_number2 - 1) as f32).round()) as usize),
            5 => self.inner.vol[1].set(value),
            6 => self.inner.detune[1].set(0.98 + value * 0.04),
            7 => self.inner.octave[1].set((((value - 0.5) * 3.).round()) as i8),
            8 => {
                for i in 0..self.inner.filter_params.len() {
                    self.inner.filter_params[i].cutoff_smoother.set(value / 4.);                }
            }
            9 => {
                for i in 0..self.inner.filter_params.len() {
                    self.inner.filter_params[i].res.set(value * 4.)
                }
            }
            10 => {
                for i in 0..self.inner.filter_params.len() {
                    self.inner.filter_params[i]
                        .poles
                        .set(((value * 3.).round()) as usize)
                }
            }
            11 => {
                for i in 0..self.inner.filter_params.len() {
                    self.inner.filter_params[i].drive.set(value * 5.)
                }
            }
            12 => self
                .inner
                .modenv_params
                .attack_time
                .set((value * 176400.) as usize),
            13 => self
                .inner
                .modenv_params
                .decay_time
                .set((value * 176400.) as usize),
            14 => self.inner.modenv_params.sustain.set(value),
            15 => self
                .inner
                .modenv_params
                .release_time
                .set((value * 176400.) as usize),
            16 => self.inner.cutoff_amount.set(value),
            17 => self.inner.g_uvoices.set(((value * 6.).ceil()) as usize + 1),
            18 => self.inner.change_spread(value),
            _ => (),
        }
    }
    fn get_parameter_name(&self, index: i32) -> String {
        match index {
            0 => "osc1 WT pos".to_string(),
            1 => "osc1 volume".to_string(),
            2 => "osc1 detune".to_string(),
            3 => "osc1 octave".to_string(),
            4 => "osc2 WT pos".to_string(),
            5 => "osc2 volume".to_string(),
            6 => "osc2 detune".to_string(),
            7 => "osc1 octave".to_string(),
            8 => "cutoff".to_string(),
            9 => "res".to_string(),
            10 => "filter order".to_string(),
            11 => "drive".to_string(),
            12 => "attack time".to_string(),
            13 => "decay time".to_string(),
            14 => "sustain level".to_string(),
            15 => "release time".to_string(),
            16 => "cutoff amount".to_string(),
            17 => "unison voices".to_string(),
            18 => "unison detune".to_string(),
            //4 => "Wet level".to_string(),
            _ => "".to_string(),
        }
    }
    fn get_parameter_label(&self, index: i32) -> String {
        match index {
            0 => "".to_string(),
            1 => "%".to_string(),
            2 => "".to_string(),
            3 => "".to_string(),
            4 => "".to_string(),
            5 => "%".to_string(),
            6 => "".to_string(),
            7 => "".to_string(),
            8 => "Hz".to_string(),
            9 => "%".to_string(),
            10 => "poles".to_string(),
            11 => "%".to_string(),
            12 => "ms".to_string(),
            13 => "ms".to_string(),
            14 => "%".to_string(),
            15 => "ms".to_string(),
            16 => "Hz".to_string(),
            17 => "voices".to_string(),
            18 => "".to_string(),
            _ => "".to_string(),
        }
    }
    // This is what will display underneath our control.  We can
    // format it into a string that makes the most sense.
    fn get_parameter_text(&self, index: i32) -> String {
        match index {
            0 => format!("{}", self.inner.pos[0].get()),
            1 => format!("{:.3} dB", 20. * self.inner.vol[0].get().log10()),
            2 => format!("{:.3}", self.inner.detune[0].get()),
            3 => format!("{}", self.inner.octave[0].get()),
            4 => format!("{}", self.inner.pos[1].get()),
            5 => format!("{:.3} dB", 20. * self.inner.vol[1].get().log10()),
            6 => format!("{:.3}", self.inner.detune[1].get()),
            7 => format!("{}", self.inner.octave[1].get()),
            // FIXME: Get the correct sample rate from daw here
            8 => format!("{:.0}", self.inner.filter_params[0].cutoff_smoother.target_get() * 88200.),
            9 => format!("{:.3}", self.inner.filter_params[0].res.get()),
            10 => format!("{}", self.inner.filter_params[0].poles.get() + 1),
            11 => format!("{:.3}", self.inner.filter_params[0].drive.get()),
            12 => format!(
                "{:.1} ms",
                self.inner.modenv_params.attack_time.get() as f32 / 88.2
            ),
            13 => format!(
                "{:.1}",
                self.inner.modenv_params.decay_time.get() as f32 / 88.2
            ),
            14 => format!("{:.3}", self.inner.modenv_params.sustain.get()),
            15 => format!(
                "{:.1}",
                self.inner.modenv_params.release_time.get() as f32 / 88.2
            ),
            16 => format!("{:.1}", self.inner.cutoff_amount.get() * 22050.),
            17 => format!("{}", self.inner.g_uvoices.get()),
            18 => format!("{:.3}", self.inner.pitch_offs_val.get()),
            _ => format!(""),
        }
    }
}
plugin_main!(Synth);
