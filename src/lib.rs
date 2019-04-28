
#[allow(dead_code)]
pub struct Interp
{

    pub(crate) source_y : Vec<f32>,
    pub(crate) waveforms : Vec<Vec<f32>>,
    wave_number : usize,
    wave_len : usize,
    pub(crate) len : usize,
    pub(crate) times_oversampled : usize,
    pub(crate) new_len : usize,
    //coeffs : Vec<Vec<f32>>, //hopefully this can make 2 vectors of f32
    c0: Vec<f32>,
    c1: Vec<f32>,
    c2: Vec<f32>,
    c3: Vec<f32>,
    pub(crate) interpolated: Vec<f32>,
    upsample_fir : Vec<f32>,
}
#[allow(dead_code)]
impl Interp
{
    pub(crate) fn oversample(&mut self, ratio: usize){
        self.times_oversampled = ratio;
        self.len = self.source_y.len();
        let mut temp = vec![0.];
        temp.resize(self.len * ratio, 0.);
        for i in 0..(self.len * ratio) {
            if i % 2 == 0 {
                temp[i] = self.source_y[i/2];
            }
            else {
                temp[i] = 0.;
            }
        }
        //self.len = self.source_y.len();
        //resize source y to fit the new length
        self.source_y.resize(self.len * ratio, 0.); 
        self.len = self.source_y.len();
        //convolve zero-stuffed vector with coefficients (sinc) of a fir, to remove mirror images above new_Fs/4 
        //upsample_fir could be turned into a polyphase implementation, to halve number of clock cycles needed
        self.source_y = self.convolve(&self.upsample_fir, temp);
        
    }

    pub(crate) fn slice(&mut self) {
        self.len = self.source_y.len();
        self.wave_len = 2048;
        for i in 0..self.wave_number {
            for j in 0..self.wave_len {
                self.waveforms[i][j] = self.source_y[j + self.wave_len * i];

            }
        }
    }
    //check for off-by-ones at some point. self.len should be fine instead of len_x
    pub(crate) fn calc_coefficients(&mut self) {
        self.len = self.source_y.len();
        let len_x = self.len - 1;
        /*
        // 4-point, 3rd-order Hermite (x-form)
        float c0 = y[0];
        float c1 = 1/2.0*(y[1]-y[-1]);
        float c2 = y[-1] - 5/2.0*y[0] + 2*y[1] - 1/2.0*y[2];
        float c3 = 1/2.0*(y[2]-y[-1]) + 3/2.0*(y[0]-y[1]);
        return ((c3*x+c2)*x+c1)*x+c0;
        */
        self.c0.resize(self.len, 0.);
        self.c1.resize(self.len, 0.);
        self.c2.resize(self.len, 0.);
        self.c3.resize(self.len, 0.);


        //this could easily be optimized away, but oh well
        for i in 0..len_x {
                self.c0[i] = self.source_y[i];
            }
        //instead of len_x it should be 0+cyclelength. doesn't seem to be a big problem
        self.c1[0] =  1./2.0*(self.source_y[0+1] - self.source_y[len_x]);
        self.c2[0] =  self.source_y[len_x] - 5./2.0*self.source_y[0] + 2.*self.source_y[0+1] - 1.0/2.0*self.source_y[0+2];
        self.c3[0] =  1./2.0*(self.source_y[0+2]-self.source_y[len_x]) + 3.0/2.0*(self.source_y[0+0]-self.source_y[0+1]);
        for i in 1..len_x {
                self.c1[i] =  1./2.0*(self.source_y[i+1] -self.source_y[i-1]);
            }
        for i in 1..len_x-1 {
                self.c2[i] =  self.source_y[i-1] - 5./2.0*self.source_y[i] + 2.*self.source_y[i+1] - 1.0/2.0*self.source_y[i+2];
            }
        for i in 1..len_x-1 {
                self.c3[i] =  1./2.0*(self.source_y[i+2]-self.source_y[i-1]) + 3.0/2.0*(self.source_y[i+0]-self.source_y[i+1]);
            }
        //instead of 0 it should be len_x-cyclelength.
        self.c2[len_x] =  self.source_y[len_x-1] - 5./2.0*self.source_y[len_x] + 2.*self.source_y[0] - 1.0/2.0*self.source_y[1];
        self.c3[len_x] =  1./2.0*(self.source_y[1]-self.source_y[len_x-1]) + 3.0/2.0*(self.source_y[len_x+0]-self.source_y[0]);
    }

    pub(crate) fn interpolation(&mut self,ratio: f32 ) {
        let mut temp : f32;
        let mut it : usize;
        //find et x-array ud fra hvor mange samples der skal til for at nå den ratio
        self.new_len = ((self.len as f32) * ratio).round() as usize;
        //resize the vector to the new size
        self.interpolated.resize(self.new_len, 0.);

        //this should describe all the necessary values of x, since x always should be between 0 and 1
        let x = 1. / ratio;
        for i in 0..(self.new_len -1) {
            it = ((i as f32) * x) as usize;
            temp = ((self.c3[it]*x+self.c2[it])*x+self.c1[it])*x+self.c0[it];
            if temp < 0. {
                self.interpolated[i] = temp.max(-1.);
            }
            else {
                self.interpolated[i] = temp.max(-1.);
            }
        }
    }
    pub(crate) fn convolve(&self, p_coeffs : &Vec<f32>, p_in : Vec<f32>) -> Vec<f32>
    {
       
        let mut convolved : Vec<f32>;
        let new_len = p_in.len() + (p_coeffs.len() - 1)/2 ;
        convolved = vec![0.;p_in.len() + p_coeffs.len()];
        //convolved.resize(p_in.len() + p_coeffs.len(), 0.);
        let mut temp = p_in.to_vec();
        temp.resize(new_len, 0.);
        //n should be the length of p_in + length of p_coeffs
        for k in 0..(new_len)  //  position in output
        {
            //convolved[k] = 0.;

            for i in 0..p_coeffs.len()  //  position in coefficients array
            {   //this might even take care of group-delay
                if k >= i //&& k < p_in.len()
                {
                    convolved[k] += p_coeffs[i] * temp[k - i];

                }

            }
        }
        //trimming the sample
        //remove initial group delay by taking number of coefficients - 1 / 2. Only works for number of coefficients
        for _i in 0..(p_coeffs.len() - 1)/2 {
            convolved.remove(0); //maybe use drain on an iterator instead?
        }
        //trims unnecessary sample at the end
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
            times_oversampled : 0,
            wave_len : 2048,
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
