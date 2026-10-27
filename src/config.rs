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

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineConfig {
    pub asr_port: u16,
    pub translation_port: u16,
    pub piper_binary: PathBuf,
    #[serde(default = "default_piper_worker")]
    pub piper_worker: PathBuf,
    #[serde(default)]
    pub piper_python_voices: BTreeMap<String, PathBuf>,
    pub voices: BTreeMap<String, PathBuf>,
    #[serde(default)]
    pub asr_custom_adapter: bool,
    #[serde(default)]
    pub translation_custom_adapter: bool,
    #[serde(default)]
    pub asr_launch_args: Vec<String>,
    #[serde(default)]
    pub translation_launch_args: Vec<String>,
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
fn default_piper_worker() -> PathBuf {
    PathBuf::from("scripts/piper_worker.py")
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
    /// Opt in only after the user selects Mandarin. Preserve active engines/model settings.
    pub fn add_installed_mandarin_voice(&mut self) -> Result<()> {
        if self.voices.contains_key(info(Language::Mandarin).text_code) {
            return Ok(());
        }
        let optional=Self::load(&self.root.join("offline.mandarin.experimental.json"))
            .context("Mandarin optional pack is not configured. Run Install-Mandarin-Experimental.cmd for local evaluation")?;
        let voice = optional.voice(Language::Mandarin).context(
            "Mandarin optional voice is missing or invalid. Run Install-Mandarin-Experimental.cmd",
        )?;
        self.voices
            .insert(info(Language::Mandarin).text_code.into(), voice.model);
        Ok(())
    }
    pub fn add_installed_french_voice(&mut self) -> Result<()> {
        if self.voices.contains_key("fr") {
            return Ok(());
        }
        let optional = Self::load(&self.root.join("offline.french.json"))
            .context("French pack not installed; run Install-French.cmd")?;
        let voice = optional.voice(Language::French)?;
        self.voices.insert("fr".into(), voice.model);
        Ok(())
    }
    /// Import only a validated installed voice, preserving active engine settings.
    pub fn add_installed_language_voice(&mut self, language: Language) -> Result<()> {
        let code = info(language).text_code;
        if self.voices.contains_key(code) {
            return Ok(());
        }
        anyhow::ensure!(
            matches!(
                language,
                Language::Russian
                    | Language::Basque
                    | Language::Italian
                    | Language::Japanese
                    | Language::Hindi
                    | Language::Arabic
            ),
            "No optional pack registered for this language"
        );
        let optional =
            Self::load(&self.root.join(format!("offline.{code}.json"))).with_context(|| {
                format!(
                    "{} pack not installed; run Install-{}.cmd",
                    info(language).name,
                    info(language).name
                )
            })?;
        let voice = optional.voice(language)?;
        let modern = optional
            .piper_python_voices
            .get(code)
            .map(|path| optional.executable(path));
        if matches!(
            language,
            Language::Italian | Language::Japanese | Language::Arabic
        ) {
            anyhow::ensure!(modern.as_ref().is_some_and(|path| path.is_file()), "This voice requires the optional Piper 1.8 Python runtime; run its language installer");
        }
        self.voices.insert(code.into(), voice.model);
        if let Some(path) = modern {
            self.piper_python_voices.insert(code.into(), path);
        }
        Ok(())
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
        if let Some(python) = self.piper_python_voices.get(info(target).text_code) {
            anyhow::ensure!(
                self.executable(python).is_file(),
                "Optional Piper Python runtime is missing; reinstall the language pack"
            );
        }
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

#[cfg(test)]
mod optional_voice_tests {
    use super::*;
    #[test]
    fn optional_mandarin_only_adds_voice_and_preserves_active_model_and_ports() {
        let root = std::env::temp_dir().join(format!(
            "polystream-voice-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let value = serde_json::json!({"asr_port":8178,"translation_port":8179,"piper_binary":"piper","voices":{},"whisper_server":"whisper","whisper_model":"active-asr","llama_server":"llama","translation_model":"active-translation","threads":4,"gpu_layers":99});
        std::fs::write(
            root.join("active.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        let mut active = OfflineConfig::load(&root.join("active.json")).unwrap();
        assert!(active.add_installed_mandarin_voice().is_err());
        assert!(active.voices.is_empty());
        let mut optional = value.clone();
        optional["translation_model"] = serde_json::json!("different-model");
        optional["translation_port"] = serde_json::json!(9001);
        optional["voices"] = serde_json::json!({"zh-Hans":"voice.onnx"});
        std::fs::write(
            root.join("offline.mandarin.experimental.json"),
            serde_json::to_vec(&optional).unwrap(),
        )
        .unwrap();
        assert!(active.add_installed_mandarin_voice().is_err());
        std::fs::write(root.join("voice.onnx"), b"test metadata fixture").unwrap();
        std::fs::write(
            root.join("voice.onnx.json"),
            br#"{"audio":{"sample_rate":22050}}"#,
        )
        .unwrap();
        active.add_installed_mandarin_voice().unwrap();
        assert_eq!(
            active.translation_model,
            PathBuf::from("active-translation")
        );
        assert_eq!(active.translation_port, 8179);
        assert_eq!(active.voice(Language::Mandarin).unwrap().sample_rate, 22050);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn optional_rollout_voices_preserve_active_settings_and_reject_missing_files() {
        let root = std::env::temp_dir().join(format!(
            "polystream-rollout-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let original = serde_json::json!({"asr_port":8178,"translation_port":8179,"piper_binary":"piper","voices":{},"whisper_server":"whisper","whisper_model":"chosen-asr","llama_server":"llama","translation_model":"chosen-translation","threads":6,"gpu_layers":0});
        std::fs::write(
            root.join("active.json"),
            serde_json::to_vec(&original).unwrap(),
        )
        .unwrap();
        let mut active = OfflineConfig::load(&root.join("active.json")).unwrap();
        for language in [Language::Russian, Language::Basque, Language::Hindi] {
            let code = info(language).text_code;
            assert!(active.add_installed_language_voice(language).is_err());
            let mut optional = original.clone();
            optional["voices"] = serde_json::json!({code:format!("{code}.onnx")});
            optional["threads"] = serde_json::json!(128);
            optional["translation_port"] = serde_json::json!(9999);
            std::fs::write(
                root.join(format!("offline.{code}.json")),
                serde_json::to_vec(&optional).unwrap(),
            )
            .unwrap();
            assert!(active.add_installed_language_voice(language).is_err());
            assert!(!active.voices.contains_key(code));
            std::fs::write(root.join(format!("{code}.onnx")), b"metadata fixture").unwrap();
            std::fs::write(
                root.join(format!("{code}.onnx.json")),
                br#"{"audio":{"sample_rate":22050}}"#,
            )
            .unwrap();
            active.add_installed_language_voice(language).unwrap();
            assert_eq!(active.voice(language).unwrap().sample_rate, 22050);
            assert_eq!(active.threads, 6);
            assert_eq!(active.gpu_layers, 0);
            assert_eq!(active.translation_port, 8179);
            assert_eq!(
                active.translation_model,
                PathBuf::from("chosen-translation")
            );
        }
        assert!(active
            .add_installed_language_voice(Language::Cantonese)
            .is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn modern_voice_import_requires_runtime_and_leaves_native_routes_unchanged() {
        let root = std::env::temp_dir().join(format!(
            "polystream-modern-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let original = serde_json::json!({"asr_port":8178,"translation_port":8179,"piper_binary":"native-piper","voices":{"es":"spanish.onnx"},"whisper_server":"whisper","whisper_model":"chosen-asr","llama_server":"llama","translation_model":"chosen-translation","threads":4,"gpu_layers":99});
        std::fs::write(
            root.join("active.json"),
            serde_json::to_vec(&original).unwrap(),
        )
        .unwrap();
        let mut active = OfflineConfig::load(&root.join("active.json")).unwrap();
        for language in [Language::Italian, Language::Japanese, Language::Arabic] {
            let code = info(language).text_code;
            let mut optional = original.clone();
            optional["voices"] = serde_json::json!({code:format!("{code}.onnx")});
            optional["piper_python_voices"] =
                serde_json::json!({code:format!("{code}-python.exe")});
            std::fs::write(
                root.join(format!("offline.{code}.json")),
                serde_json::to_vec(&optional).unwrap(),
            )
            .unwrap();
            std::fs::write(root.join(format!("{code}.onnx")), b"metadata fixture").unwrap();
            std::fs::write(
                root.join(format!("{code}.onnx.json")),
                br#"{"audio":{"sample_rate":22050}}"#,
            )
            .unwrap();
            assert!(active.add_installed_language_voice(language).is_err());
            assert!(!active.voices.contains_key(code));
            std::fs::write(root.join(format!("{code}-python.exe")), b"runtime fixture").unwrap();
            active.add_installed_language_voice(language).unwrap();
            assert_eq!(
                active.piper_python_voices.get(code).unwrap(),
                &root
                    .join(format!("{code}-python.exe"))
                    .canonicalize()
                    .unwrap()
            );
            assert_eq!(active.piper_binary, PathBuf::from("native-piper"));
            assert_eq!(
                active.voices.get("es").unwrap(),
                &PathBuf::from("spanish.onnx")
            );
            assert_eq!(
                active.translation_model,
                PathBuf::from("chosen-translation")
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
