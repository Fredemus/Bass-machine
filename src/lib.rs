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
use vst::plugin::{CanDo, Category, Info, Plugin};

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
            r"C:\Users\rasmu\RustProjects\Graintable-synth\src\Tables\op71_2.wav".to_string(),
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
    fn get_parameter(&self, index: i32) -> f32 {
        match index {
            0 => self.voices.pos[0] as f32,
            1 => self.voices.vol[0],
            2 => self.voices.detune[0],
            3 => self.voices.octave[0] as f32,
            4 => self.voices.pos[1] as f32,
            5 => self.voices.vol[1],
            6 => self.voices.detune[1],
            7 => self.voices.octave[1] as f32,
            8 => self.voices.filter[0].cutoff,
            9 => self.voices.filter[0].res,
            10 => (self.voices.filter[0].poles) as f32 + 1.,
            11 => self.voices.filter[0].drive,
            12 => self.voices.mod_env.attack_time as f32 / 88.2,
            13 => self.voices.mod_env.decay_time as f32 / 88.2,
            14 => self.voices.mod_env.sustain,
            15 => self.voices.mod_env.release_time as f32 / 88.2,
            16 => self.voices.cutoff_amount,
            17 => self.voices.g_oscs[0].pos * self.voices.g_oscs[0].source_y.len() as f32 / 88.2,
            18 => self.voices.g_oscs[0].grain_size / 88.2,
            19 => self.voices.vol_grain,
            20 => self.voices.g_uvoices as f32,
            _ => 0.0,
        }
    }
    fn set_parameter(&mut self, index: i32, value: f32) {
        match index {
            0 => {
                self.voices.pos[0] =
                    ((value * (self.voices.oscs[0].wave_number - 1) as f32).round()) as usize
            }
            1 => self.voices.vol[0] = value,
            //make some proper detune formulas. They're just eyeballed for now.
            2 => self.voices.detune[0] = 0.98 + value * 0.04,
            3 => self.voices.octave[0] = (((value - 0.5) * 3.).round()) as i8,
            4 => {
                self.voices.pos[1] =
                    ((value * (self.voices.oscs[1].wave_number - 1) as f32).round()) as usize
            }
            5 => self.voices.vol[1] = value,
            6 => self.voices.detune[1] = 0.98 + value * 0.04,
            7 => self.voices.octave[1] = (((value - 0.5) * 3.).round()) as i8,
            8 => {
                for i in 0..self.voices.filter.len() {
                    self.voices.filter[i].set_cutoff(value)
                }
            }
            //self.g = value * 10.,
            9 => {
                for i in 0..self.voices.filter.len() {
                    self.voices.filter[i].res = value * 4.
                }
            }
            10 => {
                for i in 0..self.voices.filter.len() {
                    self.voices.filter[i].poles = ((value * 3.).round()) as usize
                }
            }
            11 => {
                for i in 0..self.voices.filter.len() {
                    self.voices.filter[i].drive = value * 5.
                }
            }
            12 => self.voices.mod_env.attack_time = (value * 88200.) as usize,
            13 => self.voices.mod_env.decay_time = (value * 88200.) as usize,
            14 => self.voices.mod_env.sustain = value,
            15 => self.voices.mod_env.release_time = (value * 88200.) as usize,
            16 => self.voices.cutoff_amount = value,
            17 => self.voices.g_oscs[0].pos = value,
            18 => self.voices.g_oscs[0].grain_size = (value * 20000.).max(100.),
            19 => self.voices.vol_grain = value,
            20 => self.voices.g_uvoices = ((value * 7.).ceil()) as usize,
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
            17 => "grain pos".to_string(),
            18 => "grain size".to_string(),
            19 => "grain osc volume".to_string(),
            20 => "grain unison".to_string(),
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
            16 => "%".to_string(),
            17 => "ms".to_string(),
            18 => "ms".to_string(),
            19 => "%".to_string(),
            20 => "voices".to_string(),
            _ => "".to_string(),
        }
    }
    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        // Split out the input and output buffers into two vectors
        let (_, outputs) = buffer.split();

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
}
plugin_main!(Synth);
