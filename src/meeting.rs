//! RAM-only source buffering shared by live capture and paced replay diagnostics.
use crate::{
    dsp::{resample, Segmenter, StreamingResampler},
    language::Language,
    provider::LocalEngines,
    runtime::{guarded, PlaybackBuffer, SessionControl},
};
use anyhow::Result;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, SyncSender},
        Arc, Condvar, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

pub enum MeetingEvent {
    Status(String),
    Source(String),
    Translation(String),
    TranslationFor(Language, String),
    Timing(DeliveryTiming),
}
#[derive(Clone, Copy)]
pub struct DeliveryTiming {
    pub language: Language,
    pub clause_id: u64,
    pub source_at: Instant,
    pub source_end: Instant,
    pub first_audio_at: Instant,
    pub recognition_ms: f64,
    pub translation_ms: f64,
}
pub struct Clause {
    pub id: u64,
    pub pcm: Vec<i16>,
    pub at: Instant,
    pub end: Instant,
}
#[derive(Default)]
struct State {
    ready: VecDeque<u64>,
    retained: BTreeMap<u64, Arc<Clause>>,
    samples: usize,
    next: u64,
    closed: bool,
    capacity_reached: bool,
    completed: u64,
}
pub struct SourceBuffer {
    state: Mutex<State>,
    wake: Condvar,
    capacity: usize,
}
#[derive(Clone, Copy, Default)]
pub struct BufferSnapshot {
    pub audio_seconds: f64,
    pub oldest_seconds: f64,
    pub clauses: usize,
    pub capacity_reached: bool,
    pub accepted_clauses: u64,
    pub completed_clauses: u64,
}
impl SourceBuffer {
    pub fn new(minutes: u32) -> Result<Self> {
        anyhow::ensure!(
            (1..=120).contains(&minutes),
            "Buffer capacity must be 1–120 minutes"
        );
        Ok(Self {
            state: Mutex::new(State::default()),
            wake: Condvar::new(),
            capacity: minutes as usize * 60 * 16000,
        })
    }
    pub fn enqueue(
        &self,
        pcm: &[f32],
        at: Instant,
        end: Instant,
        control: &SessionControl,
    ) -> Result<()> {
        anyhow::ensure!(
            !pcm.is_empty() && pcm.len() <= 16000 * 10,
            "Invalid buffered clause length"
        );
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Source buffer unavailable"))?;
        anyhow::ensure!(!state.closed, "Source buffer is closed");
        let id = state.next;
        state.next += 1;
        let clause = Arc::new(Clause {
            id,
            pcm: pcm
                .iter()
                .map(|s| (s.clamp(-1., 1.) * 32767.) as i16)
                .collect(),
            at,
            end,
        });
        state.samples += clause.pcm.len();
        state.retained.insert(id, clause);
        state.ready.push_back(id);
        // Preserve the clause that crosses the limit and any already captured blocks.
        // Finish capture instead of discarding pending speech or cancelling playback.
        if state.samples >= self.capacity && !state.capacity_reached {
            state.capacity_reached = true;
            control.finish_capture();
        }
        self.wake.notify_all();
        Ok(())
    }
    pub fn take(&self, control: &SessionControl) -> Option<Arc<Clause>> {
        let mut state = self.state.lock().ok()?;
        loop {
            if control.is_stopped() {
                return None;
            }
            if let Some(id) = state.ready.pop_front() {
                return state.retained.get(&id).cloned();
            }
            if state.closed {
                return None;
            }
            state = self
                .wake
                .wait_timeout(state, Duration::from_millis(20))
                .ok()?
                .0;
        }
    }
    pub fn complete(&self, id: u64) {
        if let Ok(mut state) = self.state.lock() {
            if let Some(clause) = state.retained.remove(&id) {
                state.samples -= clause.pcm.len();
                state.completed += 1;
            }
        }
    }
    pub fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.closed = true;
            self.wake.notify_all();
        }
    }
    pub fn snapshot(&self) -> BufferSnapshot {
        self.state
            .lock()
            .map(|s| BufferSnapshot {
                audio_seconds: s.samples as f64 / 16000.,
                oldest_seconds: s
                    .retained
                    .values()
                    .next()
                    .map(|c| c.at.elapsed().as_secs_f64())
                    .unwrap_or(0.),
                clauses: s.retained.len(),
                capacity_reached: s.capacity_reached,
                accepted_clauses: s.next,
                completed_clauses: s.completed,
            })
            .unwrap_or_default()
    }
    /// Explicit recovery export; no files are written during ordinary buffering.
    pub fn save_pending(&self, path: &std::path::Path) -> Result<()> {
        use std::io::Write;
        let clauses: Vec<_> = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("Source buffer unavailable"))?
            .retained
            .values()
            .cloned()
            .collect();
        anyhow::ensure!(!clauses.is_empty(), "No pending audio to save");
        let bytes = clauses.iter().map(|c| c.pcm.len() as u64 * 2).sum::<u64>();
        anyhow::ensure!(bytes <= u32::MAX as u64 - 36, "Recovery WAV is too large");
        let mut header = crate::dsp::wav_pcm16(&[], 16000);
        header[4..8].copy_from_slice(&(bytes as u32 + 36).to_le_bytes());
        header[40..44].copy_from_slice(&(bytes as u32).to_le_bytes());
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        let mut file = std::io::BufWriter::new(file);
        file.write_all(&header)?;
        for clause in clauses {
            let bytes: Vec<_> = clause.pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
            file.write_all(&bytes)?;
        }
        file.flush()?;
        Ok(())
    }
}

/// Keep phrase boundaries identical in physical capture and file replay.
pub struct MeetingCapture {
    segmenter: Segmenter,
    rate: u32,
    epoch: u64,
    last_end: Option<Instant>,
    source: Arc<SourceBuffer>,
    control: Arc<SessionControl>,
}
impl MeetingCapture {
    pub fn new(
        rate: u32,
        threshold: f32,
        source: Arc<SourceBuffer>,
        control: Arc<SessionControl>,
    ) -> Self {
        Self {
            segmenter: Segmenter::live(threshold),
            rate,
            epoch: 0,
            last_end: None,
            source,
            control,
        }
    }
    fn save(&self, samples: &[f32], end: Instant) -> Result<()> {
        let at = end
            .checked_sub(Duration::from_secs_f64(
                samples.len() as f64 / self.rate as f64,
            ))
            .unwrap_or(end);
        self.source
            .enqueue(&resample(samples, self.rate, 16000), at, end, &self.control)
    }
    pub fn push(&mut self, frame: &[f32], end: Instant, epoch: u64) -> Result<()> {
        if self.epoch != epoch {
            self.flush()?;
            self.epoch = epoch;
        }
        self.last_end = Some(end);
        if let Some(pcm) = self.segmenter.push(frame) {
            self.save(&pcm, end)?;
        }
        Ok(())
    }
    pub fn flush(&mut self) -> Result<()> {
        if let (Some(pcm), Some(end)) = (self.segmenter.preview(), self.last_end) {
            self.save(&pcm, end)?;
        }
        self.segmenter.reset();
        self.last_end = None;
        Ok(())
    }
    pub fn finish(&mut self) -> Result<()> {
        self.flush()?;
        self.source.close();
        Ok(())
    }
}

pub struct MeetingOutput {
    pub language: Language,
    pub rate: u32,
    pub playback: Arc<PlaybackBuffer>,
}
#[derive(Default)]
struct OutputState {
    pending: BTreeMap<u64, Instant>,
    completed: u64,
    failed: u64,
    gaps: VecDeque<(f64, f64)>,
    delay: Option<f64>,
    history: VecDeque<(f64, f64)>,
}
#[derive(Clone)]
pub struct OutputSnapshot {
    pub language: Language,
    pub pending: usize,
    pub completed: u64,
    pub failed: u64,
    pub gaps: Vec<(f64, f64)>,
    pub oldest_seconds: f64,
    pub delay_seconds: Option<f64>,
    pub history: Vec<(f64, f64)>,
}
pub struct MeetingPipeline {
    pub source: Arc<SourceBuffer>,
    done: Arc<AtomicUsize>,
    outputs: Vec<(Language, Arc<Mutex<OutputState>>)>,
}
struct Delivery {
    remaining: AtomicUsize,
    source: Arc<SourceBuffer>,
    clause: Arc<Clause>,
}
impl Delivery {
    fn complete(&self) {
        if self.remaining.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.source.complete(self.clause.id);
        }
    }
}
struct Recognized {
    delivery: Arc<Delivery>,
    text: String,
    recognition_ms: f64,
}
impl MeetingPipeline {
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        engine: LocalEngines,
        source_language: Language,
        target: Language,
        output_rate: u32,
        playback: Arc<PlaybackBuffer>,
        control: Arc<SessionControl>,
        events: SyncSender<MeetingEvent>,
        minutes: u32,
    ) -> Result<Self> {
        Self::start_multi(
            engine,
            source_language,
            vec![MeetingOutput {
                language: target,
                rate: output_rate,
                playback,
            }],
            control,
            events,
            minutes,
            None,
            Instant::now(),
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn start_multi(
        engine: LocalEngines,
        source_language: Language,
        routes: Vec<MeetingOutput>,
        control: Arc<SessionControl>,
        events: SyncSender<MeetingEvent>,
        minutes: u32,
        transcripts: Option<Arc<crate::transcript::Transcripts>>,
        epoch: Instant,
    ) -> Result<Self> {
        anyhow::ensure!(
            !routes.is_empty() && routes.len() <= 8,
            "Choose one to eight translation outputs"
        );
        for (i, route) in routes.iter().enumerate() {
            crate::language::validate_pair(source_language, route.language)?;
            anyhow::ensure!(
                !routes[..i].iter().any(|r| r.language == route.language),
                "Duplicate output language"
            );
            engine.config.validate_piper(route.language)?;
        }
        let multiple = routes.len() > 1;
        let source = Arc::new(SourceBuffer::new(minutes)?);
        let done = Arc::new(AtomicUsize::new(routes.len()));
        let mut outputs = Vec::new();
        let mut senders = Vec::new();
        // Channels are logically bounded by retained source and the 12,000-job
        // per-language guard below. ASR never waits for a slow output's playback.
        for route in routes {
            let voice_rate = engine.config.validate_piper(route.language)?.sample_rate;
            let state = Arc::new(Mutex::new(OutputState::default()));
            outputs.push((route.language, state.clone()));
            let (tx, rx) = mpsc::channel::<Arc<Recognized>>();
            senders.push((tx, state.clone()));
            let engine = engine.clone();
            let c = control.clone();
            let e = events.clone();
            let finished = done.clone();
            let writer = transcripts.clone();
            thread::spawn(move || {
                guarded(&c, || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()?;
                    loop {
                        c.check()?;
                        let job = match rx.recv_timeout(Duration::from_millis(20)) {
                            Ok(j) => j,
                            Err(mpsc::RecvTimeoutError::Timeout) => continue,
                            Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        };
                        let clause = &job.delivery.clause;
                        let started = Instant::now();
                        let translated = match runtime.block_on(engine.translate(
                            &job.text,
                            source_language,
                            route.language,
                            &c,
                        )) {
                            Ok(text) => text,
                            Err(error) => {
                                // Cancellation is never converted into an untranslated cue.
                                c.check()?;
                                if let Some(writer) = &writer {
                                    writer.untranslated(
                                        route.language,
                                        clause.id,
                                        clause.at,
                                        clause.end,
                                        &job.text,
                                        &format!("{error:#}"),
                                    )?;
                                }
                                let marker = format!("\"Untranslated: {}\"", job.text);
                                let _ = e.try_send(if multiple {
                                    MeetingEvent::TranslationFor(route.language, marker)
                                } else {
                                    MeetingEvent::Translation(marker)
                                });
                                let _ = e.try_send(MeetingEvent::Status(format!(
                                    "{} clause {} untranslated; continuing",
                                    crate::language::info(route.language).name,
                                    clause.id
                                )));
                                {
                                    let mut state = state
                                        .lock()
                                        .map_err(|_| anyhow::anyhow!("Output state unavailable"))?;
                                    state.pending.remove(&clause.id);
                                    state.failed += 1;
                                    state.delay = None;
                                    state.gaps.push_back((
                                        clause.at.saturating_duration_since(epoch).as_secs_f64(),
                                        clause.end.saturating_duration_since(epoch).as_secs_f64(),
                                    ));
                                    if state.gaps.len() > 12000 {
                                        state.gaps.pop_front();
                                    }
                                }
                                job.delivery.complete();
                                continue;
                            }
                        };
                        let translation_ms = started.elapsed().as_secs_f64() * 1000.;
                        if let Some(writer) = &writer {
                            writer.write(route.language, clause.at, clause.end, &translated)?;
                        }
                        let _ = e.try_send(if multiple {
                            MeetingEvent::TranslationFor(route.language, translated.clone())
                        } else {
                            MeetingEvent::Translation(translated.clone())
                        });
                        let mut converter = StreamingResampler::new(voice_rate, route.rate);
                        let mut first = true;
                        runtime.block_on(engine.synthesize_stream(
                            &translated,
                            route.language,
                            &c,
                            &mut |pcm| {
                                let converted = converter.push(pcm);
                                route.playback.push(&converted, &c)?;
                                if first && !converted.is_empty() {
                                    let now = Instant::now();
                                    let delay = now.duration_since(clause.at).as_secs_f64();
                                    c.record_first_audio(now.duration_since(clause.at));
                                    if let Ok(mut state) = state.lock() {
                                        state.delay = Some(delay);
                                        state.history.push_back((
                                            clause
                                                .at
                                                .saturating_duration_since(epoch)
                                                .as_secs_f64(),
                                            delay,
                                        ));
                                        if state.history.len() > 12000 {
                                            state.history.pop_front();
                                        }
                                    }
                                    let _ = e.try_send(MeetingEvent::Timing(DeliveryTiming {
                                        language: route.language,
                                        clause_id: clause.id,
                                        source_at: clause.at,
                                        source_end: clause.end,
                                        first_audio_at: now,
                                        recognition_ms: job.recognition_ms,
                                        translation_ms,
                                    }));
                                    if let Some(writer) = &writer {
                                        writer.timing(route.language, clause.id, clause.at, now)?;
                                    }
                                    first = false;
                                }
                                Ok(())
                            },
                        ))?;
                        route.playback.push(&converter.flush(), &c)?;
                        anyhow::ensure!(
                            !first,
                            "Voice returned no audio for {}",
                            crate::language::info(route.language).name
                        );
                        {
                            let mut state = state
                                .lock()
                                .map_err(|_| anyhow::anyhow!("Output state unavailable"))?;
                            state.pending.remove(&clause.id);
                            state.completed += 1;
                        }
                        job.delivery.complete();
                    }
                    Ok(())
                });
                finished.fetch_sub(1, Ordering::AcqRel);
            });
        }
        let buffer = source.clone();
        let c = control;
        let e = events;
        thread::spawn(move || {
            guarded(&c, || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                while let Some(clause) = buffer.take(&c) {
                    let started = Instant::now();
                    let pcm: Vec<f32> = clause.pcm.iter().map(|s| *s as f32 / 32768.).collect();
                    if let Some(text) =
                        runtime.block_on(engine.recognize(&pcm, source_language, &c))?
                    {
                        anyhow::ensure!(
                            text.chars().count() <= 8000,
                            "Recognition phrase exceeds text buffer limit"
                        );
                        if let Some(writer) = &transcripts {
                            writer.write(source_language, clause.at, clause.end, &text)?;
                        }
                        let _ = e.try_send(MeetingEvent::Source(text.clone()));
                        let id = clause.id;
                        let at = clause.at;
                        let job = Arc::new(Recognized {
                            delivery: Arc::new(Delivery {
                                remaining: AtomicUsize::new(senders.len()),
                                source: buffer.clone(),
                                clause,
                            }),
                            text,
                            recognition_ms: started.elapsed().as_secs_f64() * 1000.,
                        });
                        for (sender, state) in &senders {
                            {
                                let mut state = state
                                    .lock()
                                    .map_err(|_| anyhow::anyhow!("Output state unavailable"))?;
                                anyhow::ensure!(state.pending.len()<12000,"Language text backlog capacity reached; pending source retained");
                                state.pending.insert(id, at);
                            }
                            sender.send(job.clone()).map_err(|_| {
                                anyhow::anyhow!("Language worker disconnected; source retained")
                            })?;
                        }
                    } else {
                        buffer.complete(clause.id);
                    }
                }
                Ok(())
            });
        });
        Ok(Self {
            source,
            done,
            outputs,
        })
    }
    pub fn processing_done(&self) -> bool {
        self.done.load(Ordering::Acquire) == 0
    }
    pub fn output_snapshots(&self) -> Vec<OutputSnapshot> {
        self.outputs
            .iter()
            .filter_map(|(language, state)| {
                let state = state.lock().ok()?;
                Some(OutputSnapshot {
                    language: *language,
                    pending: state.pending.len(),
                    completed: state.completed,
                    failed: state.failed,
                    gaps: state.gaps.iter().copied().collect(),
                    oldest_seconds: state
                        .pending
                        .values()
                        .next()
                        .map(|at| at.elapsed().as_secs_f64())
                        .unwrap_or(0.),
                    delay_seconds: state.delay,
                    history: state.history.iter().copied().collect(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translation_failures_continue_each_route_and_persist_markers_without_audio() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
        };
        fn serve(replies: Vec<(u16, &'static str)>) -> (u16, thread::JoinHandle<()>) {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let port = listener.local_addr().unwrap().port();
            let task = thread::spawn(move || {
                for (status, body) in replies {
                    let deadline = Instant::now() + Duration::from_secs(5);
                    let mut stream = loop {
                        match listener.accept() {
                            Ok((stream, _)) => break stream,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                assert!(Instant::now() < deadline, "Expected next clause request");
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(error) => panic!("{error}"),
                        }
                    };
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut bytes = Vec::new();
                    let mut chunk = [0u8; 2048];
                    loop {
                        let n = stream.read(&mut chunk).unwrap();
                        assert!(n > 0);
                        bytes.extend_from_slice(&chunk[..n]);
                        if let Some(at) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                            let headers = String::from_utf8_lossy(&bytes[..at]).to_lowercase();
                            let length = headers.lines().find_map(|line| {
                                line.strip_prefix("content-length:")
                                    .and_then(|value| value.trim().parse::<usize>().ok())
                            });
                            if length.is_some_and(|length| bytes.len() >= at + 4 + length)
                                || (length.is_none() && bytes.ends_with(b"0\r\n\r\n"))
                            {
                                break;
                            }
                        }
                    }
                    write!(stream,"HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
                }
            });
            (port, task)
        }
        let (asr_port, asr) = serve(vec![
            (200, r#"{"text":"First English clause."}"#),
            (200, r#"{"text":"Second English clause."}"#),
        ]);
        let (translation_port, translation) = serve(vec![
            (
                200,
                r#"{"choices":[{"finish_reason":"length","message":{"content":"truncated"}}]}"#,
            ),
            (503, "{}"),
            (
                200,
                r#"{"choices":[{"finish_reason":"length","message":{"content":"truncated again"}}]}"#,
            ),
            (503, "{}"),
        ]);
        let root = std::env::temp_dir().join(format!(
            "polystream-recovery-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("piper.exe"),
            b"not executed for untranslated clauses",
        )
        .unwrap();
        std::fs::write(root.join("voice.onnx"), b"fixture").unwrap();
        std::fs::write(
            root.join("voice.onnx.json"),
            br#"{"audio":{"sample_rate":22050}}"#,
        )
        .unwrap();
        let mut config:crate::config::OfflineConfig=serde_json::from_value(serde_json::json!({"asr_port":asr_port,"translation_port":translation_port,"piper_binary":"piper.exe","voices":{"es":"voice.onnx","de":"voice.onnx"},"whisper_server":"unused","whisper_model":"unused","llama_server":"unused","translation_model":"unused","threads":1,"gpu_layers":0})).unwrap();
        config.root = root.clone();
        let control = Arc::new(SessionControl::default());
        let epoch = Instant::now();
        let writer = Arc::new(
            crate::transcript::Transcripts::create(
                &root,
                &[Language::English, Language::Spanish, Language::German],
                epoch,
            )
            .unwrap(),
        );
        let queues = [
            Arc::new(PlaybackBuffer::new(16000)),
            Arc::new(PlaybackBuffer::new(16000)),
        ];
        let routes = [Language::Spanish, Language::German]
            .into_iter()
            .zip(&queues)
            .map(|(language, playback)| MeetingOutput {
                language,
                rate: 16000,
                playback: playback.clone(),
            })
            .collect();
        let (tx, _unread) = mpsc::sync_channel(1);
        let pipeline = MeetingPipeline::start_multi(
            LocalEngines::new(config).unwrap(),
            Language::English,
            routes,
            control.clone(),
            tx,
            60,
            Some(writer.clone()),
            epoch,
        )
        .unwrap();
        for second in [0, 2] {
            pipeline
                .source
                .enqueue(
                    &[0.1; 320],
                    epoch + Duration::from_secs(second),
                    epoch + Duration::from_secs(second + 1),
                    &control,
                )
                .unwrap();
        }
        pipeline.source.close();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !pipeline.processing_done() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(pipeline.processing_done());
        assert!(control.failure().is_none());
        assert!(!control.is_stopped());
        assert_eq!(pipeline.source.snapshot().clauses, 0);
        assert_eq!(pipeline.source.snapshot().completed_clauses, 2);
        for snapshot in pipeline.output_snapshots() {
            assert_eq!(snapshot.completed, 0);
            assert_eq!(snapshot.failed, 2);
            assert_eq!(snapshot.pending, 0);
            assert_eq!(snapshot.gaps.len(), 2);
            assert!(snapshot.history.is_empty());
            assert!(snapshot.delay_seconds.is_none());
        }
        assert!(queues.iter().all(|queue| queue.seconds() == 0.));
        let failures = std::fs::read_to_string(writer.directory.join("failures.jsonl")).unwrap();
        assert_eq!(failures.lines().count(), 4);
        assert!(failures.contains("finish_reason=length"));
        assert!(failures.contains("HTTP 503"));
        assert!(
            std::fs::read_to_string(writer.directory.join("delays.jsonl"))
                .unwrap()
                .is_empty()
        );
        for entry in std::fs::read_dir(&writer.directory).unwrap().flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name.starts_with("Spanish") || name.starts_with("German") {
                let cues = std::fs::read_to_string(path).unwrap();
                assert_eq!(cues.matches("\"Untranslated:").count(), 2);
                assert!(cues.contains("Second English clause."));
            }
        }
        control.stop();
        asr.join().unwrap();
        translation.join().unwrap();
        drop(pipeline);
        drop(writer);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn source_is_retained_until_every_language_finishes() {
        let c = SessionControl::default();
        let source = Arc::new(SourceBuffer::new(1).unwrap());
        let now = Instant::now();
        source.enqueue(&[0.1; 320], now, now, &c).unwrap();
        let clause = source.take(&c).unwrap();
        let delivery = Delivery {
            remaining: AtomicUsize::new(2),
            source: source.clone(),
            clause,
        };
        delivery.complete();
        assert_eq!(source.snapshot().completed_clauses, 0);
        assert_eq!(source.snapshot().clauses, 1);
        delivery.complete();
        assert_eq!(source.snapshot().completed_clauses, 1);
        assert_eq!(source.snapshot().clauses, 0);
    }
    #[test]
    fn source_retained_through_inference_and_released_in_order() {
        let c = SessionControl::default();
        let buffer = SourceBuffer::new(60).unwrap();
        let now = Instant::now();
        for _ in 0..3 {
            buffer.enqueue(&vec![0.1; 16000], now, now, &c).unwrap();
        }
        for id in 0..3 {
            let item = buffer.take(&c).unwrap();
            assert_eq!(item.id, id);
            assert_eq!(buffer.snapshot().clauses, 3 - id as usize);
            buffer.complete(id);
        }
        assert_eq!(buffer.snapshot().audio_seconds, 0.);
        buffer.close();
        assert!(buffer.take(&c).is_none());
    }
    #[test]
    fn older_speech_is_preserved_instead_of_expiring_during_a_long_backlog() {
        let c = SessionControl::default();
        let b = SourceBuffer::new(60).unwrap();
        let at = Instant::now() - Duration::from_secs(45 * 60);
        b.enqueue(&[0.1; 320], at, at + Duration::from_millis(20), &c)
            .unwrap();
        assert!(b.snapshot().oldest_seconds >= 45. * 60.);
        let clause = b.take(&c).unwrap();
        assert_eq!(clause.pcm.len(), 320);
        assert!(!c.is_stopped());
        b.complete(clause.id);
        assert_eq!(b.snapshot().clauses, 0);
    }
    #[test]
    fn failed_engine_keeps_pending_audio_in_the_open_session() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = [0u8; 4096];
            let _ = stream.read(&mut bytes);
            let _=stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        });
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "live-translate-failure-{}-{stamp}",
            std::process::id()
        ));
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("piper.exe"), b"fixture").unwrap();
        std::fs::write(dir.join("voice.onnx"), b"fixture").unwrap();
        std::fs::write(
            dir.join("voice.onnx.json"),
            br#"{"audio":{"sample_rate":22050}}"#,
        )
        .unwrap();
        let mut config:crate::config::OfflineConfig=serde_json::from_value(serde_json::json!({
            "asr_port":port,"translation_port":1,"piper_binary":"piper.exe","voices":{"es":"voice.onnx"},
            "whisper_server":"unused","whisper_model":"unused","llama_server":"unused","translation_model":"unused","threads":1,"gpu_layers":0})).unwrap();
        config.root = dir.clone();
        let engine = LocalEngines::new(config).unwrap();
        let c = Arc::new(SessionControl::default());
        let (tx, _rx) = mpsc::sync_channel(128);
        let q = Arc::new(PlaybackBuffer::new(16000));
        let pipeline = MeetingPipeline::start(
            engine,
            Language::English,
            Language::Spanish,
            16000,
            q,
            c.clone(),
            tx,
            60,
        )
        .unwrap();
        let now = Instant::now();
        pipeline.source.enqueue(&[0.1; 320], now, now, &c).unwrap();
        pipeline.source.close();
        let deadline = Instant::now() + Duration::from_secs(3);
        while !pipeline.processing_done() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(c.failure().is_some());
        assert_eq!(pipeline.source.snapshot().clauses, 1);
        assert_eq!(pipeline.source.snapshot().completed_clauses, 0);
        c.stop();
        server.join().unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn capacity_pauses_capture_and_preserves_pending_audio_for_drain() {
        let c = SessionControl::default();
        let b = SourceBuffer::new(1).unwrap();
        let now = Instant::now();
        for _ in 0..7 {
            b.enqueue(&vec![0.1; 16000 * 10], now, now, &c).unwrap();
        }
        assert!(c.capture_finished.load(Ordering::Acquire));
        assert!(!c.is_stopped());
        assert_eq!(b.snapshot().audio_seconds, 70.);
        assert!(b.snapshot().capacity_reached);
        b.close();
        for id in 0..7 {
            assert_eq!(b.take(&c).unwrap().id, id);
            b.complete(id);
        }
        assert!(b.take(&c).is_none());
    }
    #[test]
    fn music_boundary_flushes_partial_phrase_once_without_joining_speakers() {
        let c = Arc::new(SessionControl::default());
        let b = Arc::new(SourceBuffer::new(60).unwrap());
        let mut capture = MeetingCapture::new(16000, 0.015, b.clone(), c.clone());
        let start = Instant::now();
        for i in 0..10 {
            capture
                .push(&[0.1; 320], start + Duration::from_millis(20 * i), 0)
                .unwrap();
        }
        capture.flush().unwrap();
        capture.flush().unwrap();
        assert_eq!(b.snapshot().clauses, 1);
        for i in 0..10 {
            capture
                .push(
                    &[0.2; 320],
                    start + Duration::from_secs(10) + Duration::from_millis(20 * i),
                    1,
                )
                .unwrap();
        }
        capture.finish().unwrap();
        assert_eq!(b.snapshot().clauses, 2);
        let first = b.take(&c).unwrap();
        let second = b.take(&c).unwrap();
        assert!(second.at.duration_since(first.end) > Duration::from_secs(9));
    }
    #[test]
    fn finish_cannot_be_resumed_as_music_pause() {
        let c = SessionControl::default();
        c.pause_capture(true);
        c.pause_capture(false);
        assert!(!c.capture_paused.load(Ordering::Acquire));
        c.finish_capture();
        c.pause_capture(false);
        assert!(c.capture_paused.load(Ordering::Acquire));
    }
    #[test]
    fn recovery_export_keeps_inflight_audio_and_does_not_overwrite_or_delete_it() {
        let c = SessionControl::default();
        let b = SourceBuffer::new(60).unwrap();
        let now = Instant::now();
        b.enqueue(&[0.25; 320], now, now, &c).unwrap();
        let _inflight = b.take(&c).unwrap();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "live-translate-test-{}-{stamp}.wav",
            std::process::id()
        ));
        b.save_pending(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(bytes.len(), 44 + 640);
        assert!(b.save_pending(&path).is_err());
        assert_eq!(b.snapshot().clauses, 1);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn two_fifteen_minute_speakers_fit_with_another_thirty_minutes_headroom() {
        let c = SessionControl::default();
        let b = SourceBuffer::new(60).unwrap();
        let now = Instant::now();
        let pcm = vec![0.1; 16000 * 10];
        for _ in 0..180 {
            b.enqueue(&pcm, now, now, &c).unwrap();
        }
        assert_eq!(b.snapshot().audio_seconds, 1800.);
        assert_eq!(b.snapshot().clauses, 180);
        assert!(!c.capture_finished.load(Ordering::Acquire));
    }
}
