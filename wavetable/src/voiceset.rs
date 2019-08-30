use std::collections::VecDeque;
use std::f32;
mod filter;
pub mod interp;
mod modmatrix;
use std::f32::consts::PI;
use std::sync::Arc;
use crate::util::{AtomicI8, AtomicUsize, AtomicF32};
/*
        Todo:
        optimize mip_offset function (match arms?)

        the stuff to force envelope properly into release state doesn't seem to work (test)

        should probably quantise grain pos to avoid accidental fm lol
        implement unison

        avoid vec of vec

        small alias problem now. SNR at 1 kHz is about -80 dB.
        Most likely caused by quality of interpolation algorithm

        Optimization ideas : flatten vectors(possibly big improvement, way fewer cache misses.)
        iterate instead of index where it makes sense (should be ~20% faster),
        possibly change some vectors to arrays (could be done instead of flattening, easier).
        the actual samples per waveform, and number of mip maps is known at compile-time.
        Number of waveforms is not

*/
pub fn mip_offset(mip: usize, len: usize) -> usize {
    let amount = match mip {
        0 => 0.,
        1 => 1.,
        2 => 1.5,
        3 => 1.75,
        4 => 1.875,
        5 => 1.9375,
        6 => 1.96875,
        7 => 1.984375,
        8 => 1.9921875,
        9 => 1.99609375,
        _ => 0.,
    };
    (len as f32 * amount) as usize
}

pub struct Parameters {
    //tweakable synth parameters
    pub g_uvoices: AtomicUsize,
    pub vol: Vec<AtomicF32>,
    pub vol_grain: AtomicF32,
    pub detune: Vec<AtomicF32>,
    pub filter_params: Vec<Arc<filter::LadderParameters>>,
    pub modenv_params: Arc<modmatrix::EnvParams>,
    pub grain_params: Vec<Arc<interp::GrainParams>>,
    pub pos: Vec<AtomicUsize>,
    pub octave: Vec<AtomicI8>,
    pub cutoff_amount: AtomicF32,
    //other stuff
    pub wave_number1: usize,
    pub wave_number2: usize,

}

#[allow(dead_code)]
pub struct Voiceset {
    pub(crate) oscs: Vec<interp::WaveTable>,
    pub(crate) g_oscs: Vec<interp::GrainTable>,
    //vector of filters, since each voice will need its own filter when envelopes are added
    pub filter: Vec<filter::LadderFilter>,
    //pub osc2_vol : f32, pub det2 : f32,
    pub voice: Vec<Voice>,

    //interp_buffer gives room for filtering continuous output from oscillator.
    pub(crate) interp_buffer: VecDeque<f32>,

    pub vol_env: modmatrix::Env,
    pub mod_env: modmatrix::Env,
    pub params: Arc<Parameters>,
}
impl Voiceset {
    // might require more antialiasing
    pub fn step_one(&mut self) -> f32 {
        let output: f32;
        //needs to have a way to go through all unison voices
        //downsampling for loop
        for _i in 0..self.oscs[0].amt_oversample {
            let mut unfiltered_new = 0.;
            for voice in 0..8 {
                let vol_mod = self.vol_env.next(voice);
                let env2 = self.mod_env.next(voice);
                //add the output of the active voices together
                if vol_mod == None {
                    //if vol_env is none for the voice, it's done outputting
                    //break;
                    self.voice[voice].reset_its();
                } else {
                    let mut temp = 0.;
                    //the 2 oscillators
                    for osc in 0..2 {
                        temp += self.single_interp(
                            self.voice[voice].ratio * self.params.detune[osc].get(),
                            voice,
                            osc,
                        ) * self.params.vol[osc].get()
                            * vol_mod.unwrap();
                    }
                    //the graintable osc
                    for osc in 0..1 {
                        for u_voice in 0..self.params.g_uvoices.get() {
                            temp += self._single_interp_grain(
                                self.voice[voice].grain_ratio
                                    * self.voice[voice].g_ratio_offsets[u_voice],
                                voice,
                                osc,
                                u_voice,
                            ) * self.params.vol_grain.get()
                                * vol_mod.unwrap();
                        }
                    }
                    self.filter[voice].tick_pivotal(temp, env2, self.params.cutoff_amount.get());
                    //self.filter[voice].tick_pivotal(temp);
                    unfiltered_new += self.filter[voice].vout
                        [self.filter[0].params.poles.get()];
                }
            }
            //removes the sample that just got filtered
            self.interp_buffer.pop_front();
            //adds a new unfiltered sample to the end
            self.interp_buffer.push_back(unfiltered_new);
        }
        //only every 2nd sample needs to be output for downsampling. Therefore only every 2nd sample
        //needs to be filtered
        output = self.single_convolve(&self.oscs[0].downsample_fir);
        return output;
    }
    // used for getting a sample from a graintable oscillator
    pub fn _single_interp_grain(&mut self, ratio: f32, i: usize, j: usize, k: usize) -> f32 {
        let mip = (self.voice[i].grain_mip as i8) as usize; /*(1./ratio).log2().floor() as usize;*/
        let mip_offset = mip_offset(mip, self.g_oscs[j].params.len.get());
        //let downsampled_ratio = 2f32.powi(self.voice[i].grain_mip as i32);
        let grain_size =
            self.g_oscs[j].params.grain_size.get() / 2f32.powi(self.voice[i].grain_mip as i32);
        let len = self.g_oscs[j].mip_len(mip);
        let offset = (self.g_oscs[j].params.pos.get() * self.voice[i].g_pos_offsets[k] * len as f32)
            as usize;
        let mut temp: f32;
        let it: usize;
        let x = ratio / (88200. / self.g_oscs[j].params.grain_size.get());
        let z_pos; //= z.fract();
        it = self.voice[i].grain_its[k].floor() as usize + mip_offset + offset;
        z_pos = self.voice[i].grain_its[k].fract();
        temp = ((self.g_oscs[j].c3[it] * z_pos + self.g_oscs[j].c2[it]) * z_pos
            + self.g_oscs[j].c1[it])
            * z_pos
            + self.g_oscs[j].c0[it];
        self.voice[i].grain_its[k] += x;
        //loop from the grain size:
        if self.voice[i].grain_its[k] > grain_size {
            self.voice[i].grain_its[k] -= grain_size;
        }
        if self.voice[i].grain_its[k] > (len) as f32 {
            //loop back around zero.
            self.voice[i].grain_its[k] -= (len) as f32;
        }
        // save the window and iterate through it to save CPU.
        //apply a window to the grain to declick it:
        temp = temp * ((1. / (grain_size - 1.)) * PI * self.voice[i].grain_its[k]).sin();
        return temp;
    }
    //single_interp could be rethought as an iterator for WaveTable
    pub(crate) fn single_interp(&mut self, ratio: f32, i: usize, j: usize) -> f32 {
        // Optimal 2x (4-point, 3rd-order) (z-form)
        // return ((c3*z+c2)*z+c1)*z+c0;
        //find the best mip to do the interpolation from. could be moved elsewhere to avoid calling excessively
        let mip = (self.voice[i].wavetabe_mip as i8 + self.params.octave[j].get())
            as usize;
        let temp: f32;
        let it: usize;
        //x is the placement of the sample compared to the last one, or the slope
        let x = ratio;
        //self.new_len = findlen as usize;
        //let z = x - 0.5;
        let z_pos; //= z.fract();
        it = self.voice[i].wave_its[j][0].floor() as usize; // have a way to use each unison it in use
        z_pos = self.voice[i].wave_its[j][0].fract(); // should z_pos have a -0.5?
        temp = ((self.oscs[j].c3[mip][self.params.pos[j].get()][it] * z_pos
            + self.oscs[j].c2[mip][self.params.pos[j].get()][it])
            * z_pos
            + self.oscs[j].c1[mip][self.params.pos[j].get()][it])
            * z_pos
            + self.oscs[j].c0[mip][self.params.pos[j].get()][it];
        self.voice[i].wave_its[j][0] += x;
        if self.voice[i].wave_its[j][0] > (self.oscs[j].mips[mip][0].len()) as f32 {
            //loop back around zero.
            self.voice[i].wave_its[j][0] -= (self.oscs[j].mips[mip][0].len()) as f32;
        }
        return temp;
    }
    //Convolves a single sample, based on the sample buffer
    pub(crate) fn single_convolve(&self, p_coeffs: &Vec<f32>) -> f32 {
        let mut convolved: f32;
        convolved = 0.;
        //convolved.resize(p_in.len() + p_coeffs.len(), 0.);
        //let mut temp = self.interp_buffer.to_vec();
        //temp.resize(new_len, 0.);
        //n should be the length of p_in + length of p_coeffs
        //this k value should skip the group delay?
        let k = p_coeffs.len();
        for i in 0..k
        //  position in coefficients array
        {
            //if k >= i
            //{
            convolved += p_coeffs[i] * self.interp_buffer[k - i];
            //}
        }
        return convolved;
    }
}
impl Default for Voiceset {
    fn default() -> Voiceset {
        let filters = vec![filter::LadderFilter::default(); 8];
        let modenv = modmatrix::Env::default();
        let mod_env_params = modenv.params.clone();
        let filter_params = filters.iter().map(|f| f.params.clone()).collect();
        let g_oscs = vec![interp::GrainTable::default(); 1];
        let g_params = g_oscs.iter().map(|g| g.params.clone()).collect();
        let a = Voiceset {
            oscs: vec![Default::default(); 2],
            g_oscs: g_oscs,
            filter: filters,
            voice: vec![Voice::default(); 8],
            interp_buffer: VecDeque::with_capacity(200),
            vol_env: modmatrix::Env {
                params: Arc::new(modmatrix::EnvParams {
                    decay_time: AtomicUsize::new(0),
                    sustain: AtomicF32::new(1.0),
                    attack_slope: AtomicF32::new(1.0),
                    ..Default::default()
                }),
                ..Default::default()
            },
            params: Arc::new(Parameters {
                grain_params: g_params,
                modenv_params: mod_env_params,
                filter_params: filter_params,
                pos: vec![AtomicUsize::new(0), AtomicUsize::new(0)],
                octave: vec![AtomicI8::new(0), AtomicI8::new(0)],
                cutoff_amount: AtomicF32::new(0.5),
                g_uvoices: AtomicUsize::new(1),
                vol: vec![AtomicF32::new(0.), AtomicF32::new(0.)],
                vol_grain: AtomicF32::new(1.),
                detune: vec![AtomicF32::new(1.), AtomicF32::new(1.)],
                wave_number1: 7,
                wave_number2: 7,
            }),
            mod_env: modenv,
        };
        return a;
    }
}
#[derive(Clone)]
pub struct Voice {
    free: bool,
    // every voice can share the same interpolator
    // pub(crate) oscs : &'a Interp,
    wave_its: Vec<Vec<f32>>,
    grain_its: Vec<f32>,
    g_pos_offsets: Vec<f32>,
    g_ratio_offsets: Vec<f32>,
    pub ratio: f32,
    pub grain_ratio: f32,
    pub(crate) wavetabe_mip: usize,
    pub(crate) grain_mip: usize,
    // pos gives the current wave
    pub note: Option<u8>,
    pub time: usize,
    // the note parameter can allow us to have note offsets for octave and semitone switches
}

#[allow(dead_code)]
impl Voice {
    pub fn reset_its(&mut self) {
        //reset iterators. Value they get set to could be changed to change phase,
        //or made random for analog-style random phase
        //https://rust-lang-nursery.github.io/rust-cookbook/algorithms/randomness.html
        self.wave_its[0][0] = 0.;
        self.wave_its[1][0] = 0.;
        self.grain_its = vec![0.; 7];
    }
    pub fn is_free(&self) -> bool {
        return self.free;
    }
    pub fn use_voice(&mut self, note: u8) {
        self.free = false;
        self.note = Some(note);
        self.time = 0;
        //possibly call prep_buffer here?
    }
    pub fn free_voice(&mut self) {
        //if self.note == note {
        self.free = true;
        //}
    }
}
impl Default for Voice {
    fn default() -> Voice {
        let a = Voice {
            free: true,
            wave_its: vec![vec![0.; 7]; 2],
            grain_its: vec![0.; 7],
            g_pos_offsets: vec![1., 1.012, 0.988, 1.005, 0.994, 1.008, 0.992],
            g_ratio_offsets: vec![1., 1.001, 0.999, 1.002, 0.998, 1.004, 0.996],
            wavetabe_mip: 0,
            grain_mip: 0,
            ratio: 1.,
            grain_ratio: 1.,
            note: None,
            time: 0,
        };
        return a;
    }
}
