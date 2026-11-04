use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use crate::original::{
    COMMAND_DELAY_MS, LEGACY_CPU_FREQUENCY_MAX_MHZ, SCREEN_H, SCREEN_W, SERIAL_BAUD,
    SERIAL_TIMEOUT_MS,
};

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum Kind {
    Value,
    Text,
    Ticker,
    Bar,
    Ring,
    Pie,
    Line,
    Clock,
    Status,
    Image,
    Arrow,
    Rectangle,
    RoundedRectangle,
    Circle,
    Oval,
    Polygon,
    StraightLine,
}
impl Kind {
    pub const ALL: [Kind; 17] = [
        Self::Value,
        Self::Text,
        Self::Ticker,
        Self::Bar,
        Self::Ring,
        Self::Pie,
        Self::Line,
        Self::Clock,
        Self::Status,
        Self::Image,
        Self::Arrow,
        Self::Rectangle,
        Self::RoundedRectangle,
        Self::Circle,
        Self::Oval,
        Self::Polygon,
        Self::StraightLine,
    ];
    pub fn label(&self) -> &'static str {
        match self {
            Self::Value => "Data value",
            Self::Text => "Text",
            Self::Ticker => "Ticker",
            Self::Bar => "Data bar",
            Self::Ring => "Arc gauge",
            Self::Pie => "Pie chart",
            Self::Line => "Line chart",
            Self::Clock => "Clock",
            Self::Status => "Status light",
            Self::Image => "Image",
            Self::Arrow => "Trend arrow",
            Self::Rectangle => "Rectangle",
            Self::RoundedRectangle => "Rounded rectangle",
            Self::Circle => "Circle",
            Self::Oval => "Oval",
            Self::Polygon => "Polygon",
            Self::StraightLine => "Line",
        }
    }
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Value => "12",
            Self::Text => "T",
            Self::Ticker => "T↔",
            Self::Bar => "▰",
            Self::Ring => "◉",
            Self::Pie => "◕",
            Self::Line => "⌁",
            Self::Clock => "◷",
            Self::Status => "●",
            Self::Image => "▧",
            Self::Arrow => "↕",
            Self::Rectangle => "▭",
            Self::RoundedRectangle => "▢",
            Self::Circle => "○",
            Self::Oval => "⬭",
            Self::Polygon => "⬡",
            Self::StraightLine => "╱",
        }
    }
    pub fn is_shape(&self) -> bool {
        matches!(
            self,
            Self::Rectangle
                | Self::RoundedRectangle
                | Self::Circle
                | Self::Oval
                | Self::Polygon
                | Self::StraightLine
        )
    }
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Widget {
    pub id: u64,
    pub name: String,
    pub kind: Kind,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub label: String,
    pub text: String,
    pub binding: String,
    pub unit: String,
    #[serde(default)]
    pub unit_prefix: String,
    #[serde(default)]
    pub ticker_bound: bool,
    #[serde(default)]
    pub ticker_sources: Vec<TickerSource>,
    #[serde(default)]
    pub ticker_circular: bool,
    #[serde(default = "default_scroll_speed")]
    pub scroll_speed: f32,
    #[serde(default)]
    pub scroll_direction: ScrollDirection,
    pub color: [u8; 4],
    pub background: [u8; 4],
    pub size: f32,
    #[serde(default)]
    pub letter_spacing: f32,
    pub min: f64,
    pub max: f64,
    pub visible: bool,
    pub locked: bool,
    pub show_label: bool,
    pub image: Option<String>,
    #[serde(default = "default_font")]
    pub font: String,
    #[serde(default)]
    pub font_data: Option<String>,
    #[serde(default = "default_radius")]
    pub corner_radius: f32,
    #[serde(default = "default_sides")]
    pub sides: u8,
    #[serde(default)]
    pub decimals: Option<u8>,
    #[serde(default = "default_ring_thickness")]
    pub ring_thickness: f32,
    #[serde(default = "default_ring_start")]
    pub ring_start: f32,
    #[serde(default)]
    pub ring_label_inside: bool,
    // Original imported ArchBar geometry and segment quantization.
    #[serde(default)]
    pub legacy_arc: bool,
    // ArchBar.useInternal: leave segments 0, 4, ..., 96 in the track color.
    #[serde(default)]
    pub legacy_arc_gaps: bool,
    #[serde(default)]
    pub clock_format: String,
    #[serde(default = "default_true")]
    pub fill_enabled: bool,
    #[serde(default)]
    pub outline_enabled: bool,
    #[serde(default = "default_outline_color")]
    pub outline_color: [u8; 4],
    #[serde(default = "default_stroke_width")]
    pub stroke_width: f32,
    #[serde(default)]
    pub line_reverse: bool,
    #[serde(default)]
    pub rotation_degrees: f32,
    #[serde(default)]
    pub line_direction: LineDirection,
    #[serde(default)]
    pub legacy_text: Option<LegacyText>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct LegacyText {
    pub cell_width: f32,
    pub cell_height: f32,
    pub advance: f32,
    #[serde(default)]
    pub base_size: f32,
}
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum LineDirection {
    #[default]
    Diagonal,
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum ScrollDirection {
    #[default]
    RightToLeft,
    LeftToRight,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TickerSource {
    pub binding: String,
    #[serde(default)]
    pub unit_prefix: String,
    #[serde(default)]
    pub unit_suffix: String,
    #[serde(default)]
    pub decimals: Option<u8>,
}
impl TickerSource {
    pub fn is_numeric(&self, data: &DataState) -> bool {
        SYSTEM_FIELDS.iter().any(|(_, key)| *key == self.binding)
            || self.decimals.is_some()
            || data
                .values
                .get(&self.binding)
                .is_some_and(|v| v.parse::<f64>().is_ok_and(f64::is_finite))
    }
    pub fn display(&self, data: &DataState) -> String {
        let raw = data
            .values
            .get(&self.binding)
            .map(String::as_str)
            .unwrap_or("N/A");
        let value = match (self.decimals, raw.parse::<f64>()) {
            (Some(places), Ok(n)) if n.is_finite() => format!("{:.*}", places.min(6) as usize, n),
            _ => raw.to_owned(),
        };
        format!("{}{}{}", self.unit_prefix, value, self.unit_suffix)
    }
}
fn default_serial_baud() -> u32 {
    SERIAL_BAUD
}
pub const SERIAL_SPEEDS: &[u32] = &[115200, 230400, 460800, 921600, 1_000_000, 2_000_000];
fn default_scroll_speed() -> f32 {
    5.
}
fn default_true() -> bool {
    true
}
fn default_outline_color() -> [u8; 4] {
    [231, 241, 240, 255]
}
fn default_stroke_width() -> f32 {
    1.
}
fn default_ring_thickness() -> f32 {
    5.
}
fn default_ring_start() -> f32 {
    -90.
}
fn default_font() -> String {
    "Segoe UI".into()
}
fn default_radius() -> f32 {
    12.
}
fn default_sides() -> u8 {
    6
}
impl Widget {
    pub fn resize_text(&mut self, size: f32) {
        let size = size.clamp(6., 150.);
        if let Some(metrics) = &mut self.legacy_text {
            if metrics.base_size <= 0. {
                metrics.base_size = self.size;
            }
            if matches!(self.kind, Kind::Value | Kind::Text | Kind::Clock) {
                let ratio = size / self.size.max(1.);
                let right = self.x + self.w;
                self.w = (self.w * ratio).clamp(1., 2048.);
                self.h = (self.h * ratio).clamp(1., 2048.);
                if matches!(self.kind, Kind::Value | Kind::Clock) {
                    self.x = right - self.w;
                }
            }
        }
        self.size = size;
    }
    pub fn new(id: u64, kind: Kind, x: f32, y: f32) -> Self {
        Self {
            id,
            name: kind.label().into(),
            kind,
            x,
            y,
            w: 130.,
            h: 58.,
            label: "NEW FIELD".into(),
            text: "Your text".into(),
            binding: String::new(),
            unit: String::new(),
            unit_prefix: String::new(),
            ticker_bound: false,
            ticker_sources: vec![],
            ticker_circular: false,
            scroll_speed: default_scroll_speed(),
            scroll_direction: ScrollDirection::default(),
            color: [98, 225, 187, 255],
            background: [20, 42, 48, 255],
            size: 25.,
            letter_spacing: 0.,
            min: 0.,
            max: 100.,
            visible: true,
            locked: false,
            show_label: true,
            image: None,
            font: default_font(),
            font_data: None,
            corner_radius: default_radius(),
            sides: default_sides(),
            decimals: None,
            ring_thickness: default_ring_thickness(),
            ring_start: default_ring_start(),
            ring_label_inside: false,
            legacy_arc: false,
            legacy_arc_gaps: false,
            clock_format: String::new(),
            fill_enabled: true,
            outline_enabled: false,
            outline_color: default_outline_color(),
            stroke_width: default_stroke_width(),
            line_reverse: false,
            rotation_degrees: 0.,
            line_direction: LineDirection::Horizontal,
            legacy_text: None,
        }
    }
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum Auth {
    None,
    #[default]
    Bearer,
    Basic,
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum SourceKind {
    Json,
    HomeAssistant,
    Weather,
    Stock,
}
impl SourceKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Json => "HTTP / HTTPS JSON",
            Self::HomeAssistant => "Home Assistant",
            Self::Weather => "Weather",
            Self::Stock => "Stock ticker",
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: u64,
    pub name: String,
    pub kind: SourceKind,
    pub url: String,
    #[serde(skip, default)]
    pub token: String,
    #[serde(skip, default)]
    pub username: String,
    #[serde(skip, default)]
    pub password: String,
    #[serde(default)]
    pub auth: Auth,
    #[serde(default)]
    pub credential_id: String,
    pub interval: u64,
    pub latitude: String,
    pub longitude: String,
    pub symbol: String,
    #[serde(default)]
    pub stocks: Vec<String>,
}
impl Source {
    pub fn stock_symbols(&self) -> Vec<String> {
        if self.stocks.is_empty() {
            self.symbol
                .split(',')
                .map(|s| s.trim().to_uppercase())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            self.stocks.clone()
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub name: String,
    pub portrait: bool,
    pub widgets: Vec<Widget>,
    pub wallpaper: Option<String>,
    pub background: [u8; 4],
    pub sources: Vec<Source>,
    pub grid: bool,
    pub grid_size: f32,
    pub rotation: bool,
    pub brightness: u8,
    #[serde(default = "default_serial_baud")]
    pub serial_baud: u32,
    // Older Cockpit JSON called opcode 122 "invert"; the original calls it mirrorMode.
    #[serde(default, alias = "invert")]
    pub mirror_mode: bool,
    #[serde(default)]
    pub display_v2: bool,
    #[serde(default)]
    pub wallpaper_rotation: u8,
    #[serde(default)]
    pub wallpaper_mirror: bool,
}
impl Project {
    pub fn dimensions(&self) -> (u32, u32) {
        if self.portrait {
            (SCREEN_H, SCREEN_W)
        } else {
            (SCREEN_W, SCREEN_H)
        }
    }
    pub fn blank() -> Self {
        Self {
            version: 2,
            name: "Untitled cockpit".into(),
            portrait: false,
            widgets: vec![],
            wallpaper: None,
            background: [10, 25, 31, 255],
            sources: vec![],
            grid: false,
            grid_size: 8.,
            rotation: false,
            brightness: 65,
            serial_baud: SERIAL_BAUD,
            mirror_mode: false,
            display_v2: false,
            wallpaper_rotation: 0,
            wallpaper_mirror: false,
        }
    }
    pub fn starter() -> Self {
        let mut p = Self::blank();
        p.name = "Everyday cockpit".into();
        let mut title = Widget::new(1, Kind::Text, 20., 14.);
        title.text = "MISSION CONTROL".into();
        title.size = 12.;
        title.color = [130, 161, 168, 255];
        title.show_label = false;
        title.w = 220.;
        title.h = 20.;
        p.widgets.push(title);
        let mut clock = Widget::new(2, Kind::Clock, 328., 12.);
        clock.w = 132.;
        clock.h = 25.;
        clock.show_label = false;
        clock.size = 18.;
        clock.color = [221, 237, 235, 255];
        p.widgets.push(clock);
        for (id, x, label, binding, color) in [
            (3, 20., "PROCESSOR", "cpu.usage", [98, 225, 187, 255]),
            (4, 177., "MEMORY", "memory.usage", [108, 183, 248, 255]),
            (5, 334., "DISK", "disk.usage", [197, 159, 243, 255]),
        ] {
            let mut w = Widget::new(id, Kind::Ring, x, 60.);
            w.w = 126.;
            w.h = 126.;
            w.label = label.into();
            w.binding = binding.into();
            w.color = color;
            w.unit = "%".into();
            w.size = 27.;
            p.widgets.push(w);
        }
        let mut w = Widget::new(6, Kind::Line, 20., 210.);
        w.w = 282.;
        w.h = 88.;
        w.label = "CPU HISTORY".into();
        w.binding = "cpu.usage".into();
        p.widgets.push(w);
        let mut w = Widget::new(7, Kind::Value, 327., 218.);
        w.w = 132.;
        w.h = 70.;
        w.label = "NETWORK ↓".into();
        w.binding = "network.download".into();
        w.unit = "MB/s".into();
        w.size = 23.;
        w.color = [234, 242, 244, 255];
        p.widgets.push(w);
        p
    }
    pub fn landscape_technology() -> Self {
        serde_json::from_str(include_str!("../designs/LandscapeTechnologyCh.json"))
            .expect("bundled LandscapeTechnologyCh design must be valid")
    }
}
#[derive(Clone, Default)]
pub struct DataState {
    pub values: BTreeMap<String, String>,
    pub histories: BTreeMap<String, Vec<f64>>,
    pub errors: BTreeMap<u64, String>,
    pub last_refresh: String,
    pub device: String,
    pub connected: bool,
    pub port: String,
    pub streaming: bool,
    pub unavailable: BTreeMap<String, String>,
    pub sensor_status: String,
    pub display_fps: f64,
}
impl DataState {
    pub fn value(&self, w: &Widget) -> String {
        if w.binding.is_empty() {
            return w.text.clone();
        }
        let value = self
            .values
            .get(&w.binding)
            .cloned()
            .unwrap_or_else(|| "N/A".into());
        // Original network sampler RVA 0x113dc (VA 0x4113dc),
        // IL_0425..0457: bytes/second / 1024 / 1024. Shared bindings are MB/s;
        // FontConfig metrics also identify imports saved in older JSON files.
        if w.legacy_text.is_some()
            && matches!(w.binding.as_str(), "network.download" | "network.upload")
        {
            if let Ok(rate) = value.parse::<f64>() {
                if rate.is_finite() {
                    return crate::original::network_mibps(rate).to_string();
                }
            }
        }
        // Original postprocessor RVA 0x15b78 caps these fields at 99.
        // FontConfig metrics / ArchBar flag identify legacy imports, including
        // saved JSON from earlier versions. Keep the shared readings unchanged.
        if (w.legacy_text.is_some() || w.legacy_arc)
            && matches!(
                w.binding.as_str(),
                "cpu.usage"
                    | "cpu.temperature"
                    | "gpu.usage"
                    | "gpu.temperature"
                    | "gpu.memory.usage"
                    | "memory.usage"
                    | "volume.level"
            )
            && value
                .parse::<f64>()
                .is_ok_and(|v| v.is_finite() && v > crate::original::SENSOR_CEILING)
        {
            crate::original::SENSOR_CEILING.to_string()
        } else {
            value
        }
    }
    pub fn number(&self, w: &Widget) -> Option<f64> {
        self.value(w).parse::<f64>().ok()
    }
}

pub const SYSTEM_FIELDS: &[(&str, &str)] = &[
    ("Processor usage", "cpu.usage"),
    ("CPU frequency", "cpu.frequency"),
    ("CPU temperature", "cpu.temperature"),
    ("CPU fan", "cpu.fan"),
    ("CPU power", "cpu.power"),
    ("Memory usage", "memory.usage"),
    ("Used memory", "memory.used"),
    ("Disk usage", "disk.usage"),
    ("Disk temperature", "disk.temperature"),
    ("Download speed", "network.download"),
    ("Upload speed", "network.upload"),
    ("GPU usage", "gpu.usage"),
    ("GPU temperature", "gpu.temperature"),
    ("Used video memory", "gpu.memory"),
    ("Video memory usage", "gpu.memory.usage"),
    ("GPU fan", "gpu.fan"),
    ("GPU power", "gpu.power"),
    ("Audio volume", "volume.level"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_examples_load_render_and_contain_no_credentials() {
        for (name, json) in [
            ("landscape", include_str!("../config/landscape.json")),
            ("portrait", include_str!("../config/portrait.json")),
            ("ticker", include_str!("../config/ticker.json")),
            (
                "connected sources",
                include_str!("../config/connected-sources.json"),
            ),
        ] {
            let project: Project = serde_json::from_str(json).expect(name);
            assert!(!project.widgets.is_empty(), "{name}");
            let image = crate::render::frame(&project, &DataState::default());
            assert_eq!(image.dimensions(), project.dimensions(), "{name}");
            for source in project.sources {
                assert!(source.token.is_empty() && source.password.is_empty());
                assert!(source.username.is_empty() && source.credential_id.is_empty());
            }
        }
    }

    #[test]
    fn circular_ticker_is_saved_and_defaults_off_for_older_designs() {
        let mut project = Project::blank();
        project.widgets.push(Widget::new(1, Kind::Ticker, 0., 0.));
        assert!(!project.widgets[0].ticker_circular);
        project.widgets[0].ticker_circular = true;
        let mut saved = serde_json::to_value(&project).unwrap();
        let restored: Project = serde_json::from_value(saved.clone()).unwrap();
        assert!(restored.widgets[0].ticker_circular);
        saved["widgets"][0]
            .as_object_mut()
            .unwrap()
            .remove("ticker_circular");
        let older: Project = serde_json::from_value(saved).unwrap();
        assert!(!older.widgets[0].ticker_circular);
    }

    #[test]
    fn ticker_precision_is_per_source_and_survives_missing_or_text_values() {
        let mut data = DataState::default();
        let mut source: TickerSource =
            serde_json::from_str(r#"{"binding":"quote","unit_prefix":"$","unit_suffix":" USD"}"#)
                .unwrap();
        assert_eq!(source.decimals, None);
        for (raw, zero, two) in [
            ("12.5678", "$13 USD", "$12.57 USD"),
            ("-2.1251", "$-2 USD", "$-2.13 USD"),
            ("0", "$0 USD", "$0.00 USD"),
        ] {
            data.values.insert("quote".into(), raw.into());
            assert!(source.is_numeric(&data));
            source.decimals = Some(0);
            assert_eq!(source.display(&data), zero);
            source.decimals = Some(2);
            assert_eq!(source.display(&data), two);
            let restored: TickerSource =
                serde_json::from_str(&serde_json::to_string(&source).unwrap()).unwrap();
            assert_eq!(restored.display(&data), two);
            let mut other = source.clone();
            other.decimals = Some(4);
            assert_eq!(source.decimals, Some(2));
            assert_ne!(other.display(&data), two);
        }
        for text in ["Sunny", "N/A", "NaN", "inf", "2026-11-04"] {
            data.values.insert("quote".into(), text.into());
            assert_eq!(source.display(&data), format!("${text} USD"));
        }
        data.values.clear();
        assert_eq!(source.display(&data), "$N/A USD");
        source.decimals = None;
        assert!(!source.is_numeric(&data));
        source.binding = "cpu.temperature".into();
        assert!(source.is_numeric(&data));
        let mut old = serde_json::to_value(Project::blank()).unwrap();
        old.as_object_mut().unwrap().remove("serial_baud");
        let restored: Project = serde_json::from_value(old).unwrap();
        assert_eq!(restored.serial_baud, SERIAL_BAUD);
        for &speed in SERIAL_SPEEDS {
            let mut p = Project::blank();
            p.serial_baud = speed;
            let restored: Project =
                serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
            assert_eq!(restored.serial_baud, speed);
        }
    }

    #[test]
    fn imported_network_sampler_normalizes_without_changing_shared_readings() {
        let mut w = Widget::new(1, Kind::Value, 0., 0.);
        w.legacy_text = Some(LegacyText {
            cell_width: 16.,
            cell_height: 28.,
            advance: 15.,
            base_size: w.size,
        });
        let mut d = DataState::default();
        for key in ["network.download", "network.upload"] {
            w.binding = key.into();
            for (raw_mbps, normalized_mibps) in [
                ("-0.01048576", 0.),
                ("0", 0.),
                ("0.000001", 0.1), // One physical byte per second.
                ("0.01048576", 0.1),
                ("0.1048576", 0.1),
                ("0.131072", 0.125),
                ("1.048576", 1.),
            ] {
                d.values.insert(key.into(), raw_mbps.into());
                assert!(
                    (d.number(&w).unwrap() - normalized_mibps).abs() < 1e-12,
                    "{key}: {raw_mbps}MB/s must normalize to {normalized_mibps}MiB/s"
                );
                let restored: Widget =
                    serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
                assert_eq!(d.value(&restored), d.value(&w));
                let mut ordinary = w.clone();
                ordinary.legacy_text = None;
                assert_eq!(d.value(&ordinary), raw_mbps);
                assert_eq!(d.values[key], raw_mbps);
            }
            for text in ["N/A", "NaN", "inf"] {
                d.values.insert(key.into(), text.into());
                assert_eq!(d.value(&w), text);
            }
            d.values.remove(key);
            assert_eq!(d.value(&w), "N/A");
        }
    }

    #[test]
    fn original_99_ceiling_is_scoped_to_imported_sensors() {
        let mut w = Widget::new(1, Kind::Value, 0., 0.);
        let mut d = DataState::default();
        for key in [
            "cpu.usage",
            "cpu.temperature",
            "gpu.usage",
            "gpu.temperature",
            "gpu.memory.usage",
            "memory.usage",
            "volume.level",
        ] {
            w.binding = key.into();
            for (raw, expected) in [
                ("98.75", "98.75"),
                ("99", "99"),
                ("99.25", "99"),
                ("100", "99"),
                ("N/A", "N/A"),
            ] {
                d.values.insert(key.into(), raw.into());
                w.legacy_arc = true;
                assert_eq!(d.value(&w), expected, "{key}: {raw}");
                w.legacy_arc = false;
                assert_eq!(d.value(&w), raw);
                assert_eq!(d.values[key], raw);
            }
            d.values.remove(key);
            w.legacy_arc = true;
            assert_eq!(d.value(&w), "N/A");
            assert_eq!(d.number(&w), None);
        }
        for key in [
            "cpu.frequency",
            "cpu.fan",
            "cpu.power",
            "gpu.memory",
            "gpu.fan",
            "disk.usage",
            "disk.temperature",
            "memory.used",
            "network.download",
            "source:1:/value",
        ] {
            w.binding = key.into();
            d.values.insert(key.into(), "125".into());
            assert_eq!(d.value(&w), "125", "{key}");
        }
        w.binding.clear();
        w.text = "100".into();
        assert_eq!(d.value(&w), "100");
    }
}
