/*

    TODO:
    move fir filters somewhere sensible.
    Linear moog filter isn't calculated properly. Resonance doesn't work
    Implement unison
    Optimisation, it uses more CPU than it should. look into optimising single_conv (polyphase) 
    or single_interp
    Licensing. Look into MIT and copyleft
    https://docs.rs/nalgebra/0.3.2/nalgebra/struct.DVec3.html
    https://docs.rs/basic_dsp/0.2.0/basic_dsp/

*/
//vst stuff
 #[macro_use] extern crate vst;
use vst::api::{Events, Supported};
use vst::buffer::AudioBuffer;
use vst::event::Event;
use vst::plugin::{CanDo, Info, Plugin, Category};  

//used for handling .wav files
extern crate hound;      

//include interpolation module:
mod interp;

struct WaveTable
{
    note: Option<u8>,
    note_duration: f64,
    sample_rate: f32,
    //the oscillator. More can easily be added
    voices : interp::Voiceset,
    wt_len : Vec<usize>,
}

impl WaveTable
{
    //fills a buffer we can use for fir filtering. 
    //Can be used to avoid the delay from the fir filtering. Figure out how/when to call it to avoid delay.
    pub(crate) fn prep_buffer(&mut self,/* _ratio: f32*/) {
        self.voices.interp_buffer.resize(self.voices.oscs[0].downsample_fir.len() + 1, 0.);
        for i in 0..self.voices.oscs[0].downsample_fir.len() -1 {
            self.voices.interp_buffer[i] = 0.;
        }
        //not sure how to use the stuff underneath with multiple voices or legato, if it's even possible
        //fills the buffer with actual samples to avoid delay  
        // for _i in 0..(self.voices.osc1.downsample_fir.len()-1)/2
        // {  
        //     let unfiltered_new = self.voices.single_interp(_ratio, 0);
        //     //removes a sample in front
        //     self.voices.interp_buffer.pop_front();
        //     //adds a new unfiltered sample to the end
        //     self.voices.interp_buffer.push_back(unfiltered_new);

        // }

    }
    fn find_ratio(& mut self, note : u8, i : usize) -> f32 {

        let standard = /*21.827*/ 21.53320312; //default wavetable pitch
        let pn = 440f32 * (2f32.powf(1./12.)).powi(note as i32 - 69);
        //return ratio between desired pitch and standard 
        let diff = note -17;
        let mip = diff as usize /12;
        self.voices.voice[i].current_mip = mip;
        let downsampled_ratio = 2f32.powi(mip as i32);
        //standard / pn
        (pn / standard)/downsampled_ratio
    }
    fn process_midi_event(&mut self, data: [u8; 3]) {
        match data[0] {
            128 => self.note_off(data[1]),
            144 => self.note_on(data[1]),
            _ => (),
        }
        //change pitched_buffer here?
    }
    fn note_on(&mut self, note: u8) {
        self.note_duration = 0.0;
        self.note = Some(note);
        let mut i : usize = 9; 
        //get the first free voice
        for j in 0..8 {
            if self.voices.voice[j].is_free() {
                i = j;
                break;
            }
        }
        //if no free voices, nothing happens for now. Voice stealing should be implemented
        if i > 7 {
            return;
        }
        self.voices.voice[i].use_voice(note);
        self.voices.voice[i].ratio = self.find_ratio(note, i);
        //self.prep_buffer(/*self.ratio*/);
        //self.osc1.interpolated = self.osc1.static_convolve(&self.osc1.upsample_fir, &self.osc1.interpolated);
    }
    fn note_off(&mut self, note: u8) {
        for i in 0..8 {
            if self.voices.voice[i].note == Some(note) {
                self.voices.voice[i].note = None;
                self.voices.voice[i].free_voice();
                self.voices.voice[i].reset_its();
            }
        }
        self.note = None;
        for i in 0..8 {
            if !self.voices.voice[i].is_free() {
                self.note = Some(note);
                break; //it's enough if just one voice is free
            } 
        }
        
        
    }
}

impl Default for WaveTable
{
    fn default() -> WaveTable {
        let mut osc1 : interp::Interp = Default::default();
        let mut reader = hound::WavReader::open(r"C:\Users\rasmu\Documents\Xfer\Serum Presets\Tables\Analog\Basic Shapes.wav").unwrap();
        osc1.source_y = reader.samples().collect::<Result<Vec<_>,_>>().unwrap();
        osc1.slice(); osc1.oversample(2); osc1.mip_map(); osc1.optimal_coeffs();
        let mut osc2 : interp::Interp = Default::default();
        let mut reader2 = hound::WavReader::open(r"C:\Users\rasmu\Documents\Xfer\Serum Presets\Tables\Analog\Basic Shapes.wav").unwrap();
        osc2.source_y = reader2.samples().collect::<Result<Vec<_>,_>>().unwrap();
        osc2.slice(); osc2.oversample(2); osc2.mip_map(); osc2.optimal_coeffs();
        //let voiceset : interp::Voiceset::Default::default() 
        let mut a = WaveTable {
            note_duration: 0.0,
            note: None,
            sample_rate: 44100.,
            voices : interp::Voiceset{oscs : vec!(osc1, osc2), ..Default::default()}, 
            wt_len : vec!(7,7),
        };
        a.prep_buffer(); //first call fills the buffer with 0's.
        a.wt_len[0] = a.voices.oscs[0].len / (2048 * a.voices.oscs[0].amt_oversample);
        a.wt_len[1] = a.voices.oscs[1].len / (2048 * a.voices.oscs[1].amt_oversample);
        return a;
    }
}

impl Plugin for WaveTable
{
    fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate;
    }
    fn get_info(&self) -> Info 
    {
        Info  {
            name: "WaveTable".to_string(),
            unique_id: 9264,
            inputs: 0,
            outputs: 1,
            category: Category::Synth,
            parameters: 12,
            ..Default::default()
        }
    }
    fn process_events(&mut self, events: &Events) {
        for event in events.events() {
            match event {
                Event::Midi(ev) => self.process_midi_event(ev.data),
                // More events can be handled here.
                _ => (),
            }
        }
    }
    fn get_parameter(&self, index: i32) -> f32 {
    match index {
        0 => self.voices.pos[0] as f32,
        1 => self.voices.vol[0],
        2 => self.voices.detune[0],
        3 => self.voices.octave[0] as f32,
        4 => self.voices.pos[1] as f32,
        5 => self.voices.vol[1],
        6 => self.voices.detune[1],
        7 => self.voices.octave[1] as f32,
        8 => self.voices.filter[0].cutoff,
        9 => self.voices.filter[0].res,
        10 => (self.voices.filter[0].poles) as f32 + 1.,
        11 => self.voices.filter[0].drive,
        _ => 0.0,
        }
    }
    fn set_parameter(&mut self, index: i32, value: f32) {
        match index {
            0 => self.voices.pos[0] = ((value * (self.voices.oscs[0].wave_number - 1) as f32).round()) as usize,
            1 => self.voices.vol[0] = value,
            //make some proper detune formulas. They're just eyeballed for now.
            2 => self.voices.detune[0] = 0.98 + value * 0.04,
            3 => self.voices.octave[0] = (((value - 0.5) * 3.).round()) as i8,
            4 => self.voices.pos[1] = ((value * (self.voices.oscs[1].wave_number - 1) as f32).round()) as usize,
            5 => self.voices.vol[1] = value,
            6 => self.voices.detune[1] = 0.98 + value * 0.04,
            7 => self.voices.octave[1] = (((value - 0.5) * 3.).round()) as i8,
            8 => self.voices.filter[0].cutoff = 20000. * (1.8f32.powf(10. * value - 10.)).min(0.999),
            //self.g = value * 10.,
            9 => self.voices.filter[0].res = value * 4.4,
            10 => self.voices.filter[0].poles = ((value * 3.).round()) as usize,
            11 => self.voices.filter[0].drive = value * 5.,
            _ => (),
            
        }
        self.voices.filter[0].g = ( 3.1415 * self.voices.filter[0].cutoff / (self.voices.filter[0].sample_rate * self.voices.filter[0].oversample as f32)).tan()
    }
    fn get_parameter_name(&self, index: i32) -> String {
        match index {
            0 => "osc1 WT pos".to_string(),
            1 => "osc1 volume".to_string(),
            2 => "osc1 detune".to_string(),
            3 => "osc1 octave".to_string(),
            4 => "osc2 WT pos".to_string(),
            5 => "osc2 volume".to_string(),
            6 => "osc2 detune".to_string(),
            7 => "osc1 octave".to_string(),
            8 => "cutoff".to_string(),
            9 => "res".to_string(),
            10 => "filter order".to_string(),
            11 => "drive".to_string(),
            //4 => "Wet level".to_string(),
            _ => "".to_string(),
        }
    }
    fn get_parameter_label(&self, index: i32) -> String {
        match index {
            0 => "".to_string(),
            1 => "%".to_string(),
            2 => "".to_string(),
            3 => "".to_string(),
            4 => "".to_string(),
            5 => "%".to_string(),
            6 => "".to_string(),
            7 => "".to_string(),
            8 => "Hz".to_string(),
            9 => "%".to_string(),
            10 => "poles".to_string(),
            11 => "%".to_string(),
            _ => "".to_string(),
        }
    }
    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {

        // Split out the input and output buffers into two vectors
        let (_, outputs) = buffer.split();

        // Assume 2 channels
        // if inputs.len() != 2 || outputs.len() != 2 {
        //     return;
        // }
        //  // Iterate over outputs as (&mut f32, &mut f32)
        // let (mut l, mut r) = outputs.split_at_mut(1);
        // let stereo_out = l[0].iter_mut().zip(r[0].iter_mut());
    for output_channel in outputs.into_iter()  {
            for output_sample in output_channel {
                if let Some(_current_note) = self.note {
                    //outputs the next sample to be played.
                    *output_sample = self.voices.step_one();
                }
                else {
                    //behavior of it at note off can be seen as starting phase, and could be made a variable
                    //self.osc1.it_unrounded = 0.; // should be unison its instead
                    *output_sample = 0.;

                }
            }
        }
    }
    fn can_do(&self, can_do: CanDo) -> Supported {
        match can_do {
            CanDo::ReceiveMidiEvent => Supported::Yes,
            _ => Supported::Maybe,
        }
    }
}
plugin_main!(WaveTable);
