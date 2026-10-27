//! Paced recording replay using the SAME buffered pipeline as the desktop app.
use anyhow::{Context, Result};
use live_translate_rs::{
    language::Language,
    meeting::{MeetingCapture, MeetingEvent, MeetingPipeline},
    provider::LocalEngines,
    runtime::{guarded, PlaybackBuffer, SessionControl},
};
use serde_json::json;
use std::{
    fs,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
    sync::{atomic::Ordering, mpsc, Arc},
    thread,
    time::{Duration, Instant},
};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(
        (5..=6).contains(&args.len()),
        "Usage: replay_buffered CONFIG PCM16_MONO_16KHZ es|de NEW_OUTPUT_DIR [MUSIC_INTERVALS_JSON]"
    );
    let target = match args[3].as_str() {
        "es" => Language::Spanish,
        "de" => Language::German,
        _ => anyhow::bail!("Use es or de"),
    };
    let engine = LocalEngines::from_file(Path::new(&args[1]))?.with_translation_cache(false);
    let input_file = fs::File::open(&args[2])?;
    let byte_count = input_file.metadata()?.len();
    anyhow::ensure!(
        byte_count > 0 && byte_count % 2 == 0 && byte_count <= 16000u64 * 2 * 3600 * 8,
        "Expected PCM16 mono 16 kHz, maximum eight hours"
    );
    let input_seconds = byte_count as f64 / 32000.;
    let frame_count = byte_count.div_ceil(640) as usize;
    let mut input = BufReader::new(input_file);
    let mut consumed_bytes = 0u64;
    let intervals: Vec<[f64; 2]> = if args.len() == 6 {
        serde_json::from_slice(&fs::read(&args[5])?)?
    } else {
        Vec::new()
    };
    anyhow::ensure!(
        intervals.iter().all(|[a, b]| a.is_finite()
            && b.is_finite()
            && *a >= 0.
            && a < b
            && *b <= input_seconds + 0.02),
        "Invalid music intervals"
    );
    let mut excluded_seconds = 0f64;
    let mut music_paused = false;
    let output = Path::new(&args[4]);
    fs::create_dir(output).context("Use a new output directory")?;
    let control = Arc::new(SessionControl::default());
    let playback = Arc::new(PlaybackBuffer::new(16000));
    let (tx, rx) = mpsc::sync_channel(128);
    let pipeline = MeetingPipeline::start(
        engine,
        Language::English,
        target,
        16000,
        playback.clone(),
        control.clone(),
        tx,
        60,
    )?;
    let mut capture = MeetingCapture::new(16000, 0.015, pipeline.source.clone(), control.clone());
    let start = Instant::now();
    let mut peak_audio = 0f64;
    let mut peak_age = 0f64;
    let mut peak_input_lateness = 0f64;
    let mut late_input_frames = 0u64;
    thread::scope(|scope| {
        let c = control.clone();
        let q = playback.clone();
        let p = &pipeline;
        scope.spawn(move || {
            guarded(&c, || {
                let mut file = BufWriter::new(fs::File::create(output.join("playback.pcm"))?);
                let mut tick = 0u64;
                while !c.is_stopped() && (!p.processing_done() || q.seconds() > 0.) {
                    thread::sleep(
                        (start + Duration::from_millis(tick * 20))
                            .saturating_duration_since(Instant::now()),
                    );
                    let mut frame = [0f32; 320];
                    q.render(&mut frame, 1, 0, &c, |v| v);
                    for sample in frame {
                        file.write_all(&((sample.clamp(-1., 1.) * 32767.) as i16).to_le_bytes())?;
                    }
                    tick += 1;
                }
                Ok(())
            })
        });
        let result = (|| -> Result<()> {
            let mut events = fs::File::create(output.join("events.jsonl"))?;
            let mut metrics = fs::File::create(output.join("buffer.jsonl"))?;
            let mut delays = BufWriter::new(fs::File::create(output.join("delays.jsonl"))?);
            let mut latest_delay = 0f64;
            let mut source = 0;
            let mut translations = 0;
            for i in 0..frame_count {
                control.check()?;
                if control.capture_finished.load(Ordering::Acquire) {
                    break;
                }
                let end = start + Duration::from_secs_f64((i + 1) as f64 * 0.02);
                thread::sleep(end.saturating_duration_since(Instant::now()));
                let input_lateness = end.elapsed().as_secs_f64();
                peak_input_lateness = peak_input_lateness.max(input_lateness);
                if input_lateness > 0.5 {
                    late_input_frames += 1;
                }
                let count = (byte_count - consumed_bytes).min(640) as usize;
                let mut bytes = [0u8; 640];
                input.read_exact(&mut bytes[..count])?;
                consumed_bytes += count as u64;
                let frame: Vec<_> = bytes[..count]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|s| i16::from_le_bytes(*s) as f32 / 32768.)
                    .collect();
                let midpoint = (consumed_bytes as f64 - count as f64 / 2.) / 32000.;
                let excluded = intervals
                    .iter()
                    .any(|[a, b]| midpoint >= *a && midpoint < *b);
                if excluded != music_paused {
                    capture.flush()?;
                    control.pause_capture(excluded);
                    music_paused = excluded;
                }
                if excluded {
                    excluded_seconds += count as f64 / 32000.;
                } else {
                    capture.push(&frame, end, control.capture_epoch.load(Ordering::Acquire))?;
                }
                let snapshot = pipeline.source.snapshot();
                peak_audio = peak_audio.max(snapshot.audio_seconds);
                peak_age = peak_age.max(snapshot.oldest_seconds);
                if i % 50 == 0 {
                    writeln!(
                        metrics,
                        "{}",
                        json!({"wall_seconds":start.elapsed().as_secs_f64(),"pending_audio_seconds":snapshot.audio_seconds,"pending_clauses":snapshot.clauses,"oldest_seconds":snapshot.oldest_seconds})
                    )?;
                    metrics.flush()?;
                    fs::write(
                        output.join("progress.json"),
                        serde_json::to_vec_pretty(
                            &json!({"last_first_pcm_delay_seconds":latest_delay,"output_queue_seconds":playback.seconds(),"music_paused":music_paused,"phase":"playing","target":args[3],"input_seconds":input_seconds,"input_consumed_seconds":consumed_bytes as f64/32000.,"wall_seconds":start.elapsed().as_secs_f64(),"input_lateness_seconds":input_lateness,"pending_audio_seconds":snapshot.audio_seconds,"oldest_seconds":snapshot.oldest_seconds,"accepted_clauses":snapshot.accepted_clauses,"completed_clauses":snapshot.completed_clauses}),
                        )?,
                    )?;
                }
                for event in rx.try_iter() {
                    let (kind, text) = match event {
                        MeetingEvent::Source(t) => {
                            source += 1;
                            ("source", t)
                        }
                        MeetingEvent::Translation(t) | MeetingEvent::TranslationFor(_, t) => {
                            translations += 1;
                            ("translation", t)
                        }
                        MeetingEvent::Timing(t) => {
                            latest_delay = t
                                .first_audio_at
                                .saturating_duration_since(t.source_at)
                                .as_secs_f64();
                            writeln!(
                                delays,
                                "{}",
                                json!({"clause_id":t.clause_id,"source_start_seconds":t.source_at.saturating_duration_since(start).as_secs_f64(),"source_end_seconds":t.source_end.saturating_duration_since(start).as_secs_f64(),"first_pcm_seconds":t.first_audio_at.saturating_duration_since(start).as_secs_f64(),"input_start_to_first_pcm_seconds":latest_delay,"input_end_to_first_pcm_seconds":t.first_audio_at.saturating_duration_since(t.source_end).as_secs_f64(),"recognition_ms":t.recognition_ms,"translation_ms":t.translation_ms})
                            )?;
                            delays.flush()?;
                            continue;
                        }
                        MeetingEvent::Status(t) => ("status", t),
                    };
                    writeln!(
                        events,
                        "{}",
                        json!({"at":start.elapsed().as_secs_f64(),"kind":kind,"text":text})
                    )?;
                }
            }
            control.finish_capture();
            capture.finish()?;
            println!(
                "Input complete at {:.1}s; draining preserved speech",
                start.elapsed().as_secs_f64()
            );
            let mut drain_tick = 0usize;
            while !pipeline.processing_done() || playback.seconds() > 0. {
                control.check()?;
                let s = pipeline.source.snapshot();
                peak_audio = peak_audio.max(s.audio_seconds);
                peak_age = peak_age.max(s.oldest_seconds);
                if drain_tick.is_multiple_of(50) {
                    fs::write(
                        output.join("progress.json"),
                        serde_json::to_vec_pretty(
                            &json!({"last_first_pcm_delay_seconds":latest_delay,"output_queue_seconds":playback.seconds(),"phase":"draining","target":args[3],"input_seconds":input_seconds,"input_consumed_seconds":consumed_bytes as f64/32000.,"wall_seconds":start.elapsed().as_secs_f64(),"pending_audio_seconds":s.audio_seconds,"oldest_seconds":s.oldest_seconds,"accepted_clauses":s.accepted_clauses,"completed_clauses":s.completed_clauses}),
                        )?,
                    )?;
                    writeln!(
                        metrics,
                        "{}",
                        json!({"last_first_pcm_delay_seconds":latest_delay,"output_queue_seconds":playback.seconds(),"phase":"draining","wall_seconds":start.elapsed().as_secs_f64(),"pending_audio_seconds":s.audio_seconds,"oldest_seconds":s.oldest_seconds})
                    )?;
                    metrics.flush()?;
                }
                drain_tick += 1;
                thread::sleep(Duration::from_millis(20));
                for event in rx.try_iter() {
                    let (kind, text) = match event {
                        MeetingEvent::Source(t) => {
                            source += 1;
                            ("source", t)
                        }
                        MeetingEvent::Translation(t) | MeetingEvent::TranslationFor(_, t) => {
                            translations += 1;
                            ("translation", t)
                        }
                        MeetingEvent::Timing(t) => {
                            latest_delay = t
                                .first_audio_at
                                .saturating_duration_since(t.source_at)
                                .as_secs_f64();
                            writeln!(
                                delays,
                                "{}",
                                json!({"clause_id":t.clause_id,"source_start_seconds":t.source_at.saturating_duration_since(start).as_secs_f64(),"source_end_seconds":t.source_end.saturating_duration_since(start).as_secs_f64(),"first_pcm_seconds":t.first_audio_at.saturating_duration_since(start).as_secs_f64(),"input_start_to_first_pcm_seconds":latest_delay,"input_end_to_first_pcm_seconds":t.first_audio_at.saturating_duration_since(t.source_end).as_secs_f64(),"recognition_ms":t.recognition_ms,"translation_ms":t.translation_ms})
                            )?;
                            delays.flush()?;
                            continue;
                        }
                        MeetingEvent::Status(t) => ("status", t),
                    };
                    writeln!(
                        events,
                        "{}",
                        json!({"at":start.elapsed().as_secs_f64(),"kind":kind,"text":text})
                    )?;
                }
            }
            events.flush()?;
            delays.flush()?;
            anyhow::ensure!(
                consumed_bytes == byte_count,
                "Capture capacity reached before the end of the recording; test incomplete"
            );
            let snapshot = pipeline.source.snapshot();
            anyhow::ensure!(
                snapshot.clauses == 0 && snapshot.accepted_clauses == snapshot.completed_clauses,
                "Some buffered audio did not finish"
            );
            fs::write(
                output.join("summary.json"),
                serde_json::to_vec_pretty(
                    &json!({"status":"passed","target":args[3],"input_seconds":input_seconds,"wall_seconds":start.elapsed().as_secs_f64(),
                "accepted_clauses":snapshot.accepted_clauses,"completed_clauses":snapshot.completed_clauses,"recognized_clauses":source,"translated_clauses":translations,
                "peak_pending_audio_seconds":peak_audio,"peak_oldest_speech_seconds":peak_age,"pending_audio_seconds":snapshot.audio_seconds,"physical_devices_tested":false,"buffer_minutes":60,"music_excluded_seconds":excluded_seconds,"music_exclusion_applied":!intervals.is_empty(),"peak_input_scheduling_lateness_seconds":peak_input_lateness,"input_frames_over_500ms_late":late_input_frames}),
                )?,
            )?;
            Ok(())
        })();
        if let Err(e) = result {
            control.fail(format!("{e:#}"));
            pipeline.source.close();
        }
    });
    if let Some(error) = control.failure() {
        fs::write(
            output.join("summary.json"),
            serde_json::to_vec_pretty(&json!({"status":"failed","error":error}))?,
        )?;
        if pipeline.source.snapshot().clauses > 0 {
            let _ = pipeline
                .source
                .save_pending(&output.join("pending-source.wav"));
        }
        anyhow::bail!("Buffered replay failed: {error}");
    }
    println!("{}", fs::read_to_string(output.join("summary.json"))?);
    Ok(())
}
