//! Exercise the application's actual local inference adapters without audio devices.
use anyhow::{Context, Result};
use live_translate_rs::{
    dsp::wav_pcm16,
    language::{validate_pair, Language},
    provider::LocalEngines,
    runtime::SessionControl,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashSet, fs, io::Write, path::Path, time::Instant};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    source: String,
    target: String,
    text: Option<String>,
    audio: Option<String>,
    reference_source: Option<String>,
    reference: Option<String>,
    category: Option<String>,
    source_url: Option<String>,
    reference_url: Option<String>,
    #[serde(default)]
    review_focus: Vec<String>,
    reference_translation: Option<String>,
}
fn language(code: &str) -> Result<Language> {
    live_translate_rs::language::LANGUAGES
        .iter()
        .find(|entry| entry.text_code == code)
        .map(|entry| entry.language)
        .with_context(|| format!("Unsupported language: {code}"))
}
fn cases(path: &Path) -> Result<Vec<Case>> {
    anyhow::ensure!(
        fs::metadata(path)?.len() <= 1024 * 1024,
        "Case file exceeds 1 MiB"
    );
    let cases: Vec<Case> = serde_json::from_slice(&fs::read(path)?)?;
    validate_cases(&cases)?;
    Ok(cases)
}
fn validate_cases(cases: &[Case]) -> Result<()> {
    anyhow::ensure!(!cases.is_empty() && cases.len() <= 200, "Use 1–200 cases");
    let mut ids = HashSet::new();
    for case in cases {
        anyhow::ensure!(
            !case.id.is_empty()
                && case.id.len() <= 64
                && case
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                && ids.insert(case.id.to_ascii_lowercase()),
            "IDs must be unique (case insensitive), 1–64 ASCII letters/digits/hyphens/underscores"
        );
        // Prefixing output filenames avoids Windows reserved device basenames.
        validate_pair(language(&case.source)?, language(&case.target)?)?;
        anyhow::ensure!(case.target != "en", "Choose an enabled non-English target");
        anyhow::ensure!(
            case.text.is_some() != case.audio.is_some(),
            "Provide exactly one of text/audio"
        );
        if let Some(text) = &case.text {
            anyhow::ensure!(
                !text.trim().is_empty() && text.chars().count() <= 2000,
                "Text must contain 1–2000 characters"
            );
        }
    }
    Ok(())
}
fn pcm_wav(bytes: &[u8]) -> Result<Vec<f32>> {
    anyhow::ensure!(
        bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
        "Expected RIFF WAV"
    );
    let length = u32::from_le_bytes(bytes[4..8].try_into()?) as usize + 8;
    anyhow::ensure!(length == bytes.len(), "Truncated or trailing RIFF data");
    let mut at = 12usize;
    let (mut format, mut data) = (false, None);
    while at < bytes.len() {
        anyhow::ensure!(at + 8 <= bytes.len(), "Truncated WAV chunk header");
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into()?) as usize;
        let end = (at + 8).checked_add(size).context("WAV chunk overflow")?;
        anyhow::ensure!(end <= bytes.len(), "Truncated WAV chunk");
        let chunk = &bytes[at + 8..end];
        match &bytes[at..at + 4] {
            b"fmt " => {
                anyhow::ensure!(!format && chunk.len() >= 16, "Invalid/duplicate WAV format");
                let word = |i| u16::from_le_bytes([chunk[i], chunk[i + 1]]);
                anyhow::ensure!(
                    word(0) == 1
                        && word(2) == 1
                        && word(12) == 2
                        && word(14) == 16
                        && u32::from_le_bytes(chunk[4..8].try_into()?) == 16000
                        && u32::from_le_bytes(chunk[8..12].try_into()?) == 32000,
                    "Use uncompressed PCM16, mono, 16000 Hz WAV"
                );
                format = true;
            }
            b"data" => {
                anyhow::ensure!(data.is_none(), "Duplicate WAV data");
                data = Some(chunk);
            }
            _ => {}
        }
        at = end.checked_add(size % 2).context("WAV padding overflow")?;
        anyhow::ensure!(at <= bytes.len(), "Missing WAV chunk padding");
    }
    let data = data.context("Missing WAV data")?;
    anyhow::ensure!(
        format && !data.is_empty() && data.len() % 2 == 0 && data.len() <= 16000 * 2 * 60,
        "Expected nonempty audio of at most 60 seconds"
    );
    Ok(data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|s| i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0)
        .collect())
}
fn percentile(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    Some(
        sorted[((p * sorted.len() as f64).ceil() as usize)
            .saturating_sub(1)
            .min(sorted.len() - 1)],
    )
}
async fn run_case(
    engine: &LocalEngines,
    case: &Case,
    base: &Path,
    output: &Path,
    row: &mut Value,
    translation_only: bool,
) -> Result<()> {
    let source = language(&case.source)?;
    let target = language(&case.target)?;
    let voice = if translation_only {
        None
    } else {
        Some(engine.config.validate_piper(target)?)
    };
    // File reading is excluded; timing begins when a complete clause is submitted.
    let pcm = if let Some(audio) = &case.audio {
        let path = base.join(audio);
        anyhow::ensure!(
            fs::metadata(&path)?.len() <= 2 * 1024 * 1024,
            "WAV exceeds 2 MiB"
        );
        Some(pcm_wav(&fs::read(path)?)?)
    } else {
        None
    };
    let control = SessionControl::default();
    let start = Instant::now();
    let text = if let Some(pcm) = &pcm {
        row["input_audio_seconds"] = json!(pcm.len() as f64 / 16000.0);
        let text = engine
            .recognize(pcm, source, &control)
            .await?
            .context("Recognition returned empty text")?;
        row["recognition_ms"] = json!(start.elapsed().as_secs_f64() * 1000.0);
        text
    } else {
        case.text.clone().context("Missing text")?
    };
    row["recognized_or_input_text"] = json!(text);
    let translation_start = Instant::now();
    let hits = engine.translation_cache_hits();
    let translated = engine.translate(&text, source, target, &control).await?;
    row["translation_cache_hit"] = json!(engine.translation_cache_hits() > hits);
    row["translation_ms"] = json!(translation_start.elapsed().as_secs_f64() * 1000.0);
    row["translation"] = json!(translated);
    row["translation_ready_ms"] = json!(start.elapsed().as_secs_f64() * 1000.0);
    if translation_only {
        row["processing_ms"] = row["translation_ready_ms"].clone();
        return Ok(());
    }
    let voice = voice.context("Missing synthesis voice")?;
    let synthesis_start = Instant::now();
    let mut first_ms = None;
    let mut audio = Vec::new();
    engine
        .synthesize_stream(&translated, target, &control, &mut |pcm| {
            if !pcm.is_empty() && first_ms.is_none() {
                first_ms = Some(start.elapsed().as_secs_f64() * 1000.0);
            }
            anyhow::ensure!(
                audio.len() + pcm.len() <= voice.sample_rate as usize * 120,
                "Audio exceeds 120 seconds"
            );
            audio.extend_from_slice(pcm);
            Ok(())
        })
        .await?;
    let total_ms = start.elapsed().as_secs_f64() * 1000.0;
    row["synthesis_ms"] = json!(synthesis_start.elapsed().as_secs_f64() * 1000.0);
    row["first_pcm_ms"] = json!(first_ms);
    row["processing_ms"] = json!(total_ms);
    row["output_audio_seconds"] = json!(audio.len() as f64 / voice.sample_rate as f64);
    if let Some(pcm) = pcm {
        row["processing_to_input_ratio"] = json!(total_ms / 1000.0 / (pcm.len() as f64 / 16000.0));
    }
    let name = format!("case-{}.wav", case.id);
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output.join(&name))?
        .write_all(&wav_pcm16(&audio, voice.sample_rate))?;
    row["output_wav"] = json!(name);
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    anyhow::ensure!(
        args.len() >= 4 && args[4..].iter().all(|a| a == "--translation-only" || a == "--no-translation-cache"),
        "Usage: offline_benchmark CONFIG.json CASES.json NEW_OUTPUT_DIRECTORY [--translation-only] [--no-translation-cache]"
    );
    let translation_only = args[4..].iter().any(|a| a == "--translation-only");
    let use_cache = !args[4..].iter().any(|a| a == "--no-translation-cache");
    let cases_path = Path::new(&args[2]).canonicalize()?;
    let cases = cases(&cases_path)?;
    let engine = LocalEngines::from_file(Path::new(&args[1]))?.with_translation_cache(use_cache);
    if !translation_only {
        for case in &cases {
            engine.config.validate_piper(language(&case.target)?)?;
        }
    }
    let output = Path::new(&args[3]);
    fs::create_dir(output).context("Output directory must be new, with an existing parent")?;
    let mut report = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output.join("results.jsonl"))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut times = Vec::new();
    let mut failures = 0;
    for (index, case) in cases.iter().enumerate() {
        let mut row = json!({"id":case.id,"source":case.source,"target":case.target,"index":index,
            "first_case":index == 0,"reference_source":case.reference_source,
            "reference_translation":case.reference_translation,"reference":case.reference,
            "category":case.category,"source_url":case.source_url,"reference_url":case.reference_url,
            "review_focus":case.review_focus,"human_review":null});
        match runtime.block_on(run_case(
            &engine,
            case,
            cases_path.parent().context("No case directory")?,
            output,
            &mut row,
            translation_only,
        )) {
            Ok(()) => {
                row["status"] = json!("ok");
                if let Some(ms) = row[if translation_only {
                    "translation_ready_ms"
                } else {
                    "first_pcm_ms"
                }]
                .as_f64()
                {
                    times.push(ms);
                }
            }
            Err(error) => {
                failures += 1;
                row["status"] = json!("error");
                row["error"] = json!(format!("{error:#}"));
            }
        }
        serde_json::to_writer(&mut report, &row)?;
        writeln!(report)?;
        report.flush()?;
        println!("{}: {}", case.id, row["status"]);
    }
    let summary = json!({"schema_version":1,"cases":cases.len(),"failed":failures,
        "successful_first_pcm_p50_ms":if translation_only { None } else { percentile(&times,0.5) },
        "successful_first_pcm_p95_ms":if translation_only { None } else { percentile(&times,0.95) },
        "successful_ready_p50_ms":percentile(&times,0.5),"successful_ready_p95_ms":percentile(&times,0.95),
        "mode":if translation_only { "translation_only" } else { "speech" },
        "ready_metric":if translation_only { "translation_ready_ms" } else { "first_pcm_ms" },
        "translation_domain":engine.config.translation_domain,
        "translation_cache_enabled":use_cache,"translation_cache_hits":engine.translation_cache_hits(),
        "includes_first_case":true,"acoustic_latency_measured":false,"translation_accuracy_scored":false,
        "config":fs::canonicalize(&args[1])?,"application_version":env!("CARGO_PKG_VERSION")});
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output.join("summary.json"))?
        .write_all(serde_json::to_string_pretty(&summary)?.as_bytes())?;
    anyhow::ensure!(failures == 0, "{failures} cases failed; see results.jsonl");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_ambiguous_cases_and_unsafe_output_names() {
        let base = json!({"id":"test","source":"en","target":"es","text":"Hello"});
        for (key, value) in [
            ("id", json!("../escape")),
            ("text", json!(" ")),
            ("source", json!("fj")),
            ("target", json!("en")),
            ("audio", json!("input.wav")),
        ] {
            let mut case = base.clone();
            case[key] = value;
            let rows: Vec<Case> = serde_json::from_value(json!([case])).unwrap();
            assert!(validate_cases(&rows).is_err());
        }
        let mut duplicate = base.clone();
        duplicate["id"] = json!("TEST");
        let rows: Vec<Case> = serde_json::from_value(json!([base, duplicate])).unwrap();
        assert!(validate_cases(&rows).is_err());
    }
    #[test]
    fn wav_contract_and_truncation() {
        let bytes = wav_pcm16(&[0.0, 0.5, -0.5], 16000);
        assert_eq!(pcm_wav(&bytes).unwrap(), vec![0.0, 0.5, -0.5]);
        assert!(pcm_wav(&bytes[..bytes.len() - 1]).is_err());
        assert!(pcm_wav(&wav_pcm16(&[0.0], 22050)).is_err());
        assert!(pcm_wav(&wav_pcm16(&[], 16000)).is_err());
    }
    #[test]
    fn nearest_rank_percentiles() {
        assert_eq!(percentile(&[], 0.95), None);
        assert_eq!(percentile(&[4.0, 1.0, 2.0, 3.0], 0.5), Some(2.0));
        assert_eq!(percentile(&[4.0, 1.0, 2.0, 3.0], 0.95), Some(4.0));
    }
}
