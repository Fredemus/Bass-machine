/*

    TODO:
    test graintable
    figure out envelopes
    move fir filters somewhere sensible.
    Implement unison


    only way i can see to avoid groupdelay from fir is to fir filter each separately and

    Optimisation. look into optimising single_conv (polyphase) or single_interp
    Licensing. Look into MIT and copyleft
    https://docs.rs/nalgebra/0.3.2/nalgebra/struct.DVec3.html
    https://docs.rs/basic_dsp/0.2.0/basic_dsp/

*/

//vst stuff
#[macro_use]
extern crate vst;
use vst::api::{Events, Supported};
use vst::buffer::AudioBuffer;
use vst::event::Event;
use vst::plugin::{CanDo, Category, Info, Plugin, PluginParameters};

//for gui thread
//use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
//use vst::util::AtomicFloat;
//used for handling .wav files
extern crate hound;

//include voiceset module:
mod voiceset;

struct Synth {
    note_duration: f64,
    sample_rate: f32,
    //the oscillator. More can easily be added
    voices: voiceset::Voiceset,
    wt_len: Vec<usize>,
}

impl Synth {
    //fills a buffer we can use for fir filtering.
    //Can be used to avoid the delay from the fir filtering. Figure out how/when to call it to avoid delay.
    pub(crate) fn prep_buffer(&mut self) {
        self.voices
            .interp_buffer
            .resize(self.voices.oscs[0].downsample_fir.len() + 1, 0.);
        for i in 0..self.voices.oscs[0].downsample_fir.len() - 1 {
            self.voices.interp_buffer[i] = 0.;
        }
        //not sure how to use the stuff underneath with multiple voices or legato, if it's even possible
        //fills the buffer with actual samples to avoid delay
        // for _i in 0..(self.voices.osc1.downsample_fir.len()-1)/2
        // {
        //     let unfiltered_new = self.voices.single_interp(_ratio, 0);
        //     //removes a sample in front
        //     self.voices.interp_buffer.pop_front();
        //     //adds a new unfiltered sample to the end
        //     self.voices.interp_buffer.push_back(unfiltered_new);
        // }
    }
    fn find_ratio(&mut self, note: u8, i: usize) -> f32 {
        let standard = /*21.827*/ 21.533203125; //default wavetable pitch
        let pn = 440f32 * (2f32.powf(1. / 12.)).powi(note as i32 - 69);
        //return ratio between desired pitch and standard
        let diff = note - 17;
        let mip = diff as usize / 12;
        self.voices.voice[i].wavetabe_mip = mip;
        let downsampled_ratio = 2f32.powi(mip as i32);
        //standard / pn
        (pn / downsampled_ratio) / standard
    }
    fn find_ratio_grain(&mut self, note: u8, i: usize) -> f32 {
        //let standard = self.sample_rate * 2. / self.voices.g_oscs[0].grain_size;
        let pn = 440f32 * (2f32.powf(1. / 12.)).powi(note as i32 - 69);
        //return ratio between desired pitch and standard
        let diff = note - 17;
        let mip = diff as usize / 12;
        self.voices.voice[i].grain_mip = mip;
        let downsampled_ratio = 2f32.powi(mip as i32);
        //standard / pn
        (pn / downsampled_ratio)
    }
    fn process_midi_event(&mut self, data: [u8; 3]) {
        match data[0] {
            128 => self.note_off(data[1]),
            144 => self.note_on(data[1]),
            _ => (),
        }
        //change pitched_buffer here?
    }
    fn note_on(&mut self, note: u8) {
        self.note_duration = 0.0;
        //self.note = Some(note);
        let mut i: usize = 9;
        //get the first free voice
        for j in 0..8 {
            if self.voices.voice[j].is_free() {
                i = j;
                break;
            }
        }
        // if no free voices, nothing happens for now. Voice stealing should be implemented
        if i > 7 {
            return;
        }
        self.voices.vol_env.restart_env(i);
        self.voices.mod_env.restart_env(i);
        self.voices.voice[i].use_voice(note);
        self.voices.voice[i].ratio = self.find_ratio(note, i);
        self.voices.voice[i].grain_ratio = self.find_ratio_grain(note, i);
        //self.prep_buffer(/*self.ratio*/);
        //self.osc1.interpolated = self.osc1.static_convolve(&self.osc1.upsample_fir, &self.osc1.interpolated);
    }
    fn note_off(&mut self, note: u8) {
        for i in 0..8 {
            if self.voices.voice[i].note == Some(note) {
                self.voices.voice[i].note = None;
                self.voices.voice[i].free_voice();
                self.voices.vol_env.note[i] = false;
                self.voices.mod_env.note[i] = false;
            }
        }
        // self.note = None;
        // for i in 0..8 {
        //     if !self.voices.voice[i].is_free() {
        //         self.note = Some(note);
        //         break; //it's enough if just one voice is free
        //     }
        // }
    }
}

impl Default for Synth {
    fn default() -> Synth {
        let mut osc1: voiceset::interp::WaveTable = Default::default();
        // let mut dir = file!().to_owned();
        // for i in 0..8 { //remove the \lib.rs
        //     dir.pop();
        // }
        // dir.push_str(r"\Tables\Basic Shapes.wav");
        // let mut reader = hound::WavReader::open(
        //     //dir
        //     r"C:\Users\rasmu\Documents\Xfer\Serum Presets\Tables\Analog\Basic Shapes.wav"
        // )
        // .unwrap();
        // osc1.source_y = reader.samples().collect::<Result<Vec<_>, _>>().unwrap();
        // osc1.slice();
        // osc1.oversample(2);
        // osc1.mip_map();
        // osc1.optimal_coeffs();
        osc1.change_table(
            r"C:\Users\rasmu\Documents\Xfer\Serum Presets\Tables\Analog\Basic Shapes.wav"
                .to_string(),
        );
        let mut osc2: voiceset::interp::WaveTable = Default::default();
        osc2.change_table(
            r"C:\Users\rasmu\Documents\Xfer\Serum Presets\Tables\Analog\Basic Shapes.wav"
                .to_string(),
        );
        let mut osc3: voiceset::interp::GrainTable = Default::default();
        osc3.change_table(
            r"C:\Users\rasmu\RustProjects\Graintable-synth\src\Tables\12-Screamer.wav".to_string(),
        );
        //let voiceset : interp::Voiceset::Default::default()
        let mut a = Synth {
            note_duration: 0.0,
            sample_rate: 44100.,
            voices: voiceset::Voiceset {
                oscs: vec![osc1, osc2],
                g_oscs: vec![osc3],
                ..Default::default()
            },
            wt_len: vec![7, 7],
        };
        a.prep_buffer(); //first call fills the buffer with 0's.
        a.wt_len[0] = a.voices.oscs[0].len / (2048 * a.voices.oscs[0].amt_oversample);
        a.wt_len[1] = a.voices.oscs[1].len / (2048 * a.voices.oscs[1].amt_oversample);
        return a;
    }
}

impl Plugin for Synth {
    fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate;
    }
    fn get_info(&self) -> Info {
        Info {
            name: "WaveTable".to_string(),
            unique_id: 9264,
            inputs: 0,
            outputs: 1,
            category: Category::Synth,
            parameters: 21,
            ..Default::default()
        }
    }
    fn process_events(&mut self, events: &Events) {
        for event in events.events() {
            match event {
                Event::Midi(ev) => self.process_midi_event(ev.data),
                // More events can be handled here.
                _ => (),
            }
        }
    }

    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        // Split out the input and output buffers into two vectors
        let (_, mut outputs) = buffer.split();

        // Assume 2 channels
        // if inputs.len() != 2 || outputs.len() != 2 {
        //     return;
        // }
        //  // Iterate over outputs as (&mut f32, &mut f32)
        // let (mut l, mut r) = outputs.split_at_mut(1);
        // let stereo_out = l[0].iter_mut().zip(r[0].iter_mut());
        for output_channel in outputs.into_iter() {
            for output_sample in output_channel {
                *output_sample = self.voices.step_one();
            }
        }
    }
    fn can_do(&self, can_do: CanDo) -> Supported {
        match can_do {
            CanDo::ReceiveMidiEvent => Supported::Yes,
            _ => Supported::Maybe,
        }
    }
    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        Arc::clone(&self.voices.params) as Arc<dyn PluginParameters>
    }
}
plugin_main!(Synth);
