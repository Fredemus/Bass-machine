

pub struct Interp
{

    pub(crate) source_y : Vec<f32>,
    pub len : usize,
    pub(crate) new_len : usize,
    //coeffs : Vec<Vec<f32>>, //hopefully this can make 2 vectors of f32
    c0: Vec<f32>,
    c1: Vec<f32>, 
    c2: Vec<f32>, 
    c3: Vec<f32>, 
    pub(crate) interpolated: Vec<f32>,
    
}

impl Interp
{
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


        
        for i in 0..len_x {
                self.c0[i] = self.source_y[i];
            }
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
        self.c2[len_x] =  self.source_y[len_x-1] - 5./2.0*self.source_y[len_x] + 2.*self.source_y[0] - 1.0/2.0*self.source_y[1];
        self.c3[len_x] =  1./2.0*(self.source_y[1]-self.source_y[len_x-1]) + 3.0/2.0*(self.source_y[len_x+0]-self.source_y[0]);
    }

    pub(crate) fn cubic_interpolation(&mut self,ratio: f32 ) {
        
        let mut it : usize;
        //find et x-array ud fra hvor mange samples der skal til for at nå den ratio
        self.new_len = ((self.len as f32) * ratio).round() as usize; 
        //resize the vector to the new size
        self.interpolated.resize(self.new_len + 1, 0.);

        //this should describe all the necessary values of x, since x always should be between 0 and 1
        let x = 1. / ratio;
        for i in 0..(self.new_len -1) {
            it = ((i as f32) * x) as usize;
            self.interpolated[i] = ((self.c3[it]*x+self.c2[it])*x+self.c1[it])*x+self.c0[it];
        }
    }
}
impl Default for Interp 
{
    fn default() -> Interp {
        Interp {
            source_y : Vec::with_capacity(2048 * 256),
            len : 0,
            new_len: 0,
            //coeffs : Vec<Vec<f32>>, //hopefully this can make 2 vectors of f32
            c0: Vec::with_capacity(2048 * 256),
            c1: Vec::with_capacity(2048 * 256),
            c2: Vec::with_capacity(2048 * 256),
            c3: Vec::with_capacity(2048 * 256),
            interpolated: Vec::with_capacity(2048 * 256),
        }
    }
}
