#![feature(hash_drain_filter)]
use std::{collections::HashMap, f32::consts::PI};

use vst::{
    event::Event,
    plugin::{Category, HostCallback, Info, Plugin},
    plugin_main,
};
use wmidi::{MidiMessage, Note, Velocity};

struct NoteState {
    velocity: Velocity,
    age: usize,
    released: bool,
}

impl NoteState {
    fn new(velocity: Velocity) -> Self {
        NoteState {
            velocity,
            age: 0,
            released: false,
        }
    }
}

struct Snippet {
    data: Vec<f32>,
    index: usize,
}

impl Snippet {
    fn from_note_info(rate: f32, freq: f32, velocity: Velocity) -> Self {
        const N_WAVES: f32 = 10.;
        let mut data = vec![0.; (N_WAVES / (freq / rate)) as usize];
        for (i, sample) in data.iter_mut().enumerate() {
            *sample = (i as f32 * freq * PI / rate).sin() * (u8::from(velocity) as f32 / 127.);
        }
        data.into()
    }

    fn read(&mut self, len: usize) -> (&[f32], &[f32]) {
        let old_index = self.index;
        self.index += len;
        if self.index >= self.data.len() {
            self.index -= self.data.len();
            (&self.data[old_index..], &self.data[..self.index])
        } else {
            (&self.data[old_index..self.index], &[])
        }
    }
}

impl From<Vec<f32>> for Snippet {
    fn from(data: Vec<f32>) -> Self {
        Snippet { data, index: 0 }
    }
}

struct BeamyGlitch {
    wav_snippets: HashMap<Note, Snippet>,
    note_states: HashMap<Note, NoteState>,
}

impl BeamyGlitch {
    fn new() -> Self {
        BeamyGlitch {
            wav_snippets: Default::default(),
            note_states: Default::default(),
        }
    }
}

impl Default for BeamyGlitch {
    fn default() -> Self {
        BeamyGlitch::new()
    }
}

impl Plugin for BeamyGlitch {
    fn new(_host: HostCallback) -> Self {
        Default::default()
    }

    fn get_info(&self) -> Info {
        Info {
            name: "Beamy Glitch".to_owned(),
            unique_id: 77288698,
            inputs: 0,
            outputs: 2,
            category: Category::Synth,
            ..Default::default()
        }
    }

    fn process(&mut self, buffer: &mut vst::buffer::AudioBuffer<f32>) {
        let mut outputs = buffer.split().1;
        let len = outputs[0].len();
        for (note, _) in self.note_states.drain_filter(|_note, state| state.released) {
            self.wav_snippets.remove(&note);
        }
        for buffer in &mut outputs {
            buffer.fill(0.);
        }
        let n_voices = self.note_states.len() as f32;
        for (note, state) in &mut self.note_states {
            let (front, back) = self.wav_snippets.get_mut(note).unwrap().read(len);
            for buffer in &mut outputs {
                for (out, src) in buffer[..front.len()].iter_mut().zip(front.iter()) {
                    *out += src / n_voices;
                }
                for (out, src) in buffer[front.len()..].iter_mut().zip(back.iter()) {
                    *out += src / n_voices;
                }
            }
            state.age += len;
        }
    }

    fn process_events(&mut self, events: &vst::api::Events) {
        for event in events.events() {
            if let Event::Midi(ev) = event {
                if let Ok(ev) = MidiMessage::try_from(&ev.data[..]) {
                    match ev {
                        MidiMessage::NoteOn(_ch, note, velocity) => {
                            self.note_states.insert(note, NoteState::new(velocity));
                            let wav_snippet =
                                Snippet::from_note_info(44100., note.to_freq_f32(), velocity);
                            self.wav_snippets.insert(note, wav_snippet);
                        }
                        MidiMessage::NoteOff(_ch, note, _velocity) => {
                            if let Some(state) = self.note_states.get_mut(&note) {
                                state.released = true;
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
    }
}

plugin_main!(BeamyGlitch);
