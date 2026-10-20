use crate::language::{info, Language};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranslationDomain {
    #[default]
    General,
    LatterDaySaint,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineConfig {
    pub asr_port: u16,
    pub translation_port: u16,
    pub piper_binary: PathBuf,
    pub voices: BTreeMap<String, PathBuf>,
    pub whisper_server: PathBuf,
    pub whisper_model: PathBuf,
    pub llama_server: PathBuf,
    pub translation_model: PathBuf,
    pub threads: usize,
    pub gpu_layers: u32,
    #[serde(default)]
    pub translation_domain: TranslationDomain,
    #[serde(skip)]
    pub root: PathBuf,
}
impl OfflineConfig {
    pub fn load(file: &Path) -> Result<Self> {
        let file = file
            .canonicalize()
            .with_context(|| format!("Cannot find {}; see OFFLINE_SETUP.md", file.display()))?;
        let mut config: Self = serde_json::from_slice(&std::fs::read(&file)?)
            .context("Invalid offline configuration")?;
        config.root = file
            .parent()
            .context("Configuration directory missing")?
            .to_owned();
        anyhow::ensure!(
            config.asr_port > 0
                && config.translation_port > 0
                && config.asr_port != config.translation_port,
            "Use different, nonzero local engine ports"
        );
        anyhow::ensure!((1..=128).contains(&config.threads), "threads must be 1–128");
        Ok(config)
    }
    pub fn path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_owned()
        } else {
            self.root.join(path)
        }
    }
    pub fn executable(&self, path: &Path) -> PathBuf {
        let candidate = self.path(path);
        #[cfg(target_os = "windows")]
        if !candidate.exists() && candidate.extension().is_none() {
            return candidate.with_extension("exe");
        }
        candidate
    }
    pub fn voice(&self, language: Language) -> Result<LocalVoice> {
        let model = self.path(
            self.voices
                .get(info(language).text_code)
                .context("No local voice configured for this language")?,
        );
        anyhow::ensure!(
            model.is_file(),
            "Piper voice not installed: {}",
            model.display()
        );
        let config = PathBuf::from(format!("{}.json", model.display()));
        let value: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&config)
                .with_context(|| format!("Missing voice configuration {}", config.display()))?,
        )?;
        let rate = value
            .pointer("/audio/sample_rate")
            .and_then(|v| v.as_u64())
            .context("Voice config has no audio.sample_rate")?;
        anyhow::ensure!(
            (8000..=96000).contains(&rate),
            "Unsupported voice sample rate"
        );
        Ok(LocalVoice {
            model,
            sample_rate: rate as u32,
        })
    }
    pub fn validate_piper(&self, target: Language) -> Result<LocalVoice> {
        anyhow::ensure!(
            self.executable(&self.piper_binary).is_file(),
            "Piper executable is not installed; see OFFLINE_SETUP.md"
        );
        self.voice(target)
    }
    // Ports are the only network settings. Hosts cannot be redirected to a cloud service.
    pub fn asr_url(&self) -> String {
        format!("http://127.0.0.1:{}/inference", self.asr_port)
    }
    pub fn translation_url(&self) -> String {
        format!(
            "http://127.0.0.1:{}/v1/chat/completions",
            self.translation_port
        )
    }
}
pub struct LocalVoice {
    pub model: PathBuf,
    pub sample_rate: u32,
}
