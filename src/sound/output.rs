//! The device lives on a worker; the callback uses fixed voices and atomics only.
use super::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};
struct Shared {
    enabled: AtomicBool,
    shutdown: AtomicBool,
    failed: AtomicBool,
    gain: AtomicU32,
    generation: AtomicU64,
    command: AtomicU64,
    status: AtomicU8,
    epoch: Instant,
}
/// Native CPAL output, opened lazily when sound is enabled. Device errors degrade
/// to silence. Toggle sound off/on to retry. Dropping signals shutdown without joining.
pub struct SoundOutput {
    shared: Arc<Shared>,
    worker: std::thread::JoinHandle<()>,
    gate: SoundGate,
}
impl SoundOutput {
    pub fn new(wake: impl Fn() + Send + 'static) -> io::Result<Self> {
        let shared = Arc::new(Shared {
            enabled: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            gain: AtomicU32::new(0f32.to_bits()),
            generation: AtomicU64::new(0),
            command: AtomicU64::new(0),
            status: AtomicU8::new(0),
            epoch: Instant::now(),
        });
        let state = shared.clone();
        let worker = std::thread::Builder::new()
            .name("brockmann-feedback".into())
            .spawn(move || {
                while !state.shutdown.load(Ordering::Acquire) {
                    if !state.enabled.load(Ordering::Acquire) {
                        std::thread::park();
                        continue;
                    }
                    let generation = state.generation.load(Ordering::Acquire);
                    state.status.store(1, Ordering::Release);
                    wake();
                    state.failed.store(false, Ordering::Release);
                    match open(state.clone()) {
                        Ok(stream) => {
                            if state.enabled.load(Ordering::Acquire) {
                                state.status.store(2, Ordering::Release);
                                wake();
                                while state.enabled.load(Ordering::Acquire)
                                    && !state.shutdown.load(Ordering::Acquire)
                                    && !state.failed.load(Ordering::Acquire)
                                {
                                    std::thread::park();
                                }
                                // Keep a rapid off/on cycle from unmuting a stream being retired.
                                state.status.store(0, Ordering::Release);
                                // Eight gain time constants before closing; never on the UI thread.
                                std::thread::sleep(Duration::from_millis(24));
                            }
                            drop(stream);
                        }
                        Err(_) => state.failed.store(true, Ordering::Release),
                    }
                    state.status.store(
                        if state.failed.load(Ordering::Acquire) {
                            3
                        } else {
                            0
                        },
                        Ordering::Release,
                    );
                    wake();
                    // On failure wait for an off transition; never repeatedly reopen a bad device.
                    while state.failed.load(Ordering::Acquire)
                        && state.enabled.load(Ordering::Acquire)
                        && state.generation.load(Ordering::Acquire) == generation
                        && !state.shutdown.load(Ordering::Acquire)
                    {
                        std::thread::park();
                    }
                }
            })?;
        Ok(Self {
            shared,
            worker,
            gate: SoundGate::default(),
        })
    }
    pub fn status(&self) -> SoundStatus {
        match self.shared.status.load(Ordering::Acquire) {
            1 => SoundStatus::Starting,
            2 => SoundStatus::Ready,
            3 => SoundStatus::Unavailable,
            _ => SoundStatus::Disabled,
        }
    }
    /// Non-blocking UI-side dispatch. Cues during startup or older than 120 ms are dropped.
    pub fn sync(&mut self, settings: SoundSettings, cue: Option<SoundCue>) {
        let enabled = settings.mode != SoundMode::Off && settings.gain() > 0.;
        self.shared
            .gain
            .store(settings.gain().to_bits(), Ordering::Release);
        if self.shared.enabled.load(Ordering::Acquire) != enabled {
            self.shared.command.store(0, Ordering::Release);
            if enabled {
                self.shared.generation.fetch_add(1, Ordering::AcqRel);
            }
            self.shared.enabled.store(enabled, Ordering::Release);
            self.worker.thread().unpark();
        }
        if !enabled {
            self.shared.command.store(0, Ordering::Release);
            return;
        }
        if let Some(cue) = cue.filter(|_| self.status() == SoundStatus::Ready) {
            let now = self.shared.epoch.elapsed();
            if self.gate.admit(cue, settings, now.as_secs_f64()) {
                self.shared.command.store(
                    ((now.as_millis() as u64) << 8) | (cue as u64 + 1),
                    Ordering::Release,
                );
            }
        }
    }
}
impl Drop for SoundOutput {
    fn drop(&mut self) {
        self.shared.enabled.store(false, Ordering::Release);
        self.shared.shutdown.store(true, Ordering::Release);
        self.worker.thread().unpark();
    }
}
#[derive(Clone, Copy, Default)]
struct Voice {
    cue: usize,
    position: usize,
    active: bool,
}
struct Mixer {
    bank: [Vec<f32>; 8],
    voices: [Voice; 4],
    gain: f32,
    slew: f32,
}
impl Mixer {
    fn new(rate: u32) -> Result<Self, &'static str> {
        let mut bank = std::array::from_fn(|_| Vec::new());
        for (slot, cue) in bank.iter_mut().zip(SoundCue::ALL) {
            *slot = synthesize(cue, rate)?;
        }
        Ok(Self {
            bank,
            voices: [Voice::default(); 4],
            gain: 0.,
            slew: 1. - (-1. / (rate as f32 * 0.003)).exp(),
        })
    }
    fn trigger(&mut self, cue: usize) {
        if let Some(voice) = self.voices.iter_mut().find(|v| !v.active) {
            *voice = Voice {
                cue,
                position: 0,
                active: true,
            };
        }
    }
    fn next(&mut self, target: f32) -> f32 {
        self.gain += (target - self.gain) * self.slew;
        let mut sum = 0.;
        for voice in &mut self.voices {
            if voice.active {
                sum += self.bank[voice.cue][voice.position];
                voice.position += 1;
                voice.active = voice.position < self.bank[voice.cue].len();
            }
        }
        sum * self.gain
    }
}
fn open(shared: Arc<Shared>) -> Result<cpal::Stream, String> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("no output device")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let format = supported.sample_format();
    let buffer = match *supported.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } if min > 0 && max >= min => {
            cpal::BufferSize::Fixed(256u32.clamp(min, max))
        }
        _ => cpal::BufferSize::Default,
    };
    let mut config: cpal::StreamConfig = supported.into();
    config.buffer_size = buffer;
    let result = build_format(&device, &config, format, shared.clone());
    if result.is_err() && !matches!(buffer, cpal::BufferSize::Default) {
        config.buffer_size = cpal::BufferSize::Default;
        build_format(&device, &config, format, shared)
    } else {
        result
    }
}
fn build_format(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    format: cpal::SampleFormat,
    shared: Arc<Shared>,
) -> Result<cpal::Stream, String> {
    macro_rules! stream {
        ($ty:ty) => {
            build::<$ty>(device, config, shared)
        };
    }
    match format {
        cpal::SampleFormat::F32 => stream!(f32),
        cpal::SampleFormat::F64 => stream!(f64),
        cpal::SampleFormat::I8 => stream!(i8),
        cpal::SampleFormat::I16 => stream!(i16),
        cpal::SampleFormat::I32 => stream!(i32),
        cpal::SampleFormat::I64 => stream!(i64),
        cpal::SampleFormat::U8 => stream!(u8),
        cpal::SampleFormat::U16 => stream!(u16),
        cpal::SampleFormat::U32 => stream!(u32),
        cpal::SampleFormat::U64 => stream!(u64),
        _ => Err("unsupported output sample format".into()),
    }
}
fn build<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    shared: Arc<Shared>,
) -> Result<cpal::Stream, String> {
    let mut mixer = Mixer::new(config.sample_rate.0).map_err(str::to_string)?;
    let channels = config.channels as usize;
    if channels == 0 {
        return Err("empty channel layout".into());
    }
    let error_shared = shared.clone();
    let worker = std::thread::current();
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let enabled = shared.enabled.load(Ordering::Acquire)
                    && shared.status.load(Ordering::Acquire) == 2;
                let command = shared.command.swap(0, Ordering::AcqRel);
                if enabled
                    && command != 0
                    && shared
                        .epoch
                        .elapsed()
                        .as_millis()
                        .saturating_sub((command >> 8) as u128)
                        <= 120
                {
                    let index = (command & 255) as usize - 1;
                    if index < 8 {
                        mixer.trigger(index);
                    }
                }
                let target = if enabled {
                    f32::from_bits(shared.gain.load(Ordering::Acquire))
                } else {
                    0.
                };
                for frame in data.chunks_mut(channels) {
                    let sample = T::from_sample(mixer.next(target));
                    for channel in frame {
                        *channel = sample;
                    }
                }
            },
            move |_| {
                error_shared.failed.store(true, Ordering::Release);
                worker.unpark();
            },
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_voice_mixer_has_headroom_and_finishes_without_truncating_tails() {
        let mut mixer = Mixer::new(48000).unwrap();
        for cue in 0..4 {
            mixer.trigger(cue);
        }
        mixer.trigger(4); // Saturation drops the new cue; it cannot cut an active voice.
        assert_eq!(mixer.voices.iter().filter(|v| v.active).count(), 4);
        for _ in 0..8000 {
            let value = mixer.next(1.);
            assert!(value.is_finite() && value.abs() <= 0.801);
        }
        assert!(mixer.voices.iter().all(|v| !v.active));
        assert_eq!(mixer.next(1.), 0.);
    }
}
