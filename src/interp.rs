use std::f32;
use std::collections::VecDeque;
#[allow(dead_code)]

/*
        Big alias problem now. SNR at 1 kHz is about -30 dB
*/


pub struct Interp
{
    pub(crate) source_y : Vec<f32>,
    pub(crate) waveforms : Vec<Vec<f32>>,
    pub(crate) wave_number : usize,
    pub wave_len : usize,
    pub(crate) len : usize,
    pub(crate) amt_oversample : usize,
    pub(crate) new_len : usize,
    //coeffs : Vec<Vec<f32>>, //hopefully this can make 2 vectors of f32
    c0: Vec<Vec<f32>>, c1: Vec<Vec<f32>>, c2: Vec<Vec<f32>>, c3: Vec<Vec<f32>>,
    fir_len : usize,
    pub(crate) interp_buffer: VecDeque<f32>,
    pub it : usize,
    pub it_unrounded : f32,
    pub pos : usize,
    pub(crate) upsample_fir : Vec<f32>,
    //mips : Vec<Vec<Vec<f32>>>,
}
#[allow(dead_code)]
impl Interp
{
    //fills a buffer we can use for fir filtering. Has to be called every time a midi on message happens
    //or continually be filled on both note on and note off.
    pub(crate) fn prep_buffer(&mut self, _ratio: f32) {
        self.interp_buffer.resize(self.upsample_fir.len() + 1, 0.);
        for i in 0..self.upsample_fir.len() -1 {
            self.interp_buffer[i] = 0.;
        }
    }
    //needs downsampling. We might require more antialiasing
    pub fn step_one(& mut self, ratio: f32) -> f32 {
        //first we filter our sample based on our output
        let mut output = 0.;
        let unfiltered_new = self.optimal_interp(ratio);
        //downsampling for loop
        for _i in 0..self.amt_oversample
        {
            output = self.single_convolve(&self.upsample_fir);
            //removes the sample that just got filtered
            self.interp_buffer.pop_front();
            //adds a new unfiltered sample to the end
            
            self.interp_buffer.push_back(unfiltered_new);
        }
        return output;
    }


    pub(crate) fn oversample(&mut self, ratio: usize){
        self.amt_oversample = ratio;
        //resize slices to fit the new length
        for i in 0..self.wave_number {
            self.waveforms[i].resize(self.wave_len * ratio, 0.);
        }
        let mut temp = vec![0.];
        temp.resize(self.wave_len * ratio, 0.);
        for i in 0..self.wave_number {
            //fills temp with an oversampled version of current slice
            for j in 0..(self.wave_len * ratio) {
                if j % ratio == 0 {
                    temp[j] = self.waveforms[i][j/ratio];
                }
                else {
                    temp[j] = 0.;
                }
            }
            self.waveforms[i] = temp.to_vec();
        }
        //static_convolve zero-stuffed vector with coefficients (sinc) of a fir, to remove mirror images above new_Fs/4 
        //upsample_fir could be turned into a polyphase implementation, to halve number of clock cycles needed
        for i in 0..self.wave_number {
            self.waveforms[i] = self.static_convolve(&self.upsample_fir, &self.waveforms[i]);
        }
    }
    //slices the read .wav into individual waveforms
    pub(crate) fn slice(&mut self) {
        self.len = self.source_y.len();
        self.wave_len = 2048; 
        self.wave_number = self.len / self.wave_len;
        self.waveforms.resize(self.wave_number,vec![0.;2048]);
        for i in 0..self.wave_number {
            for j in 0..self.wave_len {
                self.waveforms[i][j] = self.source_y[j + self.wave_len * i];

            }
        }
    }
    //check for off-by-ones at some point. self.len should be fine instead of len_x
    pub(crate) fn hermite_coeffs(&mut self) {
        self.len = self.source_y.len();
        let new_wave_len = self.wave_len *self.amt_oversample;
        /*
        // 4-point, 3rd-order Hermite (x-form)
        float c0 = y[0];
        float c1 = 1/2.0*(y[1]-y[-1]);
        float c2 = y[-1] - 5/2.0*y[0] + 2*y[1] - 1/2.0*y[2];
        float c3 = 1/2.0*(y[2]-y[-1]) + 3/2.0*(y[0]-y[1]);
        return ((c3*x+c2)*x+c1)*x+c0;
        */ 
        self.c0.resize(self.wave_number,vec![0.;new_wave_len]);
        self.c1.resize(self.wave_number,vec![0.;new_wave_len]);
        self.c2.resize(self.wave_number,vec![0.;new_wave_len]);
        self.c3.resize(self.wave_number,vec![0.;new_wave_len]);
        //this could easily be optimized away, but oh well
        for i in 0..self.wave_number {
            for j in 0..new_wave_len {
                self.c0[i][j] = self.waveforms[i][j];
            }
        }
        //instead of len_x it should be 0+cyclelength. doesn't seem to be a big problem
        //self.c1[0] =  1./2.0*(self.source_y[0+1] - self.source_y[len_x]);
        //self.c2[0] =  self.source_y[len_x] - 5./2.0*self.source_y[0] + 2.*self.source_y[0+1] - 1.0/2.0*self.source_y[0+2];
        //self.c3[0] =  1./2.0*(self.source_y[0+2]-self.source_y[len_x]) + 3.0/2.0*(self.source_y[0+0]-self.source_y[0+1]);
        for i in 0..self.wave_number {
            for j in 1..new_wave_len - 1 {
                
                self.c1[i][j] =  1./2.0*(self.waveforms[i][j+1] -self.waveforms[i][j-1]);
            }
        }
        for i in 0..self.wave_number {
            for j in 1..new_wave_len - 2 {
                self.c2[i][j] =  self.waveforms[i][j-1] - 5./2.0*self.waveforms[i][j] + 2.*self.waveforms[i][j+1] - 1.0/2.0*self.waveforms[i][j+2];
            }
        }
        for i in 0..self.wave_number {
            for j in 1..new_wave_len - 2 {
                self.c3[i][j] =  1./2.0*(self.waveforms[i][j+2]-self.waveforms[i][j-1]) + 3.0/2.0*(self.waveforms[i][j+0]-self.waveforms[i][j+1]);
            }
        }
        //makes sure the start of waveforms are handled properly
        for i in 0..self.wave_number {
            self.c1[i][0] =  (1.0/2.0)*(self.waveforms[i][0+1] - self.waveforms[i][new_wave_len  - 1]);
            self.c2[i][0] =  self.waveforms[i][new_wave_len  - 1] - (5./2.0)*self.waveforms[i][0] + 2.*self.waveforms[i][0+1] - (1.0/2.0)*self.waveforms[i][0+2];
            self.c3[i][0] =  (1.0/2.0)*(self.waveforms[i][0+2]-self.waveforms[i][new_wave_len  - 1]) + (3.0/2.0)*(self.waveforms[i][0]-self.waveforms[i][0+1]);
        }
        //makes sure the end of waveforms are handled properly
        for i in 0..self.wave_number {
            self.c1[i][new_wave_len  - 1] =  1./2.0*(self.waveforms[i][0] - self.waveforms[i][new_wave_len - 2]);
            self.c2[i][new_wave_len - 1] =  self.waveforms[i][new_wave_len - 2] - 5./2.0*self.waveforms[i][new_wave_len - 1] + 2.*self.waveforms[i][0] - 1.0/2.0*self.waveforms[i][0+1];
            self.c2[i][new_wave_len - 2] =  self.waveforms[i][new_wave_len - 3] - 5./2.0*self.waveforms[i][new_wave_len - 2] + 2.*self.waveforms[i][new_wave_len - 1] - 1.0/2.0*self.waveforms[i][0];
            self.c3[i][new_wave_len - 1] =  1./2.0*(self.waveforms[i][0+1]-self.waveforms[i][new_wave_len - 2]) + 3.0/2.0*(self.waveforms[i][new_wave_len - 1]-self.waveforms[i][0]);
            self.c3[i][new_wave_len - 2] =  1./2.0*(self.waveforms[i][0]-self.waveforms[i][new_wave_len - 3]) + 3.0/2.0*(self.waveforms[i][new_wave_len - 2]-self.waveforms[i][new_wave_len - 1]);
        }
        
    }
    //consider fixing some discontinuities by just setting first and last sample to original lol
    //only interpolates one waveform at a time. This means for now that you cant change waveform without changing notes.
    pub(crate) fn interpolation(&mut self,ratio: f32 ) {
        let mut temp : f32;
        let mut it : usize;
        //find et x-array ud fra hvor mange samples der skal til for at nå den ratio
        //self.new_len = ((self.len as f32) * ratio).round() as usize;
        self.new_len = ((self.wave_len as f32) * self.amt_oversample as f32 *ratio).round() as usize;
        //resize the vector to the new size
        self.interp_buffer.resize(self.new_len, 0.);
        //this should describe all the necessary values of x, since x always should be between 0 and 1
        let x = 1. / ratio;
        let x_pos = x.fract();
        for i in 0..(self.new_len -1) {
            it = ((i as f32) * x) as usize;
            if i >= self.new_len - 5 {
                println!("{}",it);
            }
            temp = ((self.c3[self.pos][it]*x_pos+self.c2[self.pos][it])*x_pos+self.c1[self.pos][it])*x_pos+self.c0[self.pos][it];
            self.interp_buffer[i] = temp;
            
        }
    }
    pub(crate) fn optimal_coeffs(&mut self) {
        self.len = self.source_y.len();
        let new_wave_len = self.wave_len *self.amt_oversample;
        /*
        // Optimal 2x (4-point, 3rd-order) (z-form)
        float z = x - 1/2.0;
        float even1 = y[1]+y[0], odd1 = y[1]-y[0];
        float even2 = y[2]+y[-1], odd2 = y[2]-y[-1];
        float c0 = even1*0.45868970870461956 + even2*0.04131401926395584;
        float c1 = odd1*0.48068024766578432 + odd2*0.17577925564495955;
        float c2 = even1*-0.246185007019907091 + even2*0.24614027139700284;
        float c3 = odd1*-0.36030925263849456 + odd2*0.10174985775982505;
        */ 
        let mut even1; let mut even2 : f32;
        let mut odd1 : f32; let mut odd2 : f32;
        self.c0.resize(self.wave_number,vec![0.;new_wave_len]);
        self.c1.resize(self.wave_number,vec![0.;new_wave_len]);
        self.c2.resize(self.wave_number,vec![0.;new_wave_len]);
        self.c3.resize(self.wave_number,vec![0.;new_wave_len]);
        //println!("{}", self.waveforms[0][new_wave_len]);
        for i in 0..self.wave_number {
            for j in 1..new_wave_len - 2 {
                even1 = self.waveforms[i][j+1]+self.waveforms[i][j+0];
                odd1 = self.waveforms[i][j+1]-self.waveforms[i][j+0];
                even2 = self.waveforms[i][j+2]+self.waveforms[i][j-1];
                odd2 = self.waveforms[i][j+2]-self.waveforms[i][j-1];
                self.c0[i][j] = even1*0.45868970870461956 + even2*0.04131401926395584;
                self.c1[i][j] = odd1*0.48068024766578432 + odd2*0.17577925564495955;
                self.c2[i][j] = even1*-0.246185007019907091 + even2*0.24614027139700284;
                self.c3[i][j] = odd1*-0.36030925263849456 + odd2*0.10174985775982505;

            }
        }
        //makes sure the start of waveforms are handled properly
        for i in 0..self.wave_number {
            even1 = self.waveforms[i][0+1]+self.waveforms[i][0+0];
            odd1 = self.waveforms[i][0+1]-self.waveforms[i][0+0];
            even2 = self.waveforms[i][0+2]+self.waveforms[i][new_wave_len-1];
            odd2 = self.waveforms[i][0+2]-self.waveforms[i][new_wave_len-1];
            self.c0[i][0] = even1*0.45868970870461956 + even2*0.04131401926395584;
            self.c1[i][0] = odd1*0.48068024766578432 + odd2*0.17577925564495955;
            self.c2[i][0] = even1*-0.246185007019907091 + even2*0.24614027139700284;
            self.c3[i][0] = odd1*-0.36030925263849456 + odd2*0.10174985775982505;
        }
        //makes sure the end of waveforms are handled properly
        for i in 0..self.wave_number {
            even1 = self.waveforms[i][new_wave_len  - 1]+self.waveforms[i][new_wave_len  - 2];
            odd1 = self.waveforms[i][new_wave_len  - 1]-self.waveforms[i][new_wave_len  - 2];
            even2 = self.waveforms[i][0]+self.waveforms[i][new_wave_len  - 3];
            odd2 = self.waveforms[i][0]-self.waveforms[i][new_wave_len  - 3];
            self.c0[i][new_wave_len  - 2] = even1*0.45868970870461956 + even2*0.04131401926395584;
            self.c1[i][new_wave_len  - 2] = odd1*0.48068024766578432 + odd2*0.17577925564495955;
            self.c2[i][new_wave_len  - 2] = even1*-0.246185007019907091 + even2*0.24614027139700284;
            self.c3[i][new_wave_len  - 2] = odd1*-0.36030925263849456 + odd2*0.10174985775982505;

            even1 = self.waveforms[i][0]+self.waveforms[i][new_wave_len  - 1];
            odd1 = self.waveforms[i][0]-self.waveforms[i][new_wave_len  - 1];
            even2 = self.waveforms[i][1]+self.waveforms[i][new_wave_len  - 2];
            odd2 = self.waveforms[i][1]-self.waveforms[i][new_wave_len  - 2];
            self.c0[i][new_wave_len  - 1] = even1*0.45868970870461956 + even2*0.04131401926395584;
            self.c1[i][new_wave_len  - 1] = odd1*0.48068024766578432 + odd2*0.17577925564495955;
            self.c2[i][new_wave_len  - 1] = even1*-0.246185007019907091 + even2*0.24614027139700284;
            self.c3[i][new_wave_len  - 1] = odd1*-0.36030925263849456 + odd2*0.10174985775982505;
        }
        
    }
    //needs downsampling.
    //if we calculate some samples ahead and store them in a buffer, FIR filtering might still be viable.
    pub(crate) fn optimal_interp(&mut self, ratio: f32) -> f32 {
        // Optimal 2x (4-point, 3rd-order) (z-form)
        // return ((c3*z+c2)*z+c1)*z+c0;
        let temp : f32;
        let it : usize;
        let findlen = (self.wave_len * self.amt_oversample) as f32 *ratio;
        //x is the placement of the sample compared to the last one, or the slope
        //let x = ((self.wave_len * self.amt_oversample - 1 ) as f32 )/(findlen -1.);
        let x = ((self.wave_len * self.amt_oversample  ) as f32 )/(findlen);
        self.new_len = findlen as usize;
        //let z = x - 0.5;
        let z_pos; //= z.fract();
        it = self.it_unrounded.floor() as usize;
        //should z_pos have a -0.5?
        z_pos = self.it_unrounded.fract();
        temp = ((self.c3[self.pos][it]*z_pos+self.c2[self.pos][it])*z_pos+self.c1[self.pos][it])*z_pos
                +self.c0[self.pos][it];
        //self.interpolated[i] = temp;
        self.it_unrounded += x;
        if self.it_unrounded > ((self.wave_len - 1) * self.amt_oversample) as f32{
            //loop back around zero.
            self.it_unrounded -= ((self.wave_len - 1) * self.amt_oversample) as f32;
        }
        return temp;

    }
    //this is supposed to pick up after static_convolve is done. But how?
    pub(crate) fn single_convolve(&self, p_coeffs : &Vec<f32>) -> f32 {
        let mut convolved : f32;
        convolved = 0.;
        //convolved.resize(p_in.len() + p_coeffs.len(), 0.);
        //let mut temp = self.interp_buffer.to_vec();
        //temp.resize(new_len, 0.);
        //n should be the length of p_in + length of p_coeffs
        //this k value should skip the group delay?
        let k = p_coeffs.len();
        {
            for i in 0..p_coeffs.len()  //  position in coefficients array
            {   
                if k >= i 
                {
                    //println!("{}", k-i);
                    convolved += p_coeffs[i] * self.interp_buffer[k - i];
                }
            }
        }
        return convolved;
    }

    pub(crate) fn static_convolve(&self, p_coeffs : &Vec<f32>, p_in : &Vec<f32>) -> Vec<f32> {   
        //possibly more efficient convolution https://stackoverflow.com/questions/8424170/1d-linear-convolution-in-ansi-c-code
        //convolution could be significantly sped up by doing it in the frequency domain. from O(n^2) to O(n*log(n))
        let mut convolved : Vec<f32>;
        let new_len = p_in.len() + (p_coeffs.len() - 1)/2 ;
        convolved = vec![0.;p_in.len() + p_coeffs.len()];
        //convolved.resize(p_in.len() + p_coeffs.len(), 0.);
        let mut temp = p_in.to_vec();
        temp.resize(new_len, 0.);
        //n should be the length of p_in + length of p_coeffs
        for k in 0..(new_len)  //  position in output
        {
            for i in 0..p_coeffs.len()  //  position in coefficients array
            {   
                if k >= i 
                {
                    convolved[k] += p_coeffs[i] * temp[k - i];
                }
            }
        }
        //trimming the result
        //remove initial group delay by taking number of coefficients - 1 / 2. Only works for odd number of coefficients
        for _i in 0..(p_coeffs.len() - 1)/2 {
            convolved.remove(0); //maybe use drain on an iterator instead?
        }
        //trims unnecessary samples at the end
        convolved.truncate(p_in.len());
        return convolved;
    }
}
impl Default for Interp
{
    fn default() -> Interp {
        Interp {
            source_y : Vec::with_capacity(2048 * 256),
            waveforms : Vec::with_capacity(256),
            len : 0,
            new_len: 0,
            wave_number : 0,
            amt_oversample : 1,
            wave_len : 2048,
            it : 0,
            it_unrounded : 0.,
            pos: 0,
            fir_len: 179,
            //coeffs : Vec<Vec<f32>>, //hopefully this can make 2 vectors of f32
            //default capacity should take oversampling into account
            c0: Vec::with_capacity(2048 * 256 * 2),
            c1: Vec::with_capacity(2048 * 256 * 2),
            c2: Vec::with_capacity(2048 * 256 * 2),
            c3: Vec::with_capacity(2048 * 256 * 2),
            interp_buffer: VecDeque::with_capacity(180),
            
            upsample_fir: vec!( 5.807e-05,0.00015957,0.00017629,4.1774e-06,-0.00021049,-0.0001965,2.3485e-05,9.5114e-05,
            -0.00011663,-0.00024653,-1.8106e-05,0.00021017,3.0079e-06,-0.00032069,-0.00014625,0.00029734,0.00020506,
            -0.00034762,-0.00036956,0.00029091,0.0004813,-0.00025542,-0.00065389,0.0001256,0.00077988,1.7544e-05,
            -0.0009229,-0.00024499,0.0010065,0.00050125,-0.0010623,-0.00082416,0.0010349,0.0011698,-0.00093752,-0.0015501,
            0.00073146,0.001924,-0.00042358,-0.0022835,-8.6855e-06,0.0025865,0.00055521,-0.0028116,-0.0012206,0.0029163,
            0.0019844,-0.0028726,-0.0028315,0.0026421,0.003727,-0.0021982,-0.0046357,0.0015123,0.0055074,-0.00056789,-0.0062889,
            -0.00064776,0.0069164,0.0021342,-0.0073232,-0.0038835,0.0074351,0.005874,-0.0071756,-0.0080748,0.0064621,0.010443,
            -0.0052066,-0.012928,0.0033099,0.015469,-0.00065173,-0.018001,-0.0029281,0.020454,0.0076684,-0.022758,-0.013972,
            0.024844,0.0226,-0.026649,-0.035204,0.028117,0.056141,-0.029201,-0.10152,0.029865,0.31677,0.46991,0.31677,0.029865,
            -0.10152,-0.029201,0.056141,0.028117,-0.035204,-0.026649,0.0226,0.024844,-0.013972,-0.022758,0.0076684,0.020454,
            -0.0029281,-0.018001,-0.00065173,0.015469,0.0033099,-0.012928,-0.0052066,0.010443,0.0064621,-0.0080748,-0.0071756,
            0.005874,0.0074351,-0.0038835,-0.0073232,0.0021342,0.0069164,-0.00064776,-0.0062889,-0.00056789,0.0055074,0.0015123,
            -0.0046357,-0.0021982,0.003727,0.0026421,-0.0028315,-0.0028726,0.0019844,0.0029163,-0.0012206,-0.0028116,0.00055521,
            0.0025865,-8.6855e-06,-0.0022835,-0.00042358,0.001924,0.00073146,-0.0015501,-0.00093752,0.0011698,0.0010349,
            -0.00082416,-0.0010623,0.00050125,0.0010065,-0.00024499,-0.0009229,1.7544e-05,0.00077988,0.0001256,-0.00065389,
            -0.00025542,0.0004813,0.00029091,-0.00036956,-0.00034762,0.00020506,0.00029734,-0.00014625,-0.00032069,3.0079e-06,
            0.00021017,-1.8106e-05,-0.00024653,-0.00011663,9.5114e-05,2.3485e-05,-0.0001965,-0.00021049,4.1774e-06,0.00017629,
            0.00015957,5.807e-05),
        }
    }
}
