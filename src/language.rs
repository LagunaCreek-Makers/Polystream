#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    English,
    Spanish,
    German,
    Mandarin,
    Cantonese,
    HmongDaw,
    Fijian,
}

pub struct LanguageInfo {
    pub language: Language,
    pub name: &'static str,
    pub text_code: &'static str,
    pub enabled: bool,
    pub note: &'static str,
}

pub const LANGUAGES: &[LanguageInfo] = &[
    LanguageInfo {
        language: Language::English,
        name: "English (US)",
        text_code: "en",
        enabled: true,
        note: "Initial source default; also usable as target",
    },
    LanguageInfo {
        language: Language::Spanish,
        name: "Spanish",
        text_code: "es",
        enabled: true,
        note: "Initial release",
    },
    LanguageInfo {
        language: Language::German,
        name: "German (Germany)",
        text_code: "de",
        enabled: true,
        note: "Initial release",
    },
    LanguageInfo {
        language: Language::Mandarin,
        name: "Mandarin Chinese",
        text_code: "zh-Hans",
        enabled: false,
        note: "Planned: validate Mandarin speech and Simplified text",
    },
    LanguageInfo {
        language: Language::Cantonese,
        name: "Cantonese Chinese",
        text_code: "yue",
        enabled: false,
        note: "Planned: distinct Cantonese routing, Traditional script",
    },
    LanguageInfo {
        language: Language::HmongDaw,
        name: "Hmong Daw (provisional dialect)",
        text_code: "mww",
        enabled: false,
        note: "Text route identified; speech providers and user dialect unresolved",
    },
    LanguageInfo {
        language: Language::Fijian,
        name: "Fijian (iTaukei)",
        text_code: "fj",
        enabled: false,
        note: "Text route identified; recognition/synthesis providers unresolved",
    },
];

pub fn info(language: Language) -> &'static LanguageInfo {
    LANGUAGES
        .iter()
        .find(|l| l.language == language)
        .expect("language registry complete")
}

pub fn validate_pair(source: Language, target: Language) -> anyhow::Result<()> {
    anyhow::ensure!(
        source != target,
        "Choose different source and target languages"
    );
    let (s, t) = (info(source), info(target));
    anyhow::ensure!(
        s.enabled && t.enabled,
        "This language is planned, not enabled in this offline release"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn future_languages_cannot_be_silently_substituted() {
        for target in [
            Language::Mandarin,
            Language::Cantonese,
            Language::HmongDaw,
            Language::Fijian,
        ] {
            assert!(validate_pair(Language::English, target).is_err());
        }
        assert_ne!(
            info(Language::Mandarin).text_code,
            info(Language::Cantonese).text_code
        );
        assert!(validate_pair(Language::English, Language::Spanish).is_ok());
        assert!(validate_pair(Language::Spanish, Language::German).is_ok());
    }
}
