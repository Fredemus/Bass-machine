# Graintable-synth
WIP vst synthesizer written in rust.

The source code is in lib.rs. Cargo.toml lists dependencies.

A full wavetable oscillator has been implemented. Its SNR at 1kHz is at -80 dB, so its quality could be improved. 
^turns out the analog emulation filters are the thing adding noise. Better SNR requires finer filter solving, so it likely won't change for a while. 

The next goal is to add more features.

A granular synthesis like oscillator has been implemented.

A 4-pole lowpass ladder filter has been implemented. More filter modes and models can be added. 
Pole mixing could be an easy way of adding some multi filters: https://mutable-instruments.net/archive/documents/pole_mixing.pdf

Envelopes are implemented, but they could potentially be more efficient.


Features planned:
* Stereo processing. Mostly for unison/potential effects
* Unison voices for wavetable oscillators
* Modulation framework (envelopes done, next step adding destinations)
* Nonlinear state variable filter (almost done)




Part goals needed:
* Note on/off ✓
* wavetable pos ✓
* Let max wavetable pos change dynamically with reader.duration ✓
* Resample function to change pitch (http://yehar.com/blog/wp-content/uploads/2009/08/deip.pdf) ✓
* way to slice wavetable into individual waveforms (to avoid interpolation glitches) ✓
* FIR Filter for oversampling (2x oversampling should be fine) ✓
* Down sampling for mip-mapping ✓ 
* Mip-mapping ✓


