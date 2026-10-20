#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]
mod audio;
use audio::Mode;
use eframe::egui;
use live_translate_rs::language::{info, Language, LANGUAGES};
use std::{collections::VecDeque, sync::atomic::Ordering, time::Duration};
struct App {
    inputs: Vec<audio::DeviceChoice>,
    outputs: Vec<audio::DeviceChoice>,
    input: usize,
    output: usize,
    input_channel: usize,
    output_channel: usize,
    source: Language,
    target: Language,
    threshold: f32,
    mode: Mode,
    config_path: String,
    session: Option<audio::Session>,
    status: String,
    captions: VecDeque<String>,
    text: String,
    busy: bool,
}
impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            inputs: vec![],
            outputs: vec![],
            input: 0,
            output: 0,
            input_channel: 1,
            output_channel: 1,
            source: Language::English,
            target: Language::Spanish,
            threshold: 0.015,
            mode: Mode::LiveVoice,
            config_path: "offline.json".into(),
            session: None,
            status: "Ready — live voice translation".into(),
            captions: VecDeque::new(),
            text: String::new(),
            busy: false,
        };
        app.refresh();
        app
    }
}
impl App {
    fn refresh(&mut self) {
        match audio::devices() {
            Ok((i, o)) => {
                self.inputs = i;
                self.outputs = o;
                self.input = 0;
                self.output = 0;
                self.input_channel = 1;
                self.output_channel = 1;
            }
            Err(e) => self.status = format!("Device discovery: {e:#}"),
        }
    }
    fn start(&mut self) -> bool {
        let Some(output) = self.outputs.get(self.output) else {
            self.status = "Select an output device".into();
            return false;
        };
        let settings = audio::Settings {
            input: self.inputs.get(self.input).cloned(),
            output: output.clone(),
            input_channel: self.input_channel - 1,
            output_channel: self.output_channel - 1,
            source: self.source,
            target: self.target,
            threshold: self.threshold,
            mode: self.mode,
            config_path: self.config_path.clone().into(),
        };
        match audio::start(settings) {
            Ok(session) => {
                self.session = Some(session);
                self.captions.clear();
                self.status = if self.mode == Mode::LiveVoice {
                    "Starting local voice translation…"
                } else {
                    "Ready"
                }
                .into();
                true
            }
            Err(e) => {
                self.status = format!("Cannot start: {e:#}");
                false
            }
        }
    }
    fn stop(&mut self) {
        self.session = None;
        self.busy = false;
        self.status = "Stopped — local playback cleared".into();
    }
    fn send_text(&mut self) {
        if self.text.trim().is_empty() || self.text.chars().count() > 2000 {
            self.status = "Enter 1–2000 characters".into();
            return;
        }
        if self.session.is_none() && !self.start() {
            return;
        }
        match self
            .session
            .as_ref()
            .expect("session started")
            .send_text(&self.text)
        {
            Ok(()) => {
                self.busy = true;
                self.status = "Processing typed text…".into();
            }
            Err(e) => self.status = e.to_string(),
        }
    }
}
fn language_picker(ui: &mut egui::Ui, label: &str, selected: &mut Language) {
    egui::ComboBox::from_id_salt(label)
        .selected_text(info(*selected).name)
        .show_ui(ui, |ui| {
            for l in LANGUAGES
                .iter()
                .filter(|l| l.enabled && (label != "to" || l.language != Language::English))
            {
                ui.selectable_value(selected, l.language, l.name);
            }
        });
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(40));
        let mut fatal = None;
        if let Some(session) = &self.session {
            self.busy = session.text_busy();
            fatal = session.failure();
            if fatal.is_none() && session.stats.faults.load(Ordering::Relaxed) > 0 {
                fatal = Some("Audio device disconnected or failed; reconnect and refresh".into());
            }
            for event in session.events.try_iter() {
                match event {
                    audio::Event::Status(s) => self.status = s,

                    audio::Event::Source(s) => self.captions.push_back(format!("Source: {s}")),
                    audio::Event::Translation(s) => self
                        .captions
                        .push_back(format!("Translation / speech: {s}")),
                }
            }
        }
        if let Some(message) = fatal {
            self.stop();
            self.status = format!("Stopped: {message}");
        }
        while self.captions.len() > 100 {
            self.captions.pop_front();
        }
        egui::CentralPanel::default().show(ctx,|ui| {
            ui.heading("Live Translate");ui.label("Offline voice translation • Religious speech • Spanish & German • Rust v0.5");ui.separator();
            ui.add_enabled_ui(self.session.is_none(),|ui| {
                ui.horizontal(|ui|{ui.label("Mode");egui::ComboBox::from_id_salt("mode").selected_text(self.mode.name()).show_ui(ui,|ui| {
                    for mode in [Mode::LiveVoice,Mode::TextTranslation,Mode::TextToSpeech,Mode::AudioCheck] {ui.selectable_value(&mut self.mode,mode,mode.name());}
                });});
                if !self.mode.typed() {
                    ui.horizontal(|ui|{ui.label("Input");egui::ComboBox::from_id_salt("input").selected_text(self.inputs.get(self.input).map(|d|d.name.as_str()).unwrap_or("No input"))
                        .show_ui(ui,|ui|{for (i,d) in self.inputs.iter().enumerate(){if ui.selectable_value(&mut self.input,i,&d.name).changed(){self.input_channel=1;}}});
                        let max=self.inputs.get(self.input).map(|d|d.channels as usize).unwrap_or(1);ui.add(egui::DragValue::new(&mut self.input_channel).range(1..=max).prefix("Channel "));});
                }
                ui.horizontal(|ui|{ui.label("Output");egui::ComboBox::from_id_salt("output").selected_text(self.outputs.get(self.output).map(|d|d.name.as_str()).unwrap_or("No output"))
                    .show_ui(ui,|ui|{for (i,d) in self.outputs.iter().enumerate(){if ui.selectable_value(&mut self.output,i,&d.name).changed(){self.output_channel=1;}}});
                    let max=self.outputs.get(self.output).map(|d|d.channels as usize).unwrap_or(1);ui.add(egui::DragValue::new(&mut self.output_channel).range(1..=max).prefix("Channel "));});
                if ui.button("Refresh devices").clicked(){self.refresh();}
                if self.mode!=Mode::AudioCheck {ui.horizontal(|ui|{
                    if self.mode!=Mode::TextToSpeech {ui.label("From");language_picker(ui,"from",&mut self.source);}
                    ui.label(if self.mode==Mode::TextToSpeech {"Speak in"} else {"To"});language_picker(ui,"to",&mut self.target);
                });ui.label("Local model configuration");ui.text_edit_singleline(&mut self.config_path);}
                if !self.mode.typed() {ui.add(egui::Slider::new(&mut self.threshold,0.002..=0.1).logarithmic(true).text("Speech sensitivity"));}
            });
            ui.separator();
            if self.mode==Mode::LiveVoice {
                ui.label("Microphone / line input → local recognition → local translation → spoken output");
                ui.label("Runs on this PC. Short speech clauses are translated while capture continues.");
            }
            if self.mode.typed() {
                ui.add(egui::TextEdit::multiline(&mut self.text).desired_rows(4).desired_width(f32::INFINITY).hint_text("Type text here…"));
                ui.label(format!("{} / 2000 characters",self.text.chars().count()));
                if ui.add_enabled(!self.busy,egui::Button::new(if self.mode==Mode::TextTranslation {"Translate & speak"} else {"Speak text"})).clicked(){self.send_text();}
            } else if self.session.is_none() && ui.button("Start").clicked(){self.start();}
            ui.horizontal(|ui|{
                if self.session.is_some() && ui.button("Stop").clicked(){self.stop();}
                if ui.button("Clear captions").clicked(){self.captions.clear();}
            });
            if let Some(session)=&self.session {
                if !self.mode.typed() {
                    let level=f32::from_bits(session.stats.level.load(Ordering::Relaxed));let db=20.0*level.max(0.00001).log10();
                    ui.add(egui::ProgressBar::new(((db+60.0)/60.0).clamp(0.0,1.0)).text(format!("Input {db:.0} dBFS")));
                    ui.label(format!("Dropped blocks: {} | Clipped samples: {}",session.stats.dropped.load(Ordering::Relaxed),session.stats.clipped.load(Ordering::Relaxed)));
                }
                ui.label(format!("Playback queued: {:.0} ms (not end-to-end latency)",session.queued_seconds()*1000.0));
                if let Some(ms)=session.first_audio_ms(){ui.label(format!("Clause/text → first queued audio: {ms:.0} ms"));}
            }
            ui.label(&self.status);
            egui::ScrollArea::vertical().max_height(230.0).stick_to_bottom(true).show(ui,|ui|{for line in &self.captions{ui.label(line);}});
            ui.separator();ui.small("Standard voices; speaker timbre and inflection matching are future work.");
            ui.small("Use headphones or an isolated translation output. One target language per session.");
        });
    }
}
fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Live Translate",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([940.0, 800.0]),
            ..Default::default()
        },
        Box::new(|_| Ok(Box::new(App::default()))),
    )
}
