/*

    TODO:
    Polyphony. 
    Improve SNR.

    Licensing. Look into MIT and copyleft

    Look into half-band filters for more efficient downsampling.

    https://se.mathworks.com/help/signal/ref/intfilt.html <- fir interpolation source for upsampling
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
    osc1 : interp::Interp, 
    wt_len : usize,
    ratio : f32,
}

impl WaveTable
{
    fn find_ratio(& mut self, note : u8) -> f32 {

        let standard = /*21.827*/ 21.53320312; //default wavetable pitch
        let pn = 440f32 * (2f32.powf(1./12.)).powi(note as i32 - 69);
        //return ratio between desired pitch and standard 
        let diff = note -17;
        self.osc1.current_mip = diff as usize /12;
        

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
        self.ratio = self.find_ratio(note);
        self.osc1.prep_buffer(self.ratio);
        //self.osc1.interpolated = self.osc1.static_convolve(&self.osc1.upsample_fir, &self.osc1.interpolated);
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
            note_duration: 0.0,
            note: None,
            sample_rate: 44100.,
            osc1 : Default::default(),
            wt_len : 7,
            ratio : 0.,
        };
        let mut reader = hound::WavReader::open(r"C:\Users\rasmu\Documents\Xfer\Serum Presets\Tables\Analog\Basic Shapes.wav").unwrap();
        a.osc1.source_y = reader.samples().collect::<Result<Vec<_>,_>>().unwrap();
        a.osc1.slice();
        a.osc1.oversample(2);
        a.osc1.mip_map();
        a.osc1.optimal_coeffs();
        a.wt_len = a.osc1.len / (2048 * a.osc1.amt_oversample);
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
        0 => self.osc1.pos as f32,
        _ => 0.0,
        }
    }
    fn set_parameter(&mut self, index: i32, value: f32) {
        match index {
            0 => self.osc1.pos = ((value * (self.osc1.wave_number - 1) as f32).round()) as usize,
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
                    //outputs the next sample to be played.
                    *output_sample = self.osc1.step_one(self.ratio);
                }
                else {
                    //behavior of it at note off can be seen as starting phase, and could be made a variable
                    self.osc1.it_unrounded = 0.;
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


