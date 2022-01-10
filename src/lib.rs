#![feature(hash_drain_filter)]
use std::f32::consts::PI;

use multimap::MultiMap;
use vst::{
    event::Event,
    plugin::{Category, HostCallback, Info, Plugin},
    plugin_main,
};
use wmidi::{MidiMessage, Note, Velocity};

struct NoteState {
    velocity: f32,
    envelope: f32,
    age: usize,
    released: bool,
    wav_snippet: Snippet,
}

impl NoteState {
    fn new(velocity: Velocity, wav_snippet: Snippet) -> Self {
        NoteState {
            velocity: u8::from(velocity) as f32 / 127.,
            envelope: 0.,
            age: 0,
            released: false,
            wav_snippet,
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
        let mut data = vec![0.; (N_WAVES / (freq / rate)).round() as usize];
        let normal_freq = 1. / (data.len() as f32 / N_WAVES);
        for (i, sample) in data.iter_mut().enumerate() {
            *sample = (i as f32 * normal_freq * 2. * PI).sin() * (u8::from(velocity) as f32 / 127.);
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
    note_states: MultiMap<Note, NoteState>,
    envelope_buffer: Vec<f32>,
}

impl BeamyGlitch {
    fn new() -> Self {
        BeamyGlitch {
            note_states: Default::default(),
            envelope_buffer: Default::default(),
        }
    }
}

impl Default for BeamyGlitch {
    fn default() -> Self {
        BeamyGlitch::new()
    }
}

impl Plugin for BeamyGlitch {
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

    fn new(_host: HostCallback) -> Self {
        Default::default()
    }

    fn process(&mut self, buffer: &mut vst::buffer::AudioBuffer<f32>) {
        let mut outputs = buffer.split().1;
        let buffer_len = outputs[0].len();
        self.envelope_buffer.fill(0.);
        self.envelope_buffer.resize(buffer_len, 0.);
        self.note_states
            .retain(|_note, state| !state.released || state.envelope > 0.);
        for buffer in &mut outputs {
            buffer.fill(0.);
        }
        let mut outputs = outputs.split_at_mut(1);
        for (_note, state) in self.note_states.iter_mut() {
            let mut remaining_len = buffer_len;
            let mut pos = 0;
            while remaining_len > 0 {
                let len_to_read = state.wav_snippet.data.len().min(remaining_len);
                let (front, back) = state.wav_snippet.read(len_to_read);
                for (i, (out, src)) in outputs.0[0][pos..pos + len_to_read]
                    .iter_mut()
                    .zip(outputs.1[0][pos..pos + len_to_read].iter_mut())
                    .zip(front.iter().chain(back.iter()))
                    .enumerate()
                {
                    if state.released {
                        state.envelope = (state.envelope - 0.001).max(0.);
                    } else if state.envelope < 1. {
                        state.envelope = (state.envelope + 0.007).min(1.);
                    }
                    self.envelope_buffer[pos + i] += state.envelope * state.velocity;
                    *out.0 += src * state.envelope;
                    *out.1 += src * state.envelope;
                }
                remaining_len -= len_to_read;
                pos += len_to_read;
            }
            state.age += buffer_len;
        }
        for (envelope, out) in self
            .envelope_buffer
            .iter()
            .zip(outputs.0[0].iter_mut().zip(outputs.1[0].iter_mut()))
        {
            let compression = 1.5 * if *envelope > 1. { *envelope } else { 1. };
            *out.0 /= compression;
            *out.1 /= compression;
        }
    }

    fn process_events(&mut self, events: &vst::api::Events) {
        for event in events.events() {
            if let Event::Midi(ev) = event {
                if let Ok(ev) = MidiMessage::try_from(&ev.data[..]) {
                    match ev {
                        MidiMessage::NoteOn(_ch, note, velocity) => {
                            let wav_snippet =
                                Snippet::from_note_info(44100., note.to_freq_f32(), velocity);
                            self.note_states
                                .insert(note, NoteState::new(velocity, wav_snippet));
                        }
                        MidiMessage::NoteOff(_ch, note, _velocity) => {
                            if let Some(states) = self.note_states.get_vec_mut(&note) {
                                for state in states {
                                    state.released = true;
                                }
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
