//! Real-time file replay through production segmentation, inference and playback buffer.
//! Input is raw signed PCM16 mono 16 kHz; output is simulated playback, not a device test.
use anyhow::{Context, Result};
use live_translate_rs::{
    dsp::{wav_pcm16, Segmenter, StreamingResampler},
    language::Language,
    provider::LocalEngines,
    runtime::{guarded, PlaybackBuffer, SessionControl},
};
use serde_json::json;
use std::{
    fs,
    io::Write,
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};

struct Clause {
    id: usize,
    samples: Vec<f32>,
    at: Instant,
    end: Instant,
}
struct Job {
    clause: Clause,
    text: String,
    asr_ms: f64,
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(
        args.len() == 5,
        "Usage: replay_session CONFIG RAW_PCM16 es|de NEW_OUTPUT_DIR"
    );
    let language = match args[3].as_str() {
        "es" => Language::Spanish,
        "de" => Language::German,
        _ => anyhow::bail!("Use es or de"),
    };
    let engine = LocalEngines::from_file(Path::new(&args[1]))?.with_translation_cache(false);
    let rate = engine.config.validate_piper(language)?.sample_rate;
    let bytes = fs::read(&args[2])?;
    anyhow::ensure!(
        !bytes.is_empty() && bytes.len() % 2 == 0 && bytes.len() < 16000 * 2 * 3600,
        "Expected PCM16 of at most one hour"
    );
    let samples: Vec<_> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|s| i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.)
        .collect();
    let output = Path::new(&args[4]);
    fs::create_dir(output).context("Output directory must be new")?;
    let control = Arc::new(SessionControl::default());
    let playback = Arc::new(PlaybackBuffer::new(16000));
    let done = Arc::new(AtomicBool::new(false));
    let count = AtomicUsize::new(0);
    let start = Instant::now();
    let (clauses_tx, clauses_rx) = mpsc::sync_channel::<Clause>(2);
    let (jobs_tx, jobs_rx) = mpsc::sync_channel::<Job>(2);
    thread::scope(|scope| {
        let c = control.clone();
        let q = playback.clone();
        let finished = done.clone();
        scope.spawn(move || {
            guarded(&c, || {
                let mut file = fs::File::create(output.join("playback.pcm"))?;
                let mut tick = 0u64;
                while !c.is_stopped() && (!finished.load(Ordering::Acquire) || q.seconds() > 0.) {
                    thread::sleep(
                        (start + Duration::from_millis(tick * 20))
                            .saturating_duration_since(Instant::now()),
                    );
                    let mut frame = [0f32; 320];
                    q.render(&mut frame, 1, 0, &c, |v| v);
                    for value in frame {
                        file.write_all(&((value.clamp(-1., 1.) * 32767.) as i16).to_le_bytes())?;
                    }
                    tick += 1;
                }
                Ok(())
            })
        });
        let c = control.clone();
        let asr = engine.clone();
        scope.spawn(move || {
            guarded(&c, || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                while let Ok(clause) = clauses_rx.recv() {
                    c.check()?;
                    anyhow::ensure!(
                        clause.at.elapsed() < Duration::from_secs(15),
                        "Local recognition request became stale"
                    );
                    let begin = Instant::now();
                    if let Some(text) =
                        runtime.block_on(asr.recognize(&clause.samples, Language::English, &c))?
                    {
                        jobs_tx
                            .try_send(Job {
                                clause,
                                text,
                                asr_ms: begin.elapsed().as_secs_f64() * 1000.,
                            })
                            .map_err(|_| {
                                anyhow::anyhow!("Local translation/speech cannot keep up")
                            })?;
                    }
                }
                Ok(())
            })
        });
        let c = control.clone();
        let q = playback.clone();
        let finished = done.clone();
        let processed = &count;
        scope.spawn(move || {
            guarded(&c, || {
                let runtime=tokio::runtime::Builder::new_current_thread().enable_all().build()?;
                let mut report=fs::File::create(output.join("clauses.jsonl"))?;
                while let Ok(job)=jobs_rx.recv() {
                    c.check()?;
                    anyhow::ensure!(job.clause.at.elapsed()<Duration::from_secs(10),"Speech request became stale; stopped");
                    let begin=Instant::now();
                    let translated=runtime.block_on(engine.translate(&job.text,Language::English,language,&c))?;
                    let translation_ms=begin.elapsed().as_secs_f64()*1000.;
                    let mut converter=StreamingResampler::new(rate,16000);
                    let mut audio=Vec::new(); let mut first=None;
                    runtime.block_on(engine.synthesize_stream(&translated,language,&c,&mut |pcm| {
                        let converted=converter.push(pcm); q.push(&converted,&c)?;
                        if first.is_none() && !converted.is_empty() {first=Some(Instant::now());}
                        audio.extend_from_slice(pcm); Ok(())
                    }))?;
                    q.push(&converter.flush(),&c)?;
                    fs::write(output.join(format!("clause-{:03}.wav",job.clause.id)),wav_pcm16(&audio,rate))?;
                    let row=json!({"id":job.clause.id,"source":job.text,"translation":translated,"asr_ms":job.asr_ms,"translation_ms":translation_ms,
                        "clause_start_seconds":job.clause.at.duration_since(start).as_secs_f64(),"clause_end_seconds":job.clause.end.duration_since(start).as_secs_f64(),
                        "first_pcm_from_clause_start_ms":first.map(|t|t.duration_since(job.clause.at).as_secs_f64()*1000.),
                        "first_pcm_after_clause_end_ms":first.map(|t|t.duration_since(job.clause.end).as_secs_f64()*1000.),
                        "output_audio_seconds":audio.len() as f64/rate as f64,"completed_session_seconds":start.elapsed().as_secs_f64()});
                    writeln!(report,"{row}")?; report.flush()?;
                    processed.fetch_add(1,Ordering::Relaxed);
                    println!("clause {} completed at {:.1}s",job.clause.id,start.elapsed().as_secs_f64());
                }
                Ok(())
            });
            finished.store(true,Ordering::Release);
        });
        guarded(&control, || {
            let mut segmenter = Segmenter::live(0.015);
            let mut id = 0;
            // Trailing silence flushes the last partial clause through the same pause detector.
            let mut input = samples.clone();
            input.extend(vec![0.; 16000]);
            for (index, frame) in input.chunks(320).enumerate() {
                control.check()?;
                let end = start + Duration::from_secs_f64((index + 1) as f64 * 0.02);
                thread::sleep(end.saturating_duration_since(Instant::now()));
                anyhow::ensure!(
                    end.elapsed() < Duration::from_millis(500),
                    "Input audio gap or backlog detected"
                );
                if let Some(pcm) = segmenter.push(frame) {
                    let at = end - Duration::from_secs_f64(pcm.len() as f64 / 16000.);
                    clauses_tx
                        .try_send(Clause {
                            id,
                            samples: pcm,
                            at,
                            end,
                        })
                        .map_err(|_| anyhow::anyhow!("Local recognition cannot keep up"))?;
                    id += 1;
                }
            }
            Ok(())
        });
        drop(clauses_tx);
    });
    let report = json!({"status":if control.failure().is_none(){"passed"}else{"failed"},"error":control.failure(),"input_seconds":samples.len() as f64/16000.,
        "wall_seconds":start.elapsed().as_secs_f64(),"completed_clauses":count.load(Ordering::Relaxed),"target":args[3],
        "real_time_paced":true,"physical_devices_tested":false,"cache_enabled":false});
    fs::write(
        output.join("summary.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{report}");
    anyhow::ensure!(
        control.failure().is_none(),
        "Replay stopped; see summary.json"
    );
    Ok(())
}
