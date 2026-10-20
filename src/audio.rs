use anyhow::{bail, Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, FromSample, SampleFormat, SizedSample, Stream, StreamConfig,
};
use live_translate_rs::{
    dsp::{rms, Segmenter, StreamingResampler},
    language::{validate_pair, Language},
    provider::LocalEngines,
    runtime::{guarded, PlaybackBuffer, SessionControl},
};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct DeviceChoice {
    pub name: String,
    pub channels: u16,
    pub device: Device,
}
pub fn devices() -> Result<(Vec<DeviceChoice>, Vec<DeviceChoice>)> {
    let host = cpal::default_host();
    let inputs = host
        .input_devices()?
        .filter_map(|device| {
            let channels = device.default_input_config().ok()?.channels();
            Some(DeviceChoice {
                name: device.name().unwrap_or_else(|_| "Unnamed input".into()),
                channels,
                device,
            })
        })
        .collect();
    let outputs = host
        .output_devices()?
        .filter_map(|device| {
            let channels = device.default_output_config().ok()?.channels();
            Some(DeviceChoice {
                name: device.name().unwrap_or_else(|_| "Unnamed output".into()),
                channels,
                device,
            })
        })
        .collect();
    Ok((inputs, outputs))
}

#[derive(Default)]
pub struct Stats {
    pub level: AtomicU32,
    pub dropped: AtomicU64,
    pub clipped: AtomicU64,
    pub faults: AtomicU64,
}
pub enum Event {
    Status(String),
    Source(String),
    Translation(String),
}
struct CaptureBlock {
    samples: Vec<f32>,
    at: Instant,
    generation: u64,
}
struct Job {
    text: String,
    at: Instant,
    captioned: bool,
}

pub struct Session {
    pub stats: Arc<Stats>,
    pub events: Receiver<Event>,
    stop: Arc<SessionControl>,
    _input: Option<Stream>,
    _output: Stream,
    commands: Option<SyncSender<Job>>,
    queue: Arc<PlaybackBuffer>,
}
impl Session {
    pub fn send_text(&self, text: &str) -> Result<()> {
        anyhow::ensure!(
            !text.trim().is_empty() && text.chars().count() <= 2000,
            "Enter 1–2000 characters"
        );
        let sender = self
            .commands
            .as_ref()
            .context("This session takes voice input")?;
        self.stop.check()?;
        self.stop.text_jobs.fetch_add(1, Ordering::Release);
        let result = sender
            .try_send(Job {
                text: text.trim().into(),
                at: Instant::now(),
                captioned: false,
            })
            .map_err(|_| anyhow::anyhow!("Speech queue busy; wait or stop"));
        if result.is_err() {
            self.stop.text_jobs.fetch_sub(1, Ordering::Release);
        }
        result
    }
    pub fn queued_seconds(&self) -> f32 {
        self.queue.seconds()
    }
    pub fn failure(&self) -> Option<String> {
        self.stop.failure()
    }
    pub fn text_busy(&self) -> bool {
        self.stop.text_jobs.load(Ordering::Acquire) > 0
    }
    pub fn first_audio_ms(&self) -> Option<f64> {
        self.stop.first_audio_ms()
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop.stop();
        self.queue.clear();
    }
}

fn input_stream<T>(
    device: &Device,
    config: &StreamConfig,
    channel: usize,
    sender: SyncSender<CaptureBlock>,
    stats: Arc<Stats>,
    stop: Arc<SessionControl>,
) -> Result<Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    let errors = stats.clone();
    let error_control = stop.clone();
    Ok(device.build_input_stream(
        config,
        move |data: &[T], _| {
            if stop.is_stopped() {
                return;
            }
            // Capture uses one allocation per callback and nonblocking bounded sends.
            // A preallocated SPSC ring is a future real-time hardening step.
            let samples: Vec<f32> = data
                .chunks_exact(channels)
                .map(|frame| {
                    let v = frame[channel].to_sample::<f32>();
                    if v.abs() >= 0.999 {
                        stats.clipped.fetch_add(1, Ordering::Relaxed);
                    }
                    if v.is_finite() {
                        v.clamp(-1.0, 1.0)
                    } else {
                        0.0
                    }
                })
                .collect();
            stats
                .level
                .store(rms(&samples).to_bits(), Ordering::Relaxed);
            match sender.try_send(CaptureBlock {
                samples,
                at: Instant::now(),
                generation: stats.dropped.load(Ordering::Relaxed),
            }) {
                Err(TrySendError::Full(_)) => {
                    stats.dropped.fetch_add(1, Ordering::Relaxed);
                }
                Err(TrySendError::Disconnected(_)) | Ok(()) => {}
            }
        },
        move |_| {
            errors.faults.fetch_add(1, Ordering::Relaxed);
            error_control.fail("Input device failed or disconnected".into());
        },
        None,
    )?)
}

fn output_stream<T>(
    device: &Device,
    config: &StreamConfig,
    channel: usize,
    queue: Arc<PlaybackBuffer>,
    stats: Arc<Stats>,
    stop: Arc<SessionControl>,
) -> Result<Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let errors = stop.clone();
    Ok(device.build_output_stream(
        config,
        move |data: &mut [T], _| queue.render(data, channels, channel, &stop, T::from_sample),
        move |_| {
            stats.faults.fetch_add(1, Ordering::Relaxed);
            errors.fail("Output device failed or disconnected".into());
        },
        None,
    )?)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    LiveVoice,
    TextTranslation,
    TextToSpeech,
    AudioCheck,
}
impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Self::LiveVoice => "Live voice translation",
            Self::TextTranslation => "Text → translated speech",
            Self::TextToSpeech => "Text → speech",
            Self::AudioCheck => "Audio check (local)",
        }
    }
    pub fn typed(self) -> bool {
        matches!(self, Self::TextTranslation | Self::TextToSpeech)
    }
}
pub struct Settings {
    pub input: Option<DeviceChoice>,
    pub output: DeviceChoice,
    pub input_channel: usize,
    pub output_channel: usize,
    pub source: Language,
    pub target: Language,
    pub threshold: f32,
    pub mode: Mode,
    pub config_path: std::path::PathBuf,
}
pub fn start(settings: Settings) -> Result<Session> {
    if matches!(settings.mode, Mode::LiveVoice | Mode::TextTranslation) {
        validate_pair(settings.source, settings.target)?;
    }
    let provider = if settings.mode == Mode::AudioCheck {
        None
    } else {
        let provider = LocalEngines::from_file(&settings.config_path)?;
        provider.config.validate_piper(settings.target)?;
        Some(provider)
    };
    let stop = Arc::new(SessionControl::default());
    let stats = Arc::new(Stats::default());
    let (events_tx, events) = mpsc::sync_channel(128);
    let (audio_tx, audio_rx) = mpsc::sync_channel::<CaptureBlock>(32);
    let (jobs_tx, jobs_rx) = mpsc::sync_channel::<Job>(2);
    let output_cfg = settings.output.device.default_output_config()?;
    let output_format = output_cfg.sample_format();
    let output_cfg: StreamConfig = output_cfg.into();
    anyhow::ensure!(
        settings.output_channel < output_cfg.channels as usize,
        "Invalid output channel"
    );
    let out_rate = output_cfg.sample_rate.0;
    let queue = Arc::new(PlaybackBuffer::new(out_rate));
    let output = match output_format {
        SampleFormat::F32 => output_stream::<f32>(
            &settings.output.device,
            &output_cfg,
            settings.output_channel,
            queue.clone(),
            stats.clone(),
            stop.clone(),
        )?,
        SampleFormat::I16 => output_stream::<i16>(
            &settings.output.device,
            &output_cfg,
            settings.output_channel,
            queue.clone(),
            stats.clone(),
            stop.clone(),
        )?,
        SampleFormat::U16 => output_stream::<u16>(
            &settings.output.device,
            &output_cfg,
            settings.output_channel,
            queue.clone(),
            stats.clone(),
            stop.clone(),
        )?,
        _ => bail!("Unsupported output format; choose float32/PCM16 device defaults"),
    };
    let mut in_rate = 16000;
    let input = if !settings.mode.typed() {
        let device = &settings
            .input
            .as_ref()
            .context("Select an input device")?
            .device;
        let cfg = device.default_input_config()?;
        let format = cfg.sample_format();
        let cfg: StreamConfig = cfg.into();
        anyhow::ensure!(
            settings.input_channel < cfg.channels as usize,
            "Invalid input channel"
        );
        in_rate = cfg.sample_rate.0;
        Some(match format {
            SampleFormat::F32 => input_stream::<f32>(
                device,
                &cfg,
                settings.input_channel,
                audio_tx,
                stats.clone(),
                stop.clone(),
            )?,
            SampleFormat::I16 => input_stream::<i16>(
                device,
                &cfg,
                settings.input_channel,
                audio_tx,
                stats.clone(),
                stop.clone(),
            )?,
            SampleFormat::U16 => input_stream::<u16>(
                device,
                &cfg,
                settings.input_channel,
                audio_tx,
                stats.clone(),
                stop.clone(),
            )?,
            _ => bail!("Unsupported input format; choose float32/PCM16 device defaults"),
        })
    } else {
        None
    };
    output.play()?;
    if let Some(input) = &input {
        input.play()?;
    }
    let commands = if settings.mode.typed() {
        Some(jobs_tx.clone())
    } else {
        None
    };
    if !settings.mode.typed() {
        struct Clause {
            samples: Vec<f32>,
            at: Instant,
        }
        let (clauses_tx, clauses_rx) = mpsc::sync_channel::<Clause>(2);
        let feeder_control = stop.clone();
        let feeder_events = events_tx.clone();
        thread::spawn(move || {
            guarded(&feeder_control, || -> Result<()> {
                let mut segmenter = if settings.mode == Mode::AudioCheck {
                    Segmenter::new(settings.threshold)
                } else {
                    Segmenter::live(settings.threshold)
                };
                let mut pending = VecDeque::new();
                let frame_size = (in_rate / 50) as usize;
                let mut generation = None;
                while !feeder_control.is_stopped() {
                    let block = match audio_rx.recv_timeout(Duration::from_millis(20)) {
                        Ok(b) => b,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(_) => break,
                    };
                    anyhow::ensure!(
                        generation.is_none_or(|previous| previous == block.generation)
                            && block.at.elapsed() < Duration::from_millis(500),
                        "Input audio gap or backlog detected; reduce load and restart"
                    );
                    generation = Some(block.generation);
                    pending.extend(block.samples);
                    while pending.len() >= frame_size {
                        let frame: Vec<f32> = pending.drain(..frame_size).collect();
                        if let Some(samples) = segmenter.push(&frame) {
                            if settings.mode == Mode::AudioCheck {
                                let _ = feeder_events.try_send(Event::Status(format!(
                                    "Local check: {:.1}s phrase",
                                    samples.len() as f32 / in_rate as f32
                                )));
                            } else {
                                let at = block
                                    .at
                                    .checked_sub(Duration::from_secs_f64(
                                        samples.len() as f64 / in_rate as f64,
                                    ))
                                    .unwrap_or(block.at);
                                clauses_tx.try_send(Clause {samples,at}).map_err(|_|anyhow::anyhow!("Local recognition cannot keep up; stopped instead of accumulating delay"))?;
                            }
                        }
                    }
                }
                Ok(())
            })
        });
        if let Some(asr) = provider.clone() {
            let stop = stop.clone();
            let events = events_tx.clone();
            thread::spawn(move || {
                guarded(&stop, || -> Result<()> {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()?;
                    let _ =
                        events.try_send(Event::Status("Offline voice translation running…".into()));
                    while !stop.is_stopped() {
                        let clause = match clauses_rx.recv_timeout(Duration::from_millis(20)) {
                            Ok(c) => c,
                            Err(mpsc::RecvTimeoutError::Timeout) => continue,
                            Err(_) => break,
                        };
                        stop.check()?;
                        anyhow::ensure!(
                            clause.at.elapsed() < Duration::from_secs(15),
                            "Local recognition request became stale; use a faster/smaller model"
                        );
                        let pcm = live_translate_rs::dsp::resample(&clause.samples, in_rate, 16000);
                        if let Some(text) =
                            runtime.block_on(asr.recognize(&pcm, settings.source, &stop))?
                        {
                            stop.check()?;
                            let _ = events.try_send(Event::Source(text.clone()));
                            jobs_tx.try_send(Job {text,at:clause.at,captioned:true}).map_err(|_|anyhow::anyhow!("Local translation/speech cannot keep up; use a faster model or GPU"))?;
                        }
                    }
                    Ok(())
                })
            });
        }
    }
    if let Some(provider) = provider {
        let stop = stop.clone();
        let events = events_tx;
        let queue = queue.clone();
        thread::spawn(move || {
            guarded(&stop, || -> Result<()> {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                while !stop.is_stopped() {
                    let job = match jobs_rx.recv_timeout(Duration::from_millis(20)) {
                        Ok(j) => j,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(_) => break,
                    };
                    runtime.block_on(async {
                        anyhow::ensure!(
                            job.at.elapsed() < Duration::from_secs(10),
                            "Speech request became stale; stopped"
                        );
                        stop.check()?;
                        let text =
                            if matches!(settings.mode, Mode::TextTranslation | Mode::LiveVoice) {
                                if !job.captioned {
                                    let _ = events.try_send(Event::Source(job.text.clone()));
                                }
                                provider
                                    .translate(&job.text, settings.source, settings.target, &stop)
                                    .await?
                            } else {
                                job.text
                            };
                        let _ = events.try_send(Event::Translation(text.clone()));
                        stop.check()?;
                        let rate = provider.config.voice(settings.target)?.sample_rate;
                        let mut converter = StreamingResampler::new(rate, out_rate);
                        let mut first = true;
                        provider
                            .synthesize_stream(&text, settings.target, &stop, &mut |samples| {
                                let converted = converter.push(samples);
                                queue.push(&converted, &stop)?;
                                if first && !converted.is_empty() {
                                    stop.record_first_audio(job.at.elapsed());
                                    first = false;
                                }
                                Ok(())
                            })
                            .await?;
                        queue.push(&converter.flush(), &stop)?;
                        Ok::<(), anyhow::Error>(())
                    })?;
                    if settings.mode.typed() {
                        stop.text_jobs.fetch_sub(1, Ordering::Release);
                        let _ = events.try_send(Event::Status(
                            "Speech generated — ready for next text".into(),
                        ));
                    }
                }
                Ok(())
            })
        });
    }
    Ok(Session {
        stats,
        events,
        stop,
        _input: input,
        _output: output,
        commands,
        queue,
    })
}
