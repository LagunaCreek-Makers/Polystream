//! Durable subtitle output, independent of best-effort UI notifications.
use crate::language::{info, Language};
use anyhow::{Context, Result};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

pub struct Transcripts {
    pub directory: PathBuf,
    pub epoch: Instant,
    files: Mutex<BTreeMap<String, (File, u64)>>,
    metrics: Mutex<File>,
    failures: Mutex<File>,
}
impl Transcripts {
    pub fn create(project: &Path, languages: &[Language], epoch: Instant) -> Result<Self> {
        anyhow::ensure!(
            !project.as_os_str().is_empty(),
            "Choose a transcript project folder"
        );
        fs::create_dir_all(project).context("Cannot create transcript project folder")?;
        let now = chrono::Local::now();
        let date = now.format("%Y-%m-%d").to_string();
        let stem = now.format("Meeting_%Y-%m-%d_%H-%M-%S").to_string();
        let mut directory = project.join(&stem);
        let mut suffix = 1;
        loop {
            match fs::create_dir(&directory) {
                Ok(()) => break,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    directory = project.join(format!("{stem}_{suffix}"));
                    suffix += 1;
                }
                Err(e) => return Err(e).context("Cannot create meeting transcript directory"),
            }
        }
        let mut files = BTreeMap::new();
        for &language in languages {
            let code = info(language).text_code.to_owned();
            if files.contains_key(&code) {
                continue;
            }
            let name = info(language)
                .name
                .split(" (")
                .next()
                .unwrap_or(info(language).name);
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.join(format!("{name}_{date}.srt")))
                .context("Cannot create subtitle file")?;
            files.insert(code, (file, 0));
        }
        let metrics = Mutex::new(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.join("delays.jsonl"))?,
        );
        Ok(Self {
            directory: directory.clone(),
            epoch,
            files: Mutex::new(files),
            metrics,
            failures: Mutex::new(
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(directory.join("failures.jsonl"))?,
            ),
        })
    }
    pub fn timing(
        &self,
        language: Language,
        clause_id: u64,
        source: Instant,
        first_pcm: Instant,
    ) -> Result<()> {
        let mut file = self
            .metrics
            .lock()
            .map_err(|_| anyhow::anyhow!("Delay writer unavailable"))?;
        writeln!(
            file,
            "{}",
            serde_json::json!({"language":info(language).text_code,"clause_id":clause_id,
            "source_seconds":source.saturating_duration_since(self.epoch).as_secs_f64(),
            "first_pcm_seconds":first_pcm.saturating_duration_since(self.epoch).as_secs_f64(),
            "delay_seconds":first_pcm.saturating_duration_since(source).as_secs_f64()})
        )?;
        file.flush()?;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn untranslated(
        &self,
        language: Language,
        clause_id: u64,
        source: Instant,
        end: Instant,
        text: &str,
        error: &str,
    ) -> Result<()> {
        let marker = format!("\"Untranslated: {text}\"");
        self.write(language, source, end, &marker)?;
        let mut file = self
            .failures
            .lock()
            .map_err(|_| anyhow::anyhow!("Failure writer unavailable"))?;
        writeln!(
            file,
            "{}",
            serde_json::json!({"status":"untranslated","language":info(language).text_code,"clause_id":clause_id,"source_seconds":source.saturating_duration_since(self.epoch).as_secs_f64(),"source_end_seconds":end.saturating_duration_since(self.epoch).as_secs_f64(),"source_text":text,"error":error,"delay_seconds":null})
        )?;
        file.flush()?;
        file.sync_data()
            .context("Cannot persist untranslated-clause record")?;
        Ok(())
    }
    pub fn write(
        &self,
        language: Language,
        start: Instant,
        end: Instant,
        text: &str,
    ) -> Result<()> {
        let text = text.replace(['\r', '\n', '\0'], " ");
        let text = text.trim();
        anyhow::ensure!(!text.is_empty(), "Empty subtitle cue");
        let start = start.saturating_duration_since(self.epoch);
        let end = end
            .saturating_duration_since(self.epoch)
            .max(start + Duration::from_millis(1));
        let mut files = self
            .files
            .lock()
            .map_err(|_| anyhow::anyhow!("Transcript writer unavailable"))?;
        let (file, index) = files
            .get_mut(info(language).text_code)
            .context("Transcript language not configured")?;
        write!(
            file,
            "{}\n{} --> {}\n{text}\n\n",
            *index + 1,
            timestamp(start),
            timestamp(end)
        )
        .context("Transcript write failed; check disk space and project folder")?;
        file.flush()?;
        file.sync_data().context("Cannot persist transcript cue")?;
        *index += 1;
        Ok(())
    }
}
fn timestamp(time: Duration) -> String {
    let ms = time.as_millis();
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_alignment_gaps_and_no_overwrite() {
        let root = std::env::temp_dir().join(format!(
            "srt-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let epoch = Instant::now();
        let languages = [Language::English, Language::Spanish];
        let a = Transcripts::create(&root, &languages, epoch).unwrap();
        let b = Transcripts::create(&root, &languages, epoch).unwrap();
        assert_ne!(a.directory, b.directory);
        for lang in languages {
            a.write(
                lang,
                epoch + Duration::from_millis(3_599_999),
                epoch + Duration::from_secs(3601),
                "你好 العربية\r\nMāori",
            )
            .unwrap();
            a.write(
                lang,
                epoch + Duration::from_secs(3700),
                epoch + Duration::from_secs(3701),
                "Next",
            )
            .unwrap();
        }
        let content = fs::read_to_string(
            fs::read_dir(&a.directory)
                .unwrap()
                .find(|entry| {
                    entry
                        .as_ref()
                        .is_ok_and(|e| e.path().extension().is_some_and(|ext| ext == "srt"))
                })
                .unwrap()
                .unwrap()
                .path(),
        )
        .unwrap();
        assert!(content
            .contains("1\n00:59:59,999 --> 01:00:01,000\n你好 العربية  Māori\n\n2\n01:01:40,000"));
        assert_eq!(timestamp(Duration::from_millis(1)), "00:00:00,001");
        drop(a);
        drop(b);
        fs::remove_dir_all(root).unwrap();
    }
}
