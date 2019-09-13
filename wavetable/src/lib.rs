//use vst::util::AtomicFloat;
//used for handling .wav files
extern crate hound;

//include voiceset module:
pub mod resources;
mod util;
pub mod voiceset;

pub struct Synth<'a> {
    note_duration: f64,
    pub sample_rate: f32,               // FIXME(will): should not be pub
    pub voices: voiceset::Voiceset<'a>, // FIXME(will): should not be pub
    wt_len: Vec<usize>,
}

impl<'a> Synth<'a> {
    // fills a buffer we can use for fir filtering.
    pub(crate) fn prep_buffer(&mut self) {
        self.voices
            .interp_buffer
            .resize(self.voices.oscs[0].downsample_fir.len() + 1, 0.);
        for i in 0..self.voices.oscs[0].downsample_fir.len() - 1 {
            self.voices.interp_buffer[i] = 0.;
        }
    }
    fn find_ratio(&mut self, note: u8, i: usize) -> f32 {
        let standard = 21.533203125; // default wavetable pitch
        let pn = 440f32 * (2f32.powf(1. / 12.)).powi(note as i32 - 69);
        // return ratio between desired pitch and standard
        let diff = note - 17;
        let mip = diff as usize / 12;
        self.voices.voice[i].wavetable_mip = mip;
        let downsampled_ratio = 2f32.powi(mip as i32);
        (pn / downsampled_ratio) / standard
    }
    fn find_ratio_grain(&mut self, note: u8, i: usize) -> f32 {
        let pn = 440f32 * (2f32.powf(1. / 12.)).powi(note as i32 - 69);
        //return ratio between desired pitch and standard
        let diff = note - 17;
        let mip = diff as usize / 12;
        self.voices.voice[i].grain_mip = mip;
        let downsampled_ratio = 2f32.powi(mip as i32);
        //standard / pn
        (pn / downsampled_ratio)
    }
    pub fn process_midi_event(&mut self, data: [u8; 3]) {
        match data[0] {
            128 => self.note_off(data[1]),
            144 => self.note_on(data[1]),
            _ => (),
        }
    }
    pub fn note_on(&mut self, note: u8) {
        self.note_duration = 0.0;
        let mut i: usize = 9;
        //get the first free voice
        for j in 0..8 {
            if self.voices.voice[j].is_free() {
                i = j;
                break;
            }
        }
        // if no free voices, nothing happens for now. Voice stealing should be implemented.
        // voice stealing requires keeping track of which voice was played last.
        if i > 7 {
            return;
        }
        // setup of the voice
        self.voices.voice[i].use_voice(note);
        self.voices.vol_env.restart_env(i);
        self.voices.mod_env.restart_env(i);
        self.voices.voice[i].ratio = self.find_ratio(note, i);
        self.voices.voice[i].grain_ratio = self.find_ratio_grain(note, i);
    }
    pub fn note_off(&mut self, note: u8) {
        for i in 0..8 {
            if self.voices.voice[i].note == Some(note) {
                self.voices.voice[i].note = None;
                self.voices.voice[i].free_voice();
                self.voices.vol_env.note[i] = false;
                self.voices.mod_env.note[i] = false;
            }
        }
    }

    pub fn process<'b, I>(&mut self, samples: usize, outputs: I)
    where
        I: IntoIterator<Item = &'b mut [f32]>,
    {
        let mut output_sample;
        let mut outputs = outputs.into_iter().collect::<Vec<_>>();
        for sample_idx in 0..samples {
            output_sample = self.voices.step_one();
            for buff in outputs.iter_mut() {
                buff[sample_idx] = output_sample;
            }
        }
    }
}

impl<'a> Default for Synth<'a> {
    fn default() -> Synth<'a> {
        let mut a = Synth {
            note_duration: 0.0,
            sample_rate: 44100.,
            voices: voiceset::Voiceset {
                ..Default::default()
            },
            wt_len: vec![7, 7],
        };
        a.prep_buffer();
        a.wt_len[0] = a.voices.oscs[0].len / (2048 * a.voices.oscs[0].amt_oversample);
        a.wt_len[1] = a.voices.oscs[1].len / (2048 * a.voices.oscs[1].amt_oversample);
        return a;
    }
}

#[cfg(test)]
mod tests {
    // #[test]
    // fn it_works() {
    //     assert_eq!(2 + 2, 4);
    // }
}
