/*
    TODO:
        Make sure we stay inside of bounds (limit self.pos with duration function. (reader.duration))
        Implement note on/off functionality
        
*/

//vst stuff
 #[macro_use] extern crate vst;
use vst::buffer::AudioBuffer;

use vst::plugin::{Info, Plugin, Category};  

//filereading stuff


//reading into array stuff
// Add extern crate declaration (at the root of your crate)



struct WaveTable
{
    wave_buffer : Vec<f32>,
    //file : File,
    it : usize,
    pos : usize
}

impl Default for WaveTable
{
       

    fn default() -> WaveTable {

        let mut a = WaveTable {
            wave_buffer : vec![0.],
            it : 0,
            pos : 1,
        };
        let mut reader = hound::WavReader::open(r"C:\Users\rasmu\Documents\Xfer\Serum Presets\Tables\Analog\Basic Shapes.wav").unwrap();
        a.wave_buffer = reader.samples().collect::<Result<Vec<f32>,_>>().unwrap();

        return a;
    }
}


impl Plugin for WaveTable
{
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
    fn get_parameter(&self, index: i32) -> f32 {
    match index {
        0 => self.pos as f32,
        _ => 0.0,
        }
    }

    fn set_parameter(&mut self, index: i32, value: f32) {
        match index {
            0 => self.pos = ((value * 4.).round()) as usize,
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
        // Iterate over inputs as (&f32, &f32)
        // let (l, r) = inputs.split_at(1);
        // let stereo_in = l[0].iter().zip(r[0].iter());
        //  // Iterate over outputs as (&mut f32, &mut f32)
        // let (mut l, mut r) = outputs.split_at_mut(1);
        // let stereo_out = l[0].iter_mut().zip(r[0].iter_mut());
        for output_channel in outputs.into_iter()  {
                for output_sample in output_channel {
                    if self.it >= 2048
                    {
                        self.it = 0
                    }
                    *output_sample = self.wave_buffer[self.it + (2048 * self.pos)] * 0.5;
                    //*output_sample = 1.;
                    self.it += 1;

                }
            }
    }
}
plugin_main!(WaveTable);