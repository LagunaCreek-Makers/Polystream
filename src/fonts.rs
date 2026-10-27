//! Use an installed CJK fallback without redistributing operating-system fonts.
pub fn install(ctx: &eframe::egui::Context) {
    let mut fonts = eframe::egui::FontDefinitions::default();
    let root = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "C:/Windows".into());
    if let Ok(bytes) = std::fs::read(root.join("Fonts/segoeui.ttf")) {
        fonts.font_data.insert(
            "ui-regular".into(),
            eframe::egui::FontData::from_owned(bytes).into(),
        );
        fonts
            .families
            .entry(eframe::egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "ui-regular".into());
    }
    let bold = "ui-semibold";
    if let Ok(bytes) = std::fs::read(root.join("Fonts/seguisb.ttf")) {
        fonts.font_data.insert(
            bold.into(),
            eframe::egui::FontData::from_owned(bytes).into(),
        );
        fonts.families.insert(
            eframe::egui::FontFamily::Name("semibold".into()),
            vec![bold.into()],
        );
    } else {
        fonts.families.insert(
            eframe::egui::FontFamily::Name("semibold".into()),
            fonts.families[&eframe::egui::FontFamily::Proportional].clone(),
        );
    }
    let candidates = [
        root.join("Fonts/msyh.ttc"),
        root.join("Fonts/simsun.ttc"),
        std::path::PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"),
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "cjk-fallback".into(),
                eframe::egui::FontData::from_owned(bytes).into(),
            );
            for family in [
                eframe::egui::FontFamily::Proportional,
                eframe::egui::FontFamily::Monospace,
            ] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .push("cjk-fallback".into());
            }

            break;
        }
    }
    // Load an installed Devanagari font; no OS font files are redistributed.
    for path in [
        root.join("Fonts/Nirmala.ttf"),
        root.join("Fonts/Nirmala.ttc"),
        std::path::PathBuf::from("/usr/share/fonts/truetype/noto/NotoSansDevanagari-Regular.ttf"),
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "devanagari-fallback".into(),
                eframe::egui::FontData::from_owned(bytes).into(),
            );
            for family in [
                eframe::egui::FontFamily::Proportional,
                eframe::egui::FontFamily::Monospace,
                eframe::egui::FontFamily::Name("semibold".into()),
            ] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .push("devanagari-fallback".into());
            }
            break;
        }
    }
    ctx.set_fonts(fonts);
}
