
#[allow(dead_code)]
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
    pub(crate) interpolated: Vec<f32>,
    pub it : usize,
    pub pos : usize,
    pub(crate) upsample_fir : Vec<f32>,
    //mips : Vec<Vec<Vec<f32>>>,
}
#[allow(dead_code)]
impl Interp
{
    pub fn step_one(& mut self) -> f32 {
        if self.it >= ((self.new_len - 1 ) / self.amt_oversample )
        {
            self.it = 0
        }
        //naive downsampling for now, implement a (halfband?) filter here
        //if self.it % 2 == 0 {
            let output = self.interpolated[self.it * self.amt_oversample /*+ 
            (((self.new_len)/self.wave_number)  * self.pos)*/] ;
            //*output_sample = 1.;
            self.it += 1;
        //}
        output
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
        //convolve zero-stuffed vector with coefficients (sinc) of a fir, to remove mirror images above new_Fs/4 
        //upsample_fir could be turned into a polyphase implementation, to halve number of clock cycles needed
        for i in 0..self.wave_number {
            self.waveforms[i] = self.convolve(&self.upsample_fir, &self.waveforms[i]);
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
        // for i in 1..len_x-1 {
        //         self.c3[i] =  1./2.0*(self.source_y[i+2]-self.source_y[i-1]) + 3.0/2.0*(self.source_y[i+0]-self.source_y[i+1]);
        //     }
      
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
        self.interpolated.resize(self.new_len, 0.);
        //this should describe all the necessary values of x, since x always should be between 0 and 1
        let x = 1. / ratio;
        let x_pos = x.fract();
        for i in 0..(self.new_len -1) {
            it = ((i as f32) * x) as usize;
            temp = ((self.c3[self.pos][it]*x_pos+self.c2[self.pos][it])*x_pos+self.c1[self.pos][it])*x_pos+self.c0[self.pos][it];
            self.interpolated[i] = temp;
            //clipping ameliorates a problem with overshoots from this interpolation algorithm.
            // if temp < 0. {
            //     self.interpolated[i] = temp.max(-1.1);
            // }
            // else {
            //     self.interpolated[i] = temp.min(1.1);
            // }
        }
        //self.interpolated[0] = self.waveforms[self.pos][0];
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

    pub(crate) fn optimal_interp(&mut self, ratio: f32) {
        // Optimal 2x (4-point, 3rd-order) (z-form)
        // return ((c3*z+c2)*z+c1)*z+c0;
        let mut temp : f32;
        let mut it : usize;
        //find et x-array ud fra hvor mange samples der skal til for at nå den ratio
        //self.new_len = ((self.len as f32) * ratio).round() as usize;
        self.new_len = ((self.wave_len as f32) * self.amt_oversample as f32 *ratio).round() as usize;
        //resize the vector to the new size
        self.interpolated.resize(self.new_len, 0.);
        //this should describe all the necessary values of x, since x always should be between 0 and 1
        let x = 1. / ratio;
        let z = x - 0.5;
        let z_pos = z.fract();
        for i in 0..(self.new_len) {
            it = ((i as f32) * x) as usize;
            temp = ((self.c3[self.pos][it]*z_pos+self.c2[self.pos][it])*z_pos+self.c1[self.pos][it])*z_pos
                   +self.c0[self.pos][it];
            self.interpolated[i] = temp;
        }

        //self.interpolated[0] = self.waveforms[self.pos][0];
    }


    pub(crate) fn convolve(&self, p_coeffs : &Vec<f32>, p_in : &Vec<f32>) -> Vec<f32>
    {   //possibly more efficient convolution https://stackoverflow.com/questions/8424170/1d-linear-convolution-in-ansi-c-code
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
            pos: 0,
            //coeffs : Vec<Vec<f32>>, //hopefully this can make 2 vectors of f32
            //default capacity should take oversampling into account
            c0: Vec::with_capacity(2048 * 256 * 2),
            c1: Vec::with_capacity(2048 * 256 * 2),
            c2: Vec::with_capacity(2048 * 256 * 2),
            c3: Vec::with_capacity(2048 * 256 * 2),
            interpolated: Vec::with_capacity(2048 * 256 * 2),
            upsample_fir: vec!( 0.00012358,0.00033957,0.00037516,8.8899e-06,-0.00044795,-0.00041815,4.9978e-05,0.00020241,
            -0.0002482,-0.00052463,-3.853e-05,0.00044726,6.4009e-06,-0.00068245,-0.00031123,0.00063276,0.00043639,
            -0.00073975,-0.00078646,0.00061907,0.0010242,-0.00054354,-0.0013915,0.00026728,0.0016596,3.7334e-05,-0.001964,
            -0.00052135,0.0021419,0.0010667,-0.0022607,-0.0017539,0.0022023,0.0024895,-0.0019951,-0.0032988,0.0015566,
            0.0040944,-0.00090139,-0.0048595,-1.8483e-05,0.0055043,0.0011815,-0.0059833,-0.0025974,0.006206,0.004223,
            -0.0061131,-0.0060255,0.0056225,0.0079314,-0.0046778,-0.0098651,0.0032183,0.01172,-0.0012085,-0.013383,
            -0.0013785,0.014719,0.0045418,-0.015584,-0.0082643,0.015822,0.0125,-0.01527,-0.017184,0.013752,0.022224,
            -0.01108,-0.027512,0.0070436,0.032919,-0.0013869,-0.038307,-0.0062311,0.043527,0.016319,-0.04843,
            -0.029733,0.05287,0.048094,-0.056711,-0.074917,0.059835,0.11947,-0.062141,-0.21603,0.063554,0.6741,1.
            ,0.6741,0.063554,-0.21603,-0.062141,0.11947,0.059835,-0.074917,-0.056711,0.048094,0.05287,-0.029733,-0.04843,
            0.016319,0.043527,-0.0062311,-0.038307,-0.0013869,0.032919,0.0070436,-0.027512,-0.01108,0.022224,0.013752,
            -0.017184,-0.01527,0.0125,0.015822,-0.0082643,-0.015584,0.0045418,0.014719,-0.0013785,-0.013383,-0.0012085,
            0.01172,0.0032183,-0.0098651,-0.0046778,0.0079314,0.0056225,-0.0060255,-0.0061131,0.004223,0.006206,-0.0025974,
            -0.0059833,0.0011815,0.0055043,-1.8483e-05,-0.0048595,-0.00090139,0.0040944,0.0015566,-0.0032988,-0.0019951,
            0.0024895,0.0022023,-0.0017539,-0.0022607,0.0010667,0.0021419,-0.00052135,-0.001964,3.7334e-05,0.0016596,
            0.00026728,-0.0013915,-0.00054354,0.0010242,0.00061907,-0.00078646,-0.00073975,0.00043639,0.00063276,-0.00031123,
            -0.00068245,6.4009e-06,0.00044726,-3.853e-05,-0.00052463,-0.0002482,0.00020241,4.9978e-05,-0.00041815,-0.00044795,
            8.8899e-06,0.00037516,0.00033957,0.00012358),
        }
    }
}
