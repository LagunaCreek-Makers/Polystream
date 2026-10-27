use anyhow::{bail, Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, FromSample, SampleFormat, SizedSample, Stream, StreamConfig,
};
use live_translate_rs::{
    dsp::{Segmenter, StreamingResampler},
    language::{validate_pair, Language},
    levels::{AudioLevel, GainRamp, SampleMeter},
    meeting::{BufferSnapshot, MeetingCapture, MeetingOutput, MeetingPipeline, OutputSnapshot},
    provider::LocalEngines,
    runtime::{guarded, PlaybackBuffer, SessionControl},
};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct DeviceChoice {
    pub id: usize,
    pub name: String,
    pub channels: u16,
    pub device: Device,
}
pub fn devices() -> Result<(Vec<DeviceChoice>, Vec<DeviceChoice>)> {
    let host = cpal::default_host();
    let inputs = host
        .input_devices()?
        .enumerate()
        .filter_map(|(id, device)| {
            let channels = device.default_input_config().ok()?.channels();
            Some(DeviceChoice {
                id,
                name: device.name().unwrap_or_else(|_| "Unnamed input".into()),
                channels,
                device,
            })
        })
        .collect();
    let outputs = host
        .output_devices()?
        .enumerate()
        .filter_map(|(id, device)| {
            let channels = device.default_output_config().ok()?.channels();
            Some(DeviceChoice {
                id,
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
pub use live_translate_rs::meeting::MeetingEvent as Event;
struct CaptureBlock {
    samples: Vec<f32>,
    at: Instant,
    generation: u64,
    epoch: u64,
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
    _outputs: Vec<Stream>,
    other_queues: Vec<Arc<PlaybackBuffer>>,
    pub transcript_directory: Option<std::path::PathBuf>,
    commands: Option<SyncSender<Job>>,
    queue: Arc<PlaybackBuffer>,
    meeting: Option<MeetingPipeline>,
    drain_empty_since: Mutex<Option<Instant>>,
}
impl Session {
    pub fn output_snapshots(&self) -> Vec<OutputSnapshot> {
        self.meeting
            .as_ref()
            .map(|m| m.output_snapshots())
            .unwrap_or_default()
    }
    pub fn buffered(&self) -> Option<BufferSnapshot> {
        self.meeting.as_ref().map(|m| m.source.snapshot())
    }
    pub fn save_pending(&self) -> Result<std::path::PathBuf> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!("live-translate-pending-{stamp}.wav"));
        self.meeting
            .as_ref()
            .context("This session has no source buffer")?
            .source
            .save_pending(&path)?;
        Ok(path)
    }
    pub fn music_paused(&self) -> bool {
        self.stop.capture_paused.load(Ordering::Acquire)
    }
    pub fn finishing(&self) -> bool {
        self.stop.capture_finished.load(Ordering::Acquire)
    }
    pub fn pause_for_music(&self, paused: bool) {
        self.stop.pause_capture(paused);
    }
    pub fn finish_pending(&self) {
        self.stop.finish_capture();
    }
    pub fn drained(&self) -> bool {
        let empty = self.meeting.as_ref().is_some_and(|m| {
            m.processing_done()
                && self.queue.seconds() == 0.
                && self.other_queues.iter().all(|q| q.seconds() == 0.)
                && self.failure().is_none()
        });
        let Ok(mut since) = self.drain_empty_since.lock() else {
            return false;
        };
        if !empty {
            *since = None;
            return false;
        }
        // Keep the output stream alive after its final callback handed samples to
        // the device, rather than closing it while hardware is still playing them.
        since.get_or_insert_with(Instant::now).elapsed() >= Duration::from_secs(1)
    }
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
        for queue in &self.other_queues {
            queue.clear();
        }
    }
}

fn input_stream<T>(
    device: &Device,
    config: &StreamConfig,
    channel: usize,
    sender: SyncSender<CaptureBlock>,
    stats: Arc<Stats>,
    stop: Arc<SessionControl>,
    level: Arc<AudioLevel>,
) -> Result<Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    let errors = stats.clone();
    let error_control = stop.clone();
    let mut current_gain = level.gain();
    let sample_rate = config.sample_rate.0;
    Ok(device.build_input_stream(
        config,
        move |data: &[T], _| {
            if stop.is_stopped() || stop.capture_paused.load(Ordering::Acquire) {
                level.clear_meter();
                stats.level.store(0f32.to_bits(), Ordering::Relaxed);
                return;
            }
            // Capture uses one allocation per callback and nonblocking bounded sends.
            // A preallocated SPSC ring is a future real-time hardening step.
            let mut meter = SampleMeter::default();
            let mut ramp = GainRamp::new(current_gain, level.gain(), sample_rate);
            let samples: Vec<f32> = data
                .chunks_exact(channels)
                .map(|frame| {
                    let v = frame[channel].to_sample::<f32>();
                    if v.abs() >= 0.999 {
                        stats.clipped.fetch_add(1, Ordering::Relaxed);
                    }
                    meter.process(v, ramp.advance())
                })
                .collect();
            current_gain = ramp.current();
            level.publish(&meter);
            stats.level.store(meter.rms().to_bits(), Ordering::Relaxed);
            match sender.try_send(CaptureBlock {
                samples,
                at: Instant::now(),
                generation: stats.dropped.load(Ordering::Relaxed),
                epoch: stop.capture_epoch.load(Ordering::Acquire),
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
    routes: Vec<(usize, Arc<PlaybackBuffer>)>,
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
        move |data: &mut [T], _| {
            for sample in data.iter_mut() {
                *sample = T::from_sample(0.);
            }
            for (channel, queue) in &routes {
                queue.render_channel(data, channels, *channel, &stop, T::from_sample);
            }
        },
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
pub struct AdditionalOutput {
    pub level: Arc<AudioLevel>,
    pub device: DeviceChoice,
    pub channel: usize,
    pub language: Language,
}
pub struct Settings {
    pub input_level: Arc<AudioLevel>,
    pub output_level: Arc<AudioLevel>,
    pub additional: Vec<AdditionalOutput>,
    pub transcript_project: std::path::PathBuf,
    pub input: Option<DeviceChoice>,
    pub output: DeviceChoice,
    pub input_channel: usize,
    pub output_channel: usize,
    pub source: Language,
    pub target: Language,
    pub threshold: f32,
    pub mode: Mode,
    pub config_path: std::path::PathBuf,
    pub buffer_minutes: u32,
}
pub fn start(settings: Settings) -> Result<Session> {
    if matches!(settings.mode, Mode::LiveVoice | Mode::TextTranslation) {
        validate_pair(settings.source, settings.target)?;
    }
    let provider = if settings.mode == Mode::AudioCheck {
        None
    } else {
        let mut config = live_translate_rs::config::OfflineConfig::load(&settings.config_path)?;
        if settings.target == Language::Mandarin
            || settings
                .additional
                .iter()
                .any(|output| output.language == Language::Mandarin)
        {
            config.add_installed_mandarin_voice()?;
        }
        if settings.target == Language::French
            || settings
                .additional
                .iter()
                .any(|r| r.language == Language::French)
        {
            config.add_installed_french_voice()?;
        }
        for language in std::iter::once(settings.target)
            .chain(settings.additional.iter().map(|route| route.language))
        {
            if matches!(
                language,
                Language::Russian
                    | Language::Basque
                    | Language::Italian
                    | Language::Japanese
                    | Language::Hindi
                    | Language::Arabic
            ) {
                config.add_installed_language_voice(language)?;
            }
        }
        let provider = LocalEngines::new(config)?;
        provider.config.validate_piper(settings.target)?;
        Some(provider)
    };
    anyhow::ensure!(settings.additional.len() < 8, "Maximum eight outputs");
    for (index, route) in settings.additional.iter().enumerate() {
        anyhow::ensure!(
            settings.mode == Mode::LiveVoice,
            "Multiple outputs require live voice mode"
        );
        validate_pair(settings.source, route.language)?;
        anyhow::ensure!(
            route.language != settings.target
                && !settings.additional[..index]
                    .iter()
                    .any(|r| r.language == route.language),
            "Choose different output languages"
        );
        anyhow::ensure!(
            !(route.device.id == settings.output.id && route.channel == settings.output_channel)
                && !settings.additional[..index]
                    .iter()
                    .any(|r| r.device.id == route.device.id && r.channel == route.channel),
            "Languages cannot share the same output device/channel"
        );
        provider
            .as_ref()
            .context("Missing engines")?
            .config
            .validate_piper(route.language)?;
    }
    let epoch = Instant::now();
    let transcripts = if settings.mode == Mode::LiveVoice {
        let mut languages = vec![settings.source, settings.target];
        for route in &settings.additional {
            languages.push(route.language);
        }
        Some(Arc::new(
            live_translate_rs::transcript::Transcripts::create(
                &settings.transcript_project,
                &languages,
                epoch,
            )?,
        ))
    } else {
        None
    };
    let transcript_directory = transcripts.as_ref().map(|t| t.directory.clone());
    let stop = Arc::new(SessionControl::default());
    let stats = Arc::new(Stats::default());
    let (events_tx, events) = mpsc::sync_channel(128);
    let (audio_tx, audio_rx) = mpsc::sync_channel::<CaptureBlock>(32);
    let (jobs_tx, jobs_rx) = mpsc::sync_channel::<Job>(2);
    let mut devices = vec![(
        settings.output.clone(),
        vec![(
            settings.output_channel,
            settings.target,
            settings.output_level.clone(),
        )],
    )];
    for route in &settings.additional {
        if let Some((_, routes)) = devices
            .iter_mut()
            .find(|(device, _)| device.id == route.device.id)
        {
            routes.push((route.channel, route.language, route.level.clone()));
        } else {
            devices.push((
                route.device.clone(),
                vec![(route.channel, route.language, route.level.clone())],
            ));
        }
    }
    let mut outputs = Vec::new();
    let mut meeting_routes = Vec::new();
    let mut queues = Vec::new();
    let mut rates = Vec::new();
    for (device, routes) in devices {
        let cfg = device.device.default_output_config()?;
        let format = cfg.sample_format();
        let cfg: StreamConfig = cfg.into();
        let mut playback_routes = Vec::new();
        for (channel, language, level) in routes {
            anyhow::ensure!(channel < cfg.channels as usize, "Invalid output channel");
            let queue = Arc::new(PlaybackBuffer::with_level(cfg.sample_rate.0, level));
            rates.push(cfg.sample_rate.0);
            queues.push(queue.clone());
            meeting_routes.push(MeetingOutput {
                language,
                rate: cfg.sample_rate.0,
                playback: queue.clone(),
            });
            playback_routes.push((channel, queue));
        }
        outputs.push(match format {
            SampleFormat::F32 => output_stream::<f32>(
                &device.device,
                &cfg,
                playback_routes,
                stats.clone(),
                stop.clone(),
            )?,
            SampleFormat::I16 => output_stream::<i16>(
                &device.device,
                &cfg,
                playback_routes,
                stats.clone(),
                stop.clone(),
            )?,
            SampleFormat::U16 => output_stream::<u16>(
                &device.device,
                &cfg,
                playback_routes,
                stats.clone(),
                stop.clone(),
            )?,
            _ => bail!("Unsupported output format; choose float32/PCM16 device defaults"),
        });
    }
    let queue = queues[0].clone();
    let out_rate = rates[0];
    let other_queues = queues.into_iter().skip(1).collect();
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
                settings.input_level.clone(),
            )?,
            SampleFormat::I16 => input_stream::<i16>(
                device,
                &cfg,
                settings.input_channel,
                audio_tx,
                stats.clone(),
                stop.clone(),
                settings.input_level.clone(),
            )?,
            SampleFormat::U16 => input_stream::<u16>(
                device,
                &cfg,
                settings.input_channel,
                audio_tx,
                stats.clone(),
                stop.clone(),
                settings.input_level.clone(),
            )?,
            _ => bail!("Unsupported input format; choose float32/PCM16 device defaults"),
        })
    } else {
        None
    };
    for output in &outputs {
        output.play()?;
    }
    if let Some(input) = &input {
        input.play()?;
    }
    let commands = if settings.mode.typed() {
        Some(jobs_tx.clone())
    } else {
        None
    };
    let mut meeting = None;
    if settings.mode == Mode::LiveVoice {
        let pipeline = MeetingPipeline::start_multi(
            provider.clone().context("Missing local engines")?,
            settings.source,
            meeting_routes,
            stop.clone(),
            events_tx.clone(),
            settings.buffer_minutes,
            transcripts,
            epoch,
        )?;
        let mut capture = MeetingCapture::new(
            in_rate,
            settings.threshold,
            pipeline.source.clone(),
            stop.clone(),
        );
        let c = stop.clone();
        thread::spawn(move || {
            guarded(&c, || {
                let mut pending = VecDeque::new();
                let frame_size = (in_rate / 50) as usize;
                let mut generation = None;
                let mut previous_epoch = None;
                while !c.is_stopped() {
                    let block = match audio_rx.recv_timeout(Duration::from_millis(20)) {
                        Ok(block) => block,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            if c.capture_paused.load(Ordering::Acquire) {
                                // No more accepted blocks remain. Keep even a partial phrase.
                                if !pending.is_empty() {
                                    let mut frame: Vec<_> = pending.drain(..).collect();
                                    frame.resize(frame_size, 0.);
                                    capture.push(
                                        &frame,
                                        Instant::now(),
                                        previous_epoch.unwrap_or(0),
                                    )?;
                                }
                                capture.flush()?;
                                if c.capture_finished.load(Ordering::Acquire) {
                                    capture.finish()?;
                                    break;
                                }
                            }
                            continue;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            capture.finish()?;
                            break;
                        }
                    };
                    anyhow::ensure!(
                        generation.is_none_or(|g| g == block.generation)
                            && block.at.elapsed() < Duration::from_millis(500),
                        "Input audio gap or backlog detected; capture could not be preserved"
                    );
                    generation = Some(block.generation);
                    if previous_epoch.is_some_and(|epoch| epoch != block.epoch) {
                        if !pending.is_empty() {
                            let mut frame: Vec<_> = pending.drain(..).collect();
                            frame.resize(frame_size, 0.);
                            capture.push(&frame, block.at, previous_epoch.unwrap_or(0))?;
                        }
                        capture.flush()?;
                    }
                    previous_epoch = Some(block.epoch);
                    pending.extend(block.samples);
                    while pending.len() >= frame_size {
                        let frame: Vec<_> = pending.drain(..frame_size).collect();
                        capture.push(&frame, block.at, block.epoch)?;
                    }
                }
                if c.is_stopped() {
                    // On inference failure retain the tail already accepted by capture.
                    while let Ok(block) = audio_rx.recv_timeout(Duration::from_millis(20)) {
                        pending.extend(block.samples);
                        while pending.len() >= frame_size {
                            let frame: Vec<_> = pending.drain(..frame_size).collect();
                            capture.push(&frame, block.at, block.epoch)?;
                        }
                    }
                    if !pending.is_empty() {
                        let mut frame: Vec<_> = pending.drain(..).collect();
                        frame.resize(frame_size, 0.);
                        capture.push(&frame, Instant::now(), previous_epoch.unwrap_or(0))?;
                    }
                    capture.finish()?;
                }
                Ok(())
            })
        });
        meeting = Some(pipeline);
    } else if !settings.mode.typed() {
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
    if let Some(provider) = provider.filter(|_| settings.mode != Mode::LiveVoice) {
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
        _outputs: outputs,
        other_queues,
        transcript_directory,
        commands,
        queue,
        meeting,
        drain_empty_since: Mutex::new(None),
    })
}
