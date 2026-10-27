#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    English,
    Spanish,
    German,
    Mandarin,
    Cantonese,
    French,
    Japanese,
    Russian,
    Arabic,
    Hindi,
    Basque,
    Italian,
    Tibetan,
    Afrikaans,
    Hawaiian,
    Maori,
    HmongDaw,
    Fijian,
}

pub struct LanguageInfo {
    pub language: Language,
    pub name: &'static str,
    pub native_name: &'static str,
    pub text_code: &'static str,
    pub enabled: bool,
    pub note: &'static str,
}

pub const LANGUAGES: &[LanguageInfo] = &[
    LanguageInfo {
        language: Language::English,
        name: "English (US)",
        native_name: "English",
        text_code: "en",
        enabled: true,
        note: "Initial source default; also usable as target",
    },
    LanguageInfo {
        language: Language::Spanish,
        name: "Spanish",
        native_name: "Español",
        text_code: "es",
        enabled: true,
        note: "Initial release",
    },
    LanguageInfo {
        language: Language::German,
        name: "German (Germany)",
        native_name: "Deutsch",
        text_code: "de",
        enabled: true,
        note: "Initial release",
    },
    LanguageInfo {
        language: Language::Mandarin,
        name: "Mandarin Chinese",
        native_name: "普通话",
        text_code: "zh-Hans",
        enabled: true,
        note: "Experimental target only; native-speaker review pending",
    },
    LanguageInfo {
        language: Language::Cantonese,
        name: "Cantonese Chinese",
        native_name: "粵語",
        text_code: "yue",
        enabled: false,
        note: "Planned: distinct Cantonese routing, Traditional script",
    },
    LanguageInfo {
        language: Language::French,
        name: "French",
        native_name: "Français",
        text_code: "fr",
        enabled: true,
        note: "Local speech pipeline tested; native-speaker quality review pending",
    },
    LanguageInfo {
        language: Language::Japanese,
        name: "Japanese",
        native_name: "日本語",
        text_code: "ja",
        enabled: true,
        note: "Experimental optional Piper 1.8 voice; noncommercial dataset; native quality review pending",
    },
    LanguageInfo {
        language: Language::Russian,
        name: "Russian",
        native_name: "Русский",
        text_code: "ru",
        enabled: true,
        note: "Experimental local speech; terminology and native-speaker review pending",
    },
    LanguageInfo {
        language: Language::Arabic,
        name: "Arabic",
        native_name: "العربية",
        text_code: "ar",
        enabled: true,
        note: "Experimental Kareem voice; license provenance, RTL UI and native quality review pending",
    },
    LanguageInfo {
        language: Language::Hindi,
        name: "Hindi",
        native_name: "हिन्दी",
        text_code: "hi",
        enabled: true,
        note: "Experimental Pratham voice; short speech test passed; native/semantic review pending",
    },
    LanguageInfo {
        language: Language::Basque,
        name: "Basque",
        native_name: "Euskara",
        text_code: "eu",
        enabled: false,
        note: "Translation quality failed with current model; voice works, translation backend pending",
    },
    LanguageInfo {
        language: Language::Italian,
        name: "Italian",
        native_name: "Italiano",
        text_code: "it",
        enabled: true,
        note: "Experimental local speech with optional Piper 1.8; native quality review pending",
    },
    LanguageInfo {
        language: Language::Tibetan,
        name: "Tibetan",
        native_name: "བོད་སྐད་",
        text_code: "bo",
        enabled: false,
        note: "Planned; voice and translation validation pending",
    },
    LanguageInfo {
        language: Language::Afrikaans,
        name: "Afrikaans",
        native_name: "Afrikaans",
        text_code: "af",
        enabled: false,
        note: "Planned; voice and translation validation pending",
    },
    LanguageInfo {
        language: Language::Hawaiian,
        name: "Hawaiian",
        native_name: "ʻŌlelo Hawaiʻi",
        text_code: "haw",
        enabled: false,
        note: "Planned; voice and translation validation pending",
    },
    LanguageInfo {
        language: Language::Maori,
        name: "Maori",
        native_name: "Te reo Māori",
        text_code: "mi",
        enabled: false,
        note: "Planned; voice and translation validation pending",
    },
    LanguageInfo {
        language: Language::HmongDaw,
        name: "Hmong Daw (provisional dialect)",
        native_name: "Hmoob Dawb",
        text_code: "mww",
        enabled: false,
        note: "Text route identified; speech providers and user dialect unresolved",
    },
    LanguageInfo {
        language: Language::Fijian,
        name: "Fijian (iTaukei)",
        native_name: "Vosa Vakaviti",
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

/// New target-language packs do not imply speech recognition is validated.
pub fn source_supported(language: Language) -> bool {
    matches!(
        language,
        Language::English | Language::Spanish | Language::German
    )
}
pub fn validate_pair(source: Language, target: Language) -> anyhow::Result<()> {
    anyhow::ensure!(
        source != target,
        "Choose different source and target languages"
    );
    let (s, t) = (info(source), info(target));
    anyhow::ensure!(
        source_supported(source) && s.enabled && t.enabled,
        "This language is planned, not enabled in this offline release"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn future_languages_cannot_be_silently_substituted() {
        for target in [Language::Cantonese, Language::HmongDaw, Language::Fijian] {
            assert!(validate_pair(Language::English, target).is_err());
        }
        assert!(validate_pair(Language::English, Language::Mandarin).is_ok());
        assert!(validate_pair(Language::Mandarin, Language::Spanish).is_err());
        for language in LANGUAGES.iter().filter(|l| !l.enabled) {
            assert!(validate_pair(Language::English, language.language).is_err());
        }
        assert_ne!(
            info(Language::Mandarin).text_code,
            info(Language::Cantonese).text_code
        );
        assert!(validate_pair(Language::English, Language::Spanish).is_ok());
        assert!(validate_pair(Language::Spanish, Language::German).is_ok());
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;
    #[test]
    fn requested_targets_are_listed_in_priority_order_without_enabling_unfinished_routes() {
        let requested = [
            Language::Mandarin,
            Language::Cantonese,
            Language::French,
            Language::Japanese,
            Language::Russian,
            Language::Arabic,
            Language::Hindi,
            Language::Basque,
            Language::Italian,
            Language::Tibetan,
            Language::Afrikaans,
            Language::Hawaiian,
            Language::Maori,
        ];
        let positions = requested.map(|language| {
            LANGUAGES
                .iter()
                .position(|entry| entry.language == language)
                .unwrap()
        });
        assert!(positions.windows(2).all(|p| p[0] < p[1]));
        for language in requested {
            assert!(!info(language).native_name.is_empty());
            assert_eq!(
                info(language).enabled,
                matches!(
                    language,
                    Language::Mandarin
                        | Language::French
                        | Language::Russian
                        | Language::Italian
                        | Language::Japanese
                        | Language::Hindi
                        | Language::Arabic
                )
            );
        }
    }
}
