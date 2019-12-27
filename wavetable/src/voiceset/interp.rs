use crate::resources::Table;

mod fir;

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
        10 => 1.998046875,
        _ => 0.,
    };
    (len as f32 * amount) as usize
}

#[derive(Clone)]
pub struct WaveTable<'a> {
    source_y: Vec<f32>,
    pub(crate) waveforms: Vec<f32>,
    /// the number of waveforms in the current wavetable
    pub(crate) wave_number: usize,
    pub wave_len: usize,
    pub(crate) len: usize,
    pub(crate) amt_oversample: usize,

    pub c0: Vec<f32>,
    pub c1: Vec<f32>,
    pub c2: Vec<f32>,
    pub c3: Vec<f32>,

    pub(crate) upsample_fir: &'a [f32],
    pub(crate) downsample_fir: &'a [f32],
    pub mips: Vec<f32>,
    mip_levels: usize,
}
#[allow(dead_code)]
impl<'a> WaveTable<'a> {
    pub fn change_table(&mut self, table: &Table) {
        self.source_y = table.clone().load().unwrap();
        // self.slice();
        self.len = self.source_y.len();
        self.wave_len = 2048;
        self.wave_number = self.len / self.wave_len;
        self.oversample(2);
        self.mip_map();
        self.optimal_coeffs();
    }
    pub fn mip_len(&self, mip: usize) -> usize {
        (self.source_y.len() as f32 / 2f32.powi(mip as i32)) as usize
    }
    pub(crate) fn oversample(&mut self, ratio: usize) {
        self.amt_oversample = ratio;
        self.wave_len *= ratio;
        self.source_y.resize(self.source_y.len() * ratio, 0.);
        let mut temp = vec![0.];
        temp.resize(self.source_y.len(), 0.);
        // zero-stuffing
        for j in 0..(self.source_y.len()) {
            if j % ratio == 0 {
                temp[j] = self.source_y[j / ratio];
            } else {
                temp[j] = 0.;
            }
        }
        self.source_y = temp.to_vec();
        //static_convolve zero-stuffed vector with coefficients (sinc) of a fir, to remove mirror images above new_Fs/4
        //static_convolve could be optimised to halve number of multiplications needed
        let mut temp: Vec<f32> = Vec::new();
        //temp.resize(self.source_y.len(), 0.);
        for i in 0..self.wave_number {
            temp.append(&mut self.static_convolve(
                self.upsample_fir,
                &(&self.source_y[i * self.wave_len..(i + 1) * self.wave_len]).to_vec(),
            ));
        }
        self.source_y = temp;
    }
    pub(crate) fn downsample_2x(&self, signal: &Vec<f32>) -> Vec<f32> {
        // first we filter the signal to downsample 2x
        let temp = self.static_convolve(self.downsample_fir, &signal);
        let mut output = vec![0.];
        output.resize(temp.len() / 2, 0.);
        // then we remove every second sample
        for j in 0..(signal.len() / 2) {
            output[j] = temp[j * 2];
        }
        output
    }

    pub(crate) fn mip_map(&mut self) {
        // fill first layer with self.source_y
        let len = self.source_y.len();
        let mut temp: Vec<f32> = self.source_y.clone();
        // fills the mip_levels with continually more downsampled vectors
        for j in 0..self.mip_levels {
            for i in 0..self.wave_number {
                temp.append(
                    &mut self.downsample_2x(
                        &(&temp[i * self.wave_len / 2usize.pow(j as u32) + mip_offset(j, len)
                            ..(i + 1) * self.wave_len / 2usize.pow(j as u32) + mip_offset(j, len)])
                            .to_vec(),
                    ),
                );
            }
        }
        self.mips = temp;
    }
    //slices the read .wav into individual waveforms.
    //source_y could be avoided, by letting it take a reference to the read .wav instead
    // pub(crate) fn slice(&mut self) {
    //     self.len = self.source_y.len();
    //     self.wave_len = 2048;
    //     self.wave_number = self.len / self.wave_len;
    //     self.waveforms.resize(self.wave_number, vec![0.; 2048]);
    //     for i in 0..self.wave_number {
    //         for j in 0..self.wave_len {
    //             self.waveforms[i][j] = self.source_y[j + self.wave_len * i];
    //         }
    //     }
    // }
    pub(crate) fn optimal_coeffs(&mut self) {
        self.len = self.source_y.len();
        let len = self.len;
        // let new_wave_len = self.wave_len;
        let mut even1: f32;
        let mut even2: f32;
        let mut odd1: f32;
        let mut odd2: f32;
        self.c0.resize(self.source_y.len() * 2, 0.);
        self.c1.resize(self.source_y.len() * 2, 0.);
        self.c2.resize(self.source_y.len() * 2, 0.);
        self.c3.resize(self.source_y.len() * 2, 0.);

        for _n in 0..self.mip_levels {
            //n represent mip-map levels
            for _i in 0..self.wave_number {
                for j in 1..self.wave_len / 2usize.pow(_n as u32) - 2 {
                    let i = _i * self.wave_len / 2usize.pow(_n as u32);
                    let n = mip_offset(_n, len);

                    even1 = self.mips[n + i + j + 1] + self.mips[n + i + j];
                    odd1 = self.mips[n + i + j + 1] - self.mips[n + i + j];
                    even2 = self.mips[n + i + j + 2] + self.mips[n + i + j - 1];
                    odd2 = self.mips[n + i + j + 2] - self.mips[n + i + j - 1];
                    self.c0[n + i + j] = even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
                    self.c1[n + i + j] = odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
                    self.c2[n + i + j] =
                        even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
                    self.c3[n + i + j] = odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;
                }
            }
        }
        //makes sure the start of waveforms are handled properly
        for _n in 0..self.mip_levels {
            let j = self.wave_len / 2usize.pow(_n as u32);
            for _i in 0..self.wave_number {
                let i = _i * self.wave_len / 2usize.pow(_n as u32);
                let n = mip_offset(_n, len);
                even1 = self.mips[n + i + 1] + self.mips[n + i];
                odd1 = self.mips[n + i + 1] - self.mips[n + i];
                even2 = self.mips[n + i + 2] + self.mips[n + i + j - 1];
                odd2 = self.mips[n + i + 2] - self.mips[n + i + j - 1];
                self.c0[n + i] = even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
                self.c1[n + i] = odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
                self.c2[n + i] = even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
                self.c3[n + i] = odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;
            }
        }
        //makes sure the end of waveforms are handled properly
        for _n in 0..self.mip_levels {
            let j = self.wave_len / 2usize.pow(_n as u32);
            for _i in 0..self.wave_number {
                let i = _i * self.wave_len / 2usize.pow(_n as u32);
                let n = mip_offset(_n, len);
                even1 = self.mips[n + i + j - 1] + self.mips[n + i + j - 2];
                odd1 = self.mips[n + i + j - 1] - self.mips[n + i + j - 2];
                even2 = self.mips[n + i] + self.mips[n + i + j - 3];
                odd2 = self.mips[n + i] - self.mips[n + i + j - 3];
                self.c0[n + i + j - 2] = even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
                self.c1[n + i + j - 2] = odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
                self.c2[n + i + j - 2] =
                    even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
                self.c3[n + i + j - 2] = odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;

                even1 = self.mips[n + i] + self.mips[n + i + j - 1];
                odd1 = self.mips[n + i] - self.mips[n + i + j - 1];
                even2 = self.mips[n + i + 1] + self.mips[n + i + j - 2];
                odd2 = self.mips[n + i + 1] - self.mips[n + i + j - 2];
                self.c0[n + i + j - 1] = even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
                self.c1[n + i + j - 1] = odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
                self.c2[n + i + j - 1] =
                    even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
                self.c3[n + i + j - 1] = odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;
            }
        }
    }

    pub(crate) fn static_convolve(&self, p_coeffs: &[f32], p_in: &[f32]) -> Vec<f32> {
        //possibly more efficient convolution https://stackoverflow.com/questions/8424170/1d-linear-convolution-in-ansi-c-code
        //convolution could be significantly sped up by doing it in the frequency domain. from O(n^2) to O(n*log(n))
        let mut convolved: Vec<f32>;
        let new_len = p_in.len() + (p_coeffs.len() - 1) / 2;
        convolved = vec![0.; p_in.len() + p_coeffs.len()];
        //convolved.resize(p_in.len() + p_coeffs.len(), 0.);
        let mut temp = p_in.to_vec();
        temp.resize(new_len, 0.);
        //n should be the length of p_in + length of p_coeffs
        for k in 0..(new_len)
        //  position in output
        {
            for i in 0..p_coeffs.len()
            //  position in coefficients array
            {
                if k >= i {
                    convolved[k] += p_coeffs[i] * temp[k - i];
                }
            }
        }
        //trimming the result
        //remove initial group delay by taking number of coefficients - 1 / 2. Only works for odd number of coefficients
        for _i in 0..(p_coeffs.len() - 1) / 2 {
            convolved.remove(0); //maybe use drain on an iterator instead?
        }
        //trims unnecessary samples at the end
        convolved.truncate(p_in.len());
        return convolved;
    }
}
impl<'a> Default for WaveTable<'a> {
    fn default() -> WaveTable<'a> {
        WaveTable {
            source_y: Vec::with_capacity(2048 * 256),
            waveforms: Vec::with_capacity(256),
            mips: Vec::with_capacity(10),
            mip_levels: 8,
            len: 0,
            wave_number: 0,
            amt_oversample: 1,
            wave_len: 2048,
            //coeffs : Vec<Vec<f32>>, //hopefully this can make 2 vectors of f32
            //default capacity should take oversampling into account
            //capacity probably needs only to be the number of mips
            c0: Vec::with_capacity(10),
            c1: Vec::with_capacity(10),
            c2: Vec::with_capacity(10),
            c3: Vec::with_capacity(10),
            upsample_fir: &fir::UPSAMPLE_FIR,
            downsample_fir: &fir::DOWNSAMPLE_FIR,
        }
    }
}
