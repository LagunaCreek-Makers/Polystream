//! Test an installed offline voice without opening an audio device or using a server.
use anyhow::{Context, Result};
use live_translate_rs::{
    dsp::wav_pcm16, language::Language, provider::LocalEngines, runtime::SessionControl,
};
use std::{io::Write, path::Path, time::Instant};
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    anyhow::ensure!(
        args.len() == 4,
        "Usage: local_voice_check CONFIG.json es|de OUTPUT.wav"
    );
    let (target, text) = match args[2].as_str() {
        "es" => (
            Language::Spanish,
            "Hola. Esta es una prueba de voz sin conexión.",
        ),
        "de" => (
            Language::German,
            "Guten Tag. Dies ist ein Offline-Sprachtest.",
        ),
        _ => anyhow::bail!("Choose es or de"),
    };
    let engine = LocalEngines::from_file(Path::new(&args[1]))?;
    let voice = engine.config.validate_piper(target)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut audio = Vec::new();
    let start = Instant::now();
    runtime.block_on(engine.synthesize_stream(
        text,
        target,
        &SessionControl::default(),
        &mut |pcm| {
            audio.extend_from_slice(pcm);
            Ok(())
        },
    ))?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[3])
        .context("Output already exists or cannot be created")?;
    file.write_all(&wav_pcm16(&audio, voice.sample_rate))?;
    println!("Local voice produced {:.2}s of audio in {:.2}s; sample rate {} Hz. This checks synthesis, not translation or live latency.",audio.len() as f64/voice.sample_rate as f64,start.elapsed().as_secs_f64(),voice.sample_rate);
    Ok(())
}
