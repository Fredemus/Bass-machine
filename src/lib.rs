/*

    TODO:
    downsampling. Naive downsampling for now



    Look into half-band filters

    https://se.mathworks.com/help/signal/ref/intfilt.html <- fir interpolation source for upsampling


*/



//vst stuff
 #[macro_use] extern crate vst;
use vst::api::{Events, Supported};
use vst::buffer::AudioBuffer;
use vst::event::Event;
use vst::plugin::{CanDo, Info, Plugin, Category};  

extern crate hound;

// impl FirFilter
// {
//     //needs a calc_coefficients to put cutoff at the right place.
// }

//include interpolation module:
mod interp;

struct WaveTable
{
    //wave_buffer : Vec<f32>,
    //pitched_buffer: Vec<f32>,
    //time_vec : Vec<f32>,
    //file : File,
    it : usize,
    pos : usize,
    //for midi handling
    note: Option<u8>,
    note_duration: f64,
    sample_rate: f32,
    interpolator : interp::Interp, 
    //interpolator : interp::Interp,
    wt_len : usize,
}

impl WaveTable
{
    fn find_ratio(&self, note : u8) -> f32 {
        let standard = 21.827; //default wavetable pitch
        let pn = 440f32 * (2f32.powf(1./12.)).powi(note as i32 - 69);
        //return ratio between desired pitch and standard 
        standard / pn
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
        self.interpolator.interpolation(self.find_ratio(note))
    }
    fn note_off(&mut self, note: u8) {
        
        if self.note == Some(note) {
            self.note = None
        }
    }
}

impl Default for WaveTable
{
    fn default() -> WaveTable {

        let mut a = WaveTable {
            //wave_buffer : vec![0.],
            //pitched_buffer: vec![0.],
            it : 0,
            pos : 1,
            note_duration: 0.0,
            note: None,
            sample_rate: 44100.,
            interpolator : Default::default(),
            wt_len : 7,
        };
        let mut reader = hound::WavReader::open(r"C:\Users\rasmu\Documents\Xfer\Serum Presets\Tables\Analog\Basic Shapes.wav").unwrap();
        a.interpolator.source_y = reader.samples().collect::<Result<Vec<_>,_>>().unwrap();
        a.interpolator.oversample(2);
        //need fir filter here
        a.interpolator.calc_coefficients();
        a.wt_len = a.interpolator.len / (2048 * a.interpolator.times_oversampled);
        
        //a.pitched_buffer = a.wave_buffer.iter().step_by(2).clone();/*collect::<Vec<f32>>();*/
        //a.pitched_buffer = reader.samples().step_by(2).collect::<Result<Vec<_>,_>>().unwrap();

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
            parameters: 1,
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
        0 => self.pos as f32,
        _ => 0.0,
        }
    }
    fn set_parameter(&mut self, index: i32, value: f32) {
        match index {
            0 => self.pos = ((value * (self.wt_len - 1) as f32).round()) as usize,
            _ => (),
        }
    }
    fn get_parameter_name(&self, index: i32) -> String {
        match index {
            0 => "WT pos".to_string(),
            //4 => "Wet level".to_string(),
            _ => "".to_string(),
        }
    }
    fn get_parameter_label(&self, index: i32) -> String {
        match index {
            0 => "".to_string(),
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
                    //need oversampling process. Start tucking it away into interp.rs and make it a proper oscillator?
                    if self.it >= ((self.interpolator.new_len /self.wt_len - 1 ) / self.interpolator.times_oversampled )
                    {
                        self.it = 0
                    }
                    //naive downsampling for now, implement a (halfband?) filter here
                    //if self.it % 2 == 0 {
                        *output_sample = self.interpolator.interpolated[self.it * self.interpolator.times_oversampled + 
                        (((self.interpolator.new_len)/self.wt_len)  * self.pos )] ;
                        //*output_sample = 1.;
                        self.it += 1;
                    //}

                    // if self.it >= (2048 - 1)
                    // {
                    //     self.it = 0
                    // }
                    // *output_sample = self.interpolator.interpolated[self.it + ((2048) * self.pos)] ;
                    // //*output_sample = 1.;
                    // self.it += 1;
                    
                }
                else {
                    //behavior of it at note off can be seen as starting phase, and could be made a variable
                    self.it = 0;
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
