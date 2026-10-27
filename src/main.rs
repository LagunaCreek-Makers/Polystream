#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]
mod audio;
mod fonts;
mod resources;
use audio::Mode;
use eframe::egui;
use live_translate_rs::language::{info, source_supported, Language, LANGUAGES};
use live_translate_rs::levels::AudioLevel;
use std::{
    collections::VecDeque,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
static RESTART_ENGINES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(Clone)]
struct OutputStream {
    language: Language,
    device: usize,
    channel: usize,
    volume: f32,
    muted: bool,
    level: Arc<AudioLevel>,
}
impl OutputStream {
    fn new(language: Language) -> Self {
        Self {
            language,
            device: 0,
            channel: 1,
            volume: 100.,
            muted: false,
            level: Arc::new(AudioLevel::default()),
        }
    }
}
struct App {
    streams: Vec<OutputStream>,
    engine_device: String,
    engine_threads: u32,
    engine_slots: u32,
    engine_layers: u32,
    engine_context: u32,
    language_dialog: Option<u8>,
    dialog_language: Language,
    settings_open: bool,
    model_help_open: bool,
    custom_config_text: String,
    stock_engines: bool,
    engine_restart_pending: bool,
    preset_choice: usize,
    resource_history: VecDeque<(f32, f32)>,
    resource_tick: std::time::Instant,
    theme_configured: bool,
    input_level: Arc<AudioLevel>,
    input_gain_db: f32,
    resources: resources::Resources,
    inputs: Vec<audio::DeviceChoice>,
    outputs: Vec<audio::DeviceChoice>,
    input: usize,
    input_channel: usize,
    source: Language,
    project_folder: String,
    threshold: f32,
    mode: Mode,
    config_path: String,
    session: Option<audio::Session>,
    status: String,
    captions: VecDeque<String>,
    text: String,
    busy: bool,
    buffer_minutes: u32,
    failure_reported: bool,
}
impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            streams: vec![OutputStream::new(Language::Spanish)],
            engine_device: std::env::var("LIVE_TRANSLATE_DEVICE").unwrap_or_else(|_| "cuda".into()),
            engine_threads: env_number("LIVE_TRANSLATE_THREADS", 4),
            engine_slots: env_number("LIVE_TRANSLATE_SLOTS", 1),
            engine_layers: env_number("LIVE_TRANSLATE_LAYERS", 99),
            engine_context: env_number("LIVE_TRANSLATE_CONTEXT", 4096),
            language_dialog: None,
            dialog_language: Language::German,
            settings_open: false,
            model_help_open: false,
            engine_restart_pending: false,
            preset_choice: 0,
            custom_config_text: String::new(),
            stock_engines: std::env::var("LIVE_TRANSLATE_STOCK_ENGINES")
                .map(|v| v != "0")
                .unwrap_or(true),
            resource_history: VecDeque::new(),
            resource_tick: std::time::Instant::now(),
            theme_configured: false,
            input_level: Arc::new(AudioLevel::default()),
            input_gain_db: 0.,
            resources: resources::Resources::new(),
            inputs: vec![],
            outputs: vec![],
            input: 0,
            input_channel: 1,
            source: Language::English,
            project_folder: default_project_directory(),
            threshold: 0.015,
            mode: Mode::LiveVoice,
            config_path: std::env::var("LIVE_TRANSLATE_CONFIG").unwrap_or_else(|_| {
                if std::path::Path::new("offline.json").is_file() {
                    return "offline.json".into();
                }
                std::env::current_exe()
                    .ok()
                    .and_then(|exe| {
                        exe.ancestors()
                            .skip(1)
                            .map(|parent| parent.join("offline.json"))
                            .find(|path| path.is_file())
                    })
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "offline.json".into())
            }),
            session: None,
            status: "Ready — live voice translation".into(),
            captions: VecDeque::new(),
            text: String::new(),
            busy: false,
            buffer_minutes: 60,
            failure_reported: false,
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

                self.input_channel = 1;
            }
            Err(e) => self.status = format!("Device discovery: {e:#}"),
        }
    }
    fn remove_stream(&mut self, index: u8) {
        if self.session.is_none() && (index as usize) < self.streams.len() {
            self.streams.remove(index as usize);
            self.language_dialog = None;
        }
    }
    fn add_stream(&mut self, language: Language) -> bool {
        if self.streams.len() >= 8 || self.streams.iter().any(|s| s.language == language) {
            return false;
        }
        let mut stream = OutputStream::new(language);
        'route: for (device, choice) in self.outputs.iter().enumerate() {
            for channel in 1..=choice.channels as usize {
                if !self
                    .streams
                    .iter()
                    .any(|s| s.device == device && s.channel == channel)
                {
                    stream.device = device;
                    stream.channel = channel;
                    break 'route;
                }
            }
        }
        self.streams.push(stream);
        true
    }
    fn start(&mut self) -> bool {
        if self.engine_restart_pending {
            self.status =
                "Apply & restart engines before starting with the selected model configuration."
                    .into();
            return false;
        }
        let Some(primary) = self.streams.first() else {
            self.status = "Add a translation stream before starting.".into();
            return false;
        };
        let Some(output) = self.outputs.get(primary.device) else {
            self.status = "Select an output device".into();
            return false;
        };
        let mut additional = Vec::new();
        if self.mode == Mode::LiveVoice {
            for stream in self.streams.iter().skip(1) {
                let Some(device) = self.outputs.get(stream.device) else {
                    self.status = "Select output devices for all streams".into();
                    return false;
                };
                additional.push(audio::AdditionalOutput {
                    level: stream.level.clone(),
                    device: device.clone(),
                    channel: stream.channel - 1,
                    language: stream.language,
                });
            }
        }
        for level in std::iter::once(&self.input_level).chain(self.streams.iter().map(|s| &s.level))
        {
            level.clear_meter();
            level.reset_clipping();
        }
        let settings = audio::Settings {
            output: output.clone(),
            output_channel: primary.channel - 1,
            target: primary.language,
            input_level: self.input_level.clone(),
            output_level: primary.level.clone(),
            additional,
            transcript_project: self.project_folder.clone().into(),
            input: self.inputs.get(self.input).cloned(),
            input_channel: self.input_channel - 1,
            source: self.source,
            threshold: self.threshold,
            mode: self.mode,
            config_path: self.config_path.clone().into(),
            buffer_minutes: self.buffer_minutes,
        };
        match audio::start(settings) {
            Ok(session) => {
                self.failure_reported = false;
                self.session = Some(session);
                self.captions.clear();
                self.status = if self.mode == Mode::LiveVoice {
                    "Live translation running — pending speech is buffered locally."
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
        for level in std::iter::once(&self.input_level).chain(self.streams.iter().map(|s| &s.level))
        {
            level.clear_meter();
        }
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
fn audio_meter(ui: &mut egui::Ui, label: &str, level: &AudioLevel) {
    let rms = level.rms();
    let db = 20. * rms.max(0.000001).log10();
    let peak_db = 20. * level.peak().max(0.000001).log10();
    ui.vertical(|ui| {
        if label == "Input Level" {
            ui.strong(label);
        } else if label == "Output Level" {
            ui.label(egui::RichText::new(label).size(13.));
        } else {
            ui.small(if rms > 0. {
                format!("{label}: {db:.1} dBFS")
            } else {
                format!("{label}: silence")
            });
        }
        let width = ui.available_width().clamp(70., 500.);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 15.), egui::Sense::hover());
        let lit = ((db + 60.) / 60.).clamp(0., 1.);
        for i in 0..30 {
            let fraction = (i + 1) as f32 / 30.;
            let color = if fraction > lit {
                egui::Color32::from_gray(40)
            } else if i >= 28 {
                egui::Color32::from_rgb(245, 80, 80)
            } else if i >= 24 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::from_rgb(30, 215, 115)
            };
            let x = rect.left() + i as f32 * width / 30.;
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(x, rect.top()),
                    egui::vec2((width / 30. - 2.).max(1.), 15.),
                ),
                1.,
                color,
            );
        }
        let x = rect.left() + ((peak_db + 60.) / 60.).clamp(0., 1.) * width;
        if level.peak() > 0. {
            ui.painter().line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                egui::Stroke::new(1_f32, egui::Color32::WHITE),
            );
        }
    });
    if level.clipped() > 0 {
        ui.colored_label(
            egui::Color32::LIGHT_RED,
            format!("Post-fader clipped: {}", level.clipped()),
        );
        if ui.small_button("Reset clip count").clicked() {
            level.reset_clipping();
        }
    }
}
fn language_picker(ui: &mut egui::Ui, label: &str, selected: &mut Language) {
    egui::ComboBox::from_id_salt(label)
        .width(ui.available_width().min(145.))
        .truncate()
        .selected_text(info(*selected).name)
        .show_ui(ui, |ui| {
            for l in LANGUAGES.iter().filter(|l| {
                (label != "to" || l.language != Language::English)
                    && (label != "from" || (l.enabled && source_supported(l.language)))
            }) {
                let caption = if !l.enabled {
                    format!("{} — not implemented yet", l.name)
                } else if l.language == Language::Mandarin {
                    format!("{} (experimental)", l.name)
                } else {
                    l.name.into()
                };
                ui.selectable_value(selected, l.language, caption);
            }
        });
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if !self.theme_configured {
            install_theme(ctx);
            self.theme_configured = true;
        }
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
                    audio::Event::Timing(_) => {}
                    audio::Event::TranslationFor(language, s) => self
                        .captions
                        .push_back(format!("{}: {s}", info(language).name)),

                    audio::Event::Source(s) => self.captions.push_back(format!("Source: {s}")),
                    audio::Event::Translation(s) => self
                        .captions
                        .push_back(format!("Translation / speech: {s}")),
                }
            }
        }
        if let Some(message) = fatal {
            if self.mode == Mode::LiveVoice {
                if !self.failure_reported {
                    self.status=format!("Processing stopped: {message}. Pending source audio remains in memory; save it before stopping.");
                    self.failure_reported = true;
                }
            } else {
                self.stop();
                self.status = format!("Stopped: {message}");
            }
        }
        if self
            .session
            .as_ref()
            .is_some_and(|session| session.drained())
        {
            self.stop();
            self.status =
                "Finished — all buffered speech has played. Start for the next speaker.".into();
        }
        while self.captions.len() > 100 {
            self.captions.pop_front();
        }
        egui::CentralPanel::default().show(ctx,|ui| {
            egui::ScrollArea::vertical().id_salt("meeting-controls").show(ui, |ui| {
            ui.horizontal(|ui| {
                waveform_logo(ui);
                ui.vertical(|ui| {ui.heading(egui::RichText::new("PolyStream").strong().size(30.).color(egui::Color32::from_rgb(180,210,250)));
                    ui.label("Real-Time Multilingual Translation");});
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                    if ui.button("⚙ Settings").clicked(){self.settings_open = !self.settings_open;}
                    ui.colored_label(if self.failure_reported {egui::Color32::LIGHT_RED} else {egui::Color32::from_rgb(35,220,125)},if self.failure_reported {"● Needs attention"} else if self.session.is_some() {"● Running"} else {"○ Ready"});
                });
            });
            ui.add_space(12.);
            let running=self.session.is_some();
            let input_width=(ui.available_width()*0.32).clamp(280.,420.);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(egui::vec2(input_width,0.),egui::Layout::top_down(egui::Align::Min),|ui| {
                panel(ui, "Input Audio Stream", |ui| {
                    ui.set_min_height(275.);
                    ui.add_enabled_ui(!running,|ui| {
                        ui.label("Audio source");
                        device_picker_only(ui,"input",&self.inputs,&mut self.input,&mut self.input_channel);
                    });
                    ui.add_space(12.);
                    ui.horizontal(|ui| {ui.strong("Microphone gain");ui.label(format!("{:+.0} dB",self.input_gain_db));});
                    ui.style_mut().spacing.slider_width=(ui.available_width()-12.).max(100.);
                    blue_slider(ui,&mut self.input_gain_db,-24.0..=24.0);
                    self.input_level.set_input_db(self.input_gain_db);
                    audio_meter(ui,"Input Level",&self.input_level);

                    ui.small("App gain • Windows device volume is separate");
                });
                });
                ui.allocate_ui_with_layout(egui::vec2(ui.available_width(),0.),egui::Layout::top_down(egui::Align::Min),|ui| {
                panel(ui, "", |ui| {
                    ui.horizontal(|ui| {
                        globe_icon(ui);
                        ui.heading(egui::RichText::new("Output Translation Streams").color(egui::Color32::from_rgb(237,244,255)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                            if ui.add_enabled(!running && self.streams.len()<8,egui::Button::new("＋ Add Language Stream").fill(egui::Color32::from_rgb(25,111,245))).on_hover_text("Up to eight distinct language streams").clicked(){self.dialog_language=LANGUAGES.iter().find(|l|l.enabled && l.language!=self.source && !self.streams.iter().any(|s|s.language==l.language)).map(|l|l.language).unwrap_or(Language::French);self.language_dialog=Some(self.streams.len() as u8);}

                        });
                    });
                    ui.add_space(8.);
                    ui.set_min_height(275.);
                    let mut action=None;
                    for (index,stream) in self.streams.iter_mut().enumerate() {
                        let result=output_row(ui,&format!("stream-{index}"),index+1,&self.outputs,&mut stream.device,&mut stream.channel,&mut stream.language,&mut stream.volume,&mut stream.muted,&stream.level,running);
                        if result!=0 {action=Some((index,result));}
                    }
                    if let Some((index,kind))=action {if kind==2 {self.remove_stream(index as u8);}else{self.dialog_language=self.streams[index].language;self.language_dialog=Some(index as u8);}}
                    if self.streams.is_empty(){ui.label("No output streams. Choose Add Language Stream to begin.");}
                    ui.small("Up to eight streams • Each requires a separate device/channel");
                    let capacity=read_capacity(&self.config_path,self.streams.iter().map(|s|info(s.language).text_code).collect(),&self.engine_device,self.engine_threads,self.engine_slots,self.engine_context,self.engine_layers,self.resources.fingerprint().as_deref());
                    ui.label(capacity);

                });
                });
            });
            ui.add_space(12.);
            panel(ui,"Session",|ui| {
                ui.horizontal_wrapped(|ui| {
                    if !self.mode.typed() && !running && ui.button("▶ Start translation").clicked(){self.start();}
                    if let Some(session)=&self.session {
                        if self.mode==Mode::LiveVoice {
                            if ui.add_enabled(!session.finishing() && !self.failure_reported,egui::Button::new(if session.music_paused(){"Resume speech"}else{"Music break"})).clicked(){session.pause_for_music(!session.music_paused());}
                            if ui.add_enabled(!session.finishing() && !self.failure_reported,egui::Button::new("Finish & drain")).clicked(){session.finish_pending();}
                        }
                    }
                    if running && ui.button("Stop & discard pending").clicked(){self.stop();}
                    if ui.button("Clear captions").clicked(){self.captions.clear();}
                });
                ui.label(&self.status);
                if self.mode.typed() {
                    ui.add(egui::TextEdit::multiline(&mut self.text).desired_rows(3).desired_width(f32::INFINITY).hint_text("Enter up to 2000 characters"));
                    if ui.add_enabled(!self.busy,egui::Button::new("Speak / translate text")).clicked(){self.send_text();}
                }
            });
            ui.add_space(12.);
            panel(ui,"Real-Time Performance",|ui| {
                let resource=self.resources.sample();
                if self.resource_tick.elapsed()>=Duration::from_secs(1) {
                    self.resource_history.push_back((resource.cpu,resource.used as f32/1048576.));
                    if self.resource_history.len()>240 {self.resource_history.pop_front();}
                    self.resource_tick=std::time::Instant::now();
                }
                let delay=self.session.as_ref().and_then(|session|session.output_snapshots().iter().filter_map(|s|s.delay_seconds).fold(None,|v:Option<f64>,d|Some(v.unwrap_or(0.).max(d))));
                ui.columns(4,|columns| {
                    metric_card(&mut columns[0],"CPU Usage",&format!("{:.0}%",resource.cpu),egui::Color32::from_rgb(35,220,115),&self.resource_history.iter().map(|p|p.0).collect::<Vec<_>>());
                    metric_card(&mut columns[1],"PolyStream Memory",&format!("{:.0} MiB",resource.used as f64/1048576.),egui::Color32::from_rgb(55,150,255),&self.resource_history.iter().map(|p|p.1).collect::<Vec<_>>());
                    metric_card(&mut columns[2],"GPU Usage","Unavailable",egui::Color32::from_rgb(170,100,240),&[]);
                    metric_card(&mut columns[3],"Translation Delay",&delay.map(|d|format!("{d:.1} s")).unwrap_or_else(||"Waiting".into()),egui::Color32::from_rgb(255,165,55),&[]);
                });
                ui.small("CPU: whole system • RAM: PolyStream process only, excludes model engines • Up to 4 min history • Delay: slowest last-delivered stream, excludes device latency");
            });
            ui.add_space(12.);
            if self.session.is_some() {
            panel(ui,"Translation Delay Over Time",|ui| {
            if let Some(session)=&self.session {
                if let Some(path)=&session.transcript_directory {ui.small(format!("Saving SRT transcripts: {}",path.display()));}
                let snapshots=session.output_snapshots();
                for (index,output) in snapshots.iter().enumerate() {
                    ui.colored_label(stream_color(index),format!("{} | last delivered delay: {} | oldest pending: {:.1} s | pending: {} | delivered: {} | untranslated: {}",info(output.language).name,
                        output.delay_seconds.map(|d|format!("{d:.1} s")).unwrap_or_else(||if output.failed>0 {"untranslated (no audio)".into()} else {"waiting".into()}),output.oldest_seconds,output.pending,output.completed,output.failed));
                }
                if snapshots.iter().any(|s|!s.history.is_empty() || !s.gaps.is_empty()) {
                    ui.small("Source phrase start → first queued PCM (seconds); device latency excluded. Colors identify each language; measurements are not acoustic latency.");
                    let rect=plot_grid(ui,150.);
                    let max_x=snapshots.iter().flat_map(|s|s.history.iter().map(|p|p.0).chain(s.gaps.iter().map(|gap|gap.1))).fold(1.,f64::max);
                    let max_y=snapshots.iter().flat_map(|s|s.history.iter().map(|p|p.1)).fold(10.,f64::max);

                    for (i,snapshot) in snapshots.iter().enumerate() {
                        let color=stream_color(i);
                        for point in &snapshot.history {
                            let position=egui::pos2(rect.left()+(point.0/max_x) as f32*rect.width(),rect.bottom()-(point.1/max_y) as f32*rect.height());
                            ui.painter().circle_filled(position,2.,color);
                        }
                        for points in snapshot.history.windows(2) {if points[1].0-points[0].0>15. || snapshot.gaps.iter().any(|gap|gap.0>=points[0].0 && gap.0<=points[1].0) {continue;}
                            let map=|p:(f64,f64)|egui::pos2(rect.left()+(p.0/max_x) as f32*rect.width(),rect.bottom()-(p.1/max_y) as f32*rect.height());
                            ui.painter().line_segment([map(points[0]),map(points[1])],egui::Stroke::new(1.5_f32,color));
                        }
                    }
                    ui.small(format!("Meeting: 0–{:.1} min | delay: 0–{max_y:.1} s",max_x/60.));
                }
                if self.mode==Mode::LiveVoice {
                    if let Some(buffer)=session.buffered() {
                        ui.label(format!("Pending source: {:.1} s in {} phrases | Oldest pending speech: {:.1} s old",buffer.audio_seconds,buffer.clauses,buffer.oldest_seconds));
                        if buffer.capacity_reached {ui.colored_label(egui::Color32::RED,"Buffer capacity reached: new capture paused. Preserved speech is finishing; restart capture after draining.");}
                        else if session.finishing(){ui.label("Capture finished — speaking all pending translations.");}
                        else if session.music_paused(){ui.label("Music break: new audio is excluded; pending speech continues playing. Resume before the next speaker.");}
                        if buffer.oldest_seconds>30. {ui.colored_label(egui::Color32::YELLOW,"Translation is behind the speaker. Keep the session running to preserve pending speech.");}
                        if self.failure_reported && buffer.clauses>0 && ui.button("Save pending source audio for recovery").clicked() {
                            self.status=match session.save_pending(){Ok(path)=>format!("Saved pending source audio: {}. This recovery file remains until you delete it.",path.display()),Err(error)=>format!("Could not save pending audio: {error:#}")};
                        }
                    }
                }
                if !self.mode.typed() {
                    ui.label(format!("Dropped blocks: {} | Source clipped samples (before app gain): {}",session.stats.dropped.load(Ordering::Relaxed),session.stats.clipped.load(Ordering::Relaxed)));
                }
                ui.label(format!("Playback queued: {:.0} ms (not end-to-end latency)",session.queued_seconds()*1000.0));
                if let Some(ms)=session.first_audio_ms(){ui.label(format!("Clause/text → first queued audio: {ms:.0} ms"));}
            }
            });
            }
            egui::CollapsingHeader::new("Live captions").default_open(true).show(ui,|ui| {
            if self.captions.is_empty(){ui.label("Recognized source speech and each language translation will appear here during a session. SRT transcripts save separately in your project folder.");}
            egui::ScrollArea::vertical().max_height(230.0).stick_to_bottom(true).show(ui,|ui|{for line in &self.captions{ui.label(line);}});
            });
            ui.separator();ui.small("Standard voices; speaker timbre and inflection matching are future work.");
            ui.small("Use headphones or isolated translation outputs. Each language needs a separate device/channel.");
            });
        });
        if self.settings_open {
            let mut open = true;
            egui::Window::new("PolyStream Settings").open(&mut open).default_width(580.).default_height(650.).anchor(egui::Align2::CENTER_CENTER,egui::Vec2::ZERO).resizable(true).collapsible(false)
                .show(ctx,|ui| {
                    let running=self.session.is_some();
                    if running {ui.label("Finish or stop the session to change configuration. Live volume controls remain on the dashboard.");}
                    egui::ScrollArea::vertical().max_height(650.).show(ui,|ui| {

                ui.add_enabled_ui(!running,|ui| {
                    ui.label("Source language");language_picker(ui,"from",&mut self.source);
                    let max=self.inputs.get(self.input).map(|d|d.channels as usize).unwrap_or(1);
                    ui.add(egui::DragValue::new(&mut self.input_channel).range(1..=max).prefix("Input channel "));
                    ui.label("Transcript project folder — English and target SRT files saved automatically");
                    ui.add(egui::TextEdit::singleline(&mut self.project_folder).desired_width(f32::INFINITY));
                });
                ui.separator(); ui.heading("Pretested Models");
                ui.small("Choose a saved configuration, then Apply & restart engines. Language voices and output routing are preserved.");
                ui.label(format!("Selected configuration: {}", self.config_path));
                ui.add_enabled_ui(!running, |ui| {
                    let presets=live_translate_rs::model_presets::PRESETS;
                    egui::ComboBox::from_id_salt("tested-workflow-selector")
                        .selected_text(presets[self.preset_choice].1).width(520.)
                        .show_ui(ui,|ui| {
                            for (index,(_,title,_)) in presets.iter().enumerate() {
                                ui.selectable_value(&mut self.preset_choice,index,*title);
                            }
                        });
                    let (id,_,note)=presets[self.preset_choice];
                    ui.small(note);
                    ui.label(match id {
                        "quality"=>"Speech to text: Whisper base • Translation: Qwen3-8B • Speech: installed Piper voices",
                        "small"=>"Speech to text: Whisper small.en • Translation: Qwen3-4B • Speech: installed Piper voices",
                        "medium"=>"Speech to text: Whisper medium.en • Translation: Qwen3-4B • Speech: installed Piper voices",
                        _=>"Speech to text: Whisper base • Translation: Qwen3-4B • Speech: installed Piper voices",
                    });
                    ui.small("Tested settings: CPU + NVIDIA GPU • 4 threads per engine • 1 translation slot • 4096 context tokens • 99 GPU layers. Installed voices and routing are retained; custom voices need their own validation.");
                    if ui.button("Use this tested configuration").clicked() {
                        match live_translate_rs::model_presets::select(std::path::Path::new(&self.config_path),id) {
                            Ok(path)=>{
                                self.config_path=path.to_string_lossy().into_owned();self.stock_engines=true;
                                self.engine_device="cuda".into();self.engine_threads=4;self.engine_slots=1;self.engine_context=4096;self.engine_layers=99;
                                self.engine_restart_pending=true;
                                self.status="Tested configuration selected. Apply & restart engines to activate.".into();
                            },
                            Err(error)=>self.status=format!("Configuration unavailable: {error:#}. See Models help for installation."),
                        }
                    }
                    egui::CollapsingHeader::new("Custom engine configuration").show(ui, |ui| {
                        ui.label("Compatible STT / translation / TTS engines and model paths (JSON)");
                        if ui.button("Load selected configuration for editing").clicked() {
                            match std::fs::read_to_string(&self.config_path) {Ok(text)=>self.custom_config_text=text,Err(error)=>self.status=format!("Cannot load config: {error}")}
                        }
                        ui.add(egui::TextEdit::multiline(&mut self.custom_config_text).font(egui::TextStyle::Monospace).desired_rows(10).desired_width(f32::INFINITY));
                        if ui.button("Validate & save as new custom configuration").clicked() {
                            let result=serde_json::from_str::<live_translate_rs::config::OfflineConfig>(&self.custom_config_text)
                                .map_err(anyhow::Error::from).and_then(|config| live_translate_rs::model_presets::save_configuration(std::path::Path::new(&self.config_path),config,"custom"));
                            match result {Ok(path)=>{self.config_path=path.to_string_lossy().into_owned();self.stock_engines=false;self.engine_restart_pending=true;self.status="Custom configuration saved. Apply & restart engines to activate.".into();},Err(error)=>self.status=format!("Custom configuration rejected: {error:#}")}
                        }
                        ui.small("Relative paths resolve beside the currently selected configuration. Custom engine programs must implement the documented local interfaces and launch arguments.");
                    });
                });
                if ui.button("Models help • results, recommendations & custom engines").clicked() {self.model_help_open=true;}
                egui::CollapsingHeader::new("Advanced model and audio settings").show(ui,|ui| {
                    ui.add_enabled_ui(!running,|ui| {
                        egui::ComboBox::from_id_salt("mode").selected_text(self.mode.name()).show_ui(ui,|ui| {
                            for mode in [Mode::LiveVoice,Mode::TextTranslation,Mode::TextToSpeech,Mode::AudioCheck] {ui.selectable_value(&mut self.mode,mode,mode.name());}
                        });
                        ui.add(egui::Slider::new(&mut self.threshold,0.002..=0.1).logarithmic(true).text("Speech threshold"));
                        ui.label("Local model configuration");if ui.text_edit_singleline(&mut self.config_path).changed(){self.stock_engines=false;self.engine_restart_pending=true;}
                        ui.add(egui::DragValue::new(&mut self.buffer_minutes).range(15..=120).suffix(" minutes pending source capacity"));
                        if ui.button("Refresh devices").clicked(){self.refresh();}
                    });
                    ui.small("Stop discards temporary pending audio. Music break excludes new input while queued speech continues.");
                    ui.small("Mandarin is experimental and requires its separate voice configuration.");
                });

                    ui.separator();ui.heading("Inference hardware");
                    ui.add_enabled_ui(!running,|ui| {
                        egui::ComboBox::from_id_salt("engine-device").selected_text(if self.engine_device=="cuda" {"CPU + NVIDIA GPU"}else{"CPU only"}).show_ui(ui,|ui| {
                            ui.selectable_value(&mut self.engine_device,"cpu".into(),"CPU only");
                            ui.selectable_value(&mut self.engine_device,"cuda".into(),"CPU + NVIDIA GPU");
                        });
                        egui::CollapsingHeader::new("Advanced engine parameters").show(ui,|ui| {
                            ui.add(egui::DragValue::new(&mut self.engine_threads).range(1..=128).prefix("CPU threads per engine: "));
                            ui.add(egui::DragValue::new(&mut self.engine_layers).range(1..=999).prefix("GPU translation layers: "));
                            ui.add(egui::DragValue::new(&mut self.engine_slots).range(1..=2).prefix("Translation slots: "));
                            ui.add(egui::DragValue::new(&mut self.engine_context).range(1024..=16384).prefix("Context tokens per slot: "));
                            if ui.button("Restore advanced defaults").on_hover_text("Reset these fields; use Apply & restart engines to activate them.").clicked() {
                                self.engine_threads=4;
                                self.engine_layers=99;
                                self.engine_slots=1;
                                self.engine_context=4096;
                            }
                            ui.small("Validated baseline: 4 threads, 1 slot, 4096 tokens per slot. Additional slots or context increase memory use. GPU layers apply only in GPU mode.");
                        });
                        let managed=std::env::var_os("LIVE_TRANSLATE_MANAGED").is_some();
                        ui.small("Engine changes require a restart. Recheck audio routing after reopening. Session settings above apply on the next Start.");
                        if !managed {ui.label("Open with Start-Translation-GPU.cmd or Start-Translation.cmd to enable engine restarts.");}
                        if ui.add_enabled(managed,egui::Button::new("Apply & restart engines")).clicked() {
                            let path=std::env::var_os("LIVE_TRANSLATE_ENGINE_SETTINGS").map(std::path::PathBuf::from);
                            let result=path.ok_or_else(||"Launcher settings location missing".to_string()).and_then(|path| {
                                let value=serde_json::json!({"device":self.engine_device,"threads":self.engine_threads,"translation_slots":self.engine_slots,"gpu_layers":self.engine_layers,"context_tokens":self.engine_context,"config":self.config_path,"stock_engines":self.stock_engines});
                                std::fs::write(path,serde_json::to_vec_pretty(&value).expect("serializable settings")).map_err(|e|e.to_string())
                            });
                            match result {Ok(())=>{RESTART_ENGINES.store(true,Ordering::Release);ctx.send_viewport_cmd(egui::ViewportCommand::Close);},Err(e)=>self.status=format!("Cannot save engine settings: {e}")}
                        }
                    });
                    });
                });
            self.settings_open = open;
        }
        if self.model_help_open {
            let mut open = true;
            egui::Window::new("Models help")
                .open(&mut open)
                .default_size(egui::vec2(760., 620.))
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for line in include_str!("../MODEL_ENGINE_GUIDE.md").lines() {
                            if let Some(title) = line.strip_prefix("# ") {
                                ui.heading(title);
                            } else if let Some(title) = line.strip_prefix("## ") {
                                ui.add_space(10.);
                                ui.strong(title);
                            } else {
                                ui.label(line);
                            }
                        }
                    });
                });
            self.model_help_open = open;
        }
        if let Some(stream) = self.language_dialog {
            let mut open = true;
            let mut apply = false;
            let mut cancel = false;
            egui::Window::new(
                if (stream as usize)<self.streams.len() {
                    "Modify output language"
                } else {
                    "Add language stream"
                },
            )
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Output language");
                language_picker(ui, "to", &mut self.dialog_language);
                let implemented=info(self.dialog_language).enabled;
                if !implemented {
                    ui.colored_label(egui::Color32::YELLOW,"Not implemented yet");
                    ui.label(format!("{} is planned, but its translation and speech output are not available yet.",info(self.dialog_language).name));
                    ui.small(info(self.dialog_language).note);
                }
                if self.dialog_language == Language::Mandarin {
                    ui.small("Experimental Mandarin output: uses the installed optional voice pack. Native-speaker review and production voice licensing remain pending.");
                }
                if self.dialog_language == Language::Russian {
                    ui.small("Experimental Russian output: Denis voice installed. Broader translation quality and native-speaker review remain pending.");
                }
                if matches!(self.dialog_language, Language::Italian | Language::Japanese | Language::Hindi | Language::Arabic) {
                    ui.small(info(self.dialog_language).note);
                }
                let duplicate=self.streams.iter().enumerate().any(|(index,s)|index!=stream as usize && s.language==self.dialog_language);
                if duplicate {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        "Choose a different language for each stream.",
                    );
                }
                ui.horizontal(|ui| {
                    apply = ui
                        .add_enabled(
                            implemented && !duplicate && self.session.is_none(),
                            egui::Button::new("Apply language"),
                        )
                        .clicked();
                    cancel = ui.button("Cancel").clicked();
                    if (stream as usize)<self.streams.len() && self.session.is_none() && ui.button("Remove stream").clicked(){self.remove_stream(stream);cancel=true;}
                });
            });
            if apply {
                if let Some(output) = self.streams.get_mut(stream as usize) {
                    output.language = self.dialog_language;
                } else {
                    self.add_stream(self.dialog_language);
                }
            }
            if apply || cancel || !open {
                self.language_dialog = None;
            }
        }
    }
}

fn install_theme(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = None;
    visuals.panel_fill = egui::Color32::from_rgb(10, 22, 33);
    visuals.window_fill = egui::Color32::from_rgb(17, 33, 46);
    visuals.extreme_bg_color = egui::Color32::from_rgb(9, 21, 32);
    visuals.slider_trailing_fill = true;
    visuals.selection.bg_fill = egui::Color32::from_rgb(30, 112, 225);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(25, 44, 62);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(35, 69, 100);
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
    ] {
        widget.corner_radius = egui::CornerRadius::same(6);
        widget.bg_stroke = egui::Stroke::new(1_f32, egui::Color32::from_rgb(45, 70, 93));
        widget.fg_stroke.color = egui::Color32::from_rgb(230, 239, 252);
    }
    visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(14, 29, 42);
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(27, 58, 86);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(48, 145, 250);
    visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(30, 90, 160);
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(10., 7.);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(15.));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(15.));
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(21., egui::FontFamily::Name("semibold".into())),
        );
        style.spacing.button_padding = egui::vec2(14., 8.);
    });
}
fn panel(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(17, 33, 46))
        .corner_radius(12.)
        .stroke(egui::Stroke::new(
            1_f32,
            egui::Color32::from_rgb(33, 56, 75),
        ))
        .inner_margin(14.)
        .show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 2.).max(0.));
            if !title.is_empty() {
                ui.heading(
                    egui::RichText::new(title).color(egui::Color32::from_rgb(237, 244, 255)),
                );
            }
            ui.add_space(8.);
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), body);
        });
}
fn device_picker_only(
    ui: &mut egui::Ui,
    id: &str,
    devices: &[audio::DeviceChoice],
    selected: &mut usize,
    channel: &mut usize,
) {
    egui::ComboBox::from_id_salt(id)
        .width((ui.available_width() - 8.).max(80.))
        .truncate()
        .selected_text(
            devices
                .get(*selected)
                .map(|d| d.name.as_str())
                .unwrap_or("No device"),
        )
        .show_ui(ui, |ui| {
            for (i, d) in devices.iter().enumerate() {
                if ui.selectable_value(selected, i, &d.name).changed() {
                    *channel = 1;
                }
            }
        });
}
fn language_display(language: Language) -> (&'static str, &'static str) {
    let entry = info(language);
    let name = match language {
        Language::German => "German",
        Language::English => "English",
        _ => entry.name,
    };
    (name, entry.native_name)
}
fn language_flag(ui: &mut egui::Ui, language: Language) {
    use egui::{Color32 as C, Rect, Stroke};
    let (r, _) = ui.allocate_exact_size(egui::vec2(32., 22.), egui::Sense::hover());
    let painter = ui.painter();
    let red = C::from_rgb(220, 30, 45);
    let blue = C::from_rgb(35, 70, 180);
    let colors = match language {
        Language::German => [C::BLACK, red, C::from_rgb(255, 205, 35)],
        Language::Russian => [C::WHITE, blue, red],
        Language::Japanese => [C::WHITE; 3],
        Language::Arabic => [C::BLACK, C::WHITE, C::from_rgb(0, 122, 61)],
        Language::Hindi => [C::from_rgb(255, 153, 51), C::WHITE, C::from_rgb(19, 136, 8)],
        Language::French => [blue, C::WHITE, red],
        Language::Italian => [C::from_rgb(0, 145, 75), C::WHITE, red],
        Language::Spanish => [red, C::from_rgb(255, 205, 35), red],
        Language::Mandarin | Language::Basque => [red; 3],
        _ => [C::from_rgb(30, 54, 72); 3],
    };
    for (i, color) in colors.iter().enumerate() {
        let vertical = matches!(language, Language::French | Language::Italian);
        let (offset, fraction) = if language == Language::Spanish {
            ([0., 0.25, 0.75][i], [0.25, 0.5, 0.25][i])
        } else {
            (i as f32 / 3., 1. / 3.)
        };
        let stripe = if vertical {
            Rect::from_min_size(
                r.left_top() + egui::vec2(offset * r.width(), 0.),
                egui::vec2(fraction * r.width(), r.height()),
            )
        } else {
            Rect::from_min_size(
                r.left_top() + egui::vec2(0., offset * r.height()),
                egui::vec2(r.width(), fraction * r.height()),
            )
        };
        painter.rect_filled(stripe, 0., *color);
    }
    if language == Language::Arabic {
        painter.add(egui::Shape::convex_polygon(
            vec![
                r.left_top(),
                r.left_bottom(),
                egui::pos2(r.left() + r.width() * 0.45, r.center().y),
            ],
            red,
            egui::Stroke::NONE,
        ));
        let center = egui::pos2(r.left() + r.width() * 0.15, r.center().y);
        let radius = r.height() * 0.11;
        let star = (0..14)
            .map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 14. - std::f32::consts::FRAC_PI_2;
                center
                    + egui::vec2(angle.cos(), angle.sin())
                        * radius
                        * if i % 2 == 0 { 1. } else { 0.45 }
            })
            .collect::<Vec<_>>();
        for i in 0..14 {
            painter.add(egui::Shape::convex_polygon(
                vec![center, star[i], star[(i + 1) % 14]],
                C::WHITE,
                egui::Stroke::NONE,
            ));
        }
    } else if language == Language::Hindi {
        let center = r.center();
        let radius = r.height() * 0.14;
        painter.circle_stroke(center, radius, Stroke::new(0.8_f32, blue));
        for i in 0..24 {
            let angle = i as f32 * std::f32::consts::TAU / 24.;
            painter.line_segment(
                [
                    center,
                    center + egui::vec2(angle.cos(), angle.sin()) * radius,
                ],
                Stroke::new(0.3_f32, blue),
            );
        }
    } else if language == Language::Japanese {
        painter.circle_filled(r.center(), r.height() * 0.3, red);
    } else if language == Language::Basque {
        let green = Stroke::new(4_f32, C::from_rgb(0, 140, 70));
        painter.line_segment([r.left_top(), r.right_bottom()], green);
        painter.line_segment([r.right_top(), r.left_bottom()], green);
        painter.line_segment(
            [
                egui::pos2(r.center().x, r.top()),
                egui::pos2(r.center().x, r.bottom()),
            ],
            Stroke::new(3_f32, C::WHITE),
        );
        painter.line_segment(
            [
                egui::pos2(r.left(), r.center().y),
                egui::pos2(r.right(), r.center().y),
            ],
            Stroke::new(3_f32, C::WHITE),
        );
    } else if language == Language::Mandarin {
        painter.text(
            r.left_top() + egui::vec2(3., 0.),
            egui::Align2::LEFT_TOP,
            "★",
            egui::FontId::proportional(12.),
            C::YELLOW,
        );
    } else if !matches!(
        language,
        Language::Spanish
            | Language::German
            | Language::French
            | Language::Italian
            | Language::Russian
    ) {
        painter.text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            info(language).text_code,
            egui::FontId::proportional(10.),
            C::WHITE,
        );
    }
}
#[allow(clippy::too_many_arguments)]
fn output_row(
    ui: &mut egui::Ui,
    id: &str,
    row_number: usize,
    devices: &[audio::DeviceChoice],
    selected: &mut usize,
    channel: &mut usize,
    language: &mut Language,
    volume: &mut f32,
    muted: &mut bool,
    level: &AudioLevel,
    running: bool,
) -> u8 {
    let mut action = 0;
    ui.push_id(id, |ui| {
        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(20, 36, 49))
            .corner_radius(10.)
            .inner_margin(10.)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 7.;
                let available = ui.available_width();
                let compact = available < 720.;
                let name_width = if compact { 76. } else { 88. };
                let volume_width = if compact { 140. } else { 160. };
                let device_width = (available * 0.31).clamp(150., 250.);
                let meter_width = (available
                    - (24. + 32. + name_width + device_width + volume_width + 42. + 26. + 49.))
                    .max(80.);
                ui.horizontal(|ui| {
                    stream_cell(ui, 24., |ui| {
                        let (r, _) =
                            ui.allocate_exact_size(egui::vec2(24., 32.), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(r, 5., egui::Color32::from_rgb(32, 53, 72));
                        ui.painter().text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            row_number.to_string(),
                            egui::FontId::proportional(17.),
                            egui::Color32::LIGHT_BLUE,
                        );
                    });
                    stream_cell(ui, 32., |ui| language_flag(ui, *language));
                    stream_cell(ui, name_width, |ui| {
                        let (name, native) = language_display(*language);
                        ui.add(egui::Label::new(egui::RichText::new(name).strong()).wrap());
                        ui.label(
                            egui::RichText::new(native)
                                .size(13.)
                                .color(egui::Color32::from_rgb(160, 187, 218)),
                        );
                    });
                    stream_cell(ui, device_width, |ui| {
                        ui.label(egui::RichText::new("Output Device").size(13.));
                        ui.add_enabled_ui(!running, |ui| {
                            device_picker_only(ui, "device", devices, selected, channel)
                        });
                    });
                    stream_cell(ui, volume_width, |ui| {
                        ui.label(egui::RichText::new("Volume").size(13.));
                        ui.horizontal(|ui| {
                            speaker_icon(ui);
                            ui.style_mut().spacing.slider_width = volume_width - 18. - 36. - 14.;
                            blue_slider(ui, volume, 0.0..=100.0);
                            ui.add_sized(
                                egui::vec2(36., 20.),
                                egui::Label::new(
                                    egui::RichText::new(format!("{volume:.0}%")).size(13.),
                                ),
                            );
                        });
                    });
                    stream_cell(ui, 42., |ui| {
                        ui.label(egui::RichText::new("Mute").size(13.));
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(38., 22.), egui::Sense::click());
                        if response.clicked() {
                            *muted = !*muted;
                        }
                        ui.painter().rect_filled(
                            rect,
                            11.,
                            if *muted {
                                egui::Color32::from_rgb(20, 200, 105)
                            } else {
                                egui::Color32::from_rgb(55, 69, 85)
                            },
                        );
                        let x = if *muted {
                            rect.right() - 11.
                        } else {
                            rect.left() + 11.
                        };
                        ui.painter().circle_filled(
                            egui::pos2(x, rect.center().y),
                            8.,
                            egui::Color32::WHITE,
                        );
                        response.on_hover_text(if *muted {
                            "Muted — click to unmute"
                        } else {
                            "Output enabled — click to mute"
                        });
                    });
                    stream_cell(ui, meter_width, |ui| {
                        audio_meter(ui, "Output Level", level);
                    });
                    stream_cell(ui, 26., |ui| {
                        ui.add_enabled_ui(!running, |ui| {
                            let menu = ui.menu_button(" ", |ui| {
                                if ui.button("Modify language…").clicked() {
                                    action = 1;
                                    ui.close_menu();
                                }
                                if ui.button("Remove stream").clicked() {
                                    action = 2;
                                    ui.close_menu();
                                }
                                ui.separator();
                                ui.label("Output channel routing");
                                let max = devices
                                    .get(*selected)
                                    .map(|d| d.channels as usize)
                                    .unwrap_or(1);
                                ui.add(
                                    egui::DragValue::new(channel)
                                        .range(1..=max)
                                        .prefix("Channel "),
                                );
                            });
                            let center = menu.response.rect.center();
                            for offset in [-5., 0., 5.] {
                                ui.painter().circle_filled(
                                    center + egui::vec2(0., offset),
                                    1.4,
                                    egui::Color32::LIGHT_BLUE,
                                );
                            }
                            menu.response.on_hover_text("Language and channel routing");
                        });
                    });
                });
            });
        level.set_volume(*volume / 100.);
        level.set_muted(*muted);
    });
    action
}
fn stream_cell(ui: &mut egui::Ui, width: f32, body: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 56.),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_min_width(width);
            ui.set_max_width(width);
            body(ui);
        },
    );
}
fn globe_icon(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(24., 26.), egui::Sense::hover());
    let stroke = egui::Stroke::new(2_f32, egui::Color32::from_rgb(55, 155, 255));
    ui.painter().circle_stroke(rect.center(), 10., stroke);
    ui.painter().line_segment(
        [
            rect.center() - egui::vec2(10., 0.),
            rect.center() + egui::vec2(10., 0.),
        ],
        stroke,
    );
    ui.painter().line_segment(
        [
            rect.center() - egui::vec2(0., 10.),
            rect.center() + egui::vec2(0., 10.),
        ],
        stroke,
    );
}

fn speaker_icon(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(18., 20.), egui::Sense::hover());
    let origin = rect.left_top();
    let color = egui::Color32::from_rgb(173, 203, 235);
    let points = [
        (1., 7.),
        (5., 7.),
        (10., 3.),
        (10., 17.),
        (5., 13.),
        (1., 13.),
    ]
    .iter()
    .map(|(x, y)| origin + egui::vec2(*x, *y))
    .collect();
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::NONE,
    ));
    ui.painter().add(egui::Shape::line(
        vec![
            origin + egui::vec2(13., 5.),
            origin + egui::vec2(16., 10.),
            origin + egui::vec2(13., 15.),
        ],
        egui::Stroke::new(1.5_f32, color),
    ));
}
fn stream_color(index: usize) -> egui::Color32 {
    let palette = [
        (255, 155, 50),
        (75, 175, 255),
        (45, 220, 130),
        (195, 125, 255),
        (255, 100, 145),
        (70, 220, 225),
        (235, 215, 75),
        (190, 200, 220),
    ];
    let (r, g, b) = palette[index % palette.len()];
    egui::Color32::from_rgb(r, g, b)
}
fn blue_slider(ui: &mut egui::Ui, value: &mut f32, range: std::ops::RangeInclusive<f32>) {
    ui.scope(|ui| {
        ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(55, 153, 255);
        ui.add(egui::Slider::new(value, range).show_value(false));
    });
}
fn waveform_logo(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(58., 50.), egui::Sense::hover());
    for (i, height) in [10., 24., 40., 28., 14., 8.].iter().enumerate() {
        let x = rect.left() + 6. + i as f32 * 8.;
        ui.painter().line_segment(
            [
                egui::pos2(x, rect.center().y - height / 2.),
                egui::pos2(x, rect.center().y + height / 2.),
            ],
            egui::Stroke::new(4_f32, egui::Color32::from_rgb(58, 157, 255)),
        );
    }
}
fn plot_grid(ui: &mut egui::Ui, height: f32) -> egui::Rect {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    ui.painter()
        .rect_filled(rect, 6., egui::Color32::from_rgb(12, 27, 39));
    for i in 0..=8 {
        let x = rect.left() + rect.width() * i as f32 / 8.;
        ui.painter().line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1_f32, egui::Color32::from_rgb(25, 45, 61)),
        );
    }
    for i in 0..=4 {
        let y = rect.top() + rect.height() * i as f32 / 4.;
        ui.painter().line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1_f32, egui::Color32::from_rgb(25, 45, 61)),
        );
    }
    rect
}
fn metric_card(ui: &mut egui::Ui, title: &str, value: &str, color: egui::Color32, values: &[f32]) {
    egui::Frame::group(ui.style())
        .corner_radius(10.)
        .inner_margin(12.)
        .show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 2.).max(0.));
            ui.colored_label(color, title);
            ui.label(
                egui::RichText::new(value)
                    .size(if title == "Translation Delay" {
                        34.
                    } else {
                        24.
                    })
                    .strong()
                    .color(egui::Color32::WHITE),
            );
            if title == "Translation Delay" {
                return;
            }
            let rect = plot_grid(ui, 48.);
            let scale = if title == "PolyStream Memory" {
                values.iter().copied().fold(1., f32::max) * 1.1
            } else {
                100.
            };
            if values.len() > 1 {
                for (i, pair) in values.windows(2).enumerate() {
                    let point = |index: usize, value: f32| {
                        egui::pos2(
                            rect.left() + rect.width() * index as f32 / (values.len() - 1) as f32,
                            rect.bottom() - value.clamp(0., scale) / scale * rect.height(),
                        )
                    };
                    ui.painter().line_segment(
                        [point(i, pair[0]), point(i + 1, pair[1])],
                        egui::Stroke::new(1.5_f32, color),
                    );
                }
            }
        });
}

fn default_project_directory() -> String {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute() && path.is_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .to_string_lossy()
        .into_owned()
}
#[allow(clippy::too_many_arguments)]
fn read_capacity(
    config: &str,
    mut languages: Vec<&str>,
    device: &str,
    threads: u32,
    slots: u32,
    context: u32,
    layers: u32,
    gpu_identity: Option<&str>,
) -> String {
    let path = std::path::Path::new(config)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("capacity-profile.json");
    let Ok(bytes) = std::fs::read(path) else {
        return "Delay estimate: calibration pending on this PC. Live delay appears after Start."
            .into();
    };
    let Ok(profile) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return "Delay estimate: invalid calibration profile.".into();
    };
    if profile["machine"].as_str() != std::env::var("COMPUTERNAME").ok().as_deref()
        || profile["device"].as_str() != Some(device)
        || profile["threads"].as_u64() != Some(threads as u64)
        || profile["slots"].as_u64() != Some(slots as u64)
        || profile["context"].as_u64() != Some(context as u64)
        || profile["layers"].as_u64() != Some(layers as u64)
    {
        return "Delay estimate: hardware/engine settings differ from calibration; benchmark again.".into();
    }
    if gpu_identity.is_none() {
        return "Delay estimate: checking GPU identity.".into();
    }
    if profile["gpu_identity"].as_str() != gpu_identity
        || profile["logical_processors"].as_u64()
            != std::thread::available_parallelism()
                .ok()
                .map(|n| n.get() as u64)
    {
        return "Delay estimate: hardware changed; recalibrate this PC.".into();
    }
    // Profiles are read-only benchmark evidence. No predictions for changed models/voices.
    let Ok(config_bytes) = std::fs::read(config) else {
        return "Delay estimate: configuration unavailable.".into();
    };
    let Ok(current) = serde_json::from_slice::<serde_json::Value>(&config_bytes) else {
        return "Delay estimate: configuration unavailable.".into();
    };
    if profile["translation_model"] != current["translation_model"]
        || profile["whisper_model"] != current["whisper_model"]
    {
        return "Delay estimate: model changed; benchmark again.".into();
    }
    if let Some(signatures) = profile["file_signatures"].as_array() {
        let root = std::path::Path::new(config)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        for signature in signatures {
            let key = signature["key"].as_str().unwrap_or("");
            let expected = signature["path"].as_str().unwrap_or("");
            let configured = if key == "translation_model" || key == "whisper_model" {
                current[key].as_str()
            } else {
                current["voices"][key].as_str()
            };
            if let Some(actual) = configured {
                if std::path::Path::new(actual).file_name()
                    != std::path::Path::new(expected).file_name()
                {
                    return "Delay estimate: model/voice selection changed; recalibrate.".into();
                }
            }
            let path = root.join(configured.unwrap_or(expected));
            let Ok(metadata) = std::fs::metadata(path) else {
                return "Delay estimate: calibration model/voice missing.".into();
            };
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|t| t.as_secs());
            if signature["bytes"].as_u64() != Some(metadata.len())
                || signature["modified"].as_u64() != modified
            {
                return "Delay estimate: model/voice files changed; recalibrate.".into();
            }
        }
    }
    if let Some(validation) = profile.get("recovery_validation") {
        let mut selected = languages.clone();
        selected.sort_unstable();
        let mut tested = validation["languages"]
            .as_array()
            .map(|v| {
                v.iter()
                    .filter_map(|code| code.as_str())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        tested.sort_unstable();
        if selected == tested {
            return "Latest short eight-language test completed: 303 audio deliveries, 1 untranslated clause; maximum delivered delay 28.2 s. Historical result, not a sustained estimate.".into();
        }
    }
    if let Some(failures) = profile["failed_language_sets"].as_array() {
        let mut selected = languages.clone();
        selected.sort_unstable();
        for failure in failures {
            let mut tested = failure["languages"]
                .as_array()
                .map(|v| {
                    v.iter()
                        .filter_map(|code| code.as_str())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            tested.sort_unstable();
            if selected == tested {
                return "Previous test of these eight languages stopped on an incomplete translation. No validated delay estimate; the full session was not measured.".into();
            }
        }
    }
    if languages.iter().any(|code| {
        current["piper_python_voices"].get(*code).is_some()
            || matches!(*code, "it" | "ja" | "hi" | "ar")
    }) {
        return "Delay estimate: this optional Piper runtime is not calibrated; measure the selected combination.".into();
    }
    languages.sort_unstable();
    if let Some(cases) = profile["cases"].as_array() {
        for case in cases {
            let mut tested = case["languages"]
                .as_array()
                .map(|v| v.iter().filter_map(|l| l.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            tested.sort_unstable();
            if tested == languages {
                return format!("Measured short replay: typical {:.1} s; p95 {:.1} s; max {:.1} s. Sustained meetings may take longer.",case["median"].as_f64().unwrap_or(0.),case["p95"].as_f64().unwrap_or(0.),case["max"].as_f64().unwrap_or(0.));
            }
        }
    }
    if let Some(projection) = profile["projections"].get(languages.len().to_string()) {
        if let (Some(low), Some(high)) = (projection["low"].as_f64(), projection["high"].as_f64()) {
            return format!("Low-confidence count projection: {low:.1}–{high:.1} s. Untested languages/count; similar speech assumed. Recalibrate after changes; backlog may grow continuously.");
        }
    }
    "Delay estimate: this language combination is not calibrated. More streams can create a growing backlog.".into()
}
fn env_number(name: &str, default: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
fn main() -> eframe::Result<()> {
    let result = eframe::run_native(
        "PolyStream",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1280.0, 940.0])
                .with_min_inner_size([900.0, 700.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            fonts::install(&cc.egui_ctx);
            install_theme(&cc.egui_ctx);
            Ok(Box::new(App::default()))
        }),
    );
    if RESTART_ENGINES.load(Ordering::Acquire) {
        std::process::exit(75);
    }
    result
}

#[cfg(test)]
mod stream_tests {
    use super::*;
    #[test]
    fn removal_preserves_remaining_eight_stream_controls() {
        let mut app = App::default();
        app.streams.clear();
        for (index, language) in [
            Language::Spanish,
            Language::German,
            Language::Mandarin,
            Language::French,
            Language::Japanese,
            Language::Russian,
            Language::Arabic,
            Language::Hindi,
        ]
        .into_iter()
        .enumerate()
        {
            assert!(app.add_stream(language));
            let stream = app.streams.last_mut().unwrap();
            stream.device = index;
            stream.channel = index + 1;
            stream.volume = 40. + index as f32;
            stream.muted = index % 2 == 0;
        }
        assert!(!app.add_stream(Language::Basque));
        let old = app.streams[1..].to_vec();
        app.remove_stream(0);
        for (actual, expected) in app.streams.iter().zip(old) {
            assert_eq!(actual.language, expected.language);
            assert_eq!(actual.device, expected.device);
            assert_eq!(actual.channel, expected.channel);
            assert_eq!(actual.volume, expected.volume);
            assert_eq!(actual.muted, expected.muted);
            assert!(Arc::ptr_eq(&actual.level, &expected.level));
        }
        while !app.streams.is_empty() {
            app.remove_stream(0);
        }
        assert!(!app.start());
        assert!(app.add_stream(Language::German));
    }
}
