//! Real-time shared-ASR replay into two independent playback sinks, without devices.
use anyhow::{Context, Result};
use live_translate_rs::{
    language::{info, Language},
    meeting::{MeetingCapture, MeetingOutput, MeetingPipeline},
    provider::LocalEngines,
    runtime::{PlaybackBuffer, SessionControl},
    transcript::Transcripts,
};
use serde_json::json;
use std::{
    fs,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(
        (4..=6).contains(&args.len()),
        "Usage: replay_dual CONFIG PCM16_MONO_16KHZ NEW_OUTPUT_DIR [MUSIC_INTERVALS_JSON or -] [es,de or zh-Hans]"
    );
    let languages: Vec<Language> = args
        .get(5)
        .map(String::as_str)
        .unwrap_or("es,de")
        .split(',')
        .map(|code| {
            live_translate_rs::language::LANGUAGES
                .iter()
                .find(|entry| entry.text_code == code)
                .map(|entry| entry.language)
                .context("Unsupported test language")
        })
        .collect::<Result<_>>()?;
    anyhow::ensure!(
        !languages.is_empty() && languages.len() <= 8,
        "Use one to eight test outputs"
    );
    let intervals: Vec<[f64; 2]> = if args.get(4).is_some_and(|s| s != "-") {
        serde_json::from_slice(&fs::read(&args[4])?)?
    } else {
        Vec::new()
    };
    anyhow::ensure!(
        intervals
            .iter()
            .all(|[a, b]| a.is_finite() && b.is_finite() && *a >= 0. && b > a),
        "Invalid music intervals"
    );
    let mut music_paused = false;
    let mut excluded_seconds = 0.;
    let output = Path::new(&args[3]);
    fs::create_dir(output).context("Use a new results directory")?;
    let epoch = Instant::now();
    let c = Arc::new(SessionControl::default());
    let mut transcript_languages = vec![Language::English];
    transcript_languages.extend_from_slice(&languages);
    let writer = Arc::new(Transcripts::create(output, &transcript_languages, epoch)?);
    let engine = LocalEngines::from_file(Path::new(&args[1]))?.with_translation_cache(false);
    let mut input = BufReader::new(fs::File::open(&args[2])?);
    let bytes = input.get_ref().metadata()?.len();
    anyhow::ensure!(bytes > 0 && bytes % 2 == 0, "Expected PCM16 mono 16kHz");
    let queues: Vec<_> = (0..languages.len())
        .map(|_| Arc::new(PlaybackBuffer::new(16000)))
        .collect();
    let routes = languages
        .into_iter()
        .zip(&queues)
        .map(|(language, q)| MeetingOutput {
            language,
            rate: 16000,
            playback: q.clone(),
        })
        .collect();
    // A deliberately tiny, unread notification channel: transcripts/metrics must
    // remain complete even when every UI notification is dropped.
    let (tx, _rx) = mpsc::sync_channel(1);
    let pipeline = MeetingPipeline::start_multi(
        engine,
        Language::English,
        routes,
        c.clone(),
        tx,
        60,
        Some(writer.clone()),
        epoch,
    )?;
    let mut capture = MeetingCapture::new(16000, 0.015, pipeline.source.clone(), c.clone());
    let exit = Arc::new(AtomicBool::new(false));
    let mut sinks = Vec::new();
    for (index, queue) in queues.iter().enumerate() {
        let queue = queue.clone();
        let c = c.clone();
        let exit = exit.clone();
        let path = output.join(format!("output-{index}.pcm"));
        sinks.push(thread::spawn(move || -> Result<()> {
            let mut file = BufWriter::new(fs::File::create(path)?);
            let mut tick = 0;
            while !exit.load(Ordering::Acquire) && !c.is_stopped() {
                thread::sleep(
                    (epoch + Duration::from_millis(tick * 20))
                        .saturating_duration_since(Instant::now()),
                );
                let mut frame = [0f32; 320];
                queue.render(&mut frame, 1, 0, &c, |v| v);
                for sample in frame {
                    file.write_all(
                        &((sample * 32767.).clamp(-32768., 32767.) as i16).to_le_bytes(),
                    )?;
                }
                tick += 1;
            }
            file.flush()?;
            Ok(())
        }));
    }
    let result = (|| -> Result<()> {
        let mut consumed = 0u64;
        let mut last = Instant::now();
        loop {
            c.check()
                .with_context(|| c.failure().unwrap_or_else(|| "Stopped".into()))?;
            if consumed < bytes {
                anyhow::ensure!(
                    !c.capture_finished.load(Ordering::Acquire),
                    "Source capacity reached before input ended"
                );
                let count = (bytes - consumed).min(640) as usize;
                let mut frame = [0u8; 640];
                input.read_exact(&mut frame[..count])?;
                consumed += count as u64;
                let end = epoch + Duration::from_secs_f64(consumed as f64 / 32000.);
                thread::sleep(end.saturating_duration_since(Instant::now()));
                let pcm: Vec<_> = frame[..count]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|s| i16::from_le_bytes(*s) as f32 / 32768.)
                    .collect();
                let midpoint = (consumed as f64 - count as f64 / 2.) / 32000.;
                let excluded = intervals
                    .iter()
                    .any(|[a, b]| midpoint >= *a && midpoint < *b);
                if excluded != music_paused {
                    capture.flush()?;
                    c.pause_capture(excluded);
                    music_paused = excluded;
                }
                if excluded {
                    excluded_seconds += count as f64 / 32000.;
                } else {
                    capture.push(&pcm, end, c.capture_epoch.load(Ordering::Acquire))?;
                }
                if consumed == bytes {
                    capture.finish()?;
                }
            } else {
                if pipeline.processing_done() && queues.iter().all(|q| q.seconds() == 0.) {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
            anyhow::ensure!(
                epoch.elapsed().as_secs_f64() < bytes as f64 / 32000. + 600.,
                "Drain exceeded ten-minute limit"
            );
            let pending_source = pipeline.source.snapshot();
            let snapshots = pipeline.output_snapshots();
            anyhow::ensure!(
                snapshots
                    .iter()
                    .all(|s| s.oldest_seconds <= 600. && s.delay_seconds.unwrap_or(0.) <= 600.)
                    && pending_source.oldest_seconds <= 600.,
                "Delay exceeded ten minutes"
            );
            if last.elapsed() >= Duration::from_secs(1) {
                let outputs:Vec<_>=snapshots.iter().map(|s|json!({"language":info(s.language).text_code,"completed":s.completed,"untranslated":s.failed,"pending":s.pending,"oldest_seconds":s.oldest_seconds,"last_delay_seconds":s.delay_seconds})).collect();
                fs::write(
                    output.join("progress.json"),
                    serde_json::to_vec_pretty(
                        &json!({"input_seconds":bytes as f64/32000.,"consumed_seconds":consumed as f64/32000.,"wall_seconds":epoch.elapsed().as_secs_f64(),"outputs":outputs,"pending_source_oldest_seconds":pending_source.oldest_seconds,"pending_source_seconds":pending_source.audio_seconds}),
                    )?,
                )?;
                last = Instant::now();
            }
        }
        let snapshots = pipeline.output_snapshots();
        let mut results = Vec::new();
        for s in snapshots {
            let mut delays: Vec<_> = s.history.iter().map(|p| p.1).collect();
            delays.sort_by(f64::total_cmp);
            anyhow::ensure!(
                s.completed + s.failed > 0 && s.completed == delays.len() as u64 && s.pending == 0,
                "Missing output timing/completion"
            );
            let n = delays.len();
            results.push(json!({"language":info(s.language).text_code,"completed":s.completed,"untranslated":s.failed,"handled":s.completed+s.failed,"median_seconds":if n>0 {Some(delays[n/2])} else {None},"p95_seconds":if n>0 {Some(delays[((n as f64*0.95).ceil() as usize).saturating_sub(1)])} else {None},"max_seconds":delays.last()}));
        }
        let source = pipeline.source.snapshot();
        anyhow::ensure!(
            source.clauses == 0 && source.accepted_clauses == source.completed_clauses,
            "Pending source after finish"
        );
        let gaps = results
            .iter()
            .any(|result| result["untranslated"].as_u64().unwrap_or(0) > 0);
        fs::write(
            output.join("summary.json"),
            serde_json::to_vec_pretty(
                &json!({"status":if gaps {"completed_with_gaps"} else {"passed"},"source_passes":1,"accepted_clauses":source.accepted_clauses,"input_seconds":bytes as f64/32000.,"wall_seconds":epoch.elapsed().as_secs_f64(),"outputs":results,"transcripts":writer.directory,"physical_devices_tested":false,"music_excluded_seconds":excluded_seconds}),
            )?,
        )?;
        Ok(())
    })();
    exit.store(true, Ordering::Release);
    c.stop();
    for sink in sinks {
        sink.join()
            .map_err(|_| anyhow::anyhow!("Playback sink panicked"))??;
    }
    if let Err(error) = &result {
        fs::write(output.join("failure.txt"), format!("{error:#}"))?;
    }
    result
}
