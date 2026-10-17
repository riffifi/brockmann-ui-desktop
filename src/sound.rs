#![doc = include_str!("../docs/sound.md")]
use std::{
    f32::consts::TAU,
    io::{self, Write},
    path::Path,
};

/// A small family of dry, related tactile sounds. No reverb or stereo motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SoundCue {
    Activate,
    Select,
    ToggleOn,
    ToggleOff,
    Open,
    Close,
    Confirm,
    Reject,
}
impl SoundCue {
    pub const ALL: [Self; 8] = [
        Self::Activate,
        Self::Select,
        Self::ToggleOn,
        Self::ToggleOff,
        Self::Open,
        Self::Close,
        Self::Confirm,
        Self::Reject,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Activate => "activate",
            Self::Select => "select",
            Self::ToggleOn => "toggle-on",
            Self::ToggleOff => "toggle-off",
            Self::Open => "open",
            Self::Close => "close",
            Self::Confirm => "confirm",
            Self::Reject => "reject",
        }
    }
    pub fn duration_ms(self) -> u32 {
        match self {
            Self::Activate => 34,
            Self::Select => 30,
            Self::ToggleOn => 48,
            Self::ToggleOff => 56,
            Self::Open => 60,
            Self::Close => 54,
            Self::Confirm => 104,
            Self::Reject => 140,
        }
    }
    pub(crate) fn priority(self) -> u8 {
        match self {
            Self::Reject => 6,
            Self::Confirm => 5,
            Self::ToggleOn | Self::ToggleOff => 4,
            Self::Select => 3,
            Self::Open | Self::Close => 2,
            Self::Activate => 1,
        }
    }
    pub fn essential(self) -> bool {
        matches!(
            self,
            Self::ToggleOn | Self::ToggleOff | Self::Confirm | Self::Reject
        )
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SoundMode {
    #[default]
    Off,
    Essential,
    Subtle,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoundSettings {
    pub mode: SoundMode,
    /// Linear gain, clamped to 0–1. Non-finite values mute.
    pub volume: f32,
}
impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            mode: SoundMode::Off,
            volume: 0.35,
        }
    }
}
impl SoundSettings {
    pub fn gain(self) -> f32 {
        if self.volume.is_finite() {
            self.volume.clamp(0., 1.)
        } else {
            0.
        }
    }
    pub fn allows(self, cue: SoundCue) -> bool {
        self.gain() > 0.
            && match self.mode {
                SoundMode::Off => false,
                SoundMode::Essential => cue.essential(),
                SoundMode::Subtle => true,
            }
    }
}
/// Override semantic feedback on a widget. `Silent` also suppresses its picker layer sounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SoundFeedback {
    #[default]
    Auto,
    Silent,
    Cue(SoundCue),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SoundStatus {
    #[default]
    Disabled,
    Starting,
    Ready,
    Unavailable,
    NotBuilt,
}

/// At most one cue per dispatch. A meaningful state change wins over a decorative click.
#[derive(Default)]
pub(crate) struct SoundRequests(Option<SoundCue>);
impl SoundRequests {
    pub fn push(&mut self, cue: SoundCue) {
        if self.0.is_none_or(|old| cue.priority() >= old.priority()) {
            self.0 = Some(cue);
        }
    }
    pub fn take(&mut self, settings: SoundSettings) -> Option<SoundCue> {
        self.0.take().filter(|cue| settings.allows(*cue))
    }
}
/// Real-time admission policy: no backlog, at least 45 ms between cues, and 100 ms
/// between repeats. A higher-priority outcome can bypass the interval. Pass
/// monotonic seconds, independent of the animation clock.
#[derive(Default)]
pub struct SoundGate {
    last: Option<(SoundCue, f64)>,
}
impl SoundGate {
    pub fn admit(&mut self, cue: SoundCue, settings: SoundSettings, now: f64) -> bool {
        if !now.is_finite() || !settings.allows(cue) {
            return false;
        }
        if let Some((previous, at)) = self.last {
            let interval = if previous == cue { 0.100 } else { 0.045 };
            let outcome = cue.priority() >= 5 && cue.priority() > previous.priority();
            if now - at < 0. || (now - at < interval && !outcome) {
                return false;
            }
        }
        self.last = Some((cue, now));
        true
    }
}
/// Deterministic mono PCM. Two damped resonances, a quiet filtered contact transient,
/// a 1.5 ms attack and a 6 ms release avoid an impulse at the sample boundaries.
/// Peak is 0.2 (-14 dBFS) before the user gain. No external sample assets are needed.
pub fn synthesize(cue: SoundCue, sample_rate: u32) -> Result<Vec<f32>, &'static str> {
    if !(8000..=192000).contains(&sample_rate) {
        return Err("sound sample rate must be 8000–192000 Hz");
    }
    let duration = cue.duration_ms() as f32 / 1000.;
    let length = (sample_rate as f32 * duration).round() as usize;
    let (from, to, decay, contact, second) = match cue {
        SoundCue::Activate => (980., 760., 0.008, 0.24, None),
        SoundCue::Select => (1240., 1040., 0.007, 0.30, None),
        SoundCue::ToggleOn => (650., 940., 0.014, 0.10, None),
        SoundCue::ToggleOff => (700., 430., 0.017, 0.12, None),
        SoundCue::Open => (420., 620., 0.018, 0.06, None),
        SoundCue::Close => (580., 360., 0.015, 0.08, None),
        SoundCue::Confirm => (740., 820., 0.014, 0.08, Some((0.048, 1110.))),
        SoundCue::Reject => (470., 420., 0.016, 0.22, Some((0.038, 350.))),
    };
    let pulse = |t: f32, f0: f32, f1: f32, noise: f32| {
        if t < 0. {
            return 0.;
        }
        let attack = (t / 0.0015).clamp(0., 1.);
        let attack = attack * attack * (3. - 2. * attack);
        let phase = TAU * (f0 * t + (f1 - f0) * t * t / (2. * duration));
        let harmonic = if f0 * 2.37 < sample_rate as f32 * 0.4 {
            (phase * 2.37 + 0.25).sin() * 0.20 * (-t / 0.006).exp()
        } else {
            0.
        };
        attack
            * ((phase.sin() * (-t / decay).exp() + harmonic) * 0.8
                + noise * contact * (-t / 0.003).exp())
    };
    let mut seed = 0x9e3779b9u32.wrapping_add(cue as u32 * 7919);
    let mut low = 0.;
    let coefficient = 1. - (-TAU * 2200. / sample_rate as f32).exp();
    let mut samples = Vec::with_capacity(length);
    for i in 0..length {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        low += coefficient * (seed as f32 / u32::MAX as f32 * 2. - 1. - low);
        let t = i as f32 / sample_rate as f32;
        let mut value = pulse(t, from, to, low);
        if let Some((delay, freq)) = second {
            value += pulse(t - delay, freq, freq * 0.94, low) * 0.78;
        }
        if cue == SoundCue::Reject {
            value += pulse(t - 0.078, 300., 285., low) * 0.62;
        }
        let release = ((length - 1 - i) as f32 / (sample_rate as f32 * 0.006)).clamp(0., 1.);
        samples.push(value * release * release * (3. - 2. * release));
    }
    let mean = samples.iter().sum::<f32>() / length as f32;
    // Remove DC without adding discontinuities at either endpoint.
    for (i, sample) in samples.iter_mut().enumerate() {
        *sample -= mean
            * 2.
            * (std::f32::consts::PI * i as f32 / (length - 1) as f32)
                .sin()
                .powi(2);
    }
    let peak = samples
        .iter()
        .fold(0f32, |a, b| a.max(b.abs()))
        .max(f32::EPSILON);
    for sample in &mut samples {
        *sample *= 0.2 / peak;
    }
    samples[0] = 0.;
    samples[length - 1] = 0.;
    Ok(samples)
}
/// Export mono 16-bit PCM for review. This writes no audio device and honours gain.
pub fn write_wav(path: impl AsRef<Path>, samples: &[f32], sample_rate: u32) -> io::Result<()> {
    let size = u32::try_from(samples.len())
        .ok()
        .and_then(|n| n.checked_mul(2))
        .filter(|n| *n <= u32::MAX - 36)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "WAV too large"))?;
    let bytes_per_second = sample_rate
        .checked_mul(2)
        .filter(|n| *n > 0)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid WAV sample rate"))?;
    let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
    out.write_all(b"RIFF")?;
    out.write_all(&(size + 36).to_le_bytes())?;
    out.write_all(b"WAVEfmt ")?;
    out.write_all(&16u32.to_le_bytes())?;
    out.write_all(&1u16.to_le_bytes())?;
    out.write_all(&1u16.to_le_bytes())?;
    out.write_all(&sample_rate.to_le_bytes())?;
    out.write_all(&bytes_per_second.to_le_bytes())?;
    out.write_all(&2u16.to_le_bytes())?;
    out.write_all(&16u16.to_le_bytes())?;
    out.write_all(b"data")?;
    out.write_all(&size.to_le_bytes())?;
    for sample in samples {
        let value = if sample.is_finite() {
            sample.clamp(-1., 1.)
        } else {
            0.
        };
        out.write_all(&((value * 32767.).round() as i16).to_le_bytes())?;
    }
    out.flush()
}

#[cfg(feature = "sound")]
mod output;
#[cfg(feature = "sound")]
pub use output::SoundOutput;
