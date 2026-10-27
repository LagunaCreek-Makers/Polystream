//! Explicit, file-backed inference presets; never change a running session.
use crate::config::OfflineConfig;
use anyhow::{Context, Result};
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const PRESETS: [(&str, &str, &str); 4] = [
    ("speed", "Fastest tested baseline — Whisper base + Qwen3-4B", "Default recommendation for Spanish/German and initial French tests. Not fastest for every language; accuracy review remains separate."),
    ("quality", "Best tested translation candidate — Qwen3-8B [Experimental]", "Fewer rejected Arabic/Hindi clauses; remaining meaning errors. Eight-stream 15-min drain 77.8s versus 63.0s with 4B. Not uniformly faster or production-accuracy approved."),
    ("small", "Whisper small.en + Qwen3-4B [Experimental, not recommended]", "Recognition/name regressions and added words with hints. English-only recognition; no automatic name hint."),
    ("medium", "Whisper medium.en + Qwen3-4B [Experimental, not recommended]", "Recognition regressions; a hinted silence control hallucinated text. No automatic name hint."),
];

pub fn save_configuration(
    active: &Path,
    mut config: OfflineConfig,
    label: &str,
) -> Result<PathBuf> {
    let root = active
        .canonicalize()?
        .parent()
        .context("Configuration directory missing")?
        .to_owned();
    for path in [
        &mut config.whisper_server,
        &mut config.whisper_model,
        &mut config.llama_server,
        &mut config.translation_model,
        &mut config.piper_binary,
        &mut config.piper_worker,
    ] {
        if path.is_relative() {
            *path = root.join(&*path);
        }
    }
    for path in config
        .voices
        .values_mut()
        .chain(config.piper_python_voices.values_mut())
    {
        if path.is_relative() {
            *path = root.join(&*path);
        }
    }
    for path in [
        &config.whisper_server,
        &config.whisper_model,
        &config.llama_server,
        &config.translation_model,
        &config.piper_binary,
    ] {
        anyhow::ensure!(
            path.is_file(),
            "Required engine/model is not installed: {}",
            path.display()
        );
    }
    anyhow::ensure!(
        config.asr_port > 0
            && config.translation_port > 0
            && config.asr_port != config.translation_port,
        "Use distinct nonzero ports"
    );
    anyhow::ensure!((1..=128).contains(&config.threads), "Threads must be 1–128");
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = root.join(format!("offline.{label}-{stamp}.json"));
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    output.write_all(&serde_json::to_vec_pretty(&config)?)?;
    output.sync_all()?;
    Ok(path)
}

pub fn select(active: &Path, id: &str) -> Result<PathBuf> {
    let mut config = OfflineConfig::load(active)?;
    let root = config.root.clone();
    let (whisper, translation) = match id {
        "speed" => ("ggml-base.bin", "Qwen3-4B-Q4_K_M.gguf"),
        "quality" => ("ggml-base.bin", "Qwen3-8B-Q4_K_M.gguf"),
        "small" => ("ggml-small.en.bin", "Qwen3-4B-Q4_K_M.gguf"),
        "medium" => ("ggml-medium.en.bin", "Qwen3-4B-Q4_K_M.gguf"),
        _ => anyhow::bail!("Unknown model preset"),
    };
    // A stock preset resets adapter overrides; custom profiles retain them until selected away.
    for name in ["offline.cuda.json", "offline.json"] {
        if let Ok(stock) = OfflineConfig::load(&root.join(name)) {
            config.whisper_server = stock.path(&stock.whisper_server);
            config.llama_server = stock.path(&stock.llama_server);
            break;
        }
    }
    config.threads = 4;
    config.gpu_layers = 99;
    config.asr_custom_adapter = false;
    config.translation_custom_adapter = false;
    config.asr_launch_args.clear();
    config.translation_launch_args.clear();
    // Preserve voices, ports and terminology domain.
    config.whisper_model = root.join("models/whisper").join(whisper);
    config.translation_model = root.join("models/translation").join(translation);
    save_configuration(active, config, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_preserves_voices_ports_and_original_and_missing_models_are_atomic() {
        let root = std::env::temp_dir().join(format!(
            "polystream-selection-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("models/whisper")).unwrap();
        std::fs::create_dir_all(root.join("models/translation")).unwrap();
        for name in [
            "piper",
            "whisper",
            "llama",
            "models/whisper/ggml-base.bin",
            "models/translation/Qwen3-8B-Q4_K_M.gguf",
        ] {
            std::fs::write(root.join(name), b"fixture").unwrap();
        }
        let original = root.join("offline.json");
        let value = serde_json::json!({"asr_port":9001,"translation_port":9002,"threads":7,"gpu_layers":99,"piper_binary":"piper","whisper_server":"whisper","whisper_model":"models/whisper/ggml-base.bin","llama_server":"llama","translation_model":"old","voices":{"fr":"voice.onnx"},"translation_domain":"latter_day_saint","asr_launch_args":["adapter.py"],"asr_custom_adapter":true});
        std::fs::write(&original, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(select(&original, "medium").is_err());
        let selected = select(&original, "quality").unwrap();
        let config = OfflineConfig::load(&selected).unwrap();
        assert_eq!(config.asr_port, 9001);
        assert_eq!(config.translation_port, 9002);
        assert_eq!(config.threads, 4);
        assert_eq!(config.gpu_layers, 99);
        assert_eq!(
            config.voices["fr"],
            root.canonicalize().unwrap().join("voice.onnx")
        );
        assert!(config.asr_launch_args.is_empty());
        assert!(!config.asr_custom_adapter);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(&original).unwrap())
                .unwrap(),
            value
        );
        std::fs::remove_file(selected).unwrap();
        std::fs::remove_file(original).unwrap();
        for name in [
            "piper",
            "whisper",
            "llama",
            "models/whisper/ggml-base.bin",
            "models/translation/Qwen3-8B-Q4_K_M.gguf",
        ] {
            std::fs::remove_file(root.join(name)).unwrap();
        }
        std::fs::remove_dir(root.join("models/whisper")).unwrap();
        std::fs::remove_dir(root.join("models/translation")).unwrap();
        std::fs::remove_dir(root.join("models")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn unknown_preset_cannot_mutate_configuration() {
        let root = std::env::temp_dir().join(format!(
            "polystream-presets-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let original = root.join("offline.json");
        let value = serde_json::json!({"asr_port":8178,"translation_port":8179,"threads":4,"gpu_layers":99,"piper_binary":"piper","whisper_server":"whisper","whisper_model":"base","llama_server":"llama","translation_model":"qwen","voices":{}});
        std::fs::write(&original, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(select(&original, "unknown").is_err());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(&original).unwrap())
                .unwrap(),
            value
        );
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        std::fs::remove_file(original).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
