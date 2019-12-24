use std::collections::VecDeque;
use std::f32;
use std::sync::Arc;

mod filter;
pub mod interp;
mod modmatrix;
mod resampling;
use crate::util::{AtomicF32, AtomicI8, AtomicUsize};

/*
        Todo:

        should probably quantise grain pos to avoid accidental fm lol
        implement more filter modes


        iterate instead of index where it makes sense (should be ~20% faster),

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
    pub pitch_offsets: Vec<AtomicF32>,
    pub vol: Vec<AtomicF32>,
    pub vol_grain: AtomicF32,
    pub detune: Vec<AtomicF32>,
    pub filter_params: Vec<Arc<filter::LadderParameters>>,
    pub modenv_params: Arc<modmatrix::EnvParams>,
    pub pos: Vec<AtomicUsize>,
    pub octave: Vec<AtomicI8>,
    pub cutoff_amount: AtomicF32,
    //other stuff
    pub wave_number1: usize,
    pub wave_number2: usize,
}

#[allow(dead_code)]
pub struct Voiceset<'a> {
    pub oscs: Vec<interp::WaveTable<'a>>,
    //vector of filters, since each voice will need its own filter when envelopes are added
    pub filter: Vec<filter::LadderFilter>,
    //pub osc2_vol : f32, pub det2 : f32,
    pub voice: Vec<Voice>,

    //interp_buffer gives room for filtering continuous output from oscillator.
    pub(crate) interp_buffer: VecDeque<f32>,
    pub poly_iir: resampling::HalfbandFilter,

    pub vol_env: modmatrix::Env,
    pub mod_env: modmatrix::Env,
    pub params: Arc<Parameters>,
}
impl<'a> Voiceset<'a> {
    pub fn step_one(&mut self) -> f32 {
        let mut output: f32 = 0.;
        //downsampling for loop
        for _i in 0..self.oscs[0].amt_oversample {
            let mut unfiltered_new = 0.;
            for voice in 0..8 {
                // this if-condition needs to be happened to something that happens once instead of continually
                if self.vol_env.output[voice] == None {
                    //if vol_env is none for the voice, it's done outputting
                    //break;
                    // add the output of the active voices together
                } else {
                    let vol_mod = self.vol_env.output[voice].unwrap();
                    self.vol_env.next(voice);
                    self.mod_env.next(voice);
                    let env2 = self.mod_env.output[voice];
                    let mut temp = 0.;
                    //the 2 oscillators
                    for osc in 0..2 {
                        for u_voice in 0..self.params.g_uvoices.get() {
                            temp += self.single_interp(
                                self.voice[voice].ratio
                                    * self.params.detune[osc].get()
                                    * self.voice[voice].pitch_offsets[u_voice],
                                voice,
                                osc,
                                u_voice,
                            ) * self.params.vol[osc].get()
                                * vol_mod;
                        }
                    }
                    temp /= self.params.g_uvoices.get() as f32;
                    
                    //the graintable osc
                    self.filter[voice].tick_pivotal(temp, env2, self.params.cutoff_amount.get());
                    //self.filter[voice].tick_pivotal(temp);
                    unfiltered_new += self.filter[voice].vout[self.filter[0].params.poles.get()];
                }
            }
            //----------- IIR FILTERING ------------------//
            output = self.poly_iir.process(unfiltered_new);
        }
        return output;
    }
    // TODO: Potentially use u here to have wavetable position spread
    pub(crate) fn single_interp(&mut self, ratio: f32, i: usize, j: usize, u: usize) -> f32 {
        // Optimal 2x (4-point, 3rd-order) (z-form)
        // return ((c3*z+c2)*z+c1)*z+c0;
        //find the best mip to do the interpolation from. could be moved elsewhere to avoid calling excessively
        let mip = (self.voice[i].wavetable_mip as i8 + self.params.octave[j].get()) as usize;
        let mip_offset = mip_offset(mip, self.oscs[j].len);
        let temp: f32;
        let it: usize;
        //x is the placement of the sample compared to the last one, or the slope
        let x = ratio;
        //let z = x - 0.5;
        let z_pos; //= z.fract();
        it = self.voice[i].wave_its[j][u].floor() as usize
            + mip_offset
            + self.oscs[j].wave_len * self.params.pos[j].get() / 2usize.pow(mip as u32); // have a way to use each unison it in use
        z_pos = self.voice[i].wave_its[j][0].fract(); // should z_pos have a -0.5?
        temp = ((self.oscs[j].c3[it] * z_pos + self.oscs[j].c2[it]) * z_pos + self.oscs[j].c1[it])
            * z_pos
            + self.oscs[j].c0[it];
        self.voice[i].wave_its[j][u] += x;
        if self.voice[i].wave_its[j][u] > (self.oscs[j].wave_len / 2usize.pow(mip as u32)) as f32 {
            //loop back around zero.
            self.voice[i].wave_its[j][u] -= (self.oscs[j].wave_len / 2usize.pow(mip as u32)) as f32;
        }
        return temp;
    }
}
impl<'a> Default for Voiceset<'a> {
    fn default() -> Voiceset<'a> {
        let filters = vec![filter::LadderFilter::default(); 8];
        let modenv = modmatrix::Env::default();
        let mod_env_params = modenv.params.clone();
        let filter_params = filters.iter().map(|f| f.params.clone()).collect();
        // creates the wavetable oscillators
        let tables = crate::resources::tables().unwrap();
        let mut osc1: interp::WaveTable = Default::default();
        osc1.change_table(&tables[0]);
        let mut osc2: interp::WaveTable = Default::default();
        osc2.change_table(&tables[0]);
        // creates the graintable oscillator and gets access to its parameter object
        let mut poly_iir = resampling::HalfbandFilter::default();
        //sets the halfband filter to 8th order steep
        poly_iir.setup(8, true);
        let a = Voiceset {
            poly_iir: poly_iir,
            oscs: vec![osc1, osc2],
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
                pitch_offsets: vec![
                    AtomicF32::new(1.),
                    AtomicF32::new(1.001),
                    AtomicF32::new(0.999),
                    AtomicF32::new(1.002),
                    AtomicF32::new(0.998),
                    AtomicF32::new(1.004),
                    AtomicF32::new(0.996),
                ],
                modenv_params: mod_env_params,
                filter_params: filter_params,
                pos: vec![AtomicUsize::new(0), AtomicUsize::new(0)],
                octave: vec![AtomicI8::new(0), AtomicI8::new(0)],
                cutoff_amount: AtomicF32::new(0.0),
                g_uvoices: AtomicUsize::new(1),
                vol: vec![AtomicF32::new(1.), AtomicF32::new(0.)],
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
    pos_offsets: Vec<f32>,
    pitch_offsets: Vec<f32>,
    pub ratio: f32,
    pub grain_ratio: f32,
    pub(crate) wavetable_mip: usize,
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
        self.reset_its();
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
            pos_offsets: vec![1., 1.012, 0.988, 1.005, 0.994, 1.008, 0.992],
            pitch_offsets: vec![1., 1.001, 0.999, 1.002, 0.998, 1.004, 0.996],
            wavetable_mip: 0,
            grain_mip: 0,
            ratio: 1.,
            grain_ratio: 1.,
            note: None,
            time: 0,
        };
        return a;
    }
}
