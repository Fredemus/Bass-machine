//used for handling .wav files
extern crate hound;

//include voiceset module:
pub mod resources;
mod util;
pub mod voiceset;

pub struct Synth<'a> {
    note_duration: f64,
    pub sample_rate: f32, // FIXME(will): should not be pub
    pub voices: voiceset::Voiceset<'a>, // FIXME(will): should not be pub
}

impl<'a> Synth<'a> {
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
        // FIXME: Implement voice stealing
        // if no free voices, nothing happens for now. Voice stealing should be implemented.
        // voice stealing requires keeping track of which voice was played last.
        if i > 7 {
            return;
        }
        // setup of the voice
        self.voices.voice[i].use_voice(note, &self.voices.params.octave);
        self.voices.vol_env.restart_env(i);
        self.voices.mod_env.restart_env(i);
        self.voices.voice[i].ratio = self.find_ratio(note, i);
    }
    pub fn note_off(&mut self, note: u8) {
        for i in 0..8 {
            if self.voices.voice[i].note == Some(note) {
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
        for sample_idx in 0..samples
        /*(0..samples).step_by(2)*/
        {
            output_sample = self.voices.step_one();
            for buff in outputs.iter_mut() {
                buff[sample_idx] = output_sample[0];
                // buff[1+ sample_idx] = output_sample[1];
            }
        }
    }

    // FIXME: process doesn't work in stereo. accessing samples for the left and right channel specifically gives problems
    // All processing done in the plugin-specific process for now

    // my attempt to fix process to handle stereo in a logical way
    // problem right now is that this will only handle one sample in the buffer
    // pub fn process<'b, I>(&mut self, samples: usize, l: I, r: I)
    // where
    //     I: IntoIterator<Item = &'b mut [f32]>,
    // {
    //     let mut output_sample;
    //     // Iterate over outputs as (&mut f32, &mut f32)
    //     // let (mut l, mut r) = outputs.split_at_mut(1);
    //     let mut l = l.into_iter().collect::<Vec<_>>();
    //     let mut r = r.into_iter().collect::<Vec<_>>();
    //     let stereo_out = l[0].iter_mut().zip(r[0].iter_mut());

    //     for sample_idx in 0..samples {
    //         output_sample = self.voices.step_one();
    //         for (left_out,right_out) in stereo_out {
    //             left_out[sample_idx] = output_sample[0];
    //             right_out[sample_idx] = output_sample[1];
    //         }
    //         // for buff in outputs.iter_mut() {
    //         //     buff[sample_idx] = output_sample;
    //         // }
    //     }
    // }
    // what the heck is samples here?? How do we split output into left and right??
    // doesn't really work properly right now
    // pub fn process<'b, I>(&mut self, samples: usize, outputs: I)
    // where
    //     I: IntoIterator<Item = &'b mut [f32]>,
    // {
    //     let mut output_sample;
    //     let mut outputs = outputs.into_iter().collect::<Vec<_>>();
    //     for sample_idx in 0..samples {
    //         output_sample = self.voices.step_one();
    //         for buff in outputs.iter_mut() {
    //             buff[sample_idx] = output_sample[0];
    //             buff[sample_idx+1] = output_sample[1];
    //         }
    //     }
    // }
}

impl<'a> Default for Synth<'a> {
    fn default() -> Synth<'a> {
        Synth {
            note_duration: 0.0,
            sample_rate: 44100.,
            voices: voiceset::Voiceset {
                ..Default::default()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    // #[test]
    // fn it_works() {
    //     assert_eq!(2 + 2, 4);
    // }
}
