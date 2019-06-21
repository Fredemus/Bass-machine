//stage-focused?
//look-up tables could potentially be way faster
/*
By the way that formula for envelopes is 

y = x e^(k(x-1)), convex 

y = 1 - (1-x) e ^ (k(1-x)), concave
k >= 0

phase modulation on a linear envelope could give slope control, and only need one stage (reversed for release and limited for decay)

*/

#[allow(dead_code)]
pub struct Env {
    pub output: f32,
    pub attack_time : usize,
    pub decay_time : usize,
    pub sustain : f32,
    pub release_time : usize,
    pub time : Vec<usize>, //time in samples
    pub attack_slope : f32,
    pub decay_slope : f32,
    pub release_slope : f32,
    pub note : Vec<bool>,
    pub decay_end : f32
}
impl Env {
    pub fn restart_env(&mut self, voice : usize) {
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
    pub fn next (&mut self, voice : usize) -> Option<f32> {
        let output : f32;
        if self.note[voice] {
            if self.time[voice] < self.attack_time{
                let max = (self.attack_time as f32).powf(self.attack_slope);
                output = (self.time[voice] as f32).powf(self.attack_slope)/max;
                self.time[voice] += 1;
            } 
            else if self.time[voice] < self.attack_time + self.decay_time {
                let attack_end = 1.; //could be made a parameter
                let max = (self.decay_time as f32).powf(self.decay_slope);
                output = attack_end - ((self.time[voice] - self.attack_time) as f32).powf(self.decay_slope) * 
                (attack_end - self.sustain)/max;
                self.time[voice] += 1;
            }
            else {
                output = self.sustain;
            }
        }
        else {
            //moves to release stage forcibly
            if self.time[voice] < self.attack_time + self.decay_time { 
                //we set a decay end here, to make sure release is always smooth.
                self.note[voice] = true;
                self.time[voice] -= 1;
                self.decay_end = self.next(voice).unwrap();
                self.note[voice] = false;
                self.time[voice] = self.attack_time + self.decay_time; 
            }
            //if envelope is done, we can return None to tell the rest that it's done
            if self.time[voice] - self.attack_time - self.decay_time >= self.release_time {
                self.decay_end = 0.;
                return None;
            }
            else {
                let max = (self.release_time as f32).powf(self.release_slope);
                let decay_end : f32;
                if self.decay_end != 0. {
                    decay_end = self.decay_end;
                }
                else {
                    decay_end = self.sustain; 
                }
                output = decay_end - 
                ((self.time[voice] - self.attack_time - self.decay_time) as f32).powf(self.release_slope)
                * decay_end / max;
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
        time : vec![100000;8],
        attack_time : 882, //882 samples is 20ms
        attack_slope : 0.6,
        decay_time : 8820, //8820 samples is 200ms
        decay_slope : 0.5,
        sustain : 0.5,
        release_time : 882, //882 samples is 20ms
        release_slope : 0.6,
        note : vec![false; 8],
        decay_end : 0.,
        }
    }
}
