//stage-focused?
//look-up tables could potentially be way faster
/*
By the way that formula for envelopes is

y = x e^(k(x-1)), convex

y = 1 - (1-x) e ^ (k(1-x)), concave
k >= 0

phase modulation on a linear envelope could give slope control, and only need one stage (reversed for release and limited for decay)

*/
use crate::util::{AtomicF32, AtomicUsize};
use std::sync::Arc;
pub struct EnvParams {
    pub attack_time: AtomicUsize,
    pub decay_time: AtomicUsize,
    pub sustain: AtomicF32,
    pub release_time: AtomicUsize,
    pub attack_slope: AtomicF32,
    pub decay_slope: AtomicF32,
    pub release_slope: AtomicF32,
}
impl Default for EnvParams {
    fn default() -> EnvParams {
        EnvParams {
            attack_time: AtomicUsize::new(882),
            decay_time: AtomicUsize::new(8820),
            sustain: AtomicF32::new(1.0),
            release_time: AtomicUsize::new(882),
            attack_slope: AtomicF32::new(0.6),
            decay_slope: AtomicF32::new(0.5),
            release_slope: AtomicF32::new(0.6),
            // attack_time: 882, //882 samples is 20ms
            // attack_slope: 0.6,
            // decay_time: 8820, //8820 samples is 200ms
            // decay_slope: 0.5,
            // sustain: 0.5,
            // release_time: 882, //882 samples is 20ms
            // release_slope: 0.6,
        }
    }
}

#[allow(dead_code)]
pub struct Env {
    pub output: f32,
    pub time: Vec<usize>, //time in samples
    pub note: Vec<bool>,
    pub decay_end: f32,
    pub params: Arc<EnvParams>,
}
impl Env {
    pub fn restart_env(&mut self, voice: usize) {
        self.time[voice] = 0;
        self.note[voice] = true;
    }
    // fn fill_attack(&mut self) {
    //     let max = (self.attack.len() as f32).powf(self.attack_slope);
    //     for x in 0..self.attack.len() {
    //         self.attack[x] = (x as f32).powf(self.attack_slope)/max;
    //     }

    //(1..self.attack.len()).((2.718f32.powf(x as f32 * self.attack_slope) - 1.)/max)

    //}
    // fn fill_decay(&mut self) {
    //     let max = (self.decay.len() as f32).powf(self.decay_slope);
    //     let attack_end = self.attack[self.attack.len() - 1];
    //     for x in 0..self.decay.len() {
    //         self.decay[x] = attack_end - (x as f32).powf(self.decay_slope)* (attack_end - self.sustain)/max;
    //     }
    // }
    // fn fill_release(&mut self) {
    //     let max = (self.release.len() as f32).powf(self.release_slope);
    //     let decay_end = self.decay[self.decay.len() - 1];
    //     for x in 0..self.release.len() {
    //         self.decay[x] = decay_end - (x as f32).powf(self.release_slope)/max;
    //     }
    // }
    pub fn next(&mut self, voice: usize) -> Option<f32> {
        let output: f32;
        if self.note[voice] {
            if self.time[voice] < self.params.attack_time.get() {
                let max =
                    (self.params.attack_time.get() as f32).powf(self.params.attack_slope.get());
                output = (self.time[voice] as f32).powf(self.params.attack_slope.get()) / max;
                self.time[voice] += 1;
            } else if self.time[voice]
                < self.params.attack_time.get() + self.params.decay_time.get()
            {
                let attack_end = 1.; //could be made a parameter
                let max = (self.params.decay_time.get() as f32).powf(self.params.decay_slope.get());
                output = attack_end
                    - ((self.time[voice] - self.params.attack_time.get()) as f32)
                        .powf(self.params.decay_slope.get())
                        * (attack_end - self.params.sustain.get())
                        / max;
                self.time[voice] += 1;
            } else {
                output = self.params.sustain.get();
            }
        } else {
            //moves to release stage forcibly
            if self.time[voice] < self.params.attack_time.get() + self.params.decay_time.get() {
                //we set a decay end here, to make sure release is always smooth.
                self.note[voice] = true;
                self.time[voice] -= 1;
                self.decay_end = self.next(voice).unwrap();
                self.note[voice] = false;
                self.time[voice] = self.params.attack_time.get() + self.params.decay_time.get();
            }
            //if envelope is done, we can return None to tell the rest that it's done
            if self.time[voice] - self.params.attack_time.get() - self.params.decay_time.get()
                >= self.params.release_time.get()
            {
                self.decay_end = 0.;
                return None;
            } else {
                let max =
                    (self.params.release_time.get() as f32).powf(self.params.release_slope.get());
                let decay_end: f32;
                if self.decay_end != 0. {
                    decay_end = self.decay_end;
                } else {
                    decay_end = self.params.sustain.get();
                }
                output = decay_end
                    - ((self.time[voice]
                        - self.params.attack_time.get()
                        - self.params.decay_time.get()) as f32)
                        .powf(self.params.release_slope.get())
                        * decay_end
                        / max;
                self.time[voice] += 1;
            }
        }
        return Some(output);
    }
}
// impl Iterator for Env {
//     type Item = f32;
//     fn next (&mut self) -> Option<f32> {
//         let output : f32;
//         if self.note != None {
//             if self.time < self.attack_time{
//                 let max = (self.attack_time as f32).powf(self.attack_slope);
//                 output = (self.time as f32).powf(self.attack_slope)/max;
//                 self.time += 1;
//             }
//             else  if self.time < self.attack_time + self.decay_time {
//                 let max = (self.decay_time as f32).powf(self.decay_slope);
//                 let attack_end = 1.; //could be made a parameter
//                 output = attack_end - (self.time as f32).powf(self.decay_slope) *
//                 (attack_end - self.sustain)/max;
//                 self.time += 1;
//             }
//             else {
//                 output = self.sustain;
//             }
//         }
//         else {
//             if self.time - self.attack_time - self.decay_time >= self.release_time {
//                 return None;
//             }
//             else {
//                 let max = (self.release_time as f32).powf(self.release_slope);
//                 let decay_end = self.sustain; //figure out how to get the proper decay end
//                 output = decay_end - (self.time as f32).powf(self.release_slope)/max;
//                 self.time += 1;
//             }
//         }
//         return Some(output);
//     }
// }

impl Default for Env {
    fn default() -> Env {
        Env {
            output: 0.,
            time: vec![100000; 8],
            params: Arc::new(EnvParams::default()),
            note: vec![false; 8],
            decay_end: 0.,
        }
    }
}
