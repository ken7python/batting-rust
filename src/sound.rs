use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound};

pub struct Sounds {
    swing: Sound,
    contact: Sound,
    barrel: Sound,
}

impl Sounds {
    pub async fn load() -> Self {
        async fn load(effect: Effect) -> Sound {
            load_sound_from_bytes(&wave(effect))
                .await
                .expect("the built-in sound must decode")
        }
        Self {
            swing: load(Effect::Swing).await,
            contact: load(Effect::Contact).await,
            barrel: load(Effect::Barrel).await,
        }
    }

    pub fn swing(&self) {
        Self::play(&self.swing, 0.36);
    }

    pub fn contact(&self, barrel: bool) {
        Self::play(if barrel { &self.barrel } else { &self.contact }, 0.58);
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
}

fn wave(effect: Effect) -> Vec<u8> {
    let rate = 22_050_u32;
    let frames = rate / 7;
    let bytes = frames * 2;
    let mut wav = Vec::with_capacity((44 + bytes) as usize);
    wav.extend(b"RIFF");
    wav.extend((36 + bytes).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16_u32.to_le_bytes());
    wav.extend(1_u16.to_le_bytes());
    wav.extend(1_u16.to_le_bytes());
    wav.extend(rate.to_le_bytes());
    wav.extend((rate * 2).to_le_bytes());
    wav.extend(2_u16.to_le_bytes());
    wav.extend(16_u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(bytes.to_le_bytes());

    let mut rng = 2_375_197_u32;
    let mut previous = 0.0;
    for i in 0..frames {
        let t = i as f32 / rate as f32;
        rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = rng as f32 / u32::MAX as f32 * 2.0 - 1.0;
        let filtered = 0.5 * previous + 0.5 * noise;
        previous = filtered;
        let sample = match effect {
            Effect::Swing => filtered * (t * 36.0).min(1.0) * (-t * 27.0).exp() * 0.90,
            Effect::Contact | Effect::Barrel => {
                let pitch = if matches!(effect, Effect::Barrel) {
                    970.0
                } else {
                    500.0
                };
                let ring = (std::f32::consts::TAU * pitch * t).sin() * (-t * 75.0).exp();
                let transient = filtered * (-t * 250.0).exp();
                0.56 * ring + 0.72 * transient
            }
        };
        wav.extend(((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).to_le_bytes());
    }
    wav
}
