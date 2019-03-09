# Graintable-synth
WIP vst synthesizer written in rust.

The source code is in lib.rs. Cargo.toml lists dependencies.

The goal for now is to implement a wavetable oscillator that can be controlled by midi.

Part goals needed:
* Note on/off ✓
* wavetable pos ✓
* Let max wavetable pos change dynamically with reader.duration
* Resample function to change pitch

