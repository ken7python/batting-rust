use std::f32::consts::TAU;

use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound, stop_sound};

use crate::flight::Outcome;

const RATE: u32 = 22_050;

pub struct Sounds {
    swing: Sound,
    contact: Sound,
    barrel: Sound,
    pitch: Sound,
    glove: Sound,
    foul: Sound,
    hit: Sound,
    homer: Sound,
    out: Sound,
    music: Sound,
    music_playing: bool,
}

impl Sounds {
    pub async fn load() -> Self {
        async fn load(data: Vec<u8>) -> Sound {
            load_sound_from_bytes(&data)
                .await
                .expect("the built-in sound must decode")
        }
        Self {
            swing: load(effect_wave(Effect::Swing)).await,
            contact: load(effect_wave(Effect::Contact)).await,
            barrel: load(effect_wave(Effect::Barrel)).await,
            pitch: load(effect_wave(Effect::Pitch)).await,
            glove: load(effect_wave(Effect::Glove)).await,
            foul: load(effect_wave(Effect::Foul)).await,
            hit: load(effect_wave(Effect::Hit)).await,
            homer: load(effect_wave(Effect::Homer)).await,
            out: load(effect_wave(Effect::Out)).await,
            music: load(music_wave()).await,
            music_playing: false,
        }
    }

    pub fn swing(&self) {
        Self::play(&self.swing, 0.36);
    }

    pub fn pitch(&self) {
        Self::play(&self.pitch, 0.38);
    }

    pub fn glove(&self) {
        Self::play(&self.glove, 0.42);
    }

    pub fn contact(&self, barrel: bool) {
        Self::play(if barrel { &self.barrel } else { &self.contact }, 0.58);
    }

    pub fn result(&self, outcome: Outcome) {
        let (sound, volume) = match outcome {
            Outcome::Homer => (&self.homer, 0.60),
            Outcome::InfieldHit { .. }
            | Outcome::Fair
            | Outcome::HardFair
            | Outcome::Double
            | Outcome::Triple => (&self.hit, 0.40),
            Outcome::Foul => (&self.foul, 0.46),
            Outcome::FlyOut(_)
            | Outcome::LineOut(_)
            | Outcome::GroundOut(_)
            | Outcome::BaseOut(_) => (&self.out, 0.48),
            Outcome::Ball | Outcome::Strike | Outcome::Whiff => (&self.glove, 0.42),
        };
        Self::play(sound, volume);
    }

    pub fn set_music(&mut self, enabled: bool) {
        if self.music_playing == enabled {
            return;
        }
        if enabled {
            play_sound(
                &self.music,
                PlaySoundParams {
                    looped: true,
                    volume: 0.23,
                },
            );
        } else {
            stop_sound(&self.music);
        }
        self.music_playing = enabled;
    }

    fn play(sound: &Sound, volume: f32) {
        play_sound(
            sound,
            PlaySoundParams {
                looped: false,
                volume,
            },
        );
    }
}

#[derive(Clone, Copy)]
enum Effect {
    Swing,
    Contact,
    Barrel,
    Pitch,
    Glove,
    Foul,
    Hit,
    Homer,
    Out,
}

fn effect_wave(effect: Effect) -> Vec<u8> {
    let seconds = match effect {
        Effect::Homer => 0.94,
        Effect::Hit | Effect::Out => 0.42,
        Effect::Foul => 0.26,
        _ => 0.15,
    };
    let mut noise = 2_375_197;
    let mut previous = 0.0;
    pcm((seconds * RATE as f32) as u32, |frame| {
        let t = frame as f32 / RATE as f32;
        let white = next_noise(&mut noise);
        let filtered = 0.5 * previous + 0.5 * white;
        previous = filtered;
        let pop = (TAU * (155.0 * t - 190.0 * t * t)).sin() * (-t * 56.0).exp();
        let sample = match effect {
            Effect::Swing => filtered * (t * 36.0).min(1.0) * (-t * 27.0).exp() * 0.90,
            Effect::Contact | Effect::Barrel => {
                let frequency = if matches!(effect, Effect::Barrel) {
                    970.0
                } else {
                    500.0
                };
                let ring = (TAU * frequency * t).sin() * (-t * 75.0).exp();
                0.56 * ring + 0.72 * filtered * (-t * 250.0).exp()
            }
            Effect::Pitch => (white - filtered) * (t * 110.0).min(1.0) * (-t * 33.0).exp() * 0.63,
            Effect::Glove => 0.60 * pop + 0.56 * filtered * (-t * 120.0).exp(),
            Effect::Foul => {
                ((TAU * 840.0 * t).sin() + 0.52 * (TAU * 1331.0 * t).sin())
                    * (-t * 25.0).exp()
                    * 0.42
            }
            Effect::Hit => cue(t, &[523.25, 659.25, 783.99], 0.085),
            Effect::Homer => cue(t, &[392.0, 523.25, 659.25, 783.99, 1046.50], 0.115),
            Effect::Out => 0.38 * pop + cue((t - 0.075).max(0.0), &[349.23, 261.63], 0.13) * 0.78,
        };
        sample * (t * 800.0).min(1.0) * ((seconds - t) * 90.0).clamp(0.0, 1.0)
    })
}

fn cue(t: f32, notes: &[f32], spacing: f32) -> f32 {
    notes
        .iter()
        .enumerate()
        .map(|(index, frequency)| {
            let elapsed = t - index as f32 * spacing;
            if elapsed < 0.0 {
                0.0
            } else {
                0.37 * organ(*frequency, elapsed)
                    * (elapsed * 125.0).min(1.0)
                    * (-elapsed * 14.0).exp()
            }
        })
        .sum()
}

fn music_wave() -> Vec<u8> {
    // Eight bars at 120 BPM. The bass and backbeat leave room for the game's cues.
    let melody = [
        [64, 67, 69, 0, 67, 64, 62, 0],
        [64, 67, 72, 0, 71, 67, 64, 0],
        [65, 69, 72, 0, 71, 69, 67, 0],
        [62, 67, 71, 0, 69, 67, 62, 0],
        [64, 67, 69, 72, 0, 69, 67, 0],
        [64, 69, 72, 76, 0, 72, 69, 0],
        [65, 69, 72, 74, 72, 69, 65, 0],
        [67, 71, 74, 0, 71, 67, 62, 0],
    ]
    .map(|bar| bar.map(note));
    let chords = [
        [48, 52, 55],
        [45, 48, 52],
        [41, 45, 48],
        [43, 47, 50],
        [48, 52, 55],
        [45, 48, 52],
        [41, 45, 48],
        [43, 47, 50],
    ]
    .map(|chord| chord.map(note));
    let mut noise = 185_373;
    pcm(RATE * 16, |frame| {
        let t = frame as f32 / RATE as f32;
        let beat = t * 2.0;
        let beat_index = beat as usize;
        let bar = beat_index / 4;
        let slot = (beat * 2.0) as usize % 8;
        let eighth = (beat * 2.0).fract() * 0.25;
        let quarter = beat.fract() * 0.5;
        let attack = (eighth * 120.0).min(1.0);
        let release = ((0.25 - eighth) * 40.0).min(1.0);
        let lead = organ(melody[bar][slot], eighth) * attack * release * (-eighth * 5.0).exp();
        let chord = if slot % 2 == 1 {
            chords[bar]
                .iter()
                .map(|&frequency| organ(frequency * 2.0, eighth))
                .sum::<f32>()
                * attack
                * release
                * (-eighth * 17.0).exp()
        } else {
            0.0
        };
        let bass_frequency = chords[bar][0] * if beat_index % 4 == 2 { 0.75 } else { 0.5 };
        let bass = organ(bass_frequency, quarter)
            * (quarter * 110.0).min(1.0)
            * ((0.50 - quarter) * 25.0).min(1.0)
            * (-quarter * 6.0).exp();
        let white = next_noise(&mut noise);
        let hat = white * attack * (-eighth * 52.0).exp();
        let drums = if beat_index.is_multiple_of(2) {
            (TAU * (63.0 * quarter - 52.0 * quarter * quarter)).sin() * (-quarter * 29.0).exp()
        } else {
            white * (quarter * 500.0).min(1.0) * (-quarter * 38.0).exp() * 0.63
        };
        let edge = (t.min(16.0 - t) * 100.0).min(1.0);
        (0.27 * lead + 0.067 * chord + 0.26 * bass + 0.11 * hat + 0.17 * drums) * edge
    })
}

fn note(midi: i32) -> f32 {
    if midi == 0 {
        0.0
    } else {
        440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0)
    }
}

fn organ(frequency: f32, t: f32) -> f32 {
    let phase = TAU * frequency * t;
    0.68 * phase.sin() + 0.23 * (phase * 2.0).sin() + 0.09 * (phase * 3.0).sin()
}

fn next_noise(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *seed as f32 / u32::MAX as f32 * 2.0 - 1.0
}

fn pcm(frames: u32, mut sample: impl FnMut(u32) -> f32) -> Vec<u8> {
    let bytes = frames * 2;
    let mut wav = Vec::with_capacity((44 + bytes) as usize);
    wav.extend(b"RIFF");
    wav.extend((36 + bytes).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16_u32.to_le_bytes());
    wav.extend(1_u16.to_le_bytes());
    wav.extend(1_u16.to_le_bytes());
    wav.extend(RATE.to_le_bytes());
    wav.extend((RATE * 2).to_le_bytes());
    wav.extend(2_u16.to_le_bytes());
    wav.extend(16_u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(bytes.to_le_bytes());
    for frame in 0..frames {
        wav.extend(((sample(frame).clamp(-1.0, 1.0) * i16::MAX as f32) as i16).to_le_bytes());
    }
    wav
}
