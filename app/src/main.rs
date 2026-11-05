// Craft also launches debug builds; every Windows build should open only the app.
#![cfg_attr(windows, windows_subsystem = "windows")]
mod canvas;
mod credentials;
#[cfg(windows)]
mod elevation;
mod fonts;
#[cfg(windows)]
mod hwinfo;
mod legacy;
mod lookup;
mod model;
mod original;
mod platform;
mod render;
mod runtime;
mod sensors;
#[cfg(windows)]
mod single_instance;
mod stocks;
use base64::{engine::general_purpose::STANDARD, Engine};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use model::*;
use runtime::{Command, Runtime};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const MUTED: Color32 = Color32::from_rgb(116, 133, 138);
const ACCENT: Color32 = Color32::from_rgb(29, 125, 104);
const BORDER: Color32 = Color32::from_rgb(222, 230, 230);
fn page_fill(ctx: &egui::Context) -> Color32 {
    if ctx.style().visuals.dark_mode {
        ctx.style().visuals.panel_fill
    } else {
        Color32::WHITE
    }
}
fn heading(ui: &mut egui::Ui, title: &str) {
    ui.add_space(14.);
    ui.label(RichText::new(title).size(11.).strong().color(MUTED));
    ui.add_space(8.);
}
fn action(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add_sized([82., 32.], egui::Button::new(label).corner_radius(6.))
}
fn primary(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(Color32::WHITE))
            .fill(ACCENT)
            .corner_radius(6.)
            .min_size(Vec2::new(135., 34.)),
    )
}
fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(RichText::new(label).size(12.).color(MUTED));
    ui.add_sized(
        [ui.available_width(), 29.],
        egui::TextEdit::singleline(value),
    );
    ui.add_space(8.);
}
fn signature(p: &Project) -> Vec<u8> {
    let mut b = serde_json::to_vec(p).unwrap_or_default();
    for s in &p.sources {
        b.extend(s.token.bytes());
        b.extend(s.username.bytes());
        b.extend(s.password.bytes());
    }
    b
}
#[derive(Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
enum EditorTheme {
    #[default]
    System,
    Light,
    Dark,
}
impl EditorTheme {
    fn apply(self, ctx: &egui::Context) {
        ctx.set_theme(match self {
            Self::System => egui::ThemePreference::System,
            Self::Light => egui::ThemePreference::Light,
            Self::Dark => egui::ThemePreference::Dark,
        });
    }
}
struct DesignBrowser {
    folder: PathBuf,
    files: Vec<PathBuf>,
    selected: Option<PathBuf>,
    project: Option<Project>,
    texture: Option<egui::TextureHandle>,
    error: String,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct Session {
    project: Project,
    path: Option<PathBuf>,
    port: String,
    stream: bool,
    #[serde(default)]
    theme: EditorTheme,
}

struct Studio {
    project: Project,
    path: Option<PathBuf>,
    dirty: bool,
    selected: canvas::Selection,
    tab: usize,
    runtime: Runtime,
    data: DataState,
    texture: Option<egui::TextureHandle>,
    refresh_texture: bool,
    frame_cache: render::FrameCache,
    screen_pixels: Option<image::RgbaImage>,
    message: String,
    palette_tab: bool,
    search: String,
    source_search: String,
    payload: Option<(Kind, String)>,
    drag_origin: Option<canvas::Gesture>,
    undo: Vec<Project>,
    redo: Vec<Project>,
    last_checkpoint: Instant,
    port: String,
    ports: Vec<String>,
    stream: bool,
    tray: Option<platform::Tray>,
    tray_error: String,
    lookup: lookup::Lookup,
    binding_search: String,
    zoom: f32,
    confirm: Option<String>,
    pending: Option<String>,
    quit: bool,
    screenshot: Option<PathBuf>,
    frames: u32,
    screenshot_requested: bool,
    new_dialog: bool,
    new_name: String,
    new_portrait: bool,
    new_template: usize,
    source_index: Option<usize>,
    last_autosave: Instant,
    history_navigation: bool,
    restore_frames: u8,
    theme: EditorTheme,
    browser: Option<DesignBrowser>,
    sensor_help: bool,
}
impl Studio {
    fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<PathBuf>) -> Self {
        let ctx = &cc.egui_ctx;
        let mut fonts = egui::FontDefinitions::default();
        if let Ok(data) = std::fs::read("C:/Windows/Fonts/seguisym.ttf") {
            fonts.font_data.insert(
                "windows-symbols".into(),
                egui::FontData::from_owned(data).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("windows-symbols".into());
            ctx.set_fonts(fonts);
        }
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(8., 8.);
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(13.));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(13.));
        ctx.set_style(style);
        let session = if screenshot.is_none() {
            std::fs::read(Self::session_path()).ok().and_then(|b| {
                serde_json::from_slice::<Session>(&b).ok().or_else(|| {
                    serde_json::from_slice::<Project>(&b)
                        .ok()
                        .map(|project| Session {
                            project,
                            path: None,
                            port: String::new(),
                            stream: true,
                            theme: EditorTheme::System,
                        })
                })
            })
        } else {
            None
        };
        let Session {
            mut project,
            path,
            port,
            mut stream,
            mut theme,
        } = session.unwrap_or_else(|| Session {
            project: Project::landscape_technology(),
            path: None,
            port: String::new(),
            stream: true,
            theme: EditorTheme::System,
        });
        // Isolated screenshot fixtures never read or overwrite the person's session.
        let review = if screenshot.is_some() {
            std::env::var("COCKPIT_SCREENSHOT_VIEW").unwrap_or_default()
        } else {
            String::new()
        };
        if screenshot.is_some() && std::env::var_os("COCKPIT_SCREENSHOT_LIGHT").is_some() {
            theme = EditorTheme::Light;
        }
        if screenshot.is_some() {
            stream = false;
            if let Some(path) = std::env::var_os("COCKPIT_SCREENSHOT_DESIGN") {
                project = Self::read_design(std::path::Path::new(&path))
                    .expect("screenshot design must be readable and valid");
            }
        }
        if review == "weather" || review == "stock" {
            let kind = if review == "weather" {
                SourceKind::Weather
            } else {
                SourceKind::Stock
            };
            let interval = if kind == SourceKind::Stock { 86400 } else { 60 };
            project.sources.push(Source {
                id: 1,
                name: kind.label().into(),
                kind,
                url: String::new(),
                token: String::new(),
                username: String::new(),
                password: String::new(),
                auth: Auth::Bearer,
                credential_id: String::new(),
                interval,
                latitude: "41.9028".into(),
                longitude: "12.4964".into(),
                symbol: String::new(),
                stocks: vec![],
            });
        }
        let mut selection = canvas::Selection::default();
        if ["ticker", "ticker-circular", "multi", "bound", "static"].contains(&review.as_str()) {
            project = Project::blank();
            project.name = "Editor feature review".into();
            let mut text = Widget::new(1, Kind::Text, 30., 45.);
            text.text = "Live dashboard".into();
            text.show_label = false;
            text.w = 340.;
            text.h = 40.;
            let mut ticker = Widget::new(2, Kind::Ticker, 30., 140.);
            ticker.w = 420.;
            ticker.h = 38.;
            ticker.show_label = false;
            ticker.ticker_bound = true;
            ticker.ticker_circular = review == "ticker-circular";
            ticker.ticker_sources = vec![
                TickerSource {
                    binding: "cpu.usage".into(),
                    unit_prefix: "CPU ".into(),
                    unit_suffix: "%".into(),
                    decimals: Some(0),
                },
                TickerSource {
                    binding: "memory.usage".into(),
                    unit_prefix: "RAM ".into(),
                    unit_suffix: "%".into(),
                    decimals: Some(1),
                },
            ];
            let mut value = Widget::new(3, Kind::Value, 30., 230.);
            value.w = 280.;
            value.h = 40.;
            value.show_label = false;
            value.binding = "cpu.usage".into();
            value.unit_prefix = "CPU ".into();
            value.unit = "%".into();
            project.widgets = vec![text, ticker, value];
            match review.as_str() {
                "ticker" | "ticker-circular" => selection.only(2),
                "bound" => selection.only(3),
                "static" => selection.only(1),
                _ => {
                    selection.0.extend([1, 2, 3]);
                }
            }
        }
        theme.apply(ctx);
        Self::sanitize(&mut project);
        let mut credential_error = String::new();
        for source in &mut project.sources {
            if let Err(e) = credentials::restore(source) {
                credential_error = e;
            }
        }
        let runtime = Runtime::start(project.clone(), cc.egui_ctx.clone());
        let _ = runtime.tx.send(Command::Port(port.clone()));
        if stream {
            let _ = runtime.tx.send(Command::Stream(true));
        }
        let (tray, tray_error) = match platform::Tray::new(cc) {
            Ok(t) => (Some(t), String::new()),
            Err(e) => (None, e),
        };
        let ports = serialport::available_ports()
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.port_name)
            .collect();
        let source_index = if project.sources.is_empty() {
            None
        } else {
            Some(0)
        };
        Self {
            project,
            path,
            dirty: false,
            selected: selection,
            tab: if review == "settings" || review == "sensors" {
                2
            } else if review == "weather" || review == "stock" {
                1
            } else {
                0
            },
            runtime,
            data: DataState::default(),
            sensor_help: review == "sensors",
            texture: None,
            refresh_texture: true,
            frame_cache: render::FrameCache::default(),
            screen_pixels: None,
            message: if credential_error.is_empty() {
                "Ready. Drag a widget onto your screen to begin.".into()
            } else {
                credential_error
            },
            palette_tab: false,
            search: String::new(),
            source_search: String::new(),
            payload: None,
            drag_origin: None,
            undo: vec![],
            redo: vec![],
            last_checkpoint: Instant::now() - Duration::from_secs(2),
            port,
            ports,
            stream,
            tray,
            tray_error,
            lookup: lookup::Lookup::default(),
            binding_search: String::new(),
            zoom: 1.25,
            confirm: None,
            pending: None,
            quit: false,
            screenshot,
            frames: 0,
            screenshot_requested: false,
            new_dialog: false,
            new_name: "LandscapeTechnologyCh".into(),
            new_portrait: false,
            new_template: 0,
            source_index,
            last_autosave: Instant::now(),
            history_navigation: false,
            restore_frames: 0,
            theme,
            browser: None,
        }
    }
    fn session_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("cockpit-session.json")))
            .unwrap_or_else(|| PathBuf::from("cockpit-session.json"))
    }
    fn persist(&mut self) {
        if let Err(e) = self.store_credentials() {
            self.message = e;
        }
        let session = Session {
            project: self.project.clone(),
            path: self.path.clone(),
            port: self.port.clone(),
            stream: self.stream,
            theme: self.theme,
        };
        if let Ok(b) = serde_json::to_vec(&session) {
            let _ = std::fs::write(Self::session_path(), b);
        }
    }
    fn store_credentials(&self) -> Result<(), String> {
        for source in &self.project.sources {
            credentials::save(source)?;
        }
        Ok(())
    }
    fn sync(&mut self) {
        let _ = self.runtime.tx.send(Command::Project(self.project.clone()));
        self.refresh_texture = true;
    }
    fn add(&mut self, kind: Kind, binding: String, x: f32, y: f32) {
        let id = self.project.widgets.iter().map(|w| w.id).max().unwrap_or(0) + 1;
        let mut w = Widget::new(id, kind, x, y);
        if !binding.is_empty() {
            w.name = binding
                .split(':')
                .next_back()
                .unwrap_or(&binding)
                .trim_start_matches('/')
                .to_owned();
            w.label = w.name.to_uppercase();
            w.binding = binding;
        }
        if w.kind.is_shape() {
            w.show_label = false;
            w.w = 100.;
            w.h = 70.;
        }
        if w.kind == Kind::StraightLine {
            w.h = 2.;
            w.outline_enabled = true;
            w.outline_color = w.color;
            w.fill_enabled = false;
        }
        if w.kind == Kind::Text || w.kind == Kind::Ticker {
            w.show_label = false;
            w.h = 35.;
        }
        if w.kind == Kind::Ticker {
            w.w = 260.;
        }
        if w.kind == Kind::Ring || w.kind == Kind::Pie {
            w.w = 110.;
            w.h = 125.;
        }
        if w.kind == Kind::Line {
            w.w = 200.;
            w.h = 80.;
        }
        self.project.widgets.push(w);
        self.selected.only(id);
        self.tab = 0;
    }
    fn remove(&mut self) {
        self.project
            .widgets
            .retain(|w| !self.selected.contains(w.id));
        self.selected.clear();
        self.drag_origin = None;
    }
    fn duplicate(&mut self) {
        let copies: Vec<_> = self
            .project
            .widgets
            .iter()
            .filter(|w| self.selected.contains(w.id))
            .cloned()
            .collect();
        let mut next = self.project.widgets.iter().map(|w| w.id).max().unwrap_or(0) + 1;
        self.selected.clear();
        for mut w in copies {
            w.id = next;
            next += 1;
            w.name = format!("{} copy", w.name);
            w.x = (w.x + 8.).min(self.project.dimensions().0 as f32 - w.w);
            w.y = (w.y + 8.).min(self.project.dimensions().1 as f32 - w.h);
            self.selected.0.insert(w.id);
            self.project.widgets.push(w);
        }
    }
    fn request(&mut self, action: &str) {
        if self.dirty {
            self.pending = Some(action.into());
            self.confirm = Some("Save your changes before continuing?".into());
        } else {
            self.perform(action);
        }
    }
    fn perform(&mut self, action: &str) {
        match action {
            "new" => self.new_dialog = true,
            "open" => self.open(),
            "exit" => self.quit = true,
            _ => {}
        }
    }
    fn read_design(path: &std::path::Path) -> Result<Project, String> {
        if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 16_000_000 {
            return Err("Layout exceeds 16 MB".into());
        }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let mut p = if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("data"))
        {
            legacy::import(&bytes)?
        } else {
            serde_json::from_slice::<Project>(&bytes).map_err(|e| e.to_string())?
        };
        Self::sanitize(&mut p);
        Ok(p)
    }
    fn open(&mut self) {
        let folder = self
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| {
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("config")))
                    .filter(|p| p.is_dir())
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            });
        self.browser = Some(DesignBrowser {
            folder,
            files: vec![],
            selected: None,
            project: None,
            texture: None,
            error: String::new(),
        });
        self.scan_designs();
    }
    fn scan_designs(&mut self) {
        if let Some(browser) = &mut self.browser {
            browser.files = std::fs::read_dir(&browser.folder)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.is_file()
                        && p.extension().is_some_and(|e| {
                            e.eq_ignore_ascii_case("json") || e.eq_ignore_ascii_case("data")
                        })
                })
                .collect();
            browser.files.sort();
        }
    }
    fn preview_design(&mut self, path: PathBuf, ctx: &egui::Context) {
        let result = Self::read_design(&path);
        if let Some(browser) = &mut self.browser {
            browser.selected = Some(path);
            browser.project = None;
            browser.texture = None;
            browser.error.clear();
            match result {
                Ok(project) => {
                    let img = render::frame(&project, &self.data);
                    browser.texture = Some(ctx.load_texture(
                        "design_preview",
                        egui::ColorImage::from_rgba_unmultiplied(
                            [img.width() as usize, img.height() as usize],
                            img.as_raw(),
                        ),
                        egui::TextureOptions::LINEAR,
                    ));
                    browser.project = Some(project);
                }
                Err(e) => browser.error = e,
            }
        }
    }
    fn design_browser(&mut self, ctx: &egui::Context) {
        let Some(browser) = &mut self.browser else {
            return;
        };
        let mut open = true;
        let mut chosen = None;
        let mut folder = false;
        let mut browse = false;
        let mut load = false;
        let mut cancel = false;
        egui::Window::new("Open a design")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(810.)
            .show(ctx, |ui| {
                ui.label(browser.folder.display().to_string());
                ui.horizontal(|ui| {
                    folder = ui.button("Choose folder…").clicked();
                    browse = ui.button("Browse files…").clicked();
                });
                ui.separator();
                ui.columns(2, |columns| {
                    egui::ScrollArea::vertical()
                        .max_height(420.)
                        .show(&mut columns[0], |ui| {
                            if browser.files.is_empty() {
                                ui.label("No JSON or legacy .data layouts in this folder.");
                            }
                            for path in &browser.files {
                                let selected = browser.selected.as_ref() == Some(path);
                                if ui
                                    .selectable_label(
                                        selected,
                                        path.file_name().unwrap_or_default().to_string_lossy(),
                                    )
                                    .clicked()
                                {
                                    chosen = Some(path.clone());
                                }
                            }
                        });
                    let ui = &mut columns[1];
                    if let (Some(project), Some(texture)) = (&browser.project, &browser.texture) {
                        ui.label(RichText::new(&project.name).strong());
                        let (w, h) = project.dimensions();
                        ui.label(format!("{} × {} • {} layers", w, h, project.widgets.len()));
                        ui.add(
                            egui::Image::new(texture)
                                .fit_to_exact_size(Vec2::new(w as f32, h as f32) * 0.7),
                        );
                        ui.label("Live fields use current readings where available.");
                    } else if browser.error.is_empty() {
                        ui.label("Select a design to preview it before opening.");
                    }
                    if !browser.error.is_empty() {
                        ui.colored_label(Color32::from_rgb(210, 80, 70), &browser.error);
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    load = ui
                        .add_enabled(
                            browser.project.is_some(),
                            egui::Button::new("Open selected design"),
                        )
                        .clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if folder {
            if let Some(path) = rfd::FileDialog::new()
                .set_directory(&browser.folder)
                .pick_folder()
            {
                browser.folder = path;
                browser.project = None;
                browser.texture = None;
                browser.selected = None;
                self.scan_designs();
            }
        }
        if browse {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Cockpit layouts", &["json", "data"])
                .pick_file()
            {
                if let Some(browser) = &mut self.browser {
                    browser.folder = path
                        .parent()
                        .unwrap_or(std::path::Path::new("."))
                        .to_path_buf();
                }
                self.scan_designs();
                chosen = Some(path);
            }
        }
        if let Some(path) = chosen {
            self.preview_design(path, ctx);
        }
        if load {
            if let Some(mut browser) = self.browser.take() {
                if let Some(mut project) = browser.project.take() {
                    for source in &mut project.sources {
                        if let Err(e) = credentials::restore(source) {
                            self.message = e;
                        }
                    }
                    self.project = project;
                    self.path = browser.selected;
                    self.dirty = false;
                    self.selected.clear();
                    self.drag_origin = None;
                    self.undo.clear();
                    self.redo.clear();
                    self.sync();
                }
            }
        } else if !open || cancel {
            self.browser = None;
        }
    }
    fn sanitize(p: &mut Project) {
        for source in &mut p.sources {
            credentials::ensure_id(source);
        }
        p.wallpaper_rotation %= 4;
        p.mirror_mode = false;
        p.display_v2 = false;
        for source in &mut p.sources {
            if source.kind == SourceKind::Stock
                && source.stocks.is_empty()
                && !source.symbol.trim().is_empty()
            {
                source.stocks = source.stock_symbols();
                source.symbol.clear();
            }
        }
        if !SERIAL_SPEEDS.contains(&p.serial_baud) {
            p.serial_baud = SERIAL_BAUD;
        }
        p.grid_size = p.grid_size.clamp(2., 64.);
        let (width, height) = p.dimensions();
        p.widgets.truncate(500);
        let mut ids = std::collections::BTreeSet::new();
        let mut next = p.widgets.iter().map(|w| w.id).max().unwrap_or(0) + 1;
        for w in p.widgets.iter_mut() {
            if !ids.insert(w.id) {
                w.id = next;
                next += 1;
                ids.insert(w.id);
            }
            for v in [&mut w.x, &mut w.y, &mut w.w, &mut w.h, &mut w.size] {
                if !v.is_finite() {
                    *v = 0.;
                }
            }
            w.w = w.w.clamp(1., 2048.);
            w.h = w.h.clamp(1., 2048.);
            w.x = w.x.clamp(-2048., width as f32);
            w.y = w.y.clamp(-2048., height as f32);
            w.size = w.size.clamp(6., 150.);
            w.scroll_speed = if w.scroll_speed.is_finite() {
                w.scroll_speed.clamp(0., 10.)
            } else {
                5.
            };
            w.ticker_sources.truncate(100);
            for source in &mut w.ticker_sources {
                source.decimals = source.decimals.map(|n| n.min(6));
            }
            if let Some(metrics) = &mut w.legacy_text {
                if !metrics.base_size.is_finite() || metrics.base_size <= 0. {
                    metrics.base_size = w.size;
                }
                metrics.cell_width = metrics.cell_width.clamp(1., 512.);
                metrics.cell_height = metrics.cell_height.clamp(1., 512.);
                metrics.advance = metrics.advance.clamp(1., 512.);
            }
            w.letter_spacing = if w.letter_spacing.is_finite() {
                w.letter_spacing.clamp(-10., 40.)
            } else {
                0.
            };
            if !w.rotation_degrees.is_finite() {
                w.rotation_degrees = 0.;
            }
            w.rotation_degrees %= 360.;
            w.stroke_width = if w.stroke_width.is_finite() {
                w.stroke_width.clamp(1., 100.)
            } else {
                1.
            };
            w.sides = w.sides.clamp(3, 12);
            if !w.corner_radius.is_finite() {
                w.corner_radius = 12.;
            }
        }
    }
    fn save(&mut self, as_new: bool) -> bool {
        if let Err(e) = self.store_credentials() {
            self.message = e;
            return false;
        }
        let path = if !as_new {
            self.path
                .clone()
                .filter(|p| p.extension().is_some_and(|e| e == "json"))
        } else {
            None
        }
        .or_else(|| {
            rfd::FileDialog::new()
                .add_filter("Cockpit JSON", &["json"])
                .set_file_name(format!("{}.json", self.project.name))
                .save_file()
        });
        if let Some(mut path) = path {
            path.set_extension("json");
            let result = serde_json::to_vec_pretty(&self.project)
                .map_err(|e| e.to_string())
                .and_then(|b| std::fs::write(&path, b).map_err(|e| e.to_string()));
            match result {
                Ok(()) => {
                    self.path = Some(path);
                    self.dirty = false;
                    self.message="Saved. Live data bindings and embedded images are included; credentials are saved in Windows Credential Manager.".into();
                    true
                }
                Err(e) => {
                    self.message = format!("Save failed: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }
    fn export_legacy(&mut self) {
        if let Some(mut path) = rfd::FileDialog::new()
            .add_filter("Legacy screen theme", &["data"])
            .set_file_name(format!("{}.data", self.project.name))
            .save_file()
        {
            path.set_extension("data");
            self.message=match legacy::export(&self.project,&self.data).and_then(|b|std::fs::write(path,b).map_err(|e|e.to_string())){Ok(())=>"Legacy snapshot exported. Open it here to restore editable live widgets; the older app displays the captured image.".into(),Err(e)=>format!("Export failed: {}",e)};
        }
    }
    fn image_file() -> Option<String> {
        rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "gif", "webp"])
            .pick_file()
            .and_then(|p| std::fs::read(p).ok())
            .filter(|b| b.len() <= 8_000_000)
            .and_then(|b| {
                image::load_from_memory(&b).ok().and_then(|img| {
                    render::png(&image::imageops::resize(
                        &img.to_rgba8(),
                        img.width().min(1920),
                        img.height().min(1920),
                        image::imageops::FilterType::Lanczos3,
                    ))
                    .ok()
                })
            })
            .map(|b| STANDARD.encode(b))
    }
    fn export_png(&mut self) {
        if let Some(mut p) = rfd::FileDialog::new()
            .add_filter("PNG image", &["png"])
            .set_file_name("cockpit.png")
            .save_file()
        {
            p.set_extension("png");
            self.message = match render::frame(&self.project, &self.data).save(p) {
                Ok(()) => "Screen image exported at the exact screen resolution.".into(),
                Err(e) => format!("Export failed: {}", e),
            };
        }
    }
    fn set_port(&mut self) {
        let _ = self.runtime.tx.send(Command::Port(self.port.clone()));
    }
    fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("brand").exact_height(66.).frame(egui::Frame::new().fill(ctx.style().visuals.panel_fill).inner_margin(egui::Margin::symmetric(22,12))).show(ctx,|ui|{
            ui.horizontal_centered(|ui|{
                let (r,_)=ui.allocate_exact_size(Vec2::splat(33.),Sense::hover());ui.painter().rect_filled(r,8.,ACCENT);ui.painter().text(r.center(),Align2::CENTER_CENTER,"uM",FontId::proportional(20.),Color32::WHITE);
                ui.add_space(4.);ui.label(RichText::new("uMonitor").size(20.).strong());ui.add_space(24.);
                for (i,label)in ["Design","Data sources","Display settings"].iter().enumerate(){if ui.selectable_label(self.tab==i,RichText::new(*label).size(13.)).clicked(){self.tab=i;}}
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                    if primary(ui,"↗  Upload to screen").clicked(){self.set_port();let _=self.runtime.tx.send(Command::Upload);self.message=if self.port.is_empty()&&self.data.port.is_empty(){self.tab=2;"Select your screen's COM port in Display settings.".into()}else{format!("Sending to {}…",self.port)};}
                    if action(ui,"Save ▾").clicked(){self.save(false);}
                    let export = action(ui,"Export \u{25be}");
                    egui::popup::popup_below_widget(ui,egui::Id::new("export_menu"),&export,egui::PopupCloseBehavior::CloseOnClickOutside,|ui|{if ui.button("Save JSON as…").clicked(){self.save(true);ui.close_menu();}if ui.button("Legacy .data snapshot…").clicked(){self.export_legacy();ui.close_menu();}ui.label(RichText::new("Older software shows a snapshot.\nOpen here to keep editing live fields.").size(11.).color(MUTED));ui.separator();if ui.button("Screen image (.png)…").clicked(){self.export_png();ui.close_menu();}});
                    if export.clicked(){ui.memory_mut(|memory|memory.toggle_popup(egui::Id::new("export_menu")));}
                    if action(ui,"Open").clicked(){self.request("open");}if action(ui,"+ New").clicked(){self.request("new");}
                });
            });
        });
        egui::TopBottomPanel::bottom("status")
            .exact_height(35.)
            .frame(
                egui::Frame::new()
                    .fill(ctx.style().visuals.panel_fill)
                    .inner_margin(egui::Margin::symmetric(20, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("●")
                            .color(if self.data.connected {
                                ACCENT
                            } else {
                                Color32::from_rgb(220, 70, 70)
                            })
                            .size(10.),
                    );
                    ui.label(RichText::new(&self.message).size(11.).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(if self.data.last_refresh.is_empty() {
                                "Starting sensors…".into()
                            } else {
                                format!("Updated {}", self.data.last_refresh)
                            })
                            .size(11.)
                            .color(MUTED),
                        );
                    });
                });
            });
    }
    fn library(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("library")
            .exact_width(240.)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(ctx.style().visuals.panel_fill)
                    .inner_margin(egui::Margin::same(17)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(!self.palette_tab, "Widgets").clicked() {
                        self.palette_tab = false;
                    }
                    if ui.selectable_label(self.palette_tab, "Layers").clicked() {
                        self.palette_tab = true;
                    }
                });
                ui.add_space(10.);
                ui.separator();
                if self.palette_tab {
                    heading(ui, &format!("{} LAYERS", self.project.widgets.len()));
                    let mut select = None;
                    let mut remove = None;
                    let mut reorder = None;
                    ui.label(
                        RichText::new("Drag layers to change their order.")
                            .size(10.)
                            .color(MUTED),
                    );
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for w in self.project.widgets.iter_mut().rev() {
                            let row = ui
                                .horizontal(|ui| {
                                    ui.checkbox(&mut w.visible, "");
                                    let response = ui.add(
                                        egui::Label::new(format!(
                                            "≡ {}  {}",
                                            w.kind.icon(),
                                            w.name
                                        ))
                                        .selectable(false)
                                        .sense(Sense::click_and_drag()),
                                    );
                                    if response.clicked() {
                                        select = Some(w.id);
                                    }
                                    if response.drag_started() {
                                        response.dnd_set_drag_payload(w.id);
                                    }
                                    if ui.small_button("×").clicked() {
                                        remove = Some(w.id);
                                    }
                                })
                                .response;
                            let above = ui
                                .input(|i| i.pointer.interact_pos())
                                .is_none_or(|p| p.y < row.rect.center().y);
                            if row.contains_pointer()
                                && egui::DragAndDrop::has_payload_of_type::<u64>(ui.ctx())
                            {
                                let y = if above {
                                    row.rect.top()
                                } else {
                                    row.rect.bottom()
                                };
                                ui.painter().line_segment(
                                    [
                                        Pos2::new(row.rect.left(), y),
                                        Pos2::new(row.rect.right(), y),
                                    ],
                                    Stroke::new(2_f32, ACCENT),
                                );
                            }
                            if let Some(from) = row.dnd_release_payload::<u64>() {
                                reorder = Some((*from, w.id, above));
                            }
                        }
                    });
                    if select.is_some() {
                        self.selected.click(
                            select.unwrap(),
                            ui.input(|i| {
                                i.modifiers.shift || i.modifiers.ctrl || i.modifiers.command
                            }),
                        );
                    }
                    if let Some(id) = remove {
                        self.project.widgets.retain(|w| w.id != id);
                    }
                    if let Some((from, to, above)) = reorder {
                        if from != to {
                            if let Some(index) =
                                self.project.widgets.iter().position(|w| w.id == from)
                            {
                                let widget = self.project.widgets.remove(index);
                                if let Some(index) =
                                    self.project.widgets.iter().position(|w| w.id == to)
                                {
                                    self.project
                                        .widgets
                                        .insert(index + usize::from(above), widget);
                                } else {
                                    self.project.widgets.push(widget);
                                }
                            }
                        }
                    }
                } else {
                    heading(ui, "BUILD YOUR SCREEN");
                    ui.add_sized(
                        [205., 28.],
                        egui::TextEdit::singleline(&mut self.search).hint_text("Search widgets…"),
                    );
                    ui.add_space(10.);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for kind in Kind::ALL {
                            if !kind
                                .label()
                                .to_lowercase()
                                .contains(&self.search.to_lowercase())
                            {
                                continue;
                            }
                            let r = ui.add_sized(
                                [205., 36.],
                                egui::Button::new(format!("{}     {}", kind.icon(), kind.label()))
                                    .fill(ctx.style().visuals.faint_bg_color)
                                    .stroke(Stroke::new(1_f32, BORDER))
                                    .sense(Sense::click_and_drag())
                                    .corner_radius(5.),
                            );
                            if r.clicked() {
                                self.add(kind.clone(), String::new(), 24., 45.);
                            }
                            if r.drag_started() {
                                self.payload = Some((kind, String::new()));
                            }
                        }
                        ui.add_space(12.);
                        egui::Frame::new()
                            .fill(ctx.style().visuals.faint_bg_color)
                            .corner_radius(6.)
                            .inner_margin(12.)
                            .show(ui, |ui| {
                                ui.label(RichText::new("Bring your own data").strong().size(12.));
                                ui.label(
                                    RichText::new(
                                        "Weather, markets, your home, or any JSON endpoint.",
                                    )
                                    .size(11.)
                                    .color(MUTED),
                                );
                                if ui.button("Connect a source  →").clicked() {
                                    self.tab = 1;
                                }
                            });
                    });
                }
            });
    }
    fn wallpaper_controls(ui: &mut egui::Ui, project: &mut Project) {
        if ui.button("Choose wallpaper…").clicked() {
            if let Some(image) = Self::image_file() {
                project.wallpaper = Some(image);
            }
        }
        if project.wallpaper.is_some() {
            ui.horizontal(|ui| {
                if ui.button("↶ 90°").clicked() {
                    project.wallpaper_rotation = (project.wallpaper_rotation + 3) % 4;
                }
                if ui.button("↷ 90°").clicked() {
                    project.wallpaper_rotation = (project.wallpaper_rotation + 1) % 4;
                }
                ui.label(format!("{}°", project.wallpaper_rotation * 90));
            });
            ui.checkbox(&mut project.wallpaper_mirror, "Mirror wallpaper");
            if ui.button("Reset wallpaper transform").clicked() {
                project.wallpaper_rotation = 0;
                project.wallpaper_mirror = false;
            }
            if ui.button("Remove wallpaper").clicked() {
                project.wallpaper = None;
            }
            ui.label(
                RichText::new("Resized to the screen resolution after rotation.")
                    .size(10.)
                    .color(MUTED),
            );
        }
    }
    fn units(ui: &mut egui::Ui, prefix: &mut String, suffix: &mut String) {
        ui.columns(2, |columns| {
            field(&mut columns[0], "Unit prefix", prefix);
            field(&mut columns[1], "Unit suffix", suffix);
        });
    }
    fn binding_picker(
        ui: &mut egui::Ui,
        binding: &mut String,
        search: &mut String,
        sources: &[Source],
        data: &DataState,
        allow_static: bool,
    ) {
        ui.label(RichText::new("Data binding").size(12.).color(MUTED));
        egui::ComboBox::from_id_salt("binding")
            .selected_text(if binding.is_empty() {
                if allow_static {
                    "Static value"
                } else {
                    "Choose a field"
                }
            } else {
                binding.as_str()
            })
            .width(210.)
            .height(380.)
            .show_ui(ui, |ui| {
                ui.add(egui::TextEdit::singleline(search).hint_text("Find a field..."));
                if allow_static {
                    ui.selectable_value(binding, String::new(), "Static value");
                }
                let query = search.to_lowercase();
                ui.label(RichText::new("SYSTEM SENSORS").small().strong());
                for (label, key) in SYSTEM_FIELDS {
                    if format!("{label} {key}").to_lowercase().contains(&query) {
                        ui.selectable_value(
                            binding,
                            (*key).into(),
                            format!(
                                "{} ({})",
                                label,
                                data.values.get(*key).map(String::as_str).unwrap_or("N/A")
                            ),
                        );
                    }
                }
                for source in sources {
                    ui.separator();
                    ui.label(RichText::new(&source.name).strong());
                    let prefix = format!("source:{}:", source.id);
                    let fields: Vec<_> = data
                        .values
                        .iter()
                        .filter(|(key, _)| {
                            key.starts_with(&prefix)
                                && format!("{} {}", source.name, key)
                                    .to_lowercase()
                                    .contains(&query)
                        })
                        .collect();
                    if fields.is_empty() {
                        ui.label(
                            RichText::new("No matching fields. Connect in Data sources.")
                                .small()
                                .color(MUTED),
                        );
                    }
                    for (key, value) in fields {
                        ui.selectable_value(
                            binding,
                            key.clone(),
                            format!("{}   {}", key.trim_start_matches(&prefix), value),
                        );
                    }
                }
                for (key, value) in data.values.iter().filter(|(key, _)| {
                    key.starts_with("sensor:") && key.to_lowercase().contains(&query)
                }) {
                    ui.selectable_value(binding, key.clone(), format!("{key} ({value})"));
                }
            });
        ui.add_sized(
            [ui.available_width(), 27.],
            egui::TextEdit::singleline(binding).hint_text("Or enter a source field path"),
        );
    }
    fn multi_properties(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new(format!("{} objects selected", self.selected.0.len()))
                .size(18.)
                .strong(),
        );
        ui.label(
            RichText::new("Changes apply to all selected objects. Mixed values are marked below.")
                .small()
                .color(MUTED),
        );
        let Some(first) = self
            .project
            .widgets
            .iter()
            .find(|w| self.selected.contains(w.id))
            .cloned()
        else {
            return;
        };
        let text_first = self
            .project
            .widgets
            .iter()
            .find(|w| self.selected.contains(w.id) && !w.kind.is_shape() && w.kind != Kind::Image)
            .cloned();
        egui::ScrollArea::vertical().show(ui, |ui| {
            heading(ui, "APPEARANCE");
            if let Some(first) = text_first {
                let mixed = self.project.widgets.iter().filter(|w| self.selected.contains(w.id) && !w.kind.is_shape() && w.kind != Kind::Image)
                    .any(|w| w.font != first.font || w.font_data != first.font_data || w.size != first.size || w.letter_spacing != first.letter_spacing);
                if mixed { ui.label(RichText::new("Mixed text styles").small().color(MUTED)); }
                ui.label("Font");
                let mut font = first.font.clone();
                let mut font_changed = false;
                let mut font_data = None;
                egui::ComboBox::from_id_salt("multi_font").width(210.).selected_text(&font).show_ui(ui, |ui| {
                    for name in fonts::installed().keys() { font_changed |= ui.selectable_value(&mut font, name.clone(), name).clicked(); }
                });
                if ui.button("Choose font file (.ttf / .otf)").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Fonts", &["ttf", "otf"]).pick_file() {
                        if let Ok(data) = std::fs::read(&path) {
                            if data.len() <= 4_000_000 && ab_glyph::FontArc::try_from_vec(data.clone()).is_ok() {
                                font = path.file_stem().unwrap_or_default().to_string_lossy().into();
                                font_data = Some(STANDARD.encode(data));
                                font_changed = true;
                            }
                        }
                    }
                }
                let mut size = first.size;
                let mut size_changed = false;
                ui.horizontal(|ui| { ui.label("Text size"); size_changed = ui.add(egui::DragValue::new(&mut size).range(6..=150).suffix(" px")).changed(); });
                let mut spacing = first.letter_spacing;
                let mut spacing_changed = false;
                ui.horizontal(|ui| { ui.label("Letter spacing"); spacing_changed = ui.add(egui::DragValue::new(&mut spacing).range(-10.0..=40.0).speed(0.1).suffix(" px")).changed(); });
                for w in self.project.widgets.iter_mut().filter(|w| self.selected.contains(w.id) && !w.kind.is_shape() && w.kind != Kind::Image) {
                    if font_changed { w.font = font.clone(); w.font_data = font_data.clone(); }
                    if size_changed { w.resize_text(size); }
                    if spacing_changed { w.letter_spacing = spacing; }
                }
            }
            let mixed_color = self.project.widgets.iter().filter(|w| self.selected.contains(w.id)).any(|w| w.color != first.color);
            if mixed_color { ui.label(RichText::new("Mixed colors").small().color(MUTED)); }
            let mut color = first.color;
            let mut background = first.background;
            let mut color_changed = false;
            let mut background_changed = false;
            ui.horizontal(|ui| { ui.label("Color"); color_changed = ui.color_edit_button_srgba_unmultiplied(&mut color).changed(); });
            ui.horizontal(|ui| { ui.label("Track / background"); background_changed = ui.color_edit_button_srgba_unmultiplied(&mut background).changed(); });
            for w in self.project.widgets.iter_mut().filter(|w| self.selected.contains(w.id)) {
                if color_changed { w.color = color; }
                if background_changed { w.background = background; }
            }
            heading(ui, "LAYER");
            ui.horizontal(|ui| {
                if ui.button("Duplicate").clicked() { self.duplicate(); }
                if ui.button("Delete").clicked() { self.remove(); }
            });
            ui.add_space(16.);
            ui.label(RichText::new("Shift or Ctrl + click to add or remove.\nDrag empty canvas to box select.\nEsc returns to canvas properties.").small().color(MUTED));
        });
    }
    fn properties(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("properties").exact_width(266.).resizable(false).frame(egui::Frame::new().fill(ctx.style().visuals.panel_fill).inner_margin(egui::Margin::same(18))).show(ctx,|ui|{
            heading(ui,"PROPERTIES");
            if self.selected.0.len() > 1 { self.multi_properties(ui); return; }
            let Some(index)=self.project.widgets.iter().position(|w|self.selected.contains(w.id))else{ui.label("Select a widget on the screen");ui.label(RichText::new("Adjust its position, appearance, and data here.").color(MUTED));heading(ui,"CANVAS");field(ui,"Design name",&mut self.project.name);Self::wallpaper_controls(ui,&mut self.project);ui.label("Background color");ui.color_edit_button_srgba_unmultiplied(&mut self.project.background);return;};
            let mut image=false;let mut duplicate=false;let mut remove=false;let mut front=false;let mut back=false;
            egui::ScrollArea::vertical().show(ui,|ui|{
                let w=&mut self.project.widgets[index];ui.label(RichText::new(w.kind.label()).size(18.).strong());ui.add_space(10.);field(ui,"Layer name",&mut w.name);
                heading(ui,"POSITION & SIZE");egui::Grid::new("geometry").num_columns(4).spacing([8.,10.]).show(ui,|ui|{ui.label("X");ui.add(egui::DragValue::new(&mut w.x).range(0..=480).speed(1.));ui.label("Y");ui.add(egui::DragValue::new(&mut w.y).range(0..=480).speed(1.));ui.end_row();ui.label("W");ui.add(egui::DragValue::new(&mut w.w).range(1..=480).speed(1.));ui.label("H");ui.add(egui::DragValue::new(&mut w.h).range(1..=480).speed(1.));ui.end_row();});
                ui.add_space(6.);ui.horizontal(|ui|{ui.checkbox(&mut w.visible,"Visible");ui.checkbox(&mut w.locked,"Lock position");});
                heading(ui,"CONTENT");if w.kind==Kind::Clock{egui::ComboBox::from_id_salt("clock_format").selected_text(if w.text=="date"{"Date"}else{"Time"}).show_ui(ui,|ui|{ui.selectable_value(&mut w.text,"time".into(),"Time");ui.selectable_value(&mut w.text,"date".into(),"Date");});
                    egui::ComboBox::from_id_salt("clock_display_format").selected_text(if w.clock_format.is_empty(){"Default"}else{&w.clock_format}).show_ui(ui,|ui|{
                        for (format,label) in [("","Default"),("%Y/%m/%d","YYYY/MM/DD"),("%H:%M","HH:MM"),("%H:%M:%S","HH:MM:SS")] { ui.selectable_value(&mut w.clock_format,format.into(),label); }
                    });}else if w.kind==Kind::Image{image=ui.button("Choose image…").clicked();}else if w.kind.is_shape(){
                    ui.horizontal(|ui| {ui.label("Rotation");ui.add(egui::DragValue::new(&mut w.rotation_degrees).range(-360.0..=360.0).suffix("\u{b0}"));});
                    if w.kind==Kind::StraightLine {let before=w.line_direction;ui.horizontal_wrapped(|ui|{ui.selectable_value(&mut w.line_direction,LineDirection::Horizontal,"Horizontal");ui.selectable_value(&mut w.line_direction,LineDirection::Vertical,"Vertical");ui.selectable_value(&mut w.line_direction,LineDirection::Diagonal,"Diagonal");});if before!=w.line_direction {match w.line_direction {LineDirection::Vertical if w.w>w.h=>std::mem::swap(&mut w.w,&mut w.h),LineDirection::Horizontal if w.h>w.w=>std::mem::swap(&mut w.w,&mut w.h),LineDirection::Diagonal if w.h<8.=>w.h=50.,_=>{}}}ui.checkbox(&mut w.outline_enabled,"Outline");ui.checkbox(&mut w.line_reverse,"Reverse direction");}else{ui.checkbox(&mut w.fill_enabled,"Fill");ui.checkbox(&mut w.outline_enabled,"Outline");}if w.outline_enabled {ui.horizontal(|ui|{ui.label("Outline color");ui.color_edit_button_srgba_unmultiplied(&mut w.outline_color);});}ui.add(egui::Slider::new(&mut w.stroke_width,1.0..=40.0).text("Stroke width"));
                    if w.kind==Kind::RoundedRectangle { ui.add(egui::Slider::new(&mut w.corner_radius,0.0..=100.0).text("Corner radius")); }
                    if w.kind==Kind::Polygon { ui.add(egui::Slider::new(&mut w.sides,3..=12).text("Sides")); }
                }else{
                    if w.kind == Kind::Ticker {
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut w.ticker_bound, false, "Static text");
                            ui.selectable_value(&mut w.ticker_bound, true, "Data binding");
                        });
                        ui.label("Scroll speed");
                        ui.add(egui::Slider::new(&mut w.scroll_speed, 0.0..=10.0));
                        ui.label(RichText::new("0 = no scroll; 10 = one full scroll per second").small().color(MUTED));
                        ui.label("Scroll direction");
                        egui::ComboBox::from_id_salt("scroll_direction").selected_text(match w.scroll_direction { ScrollDirection::RightToLeft => "Right to left", ScrollDirection::LeftToRight => "Left to right" }).show_ui(ui, |ui| {
                            ui.selectable_value(&mut w.scroll_direction, ScrollDirection::RightToLeft, "Right to left");
                            ui.selectable_value(&mut w.scroll_direction, ScrollDirection::LeftToRight, "Left to right");
                        });
                        ui.checkbox(&mut w.ticker_circular, "Circular").on_hover_text("Repeat the first item immediately after the last, without waiting for the ticker to clear.");
                        if w.ticker_bound {
                            let mut remove_source = None;
                            for (i, source) in w.ticker_sources.iter_mut().enumerate() {
                                ui.push_id(i, |ui| {
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        ui.label(format!("Data source {}", i + 1));
                                        if ui.small_button("Remove").clicked() { remove_source = Some(i); }
                                    });
                                    Self::binding_picker(ui, &mut source.binding, &mut self.binding_search, &self.project.sources, &self.data, false);
                                    Self::units(ui, &mut source.unit_prefix, &mut source.unit_suffix);
                                    if source.is_numeric(&self.data) {
                                        ui.label("Decimal places");
                                        egui::ComboBox::from_id_salt("ticker_decimals").selected_text(source.decimals.map_or("Auto (source precision)".into(), |n| n.to_string())).show_ui(ui, |ui| {
                                            ui.selectable_value(&mut source.decimals, None, "Auto (source precision)");
                                            for n in 0..=6 { ui.selectable_value(&mut source.decimals, Some(n), n.to_string()); }
                                        });
                                    }
                                    ui.label(RichText::new(format!("Current: {}", source.display(&self.data))).small().color(ACCENT));
                                    if let Some(reason) = self.data.unavailable.get(&source.binding) { ui.label(RichText::new(reason).small().color(MUTED)); }
                                });
                            }
                            if let Some(i) = remove_source { w.ticker_sources.remove(i); }
                            if w.ticker_sources.len() < 100 && ui.button("+ Add data source").clicked() {
                                w.ticker_sources.push(TickerSource::default());
                            }
                            if w.ticker_sources.is_empty() { ui.label("Add a data source to start the ticker."); }
                        } else { field(ui, "Text", &mut w.text); }
                    } else {
                        Self::binding_picker(ui, &mut w.binding, &mut self.binding_search, &self.project.sources, &self.data, true);
                        if w.binding.is_empty() { field(ui, if w.kind == Kind::Text { "Text" } else { "Value" }, &mut w.text); }
                        else {
                            Self::units(ui, &mut w.unit_prefix, &mut w.unit);
                            ui.label(RichText::new(format!("Current: {}{}{}", w.unit_prefix, self.data.value(w), w.unit)).size(11.).color(ACCENT));
                            if let Some(reason) = self.data.unavailable.get(&w.binding) { ui.label(RichText::new(reason).size(11.).color(MUTED)); }
                        }
                    }
                }
                if w.show_label{field(ui,"Label",&mut w.label);}
                heading(ui,"APPEARANCE");
                if !w.kind.is_shape() && w.kind!=Kind::Image {
                    ui.label("Font");
                    egui::ComboBox::from_id_salt("widget_font").width(215.).selected_text(&w.font).show_ui(ui,|ui|{
                        for name in fonts::installed().keys() { if ui.selectable_value(&mut w.font,name.clone(),name).clicked() { w.font_data=None; } }
                    });
                    if ui.button("Choose font file (.ttf / .otf)…").clicked() {
                        if let Some(path)=rfd::FileDialog::new().add_filter("Fonts",&["ttf","otf"]).pick_file() {
                            if let Ok(data)=std::fs::read(&path) { if data.len()<=4_000_000 && ab_glyph::FontArc::try_from_vec(data.clone()).is_ok() {
                                w.font=path.file_stem().unwrap_or_default().to_string_lossy().into();w.font_data=Some(STANDARD.encode(data));
                            }}
                        }
                    }
                    ui.horizontal_wrapped(|ui|{ui.label("Original sizes:");for size in fonts::LEGACY_SIZES { if ui.small_button(format!("{}",size)).clicked() { w.resize_text(size); } }});
                }
                ui.horizontal(|ui|{ui.label("Text size");let mut size=w.size;if ui.add(egui::DragValue::new(&mut size).range(6..=150).suffix(" px")).changed(){w.resize_text(size);}});
                if !w.kind.is_shape() && w.kind!=Kind::Image {ui.horizontal(|ui|{ui.label("Letter spacing");ui.add(egui::DragValue::new(&mut w.letter_spacing).range(-10.0..=40.0).speed(0.1).suffix(" px"));});}ui.horizontal(|ui|{ui.label("Accent");ui.color_edit_button_srgba_unmultiplied(&mut w.color);ui.label("Track");ui.color_edit_button_srgba_unmultiplied(&mut w.background);});
                if [Kind::Bar,Kind::Ring,Kind::Pie,Kind::Line].contains(&w.kind){ui.horizontal(|ui|{ui.label("Min");ui.add(egui::DragValue::new(&mut w.min));ui.label("Max");ui.add(egui::DragValue::new(&mut w.max));});}
                if w.kind==Kind::Ring {
                    ui.add(egui::Slider::new(&mut w.ring_thickness,1.0..=30.0).text("Ring thickness"));
                    ui.add(egui::Slider::new(&mut w.ring_start,-180.0..=180.0).text("Start angle"));
                    ui.checkbox(&mut w.ring_label_inside,"Place label inside ring");
                    if w.legacy_arc {ui.checkbox(&mut w.legacy_arc_gaps,"Gapped segments");}
                }
                if [Kind::Value,Kind::Ring,Kind::Bar,Kind::Pie].contains(&w.kind) {
                    egui::ComboBox::from_id_salt("value_decimals").selected_text(w.decimals.map_or("Auto decimals".into(),|n|format!("{} decimal places",n))).show_ui(ui,|ui|{
                        ui.selectable_value(&mut w.decimals,None,"Auto decimals");
                        for n in 0..=3 {ui.selectable_value(&mut w.decimals,Some(n),format!("{} decimal places",n));}
                    });
                }
                heading(ui,"LAYER");ui.horizontal(|ui|{back=ui.button("Send back").clicked();front=ui.button("Bring front").clicked();});ui.horizontal(|ui|{duplicate=ui.button("Duplicate").clicked();remove=ui.button(RichText::new("Delete").color(Color32::from_rgb(185,78,73))).clicked();});
                ui.add_space(16.);ui.label(RichText::new("Drag to move • corner handle to resize\nArrow keys to nudge • Shift for 10 px\nCtrl+D duplicate • Delete remove").size(10.).color(MUTED));
            });
            if image{self.project.widgets[index].image=Self::image_file();}if front{let w=self.project.widgets.remove(index);self.project.widgets.push(w);}if back{let w=self.project.widgets.remove(index);self.project.widgets.insert(0,w);}if duplicate{self.duplicate();}if remove{self.remove();}
        });
    }
    fn design(&mut self, ctx: &egui::Context) {
        self.library(ctx);
        self.properties(ctx);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(if ctx.style().visuals.dark_mode {
                        Color32::from_rgb(24, 31, 34)
                    } else {
                        Color32::from_rgb(240, 245, 244)
                    })
                    .inner_margin(egui::Margin::same(25)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.project.name).size(18.).strong());
                    if self.dirty {
                        ui.label(RichText::new("• Unsaved").size(11.).color(MUTED));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("↶").on_hover_text("Undo (Ctrl+Z)").clicked() {
                            self.undo_action();
                        }
                        if ui.button("↷").on_hover_text("Redo (Ctrl+Y)").clicked() {
                            self.redo_action();
                        }
                    });
                });
                ui.add_space(12.);
                ui.horizontal(|ui| {
                    let was = self.project.portrait;
                    ui.selectable_value(&mut self.project.portrait, false, "▭ Landscape");
                    ui.selectable_value(&mut self.project.portrait, true, "▯ Portrait");
                    if was != self.project.portrait {
                        Self::sanitize(&mut self.project);
                    }
                    ui.add_space(8.);
                    ui.checkbox(&mut self.project.grid, "Snap to grid");
                    if self.project.grid {
                        ui.add(
                            egui::DragValue::new(&mut self.project.grid_size)
                                .range(2..=64)
                                .suffix(" px"),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        egui::ComboBox::from_id_salt("zoom")
                            .selected_text(format!("{}%", (self.zoom * 100.) as i32))
                            .show_ui(ui, |ui| {
                                for z in [0.75, 1., 1.25, 1.5, 2.] {
                                    ui.selectable_value(
                                        &mut self.zoom,
                                        z,
                                        format!("{}%", (z * 100.) as i32),
                                    );
                                }
                            });
                    });
                });
                ui.horizontal(|ui| {
                    let color = if self.data.connected {
                        ACCENT
                    } else {
                        Color32::from_rgb(220, 70, 70)
                    };
                    ui.label(RichText::new("●").color(color));
                    ui.label(if self.data.connected {
                        format!(
                            "{} • {}",
                            self.data.port,
                            if self.data.streaming {
                                "Streaming live"
                            } else {
                                "Connected • paused"
                            }
                        )
                    } else {
                        "No screen connected • detecting automatically".into()
                    });
                });
                ui.add_space(12.);
                ui.separator();
                let (w, h) = self.project.dimensions();
                let avail = ui.available_rect_before_wrap();
                let scale = self
                    .zoom
                    .min((avail.width() - 65.) / w as f32)
                    .min((avail.height() - 110.) / h as f32)
                    .max(0.2);
                let size = Vec2::new(w as f32, h as f32) * scale;
                let origin = Pos2::new(
                    avail.center().x - size.x / 2.,
                    avail.center().y - size.y / 2. - 6.,
                );
                let screen = Rect::from_min_size(origin, size);
                let bezel = screen.expand(12.);
                ui.painter().rect_filled(
                    bezel.translate(Vec2::new(0., 8.)),
                    15.,
                    Color32::from_black_alpha(12),
                );
                ui.painter()
                    .rect_filled(bezel, 12., Color32::from_rgb(34, 51, 54));
                if let Some(t) = &self.texture {
                    ui.painter().image(
                        t.id(),
                        screen,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
                        Color32::WHITE,
                    );
                }
                if self.project.grid {
                    let step = self.project.grid_size * scale;
                    if step > 3. {
                        for x in 0..=(size.x / step) as i32 {
                            for y in 0..=(size.y / step) as i32 {
                                ui.painter().circle_filled(
                                    origin + Vec2::new(x as f32 * step, y as f32 * step),
                                    0.65,
                                    Color32::from_white_alpha(26),
                                );
                            }
                        }
                    }
                }
                match canvas::interact(
                    ui,
                    screen,
                    scale,
                    &mut self.project,
                    &mut self.selected,
                    &mut self.drag_origin,
                ) {
                    Some("duplicate") => self.duplicate(),
                    Some("delete") => self.remove(),
                    _ => {}
                }
                if ctx.input(|i| i.pointer.any_released()) {
                    if let Some((kind, binding)) = self.payload.take() {
                        if let Some(pos) = ctx.pointer_hover_pos().filter(|p| screen.contains(*p)) {
                            let relative = (pos - origin) / scale;
                            self.add(
                                kind,
                                binding,
                                relative.x.min(w as f32 - 130.).max(0.),
                                relative.y.min(h as f32 - 60.).max(0.),
                            );
                        }
                    }
                    self.drag_origin = None;
                }
                if let Some((kind, _)) = &self.payload {
                    if let Some(pos) = ctx.pointer_hover_pos() {
                        let r =
                            Rect::from_min_size(pos + Vec2::new(10., 10.), Vec2::new(110., 30.));
                        ui.painter().rect_filled(r, 5., ACCENT);
                        ui.painter().text(
                            r.center(),
                            Align2::CENTER_CENTER,
                            kind.label(),
                            FontId::proportional(12.),
                            Color32::WHITE,
                        );
                    }
                }
                ui.painter().text(
                    Pos2::new(screen.center().x, bezel.bottom() + 29.),
                    Align2::CENTER_CENTER,
                    format!(
                        "{} × {} px   /   3.5-inch display   /   {}",
                        w,
                        h,
                        if self.project.portrait {
                            "Portrait"
                        } else {
                            "Landscape"
                        }
                    ),
                    FontId::proportional(11.),
                    MUTED,
                );
                ui.painter().text(
                    Pos2::new(screen.center().x, bezel.bottom() + 51.),
                    Align2::CENTER_CENTER,
                    "Drag widgets onto the canvas. Select any widget to make it yours.",
                    FontId::proportional(11.),
                    MUTED,
                );
            });
    }
    fn undo_action(&mut self) {
        if let Some(p) = self.undo.pop() {
            self.redo.push(self.project.clone());
            self.project = p;
            self.selected.clear();
            self.drag_origin = None;
            self.sync();
            self.history_navigation = true;
        }
    }
    fn redo_action(&mut self) {
        if let Some(p) = self.redo.pop() {
            self.undo.push(self.project.clone());
            self.project = p;
            self.selected.clear();
            self.drag_origin = None;
            self.sync();
            self.history_navigation = true;
        }
    }
    fn sources(&mut self, ctx: &egui::Context) {
        self.lookup.poll();
        if self.lookup.busy() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        egui::SidePanel::left("source_navigation")
            .exact_width(260.)
            .resizable(false)
            .frame(egui::Frame::new().fill(page_fill(ctx)).inner_margin(18.))
            .show(ctx, |ui| {
                heading(ui, "DATA SOURCES");
                ui.menu_button("+ Add a source", |ui| {
                    for kind in [
                        SourceKind::Json,
                        SourceKind::HomeAssistant,
                        SourceKind::Weather,
                        SourceKind::Stock,
                    ] {
                        if ui.button(kind.label()).clicked() {
                            let id =
                                self.project.sources.iter().map(|s| s.id).max().unwrap_or(0) + 1;
                            let interval = if kind == SourceKind::Stock { 86400 } else { 60 };
                            self.project.sources.push(Source {
                                id,
                                name: kind.label().into(),
                                kind,
                                url: String::new(),
                                token: String::new(),
                                username: String::new(),
                                password: String::new(),
                                auth: Auth::Bearer,
                                credential_id: String::new(),
                                interval,
                                latitude: "41.90".into(),
                                longitude: "12.50".into(),
                                symbol: String::new(),
                                stocks: vec![],
                            });
                            self.source_index = Some(self.project.sources.len() - 1);
                            ui.close_menu();
                        }
                    }
                });
                ui.add_space(16.);
                egui::ScrollArea::vertical()
                    .id_salt("source_list")
                    .show(ui, |ui| {
                        if self.project.sources.is_empty() {
                            ui.label("Add weather, stocks, Home Assistant, or a JSON source.");
                        }
                        for (i, source) in self.project.sources.iter().enumerate() {
                            let count = self
                                .data
                                .values
                                .keys()
                                .filter(|k| k.starts_with(&format!("source:{}:", source.id)))
                                .count();
                            let status = if self.data.errors.contains_key(&source.id) {
                                "Needs attention"
                            } else if count > 0 {
                                "Connected"
                            } else {
                                "Set up source"
                            };
                            if ui
                                .add_sized(
                                    [ui.available_width(), 62.],
                                    egui::Button::new(format!(
                                        "{}\n{} | {} fields",
                                        source.name, status, count
                                    ))
                                    .selected(self.source_index == Some(i))
                                    .wrap(),
                                )
                                .clicked()
                            {
                                self.source_index = Some(i);
                            }
                        }
                    });
            });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(page_fill(ctx)).inner_margin(32.)).show(ctx,|ui|{
            if !ui.visuals().dark_mode {ui.visuals_mut().extreme_bg_color=Color32::from_rgb(242,246,245);}
            let Some(index)=self.source_index.filter(|i|*i<self.project.sources.len()) else {
                ui.heading("Connect the things you follow");ui.label("Choose a source on the left, or add a new one.");
                ui.add_space(12.);ui.label("Connected fields appear in the Data binding picker in Properties on the right of the Design screen.");return;
            };
            let id=self.project.sources[index].id;
            self.lookup.select(id);
            let mut refresh=false;let mut remove=false;
            egui::ScrollArea::vertical().id_salt(("source_form",id)).show(ui,|ui|{
                ui.set_max_width(780.);
                let source=&mut self.project.sources[index];
                ui.heading(source.kind.label());ui.add_space(16.);
                field(ui,"Source name",&mut source.name);
                match source.kind {
                    SourceKind::Weather=>{
                        heading(ui,"FIND A LOCATION");
                        ui.horizontal(|ui|{
                            let input=ui.add_sized([(ui.available_width()-100.).max(100.),30.],egui::TextEdit::singleline(&mut self.lookup.query).hint_text("City or postal code..."));
                            if ui.add_enabled(!self.lookup.busy(),egui::Button::new("Search").min_size(Vec2::new(82.,30.))).clicked() || (input.lost_focus()&&ui.input(|i|i.key_pressed(egui::Key::Enter))){self.lookup.search(SourceKind::Weather,String::new(),ctx.clone());}
                        });
                        for result in self.lookup.results.clone(){
                            if ui.button(&result.label).clicked(){if let Some((lat,lon))=result.coordinates {source.latitude=format!("{lat:.6}");source.longitude=format!("{lon:.6}");self.lookup.message="Coordinates updated. Connect / refresh to load weather.".into();self.lookup.results.clear();}}
                        }
                        if !self.lookup.message.is_empty(){ui.label(&self.lookup.message);}
                        ui.add_space(12.);
                        ui.columns(2,|columns|{field(&mut columns[0],"Latitude",&mut source.latitude);field(&mut columns[1],"Longitude",&mut source.longitude);});
                        ui.label(RichText::new("Current weather from Open-Meteo | \u{b0}C | wind in km/h").small().color(MUTED));
                    }
                    SourceKind::Stock=>{
                        ui.label("Alpha Vantage API key");ui.add_sized([ui.available_width(),30.],egui::TextEdit::singleline(&mut source.token).password(true));
                        heading(ui,"FIND STOCKS");
                        ui.horizontal(|ui|{
                            let input=ui.add_sized([(ui.available_width()-100.).max(100.),30.],egui::TextEdit::singleline(&mut self.lookup.query).hint_text("Company name or symbol..."));
                            if ui.add_enabled(!self.lookup.busy(),egui::Button::new("Search").min_size(Vec2::new(82.,30.))).clicked() || (input.lost_focus()&&ui.input(|i|i.key_pressed(egui::Key::Enter))){self.lookup.search(SourceKind::Stock,source.token.clone(),ctx.clone());}
                        });
                        for result in self.lookup.results.clone(){ui.horizontal_wrapped(|ui|{
                            ui.label(&result.label);let following=source.stocks.contains(&result.symbol);
                            if ui.add_enabled(!following,egui::Button::new(if following{"Following"}else{"+ Follow"})).clicked(){source.stocks.push(result.symbol);refresh=true;}
                        });}
                        if !self.lookup.message.is_empty(){ui.label(&self.lookup.message);}
                        heading(ui,"STOCKS YOU FOLLOW");
                        if source.stocks.is_empty(){ui.label("Search above and choose + Follow to start your list.");}
                        let mut unfollow=None;
                        for (i,symbol) in source.stocks.iter().enumerate(){ui.horizontal(|ui|{ui.label(RichText::new(symbol).strong());if ui.small_button("Remove").clicked(){unfollow=Some(i);}});}
                        if let Some(i)=unfollow{source.stocks.remove(i);source.symbol.clear();refresh=true;}
                        ui.label(RichText::new("Alpha Vantage quotes | end-of-day by default. Refresh uses one API request per followed stock; allowance depends on your plan.").small().color(MUTED));
                    }
                    SourceKind::Json=>field(ui,"JSON URL (http:// or https://)",&mut source.url),
                    SourceKind::HomeAssistant=>field(ui,"Home Assistant address",&mut source.url),
                }
                if matches!(source.kind,SourceKind::Json|SourceKind::HomeAssistant){
                    heading(ui,"AUTHENTICATION");
                    egui::ComboBox::from_id_salt("source_auth").selected_text(match source.auth{Auth::None=>"None",Auth::Bearer=>"Bearer token",Auth::Basic=>"Username and password"}).show_ui(ui,|ui|{
                        ui.selectable_value(&mut source.auth,Auth::None,"None");ui.selectable_value(&mut source.auth,Auth::Bearer,"Bearer token");ui.selectable_value(&mut source.auth,Auth::Basic,"Username and password");
                    });
                    match source.auth{
                        Auth::Basic=>{field(ui,"Username",&mut source.username);ui.label("Password");ui.add(egui::TextEdit::singleline(&mut source.password).password(true));},
                        Auth::Bearer=>{ui.label("Bearer token");ui.add_sized([ui.available_width(),30.],egui::TextEdit::singleline(&mut source.token).password(true));},Auth::None=>{}
                    }
                }
                if source.kind==SourceKind::Stock || (matches!(source.kind,SourceKind::Json|SourceKind::HomeAssistant)&&source.auth!=Auth::None){
                    ui.add_space(8.);ui.label(RichText::new("Credentials are saved in Windows Credential Manager.").small().color(MUTED));
                    if ui.small_button("Forget saved credentials").clicked(){match credentials::delete(source){Ok(())=>{source.token.clear();source.username.clear();source.password.clear();},Err(e)=>self.message=e}}
                }
                ui.add_space(18.);ui.horizontal(|ui|{ui.label("Refresh every");ui.add(egui::DragValue::new(&mut source.interval).range(5..=86400).suffix(" sec"));});
                ui.horizontal(|ui|{refresh|=primary(ui,"Connect / refresh").clicked();remove=action(ui,"Remove source").clicked();});
                if let Some(error)=self.data.errors.get(&id){ui.colored_label(Color32::from_rgb(190,76,55),error);}
                ui.add_space(20.);ui.separator();
                heading(ui,"CONNECTED FIELDS");
                ui.label("Choose these fields in Design > Properties > Data binding.");
                ui.add_sized([ui.available_width(),30.],egui::TextEdit::singleline(&mut self.source_search).hint_text("Filter fields..."));
                let prefix=format!("source:{id}:");
                egui::Grid::new(("connected_fields",id)).striped(true).show(ui,|ui|{
                    for (key,value) in self.data.values.iter().filter(|(k,_)|k.starts_with(&prefix)&&k.to_lowercase().contains(&self.source_search.to_lowercase())) {ui.label(key.trim_start_matches(&prefix));ui.label(value);ui.end_row();}
                });
            });
            if remove{self.project.sources.remove(index);self.source_index=None;}
            else if refresh{if let Err(error)=self.store_credentials(){self.message=error;}else{self.sync();let _=self.runtime.tx.send(Command::Refresh);self.message="Refreshing data sources...".into();}}
        });
    }
    fn settings(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(egui::Frame::new().fill(page_fill(ctx)).inner_margin(egui::Margin::same(32))).show(ctx,|ui|{if !ui.visuals().dark_mode {ui.visuals_mut().extreme_bg_color=Color32::from_rgb(242,246,245);}egui::ScrollArea::vertical().show(ui,|ui|{
            ui.label(RichText::new("Your screen, connected.").size(25.).strong());ui.label(RichText::new("Display output and Windows preferences.").color(MUTED));ui.add_space(24.);
            ui.columns(2,|columns|{
                egui::Frame::new().fill(page_fill(ctx)).stroke(Stroke::new(1_f32,BORDER)).corner_radius(9.).inner_margin(22.).show(&mut columns[0],|ui|{
                    ui.label(RichText::new("3.5-inch USB display").size(19.).strong());heading(ui,"CONNECTION");
                    egui::ComboBox::from_id_salt("port").width(230.).selected_text(if self.port.is_empty(){"Automatic detection"}else{&self.port}).show_ui(ui,|ui|{ui.selectable_value(&mut self.port,String::new(),"Automatic detection");for p in &self.ports{ui.selectable_value(&mut self.port,p.clone(),p);}});ui.horizontal(|ui|{if ui.button("Rescan ports").clicked(){self.ports=serialport::available_ports().unwrap_or_default().into_iter().map(|p|p.port_name).collect();}if ui.button("Send now").clicked(){self.set_port();let _=self.runtime.tx.send(Command::Upload);}});
                    if ui.button("Use automatic detection").clicked(){self.port.clear();self.set_port();}ui.label(RichText::new("Close the original monitoring software before connecting, so this app can use the screen's port.").size(11.).color(MUTED));ui.add_space(14.);
                    ui.label("Connection speed (baud)");
                    egui::ComboBox::from_id_salt("serial_baud").selected_text(self.project.serial_baud.to_string()).show_ui(ui, |ui| {
                        for &baud in SERIAL_SPEEDS {
                            ui.selectable_value(&mut self.project.serial_baud, baud, if baud == SERIAL_BAUD { format!("{} (original)", baud) } else { baud.to_string() });
                        }
                    });
                    ui.label(RichText::new("115200 is the verified original setting. Higher speeds depend on your display firmware; return to 115200 if the picture stops updating.").small().color(MUTED));
                    if ui.checkbox(&mut self.stream,"Keep this screen live").changed(){self.set_port();let _=self.runtime.tx.send(Command::Stream(self.stream));}
                    ui.label(RichText::new("Updates continue in the tray. The screen reconnects automatically after unplugging.").size(11.).color(MUTED));if !self.data.device.is_empty(){ui.label(&self.data.device);}
                    if self.data.streaming { ui.label(format!("Display updates: {:.1} frames/sec", self.data.display_fps)); }
                    heading(ui,"SCREEN");ui.horizontal(|ui|{ui.selectable_value(&mut self.project.portrait,false,"Landscape 480 × 320");ui.selectable_value(&mut self.project.portrait,true,"Portrait 320 × 480");});ui.checkbox(&mut self.project.rotation,"Flip display 180° (upload only)");ui.add(egui::Slider::new(&mut self.project.brightness,0..=100).text("Brightness %"));ui.label(RichText::new("Send a test image to check orientation and brightness on your screen.").size(11.).color(MUTED));
                });
                egui::Frame::new().fill(page_fill(ctx)).stroke(Stroke::new(1_f32,BORDER)).corner_radius(9.).inner_margin(22.).show(&mut columns[1],|ui|{
                    ui.label(RichText::new("Windows & canvas").size(19.).strong());heading(ui,"APPEARANCE");
                    ui.horizontal(|ui|{for (theme,label) in [(EditorTheme::System,"System"),(EditorTheme::Light,"Light"),(EditorTheme::Dark,"Dark")] {
                        if ui.selectable_value(&mut self.theme,theme,label).changed(){self.theme.apply(ctx);}
                    }});ui.label(RichText::new("System follows your Windows appearance. Used on first launch.").size(11.).color(MUTED));
                    heading(ui,"WINDOWS");let mut startup=platform::startup_enabled();if ui.checkbox(&mut startup,"Launch when I sign in to Windows").changed(){self.message=match platform::set_startup(startup){Ok(())=>"Windows startup preference updated.".into(),Err(e)=>format!("Startup setting failed: {}",e)};if let Some(t)=&self.tray{t.update_startup();}}
                    ui.label(RichText::new("Minimizing hides the window in the tray. Click its icon to reopen maximized. Right-click for Open, startup, and Exit.").size(12.).color(MUTED));if !self.tray_error.is_empty(){ui.colored_label(Color32::from_rgb(170,76,55),format!("Tray unavailable: {}. The window will minimize normally.",self.tray_error));}
                    heading(ui,"CANVAS");field(ui,"Design name",&mut self.project.name);Self::wallpaper_controls(ui,&mut self.project);ui.horizontal(|ui|{ui.label("Background");ui.color_edit_button_srgba_unmultiplied(&mut self.project.background);});
                    heading(ui,"HARDWARE SENSORS");
                    for (label,key) in [("CPU temperature","cpu.temperature"),("CPU fan","cpu.fan"),("CPU power","cpu.power"),("Disk temperature","disk.temperature")] {ui.horizontal(|ui|{ui.label(label);ui.label(self.data.values.get(key).map(String::as_str).unwrap_or("N/A"));});}
                    if ui.link("Where do these readings come from?").clicked(){self.sensor_help=true;}
                    ui.collapsing("Sensor scan details",|ui|{ui.label(&self.data.sensor_status);ui.label("The bundled hardware library checks CPU temperature, fan speed, power and drive temperatures automatically. If hardware access is denied, try running the app as administrator. Windows temperatures and other hardware-monitor providers are also checked.");});

                });
            });
        });});
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        let mut close_sensor_help = false;
        egui::Window::new("Where hardware readings come from")
            .open(&mut self.sensor_help)
            .collapsible(false)
            .resizable(false)
            .default_width(560.)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_max_width(560.);
                ui.label("These readings measure the hardware inside your PC.");
                heading(ui, "PROVIDED USBMONITOR EXAMPLE");
                ui.label("The original app reads CPU temperature, fan speed and power through its bundled HWiNFO sensor library, saved as msvcr120d.dll. Disk temperature comes from the selected disk's HWiNFO temperature sensor.");
                heading(ui, "THIS REBUILT APP");
                egui::Grid::new("hardware_reading_sources")
                    .spacing([18., 10.])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Reading").strong());
                        ui.label(RichText::new("Source").strong());
                        ui.end_row();
                        ui.label("CPU temperature (°C)");
                        ui.label("Bundled HWiNFO library; Windows / monitor fallback");
                        ui.end_row();
                        ui.label("CPU fan (RPM)");
                        ui.label("Bundled HWiNFO library; monitor fallback");
                        ui.end_row();
                        ui.label("CPU power (W)");
                        ui.label("Bundled HWiNFO library; monitor fallback");
                        ui.end_row();
                        ui.label("Disk temperature (°C)");
                        ui.label("Bundled HWiNFO library; Windows / monitor fallback");
                        ui.end_row();
                    });
                ui.add_space(10.);
                ui.label("The app now includes the same hardware sensor library as the example and scans it automatically. Available temperature, fan and power sensors can also be chosen individually in Data binding. Windows components, LibreHardwareMonitor / OpenHardwareMonitor and HWiNFO gadget sensors remain available as additional sources.");
                heading(ui, "WHY A READING CAN SHOW N/A");
                ui.label("N/A means no recent reading was available. The sensor may be unsupported or Windows may deny hardware access; running the app as administrator may help. Open Sensor scan details in Display settings to see what was detected.");
                ui.add_space(10.);
                if ui.button("Done").clicked() {
                    close_sensor_help = true;
                }
            });
        if close_sensor_help {
            self.sensor_help = false;
        }
        if self.new_dialog {
            egui::Window::new("Create a cockpit")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .default_width(365.)
                .show(ctx, |ui| {
                    field(ui, "Design name", &mut self.new_name);
                    ui.add_enabled_ui(self.new_template != 0, |ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.new_portrait, false, "Landscape");
                            ui.selectable_value(&mut self.new_portrait, true, "Portrait");
                        });
                    });
                    ui.label("Template");
                    let previous = self.new_template;
                    egui::ComboBox::from_id_salt("new_template")
                        .selected_text(["LandscapeTechnologyCh", "Everyday cockpit", "Blank canvas"][self.new_template])
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.new_template, 0, "LandscapeTechnologyCh");
                            ui.selectable_value(&mut self.new_template, 1, "Everyday cockpit");
                            ui.selectable_value(&mut self.new_template, 2, "Blank canvas");
                        });
                    if previous != self.new_template {
                        self.new_name = ["LandscapeTechnologyCh", "Everyday cockpit", "My cockpit"][self.new_template].into();
                    }
                    if self.new_template == 0 {
                        self.new_portrait = false;
                        ui.label(RichText::new("480 × 320 · Based on your reference picture. All layers are editable.").size(11.).color(MUTED));
                        ui.label(RichText::new("Hardware headings are sample text. Set weather and volume bindings to your data sources; unavailable values show —.").size(11.).color(MUTED));
                    }
                    ui.add_space(16.);
                    ui.horizontal(|ui| {
                        if primary(ui, "Create design").clicked() {
                            self.project = match self.new_template {
                                0 => Project::landscape_technology(),
                                1 => Project::starter(),
                                _ => Project::blank(),
                            };
                            self.project.name = self.new_name.clone();
                            self.project.portrait = self.new_portrait;
                            Self::sanitize(&mut self.project);
                            self.path = None;
                            self.dirty = true;
                            self.selected.clear();
                    self.drag_origin = None;
                            self.undo.clear();
                            self.redo.clear();
                            self.new_dialog = false;
                            self.tab = 0;
                            self.sync();
                        }
                        if ui.button("Cancel").clicked() {
                            self.new_dialog = false;
                        }
                    });
                });
        }
        if let Some(question) = self.confirm.clone() {
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(question);
                    ui.horizontal(|ui| {
                        if primary(ui, "Save changes").clicked() {
                            if self.save(false) {
                                self.confirm = None;
                                if let Some(a) = self.pending.take() {
                                    self.perform(&a);
                                }
                            }
                        }
                        if ui.button("Discard").clicked() {
                            self.confirm = None;
                            if let Some(a) = self.pending.take() {
                                self.perform(&a);
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.confirm = None;
                            self.pending = None;
                        }
                    });
                });
        }
    }
}
impl eframe::App for Studio {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.frames += 1;
        if self.screenshot.is_some() {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(if render::has_animation(&self.project) {
                Duration::ZERO
            } else {
                Duration::from_secs(1)
            });
        }
        for update in self.runtime.rx.try_iter() {
            self.data = update;
            self.refresh_texture = true;
        }
        if let Some(tray) = &mut self.tray {
            tray.set_connection(self.data.connected, &self.data.port);
            let actions = tray.poll();
            for a in actions {
                self.restore_frames = 3;
                match a {
                    platform::Action::Open => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    platform::Action::Startup => {
                        let enabled = !platform::startup_enabled();
                        self.message = match platform::set_startup(enabled) {
                            Ok(()) => "Windows startup preference updated.".into(),
                            Err(e) => e,
                        };
                    }
                    platform::Action::Exit => self.request("exit"),
                }
            }
            if let Some(t) = &self.tray {
                t.update_startup();
            }
        }
        if self.restore_frames == 0
            && self.tray.is_some()
            && self.screenshot.is_none()
            && ctx.input(|i| i.viewport().minimized.unwrap_or(false))
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        self.restore_frames = self.restore_frames.saturating_sub(1);
        if self.frames == 2 && std::env::args().any(|a| a == "--tray") && self.tray.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.quit && self.screenshot.is_none()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.request("exit");
        }
        let old_port = self.port.clone();
        let old = self.project.clone();
        let old_sig = signature(&old);
        let old_dirty = self.dirty;
        let old_path = self.path.clone();
        self.selected
            .0
            .retain(|id| self.project.widgets.iter().any(|w| w.id == *id));
        if self.tab == 0
            && self.confirm.is_none()
            && !self.new_dialog
            && self.browser.is_none()
            && canvas::escape(ctx, &mut self.selected, &mut self.drag_origin)
        {
            self.payload = None;
        }
        if !ctx.wants_keyboard_input()
            && self.confirm.is_none()
            && !self.new_dialog
            && self.browser.is_none()
        {
            let mods = ctx.input(|i| i.modifiers);
            if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
                self.remove();
            }
            if mods.command && ctx.input(|i| i.key_pressed(egui::Key::D)) {
                self.duplicate();
            }
            if mods.command && ctx.input(|i| i.key_pressed(egui::Key::Z)) {
                self.undo_action();
            }
            if mods.command && ctx.input(|i| i.key_pressed(egui::Key::Y)) {
                self.redo_action();
            }
            if mods.command && ctx.input(|i| i.key_pressed(egui::Key::S)) {
                self.save(false);
            }
            if mods.command && ctx.input(|i| i.key_pressed(egui::Key::O)) {
                self.request("open");
            }
            for w in self
                .project
                .widgets
                .iter_mut()
                .filter(|w| self.selected.contains(w.id))
            {
                if !w.locked {
                    let step = if mods.shift { 10. } else { 1. };
                    for (key, dx, dy) in [
                        (egui::Key::ArrowLeft, -step, 0.),
                        (egui::Key::ArrowRight, step, 0.),
                        (egui::Key::ArrowUp, 0., -step),
                        (egui::Key::ArrowDown, 0., step),
                    ] {
                        if ctx.input(|i| i.key_pressed(key)) {
                            w.x = (w.x + dx).max(0.);
                            w.y = (w.y + dy).max(0.);
                        }
                    }
                }
            }
        }
        if self.refresh_texture || render::has_animation(&self.project) {
            let img = self.frame_cache.frame(&self.project, &self.data);
            let options = egui::TextureOptions::LINEAR;
            if let Some(texture) = &mut self.texture {
                if let Some(old) = &self.screen_pixels {
                    if old.dimensions() == img.dimensions() {
                        for region in render::changed_regions(
                            old.as_raw(),
                            img.as_raw(),
                            img.width(),
                            img.height(),
                            4,
                        ) {
                            let tile = image::imageops::crop_imm(
                                &img, region.x, region.y, region.w, region.h,
                            )
                            .to_image();
                            texture.set_partial(
                                [region.x as usize, region.y as usize],
                                egui::ColorImage::from_rgba_unmultiplied(
                                    [region.w as usize, region.h as usize],
                                    tile.as_raw(),
                                ),
                                options,
                            );
                        }
                    } else {
                        texture.set(
                            egui::ColorImage::from_rgba_unmultiplied(
                                [img.width() as usize, img.height() as usize],
                                img.as_raw(),
                            ),
                            options,
                        );
                    }
                }
            } else {
                self.texture = Some(ctx.load_texture(
                    "live_screen",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [img.width() as usize, img.height() as usize],
                        img.as_raw(),
                    ),
                    options,
                ));
            }
            self.screen_pixels = Some(img);
            self.refresh_texture = false;
        }
        self.toolbar(ctx);
        match self.tab {
            0 => self.design(ctx),
            1 => self.sources(ctx),
            _ => self.settings(ctx),
        }
        self.dialogs(ctx);
        self.design_browser(ctx);
        if self.port != old_port {
            self.set_port();
        }
        if signature(&self.project) != old_sig {
            Self::sanitize(&mut self.project);
            if !self.history_navigation
                && self.last_checkpoint.elapsed() > Duration::from_millis(500)
            {
                self.undo.push(old);
                if self.undo.len() > 80 {
                    self.undo.remove(0);
                }
                self.redo.clear();
                self.last_checkpoint = Instant::now();
            }
            if self.path == old_path && self.dirty == old_dirty {
                self.dirty = true;
            }
            self.sync();
            ctx.request_repaint();
        }
        self.history_navigation = false;
        if self.last_autosave.elapsed() > Duration::from_secs(10) {
            if self.screenshot.is_none() {
                self.persist();
            }
            self.last_autosave = Instant::now();
        }
        if let Some(path) = self.screenshot.clone() {
            if self.frames >= 120 && !self.screenshot_requested {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                self.screenshot_requested = true;
            }
            for event in ctx.input(|i| i.events.clone()) {
                if let egui::Event::Screenshot { image, .. } = event {
                    let pixels: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    match image::save_buffer(
                        &path,
                        &pixels,
                        image.width() as u32,
                        image.height() as u32,
                        image::ColorType::Rgba8,
                    ) {
                        Ok(()) => self.quit = true,
                        Err(e) => {
                            eprintln!("Screenshot failed: {}", e);
                            self.quit = true;
                        }
                    }
                }
            }
        }
        if self.quit {
            let _ = self.runtime.tx.send(Command::Stop);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        // Remove the icon and disconnect its actions before background I/O stops.
        self.tray = None;
        if self.screenshot.is_none() {
            self.persist();
        }
        self.runtime.shutdown();
    }
}
fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    #[cfg(windows)]
    if args
        .iter()
        .any(|a| a == "--hardware-sensors" || a == "--hardware-scan")
    {
        hwinfo::run_helper(args.iter().any(|a| a == "--hardware-scan"));
        return Ok(());
    }
    let screenshot = args
        .iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from);
    // Interactive launches (including --tray) must acquire the instance guard
    // before opening the session or starting workers. Keep it alive through exit.
    // Isolated screenshots and sensor helpers retain their diagnostic behavior.
    #[cfg(windows)]
    let _instance = if screenshot.is_none() {
        match single_instance::start() {
            Ok(Some(instance)) => Some(instance),
            Ok(None) => return Ok(()),
            Err(error) => {
                rfd::MessageDialog::new()
                    .set_title("uMonitor")
                    .set_description(format!("uMonitor could not start: {error}"))
                    .set_level(rfd::MessageLevel::Error)
                    .show();
                return Err(eframe::Error::AppCreation(Box::new(error)));
            }
        }
    } else {
        None
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("uMonitor")
            .with_icon(platform::app_icon())
            .with_inner_size([1380., 850.])
            .with_min_inner_size([1120., 720.]),
        ..Default::default()
    };
    eframe::run_native(
        "uMonitor",
        options,
        Box::new(move |cc| Ok(Box::new(Studio::new(cc, screenshot)))),
    )
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    #[test]
    fn older_saved_imports_get_a_stable_reference_size() {
        let mut w = Widget::new(1, Kind::Value, 0., 0.);
        w.size = 40.;
        w.w = 62.;
        w.h = 28.;
        w.legacy_text = Some(LegacyText {
            cell_width: 22.,
            cell_height: 28.,
            advance: 20.,
            base_size: 0.,
        });
        let mut json = serde_json::to_value(&w).unwrap();
        json["legacy_text"]
            .as_object_mut()
            .unwrap()
            .remove("base_size");
        json.as_object_mut().unwrap().remove("letter_spacing");
        let mut p = Project::blank();
        p.widgets.push(serde_json::from_value(json).unwrap());
        Studio::sanitize(&mut p);
        assert_eq!(p.widgets[0].legacy_text.as_ref().unwrap().base_size, 40.);
        p.widgets[0].resize_text(20.);
        assert_eq!((p.widgets[0].w, p.widgets[0].h), (31., 14.));
    }
}
