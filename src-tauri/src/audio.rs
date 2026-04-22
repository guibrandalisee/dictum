//! Microphone capture using `cpal` running on a dedicated worker thread.
//!
//! The `cpal::Stream` type is `!Send`, so it cannot live inside a shared
//! `AppState`. We instead spawn one thread that owns the stream and
//! receives commands over an `mpsc::Sender<AudioCommand>`. Recorded
//! samples are buffered in memory, downmixed to mono and resampled to
//! 16 kHz f32 (Whisper's expected input format) when the recording stops.

use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, StreamConfig};
use parking_lot::Mutex;
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Result of a completed recording.
pub struct Recording {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub duration: Duration,
}

enum AudioCommand {
    Start {
        device_name: Option<String>,
        max_duration: Duration,
        reply: Sender<Result<()>>,
    },
    Stop {
        reply: Sender<Result<Recording>>,
    },
    ListDevices {
        reply: Sender<Result<Vec<String>>>,
    },
}

#[derive(Clone)]
pub struct AudioCaptureService {
    tx: Sender<AudioCommand>,
}

impl AudioCaptureService {
    pub fn spawn() -> Self {
        let (tx, rx) = channel::<AudioCommand>();
        thread::Builder::new()
            .name("audio-worker".into())
            .spawn(move || worker_loop(rx))
            .expect("failed to spawn audio worker thread");
        Self { tx }
    }

    pub fn list_devices(&self) -> Result<Vec<String>> {
        let (reply, rx) = channel();
        self.tx
            .send(AudioCommand::ListDevices { reply })
            .map_err(|e| anyhow!("audio worker dropped: {e}"))?;
        rx.recv().map_err(|e| anyhow!("audio worker no reply: {e}"))?
    }

    pub fn start(&self, device_name: Option<String>, max_duration: Duration) -> Result<()> {
        let (reply, rx) = channel();
        self.tx
            .send(AudioCommand::Start {
                device_name,
                max_duration,
                reply,
            })
            .map_err(|e| anyhow!("audio worker dropped: {e}"))?;
        rx.recv().map_err(|e| anyhow!("audio worker no reply: {e}"))?
    }

    pub fn stop(&self) -> Result<Recording> {
        let (reply, rx) = channel();
        self.tx
            .send(AudioCommand::Stop { reply })
            .map_err(|e| anyhow!("audio worker dropped: {e}"))?;
        rx.recv().map_err(|e| anyhow!("audio worker no reply: {e}"))?
    }
}

struct ActiveCapture {
    stream: cpal::Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    source_rate: u32,
    channels: u16,
    started_at: Instant,
    max_duration: Duration,
}

fn worker_loop(rx: std::sync::mpsc::Receiver<AudioCommand>) {
    let mut active: Option<ActiveCapture> = None;

    // The loop naturally ends when all senders are dropped (rx.recv() -> Err).
    while let Ok(cmd) = rx.recv() {
        match cmd {
            AudioCommand::ListDevices { reply } => {
                let _ = reply.send(list_input_devices());
            }
            AudioCommand::Start {
                device_name,
                max_duration,
                reply,
            } => {
                if active.is_some() {
                    let _ = reply.send(Err(anyhow!("recording already in progress")));
                    continue;
                }
                match start_capture(device_name, max_duration) {
                    Ok(cap) => {
                        active = Some(cap);
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            AudioCommand::Stop { reply } => {
                let Some(cap) = active.take() else {
                    let _ = reply.send(Err(anyhow!("no recording in progress")));
                    continue;
                };
                let _ = reply.send(finish_capture(cap));
            }
        }
    }
}

fn list_input_devices() -> Result<Vec<String>> {
    let host = cpal::default_host();
    let mut names = Vec::new();
    for device in host.input_devices().context("listing input devices")? {
        if let Ok(name) = device.name() {
            names.push(name);
        }
    }
    Ok(names)
}

fn pick_device(name: Option<String>) -> Result<cpal::Device> {
    let host = cpal::default_host();
    if let Some(target) = name {
        for device in host.input_devices().context("listing input devices")? {
            if device.name().map(|n| n == target).unwrap_or(false) {
                return Ok(device);
            }
        }
        log::warn!("requested mic \"{target}\" not found, falling back to default");
    }
    host.default_input_device()
        .ok_or_else(|| anyhow!("no default input device"))
}

fn start_capture(device_name: Option<String>, max_duration: Duration) -> Result<ActiveCapture> {
    let device = pick_device(device_name)?;
    let supported = device
        .default_input_config()
        .context("getting default input config")?;
    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.config();
    let source_rate = config.sample_rate.0;
    let channels = config.channels;

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(
        (source_rate as usize) * (channels as usize) * 2,
    )));
    let samples_cb = samples.clone();

    let err_fn = |err| log::warn!("audio stream error: {err}");

    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                let mut buf = samples_cb.lock();
                buf.extend_from_slice(data);
            },
            err_fn,
            None,
        )?,
        SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _| {
                let mut buf = samples_cb.lock();
                buf.extend(data.iter().map(|s| s.to_sample::<f32>()));
            },
            err_fn,
            None,
        )?,
        SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _| {
                let mut buf = samples_cb.lock();
                buf.extend(data.iter().map(|s| s.to_sample::<f32>()));
            },
            err_fn,
            None,
        )?,
        other => return Err(anyhow!("unsupported sample format: {other:?}")),
    };

    stream.play().context("starting audio stream")?;

    Ok(ActiveCapture {
        stream,
        samples,
        source_rate,
        channels,
        started_at: Instant::now(),
        max_duration,
    })
}

fn finish_capture(cap: ActiveCapture) -> Result<Recording> {
    let elapsed = cap.started_at.elapsed();
    let truncated = elapsed > cap.max_duration;
    drop(cap.stream); // stop the stream

    let raw = std::mem::take(&mut *cap.samples.lock());
    if raw.is_empty() {
        return Err(anyhow!("captured no audio samples"));
    }

    // Downmix to mono.
    let mono = if cap.channels > 1 {
        downmix_to_mono(&raw, cap.channels as usize)
    } else {
        raw
    };

    // Resample to 16 kHz if needed.
    let resampled = if cap.source_rate == TARGET_SAMPLE_RATE {
        mono
    } else {
        resample_to_target(&mono, cap.source_rate, TARGET_SAMPLE_RATE)?
    };

    if truncated {
        log::warn!(
            "recording exceeded max duration ({:?} > {:?}), truncated at stream stop",
            elapsed,
            cap.max_duration
        );
    }

    Ok(Recording {
        samples: resampled,
        sample_rate: TARGET_SAMPLE_RATE,
        duration: elapsed.min(cap.max_duration),
    })
}

fn downmix_to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    let frames = interleaved.len() / channels;
    let mut out = Vec::with_capacity(frames);
    for f in 0..frames {
        let start = f * channels;
        let sum: f32 = interleaved[start..start + channels].iter().sum();
        out.push(sum / channels as f32);
    }
    out
}

fn resample_to_target(input: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
    let ratio = to_rate as f64 / from_rate as f64;
    let params = SincInterpolationParameters {
        sinc_len: 128,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 128,
        window: WindowFunction::BlackmanHarris2,
    };

    let chunk_size = 1024;
    let mut resampler =
        SincFixedIn::<f32>::new(ratio, 2.0, params, chunk_size, 1)
            .context("building resampler")?;

    let mut out: Vec<f32> = Vec::with_capacity((input.len() as f64 * ratio) as usize + 1024);
    let mut cursor = 0usize;

    while cursor + chunk_size <= input.len() {
        let chunk: &[f32] = &input[cursor..cursor + chunk_size];
        let processed = resampler
            .process(&[chunk], None)
            .context("resampling chunk")?;
        out.extend_from_slice(&processed[0]);
        cursor += chunk_size;
    }

    if cursor < input.len() {
        let mut tail = input[cursor..].to_vec();
        tail.resize(chunk_size, 0.0);
        let processed = resampler
            .process(&[&tail], None)
            .context("resampling tail")?;
        // Approximate how many output samples correspond to the real tail.
        let real_out = ((input.len() - cursor) as f64 * ratio) as usize;
        let processed_slice = &processed[0];
        let take = real_out.min(processed_slice.len());
        out.extend_from_slice(&processed_slice[..take]);
    }

    Ok(out)
}
