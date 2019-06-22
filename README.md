# Graintable-synth
WIP vst synthesizer written in rust.

The source code is in lib.rs. Cargo.toml lists dependencies.

A full wavetable oscillator has been implemented. Its SNR at 1kHz is at -80 dB, so its quality could be improved. 
The next goal is to add more features.

A granular synthesis like oscillator has been implemented.

A 4-pole lowpass ladder filter has been implemented. More filter modes and models can be added. 
Pole mixing could be an easy way of adding some multi filters: https://mutable-instruments.net/archive/documents/pole_mixing.pdf

Envelopes are implemented, but they could potentially be more efficient.

File paths for wavetables should be gotten relative to source file so other people can actually build this lol



Features planned:
* Unison voices (done on graintable. Really could use optimization)
* Modulation framework (envelopes done, next step adding destinations)
* filtering ✓
* fine and coarse ✓ tune
* octave switches ✓



Part goals needed:
* Note on/off ✓
* wavetable pos ✓
* Let max wavetable pos change dynamically with reader.duration ✓
* Resample function to change pitch (http://yehar.com/blog/wp-content/uploads/2009/08/deip.pdf) ✓
* way to slice wavetable into individual waveforms (to avoid interpolation glitches) ✓
* FIR Filter for oversampling (2x oversampling should be fine) ✓
* Down sampling for mip-mapping ✓ 
* Mip-mapping ✓


