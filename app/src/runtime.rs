#[cfg(test)]
use crate::original::{command_header as packet, BRIGHTNESS_OPCODE, IMAGE_OPCODE};
use crate::{model::*, original, render};
use serde_json::Value;
#[cfg(test)]
use std::io::Write;
use std::{
    collections::BTreeMap,
    io::Read,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender, SyncSender},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use sysinfo::{Disks, Networks, System};

pub enum Command {
    Project(Project),
    Port(String),
    Upload,
    Stream(bool),
    Refresh,
    Stop,
}
pub struct Runtime {
    pub tx: Sender<Command>,
    pub rx: Receiver<DataState>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Runtime {
    pub fn start(p: Project, ctx: eframe::egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        let (out, updates) = mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let cancelled = stop.clone();
        let worker = Some(thread::spawn(move || run(p, rx, out, cancelled, ctx)));
        Self {
            tx,
            rx: updates,
            stop,
            worker,
        }
    }
}
impl Runtime {
    pub fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.tx.send(Command::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Only select display-specific USB identities. Other serial devices require explicit selection.
pub fn detect_port(ports: &[serialport::SerialPortInfo], preferred: &str) -> Option<String> {
    if !preferred.is_empty() {
        return ports
            .iter()
            .find(|p| p.port_name == preferred)
            // The FTDI adapter belongs to the independent uMonitorX2 application.
            .filter(|p| !matches!(&p.port_type, serialport::SerialPortType::UsbPort(info) if info.vid == 0x0403 && info.pid == 0x6001))
            .map(|p| p.port_name.clone());
    }
    let matches: Vec<_> = ports
        .iter()
        .filter(|p| match &p.port_type {
            serialport::SerialPortType::UsbPort(info) => {
                let product = info.product.as_deref().unwrap_or("").to_ascii_lowercase();
                (info.vid == 0x1a86 && info.pid == 0x5722)
                    || [
                        "usb35",
                        "usb 35",
                        "turing",
                        "usbmonitor",
                        "3.5-inch display",
                    ]
                    .iter()
                    .any(|name| product.contains(name))
            }
            _ => false,
        })
        .collect();
    if matches.len() == 1 {
        Some(matches[0].port_name.clone())
    } else {
        None
    }
}
pub fn flatten(v: &Value, path: &str, out: &mut BTreeMap<String, String>) {
    match v {
        Value::Object(m) => {
            for (k, v) in m {
                flatten(
                    v,
                    &format!("{}/{}", path, k.replace('~', "~0").replace('/', "~1")),
                    out,
                )
            }
        }
        Value::Array(a) => {
            for (i, v) in a.iter().take(5000).enumerate() {
                flatten(v, &format!("{}/{}", path, i), out)
            }
        }
        Value::Null => {
            out.insert(path.into(), "null".into());
        }
        _ => {
            out.insert(
                path.into(),
                v.as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| v.to_string()),
            );
        }
    }
}
fn fetch(
    s: &Source,
    client: &reqwest::blocking::Client,
) -> Result<BTreeMap<String, String>, String> {
    if s.kind == SourceKind::Stock {
        if s.token.trim().is_empty() {
            return Err("Enter your Alpha Vantage API key, then connect.".into());
        }
        let symbols = s.stock_symbols();
        if symbols.is_empty() {
            return Err("Search for a stock and add it to your watchlist.".into());
        }
        let mut fields = BTreeMap::new();
        for symbol in symbols {
            let response = client
                .get(crate::stocks::URL)
                .query(&[
                    ("function", "GLOBAL_QUOTE"),
                    ("symbol", symbol.as_str()),
                    ("apikey", s.token.trim()),
                ])
                .send()
                .map_err(|_| {
                    "Could not connect to Alpha Vantage. Check your internet connection."
                })?;
            if !response.status().is_success() {
                return Err(format!(
                    "Alpha Vantage returned {} for {symbol}. Check your API key and plan limits.",
                    response.status()
                ));
            }
            let quote: Value = response
                .json()
                .map_err(|_| "Alpha Vantage returned an unreadable quote.")?;
            fields.extend(stock_fields(s.id, &symbol, &quote)?);
        }
        return Ok(fields);
    }
    if s.kind == SourceKind::Weather {
        let lat: f64 = s
            .latitude
            .trim()
            .parse()
            .map_err(|_| "Enter a valid latitude, or search for a location.")?;
        let lon: f64 = s
            .longitude
            .trim()
            .parse()
            .map_err(|_| "Enter a valid longitude, or search for a location.")?;
        if !lat.is_finite()
            || !lon.is_finite()
            || !(-90.0..=90.0).contains(&lat)
            || !(-180.0..=180.0).contains(&lon)
        {
            return Err("Latitude must be −90 to 90, longitude −180 to 180.".into());
        }
    }
    let mut q = match s.kind {
        SourceKind::HomeAssistant => {
            client.get(format!("{}/api/states", s.url.trim_end_matches('/')))
        }
        SourceKind::Weather => client
            .get("https://api.open-meteo.com/v1/forecast")
            .query(&[
                ("latitude", s.latitude.trim()),
                ("longitude", s.longitude.trim()),
                (
                    "current",
                    "temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m",
                ),
            ]),
        SourceKind::Stock => unreachable!(),
        SourceKind::Json => client.get(&s.url),
    };
    if matches!(s.kind, SourceKind::Json | SourceKind::HomeAssistant) {
        q = match s.auth {
            Auth::None => q,
            Auth::Bearer if !s.token.is_empty() => q.bearer_auth(&s.token),
            Auth::Basic => q.basic_auth(&s.username, Some(&s.password)),
            _ => q,
        };
    }
    let response = q.send().map_err(|_| {
        "Could not reach this source. Check its address and connection.".to_string()
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Server returned {}", status));
    }
    if response.content_length().unwrap_or(0) > 4_000_000 {
        return Err("Response exceeds the 4 MB limit".into());
    }
    let mut bytes = vec![];
    std::io::Read::take(response, 4_000_001)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4_000_000 {
        return Err("Response exceeds the 4 MB limit".into());
    }
    let v: Value = serde_json::from_slice(&bytes)
        .map_err(|_| "This source did not return JSON".to_string())?;
    let prefix = format!("source:{}:", s.id);
    let mut fields = BTreeMap::new();
    if s.kind == SourceKind::HomeAssistant {
        for state in v.as_array().ok_or("Expected Home Assistant state list")? {
            if let Some(id) = state["entity_id"].as_str() {
                flatten(state, &format!("{}/{}", prefix, id), &mut fields);
            }
        }
    } else {
        flatten(&v, &prefix, &mut fields);
    }
    if fields.is_empty() {
        return Err("The source returned no fields".into());
    }
    Ok(fields)
}
fn stock_fields(id: u64, symbol: &str, quote: &Value) -> Result<BTreeMap<String, String>, String> {
    crate::stocks::check_response(quote)?;
    let quote = &quote["Global Quote"];
    if quote["05. price"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .is_none_or(|n| !n.is_finite() || n <= 0.)
        || quote["07. latest trading day"]
            .as_str()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .is_none()
    {
        return Err(format!(
            "Alpha Vantage has no quote for {symbol}. Check the symbol and your plan."
        ));
    }
    let mut fields = BTreeMap::new();
    for (key, label) in [
        ("05. price", "price"),
        ("09. change", "change"),
        ("10. change percent", "change_percent"),
        ("03. high", "high"),
        ("04. low", "low"),
        ("02. open", "open"),
        ("08. previous close", "previous_close"),
        ("06. volume", "volume"),
        ("07. latest trading day", "latest_trading_day"),
    ] {
        if let Some(value) = quote.get(key).and_then(Value::as_str) {
            let value = value.trim_end_matches('%');
            if label == "latest_trading_day" || value.parse::<f64>().is_ok_and(|n| n.is_finite()) {
                fields.insert(format!("source:{id}:/{symbol}/{label}"), value.to_owned());
            }
        }
    }
    // Compatibility for existing bindings: date only, not an invented intraday time.
    fields.insert(
        format!("source:{id}:/{symbol}/timestamp"),
        quote["07. latest trading day"].as_str().unwrap().into(),
    );
    Ok(fields)
}
#[derive(Default)]
struct UploadedFrame {
    pixels: Vec<u8>,
    settings: Option<[u32; 7]>,
}
/// Trim tile padding and combine nearby patches when one command costs less
/// than multiple commands plus the display's mandatory processing waits.
fn transfer_regions(
    previous: &[u8],
    pixels: &[u8],
    w: u32,
    h: u32,
    command_cost: u64,
) -> Vec<render::Region> {
    if previous.len() != pixels.len() || previous.is_empty() {
        return vec![render::Region { x: 0, y: 0, w, h }];
    }
    let mut regions: Vec<render::Region> = render::changed_regions(previous, pixels, w, h, 2)
        .into_iter()
        .map(|r| {
            let (mut left, mut top, mut right, mut bottom) = (r.x + r.w, r.y + r.h, r.x, r.y);
            for y in r.y..r.y + r.h {
                for x in r.x..r.x + r.w {
                    let i = ((y * w + x) * 2) as usize;
                    if previous[i..i + 2] != pixels[i..i + 2] {
                        left = left.min(x);
                        top = top.min(y);
                        right = right.max(x);
                        bottom = bottom.max(y);
                    }
                }
            }
            render::Region {
                x: left,
                y: top,
                w: right - left + 1,
                h: bottom - top + 1,
            }
        })
        .collect();
    loop {
        let mut best = None;
        let mut saving = 0;
        for i in 0..regions.len() {
            for j in i + 1..regions.len() {
                let (a, b) = (regions[i], regions[j]);
                let x = a.x.min(b.x);
                let y = a.y.min(b.y);
                let union = render::Region {
                    x,
                    y,
                    w: (a.x + a.w).max(b.x + b.w) - x,
                    h: (a.y + a.h).max(b.y + b.h) - y,
                };
                let separate_cost =
                    (a.w as u64 * a.h as u64 + b.w as u64 * b.h as u64) * 2 + command_cost;
                let joined_cost = union.w as u64 * union.h as u64 * 2;
                if separate_cost > joined_cost && separate_cost - joined_cost > saving {
                    saving = separate_cost - joined_cost;
                    best = Some((i, j, union));
                }
            }
        }
        let Some((i, j, union)) = best else {
            break;
        };
        regions[i] = union;
        regions.remove(j);
    }
    regions
}
#[cfg(test)]
fn upload_with_delay(
    serial: &mut dyn std::io::Write,
    p: &Project,
    d: &DataState,
    stop: &AtomicBool,
    delay: impl FnMut(Duration),
) -> Result<(), String> {
    upload_frame(
        serial,
        p,
        &render::frame(p, d),
        &mut UploadedFrame::default(),
        true,
        stop,
        delay,
    )
    .map(|_| ())
}
fn upload_frame(
    serial: &mut dyn std::io::Write,
    p: &Project,
    frame: &image::RgbaImage,
    previous: &mut UploadedFrame,
    force: bool,
    stop: &AtomicBool,
    mut delay: impl FnMut(Duration),
) -> Result<bool, String> {
    if stop.load(Ordering::Acquire) {
        return Err("Application is exiting".into());
    }
    let (w, h) = p.dimensions();
    let settings = [
        w,
        h,
        p.rotation as u32,
        p.mirror_mode as u32,
        p.brightness.min(100) as u32,
        p.display_v2 as u32,
        0,
    ];
    let reset = force
        || previous
            .settings
            .is_none_or(|old| old[..4] != settings[..4] || old[5] != settings[5]);
    if reset {
        serial
            .write_all(&original::mirror_packet(p.mirror_mode))
            .map_err(|e| e.to_string())?;
        delay(Duration::from_millis(COMMAND_DELAY_MS));
        serial
            .write_all(&original::orientation_packet(
                p.portrait, p.rotation, w as u16, h as u16,
            ))
            .map_err(|e| e.to_string())?;
        delay(Duration::from_millis(COMMAND_DELAY_MS));
    }
    let brightness_changed = reset || previous.settings.is_none_or(|old| old[4] != settings[4]);
    if brightness_changed {
        serial
            .write_all(&original::brightness_header(
                ((100 - p.brightness.min(100)) as u16 * 255 / 100) as u8,
            ))
            .map_err(|e| e.to_string())?;
    }
    let pixels = render::rgb565(frame);
    let command_cost = original::COMMAND_HEADER_LEN as u64
        + if p.display_v2 {
            0
        } else {
            p.serial_baud as u64 * (2 * COMMAND_DELAY_MS) / 10_000
        };
    let regions = transfer_regions(
        if reset { &[] } else { &previous.pixels },
        &pixels,
        w,
        h,
        command_cost,
    );
    let changed = reset || brightness_changed || !regions.is_empty();
    for region in regions {
        if stop.load(Ordering::Acquire) {
            return Err("Application is exiting".into());
        }
        serial
            .write_all(&original::image_header(
                region.x as u16,
                region.y as u16,
                region.w as u16,
                region.h as u16,
            ))
            .map_err(|e| e.to_string())?;
        if !p.display_v2 {
            delay(Duration::from_millis(COMMAND_DELAY_MS));
        }
        let mut bytes = Vec::with_capacity((region.w * region.h * 2) as usize);
        for y in region.y..region.y + region.h {
            let offset = ((y * w + region.x) * 2) as usize;
            bytes.extend_from_slice(&pixels[offset..offset + (region.w * 2) as usize]);
        }
        for chunk in bytes.chunks(4096) {
            if stop.load(Ordering::Acquire) {
                return Err("Application is exiting".into());
            }
            serial.write_all(chunk).map_err(|e| e.to_string())?;
        }
        if !p.display_v2 {
            delay(Duration::from_millis(COMMAND_DELAY_MS));
        }
    }
    if changed {
        serial.flush().map_err(|e| e.to_string())?;
    }
    previous.pixels = pixels;
    previous.settings = Some(settings);
    Ok(changed)
}
fn source_signature(sources: &[Source]) -> Vec<u8> {
    let mut signature = serde_json::to_vec(sources).unwrap_or_default();
    for source in sources {
        signature.extend(source.token.bytes());
        signature.extend(source.username.bytes());
        signature.extend(source.password.bytes());
    }
    signature
}
enum SampleCommand {
    Sources(Vec<Source>),
    Refresh,
    Stop,
}
// Blocking providers never share the display's rendering/serial thread.
fn sample_loop(
    mut sources: Vec<Source>,
    rx: Receiver<SampleCommand>,
    out: SyncSender<DataState>,
    stop: Arc<AtomicBool>,
) {
    let mut sys = System::new_all();
    let mut disks = Disks::new_with_refreshed_list();
    let mut nets = Networks::new_with_refreshed_list();
    let mut local_sensors = crate::sensors::LocalSensors::new();
    let mut sysinfo_sensors = crate::sensors::SysinfoSensors::new();
    #[cfg(windows)]
    let mut direct_sensors = crate::hwinfo::DirectSensors::new();
    let mut hardware_tick = Instant::now() - Duration::from_secs(10);
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("uMonitor/0.1.0")
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut due: BTreeMap<u64, Instant> = BTreeMap::new();
    let mut d = DataState::default();
    let mut tick = Instant::now() - Duration::from_secs(2);
    let mut next_sample = Instant::now();
    while !stop.load(Ordering::Acquire) {
        match rx.recv_timeout(next_sample.saturating_duration_since(Instant::now())) {
            Ok(SampleCommand::Sources(new)) => {
                sources = new;
                due.clear();
                next_sample = Instant::now();
            }
            Ok(SampleCommand::Refresh) => {
                due.clear();
                next_sample = Instant::now();
            }
            Ok(SampleCommand::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if Instant::now() < next_sample {
            continue;
        }
        let seconds = tick.elapsed().as_secs_f64().max(0.1);
        tick = Instant::now();
        #[cfg(not(windows))]
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        sys.refresh_cpu_frequency();
        disks.refresh(true);
        nets.refresh(true);
        let mut add = |key: &str, n: f64| {
            d.values.insert(key.into(), format!("{:.1}", n));
        };
        #[cfg(not(windows))]
        add("cpu.usage", sys.global_cpu_usage() as f64);
        add(
            "memory.usage",
            sys.used_memory() as f64 / sys.total_memory().max(1) as f64 * 100.,
        );
        add("memory.used", sys.used_memory() as f64 / 1_073_741_824.);
        if let Some(disk) = disks
            .list()
            .iter()
            .find(|x| x.mount_point().to_string_lossy().starts_with("C:"))
            .or_else(|| disks.list().first())
        {
            add(
                "disk.usage",
                (disk.total_space() - disk.available_space()) as f64
                    / disk.total_space().max(1) as f64
                    * 100.,
            );
        }
        let download: u64 = nets.values().map(|n| n.received()).sum();
        let transmitted: u64 = nets.values().map(|n| n.transmitted()).sum();
        network_readings(download, transmitted, seconds, &mut d);
        // Keep MHz precision in the GHz binding for original 8000-MHz bars.
        // Widget decimal settings still control the visible text.
        d.values.insert(
            "cpu.frequency".into(),
            format!(
                "{:.3}",
                sys.cpus().first().map(|c| c.frequency()).unwrap_or(0) as f64 / 1000.
            ),
        );
        local_sensors.refresh(&mut d);
        sysinfo_sensors.refresh(&mut d);
        #[cfg(windows)]
        direct_sensors.refresh(&mut d);
        if hardware_tick.elapsed() >= Duration::from_secs(5) {
            nvidia(&mut d, &stop);
            hardware_tick = Instant::now();
        }
        for s in &sources {
            if stop.load(Ordering::Acquire) {
                break;
            }
            if due
                .get(&s.id)
                .is_none_or(|last| last.elapsed() >= Duration::from_secs(s.interval.max(5)))
            {
                match fetch(s, &client) {
                    Ok(fields) => {
                        d.values
                            .retain(|k, _| !k.starts_with(&format!("source:{}:", s.id)));
                        d.values.extend(fields);
                        d.errors.remove(&s.id);
                    }
                    Err(e) => {
                        d.values
                            .retain(|key, _| !key.starts_with(&format!("source:{}:", s.id)));
                        d.errors.insert(s.id, e);
                    }
                }
                due.insert(s.id, Instant::now());
            }
        }
        for (key, value) in &d.values {
            if let Ok(n) = value.parse::<f64>() {
                let h = d.histories.entry(key.clone()).or_default();
                h.push(n);
                if h.len() > 60 {
                    h.remove(0);
                }
            }
        }
        d.last_refresh = chrono::Local::now().format("%H:%M:%S").to_string();
        if matches!(
            out.try_send(d.clone()),
            Err(mpsc::TrySendError::Disconnected(_))
        ) {
            break;
        }
        next_sample = Instant::now() + Duration::from_secs(1);
    }
}

fn run(
    mut p: Project,
    rx: Receiver<Command>,
    out: SyncSender<DataState>,
    stop: Arc<AtomicBool>,
    ctx: eframe::egui::Context,
) {
    let (sample_tx, sample_rx) = mpsc::channel();
    let (samples, latest_samples) = mpsc::sync_channel(1);
    let sample_sources = p.sources.clone();
    let sample_stop = stop.clone();
    let sampler =
        thread::spawn(move || sample_loop(sample_sources, sample_rx, samples, sample_stop));
    let mut d = DataState::default();
    let mut port = String::new();
    let mut stream = false;
    let mut serial: Option<Box<dyn serialport::SerialPort>> = None;
    let mut current_port = String::new();
    let mut scan = Instant::now() - Duration::from_secs(3);
    let mut force = false;
    let mut uploaded = UploadedFrame::default();
    let mut frame_cache = render::FrameCache::default();
    let mut next_frame = Instant::now();
    let mut current_baud = p.serial_baud;
    let mut measured_since = Instant::now();
    let mut sent_frames = 0u32;
    'display: while !stop.load(Ordering::Acquire) {
        let first = match rx.recv_timeout(next_frame.saturating_duration_since(Instant::now())) {
            Ok(command) => Some(command),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        for command in first.into_iter().chain(rx.try_iter()) {
            match command {
                Command::Project(new) => {
                    if source_signature(&new.sources) != source_signature(&p.sources) {
                        let _ = sample_tx.send(SampleCommand::Sources(new.sources.clone()));
                    }
                    p = new;
                }
                Command::Refresh => {
                    let _ = sample_tx.send(SampleCommand::Refresh);
                }
                Command::Port(s) => {
                    port = s;
                    serial = None;
                    current_port.clear();
                    scan = Instant::now() - Duration::from_secs(3);
                }
                Command::Upload => force = true,
                Command::Stream(b) => {
                    stream = b;
                    force = b;
                }
                Command::Stop => break 'display,
            }
        }
        for reading in latest_samples.try_iter() {
            d.values = reading.values;
            d.histories = reading.histories;
            d.errors = reading.errors;
            d.unavailable = reading.unavailable;
            d.sensor_status = reading.sensor_status;
            d.last_refresh = reading.last_refresh;
        }
        if !force && Instant::now() < next_frame {
            continue;
        }
        let frame_started = Instant::now();

        if stop.load(Ordering::Acquire) {
            break;
        }
        if scan.elapsed() >= Duration::from_secs(2) || current_baud != p.serial_baud {
            if current_baud != p.serial_baud {
                serial = None;
                current_baud = p.serial_baud;
            }
            let ports = serialport::available_ports().unwrap_or_default();
            let found = detect_port(&ports, &port).unwrap_or_default();
            if found != current_port {
                serial = None;
                current_port = found;
            }
            if serial.is_none() && !current_port.is_empty() {
                match serialport::new(&current_port, current_baud)
                    .data_bits(original::SERIAL_DATA_BITS)
                    .stop_bits(original::SERIAL_STOP_BITS)
                    .parity(original::SERIAL_PARITY)
                    .flow_control(serialport::FlowControl::None)
                    .dtr_on_open(original::SERIAL_DTR)
                    .timeout(Duration::from_millis(SERIAL_TIMEOUT_MS))
                    .open()
                    .and_then(|mut handle| {
                        handle.write_data_terminal_ready(original::SERIAL_DTR)?;
                        handle.write_request_to_send(original::SERIAL_RTS)?;
                        Ok(handle)
                    }) {
                    Ok(handle) => {
                        uploaded = UploadedFrame::default();
                        serial = Some(handle);
                        d.device =
                            format!("Connected to {} at {} baud", current_port, current_baud);
                    }
                    Err(_) => {
                        d.device = format!(
                            "{} is unavailable. Close other software using the screen.",
                            current_port
                        )
                    }
                }
            }
            d.port = current_port.clone();
            d.connected = serial.is_some();
            if current_port.is_empty() {
                d.device =
                    "No screen detected. Connect the USB display or select its COM port.".into();
            }
            scan = Instant::now();
        }
        if (stream || force) && serial.is_some() {
            let image = frame_cache.frame(&p, &d);
            match upload_frame(
                serial.as_mut().unwrap().as_mut(),
                &p,
                &image,
                &mut uploaded,
                force,
                &stop,
                thread::sleep,
            ) {
                Ok(true) => {
                    sent_frames += 1;
                    d.device = format!(
                        "{} • updated {}",
                        current_port,
                        chrono::Local::now().format("%H:%M:%S")
                    )
                }
                Ok(false) => {}
                Err(e) => {
                    d.device = format!("Connection lost: {}. Retrying automatically.", e);
                    serial = None;
                    d.connected = false;
                }
            }
        }
        force = false;
        d.streaming = stream && d.connected;
        if measured_since.elapsed() >= Duration::from_secs(1) {
            d.display_fps = sent_frames as f64 / measured_since.elapsed().as_secs_f64();
            sent_frames = 0;
            measured_since = Instant::now();
        }
        if matches!(
            out.try_send(d.clone()),
            Err(mpsc::TrySendError::Disconnected(_))
        ) {
            break;
        }
        ctx.request_repaint();
        // Render the newest position immediately after a slow transfer; never queue stale frames.
        let cadence = if stream && render::has_animation(&p) {
            Duration::from_nanos(1_000_000_000 / 60)
        } else {
            Duration::from_millis(100)
        };
        next_frame = frame_started + cadence;
    }
    stop.store(true, Ordering::Release);
    let _ = sample_tx.send(SampleCommand::Stop);
    let _ = sampler.join();
}

#[cfg(windows)]
fn nvidia(d: &mut DataState, stop: &AtomicBool) {
    use std::os::windows::process::CommandExt;
    let Some(executable) = [
        "C:/Windows/System32/nvidia-smi.exe",
        "C:/Program Files/NVIDIA Corporation/NVSMI/nvidia-smi.exe",
    ]
    .into_iter()
    .find(|p| std::path::Path::new(p).exists()) else {
        return;
    };
    let Ok(mut child) = std::process::Command::new(executable)
        .args([
            "--query-gpu=utilization.gpu,temperature.gpu,memory.used,fan.speed,power.draw,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x08000000)
        .spawn()
    else {
        return;
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None)
                if !stop.load(Ordering::Acquire)
                    && start.elapsed() < Duration::from_millis(1500) =>
            {
                thread::sleep(Duration::from_millis(10))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
        }
    }
    if let Ok(output) = child.wait_with_output() {
        if let Some(line) = String::from_utf8_lossy(&output.stdout).lines().next() {
            nvidia_readings(line, d);
        }
    }
}
#[cfg(not(windows))]
fn nvidia(_: &mut DataState, _: &AtomicBool) {}

fn network_readings(download: u64, upload: u64, seconds: f64, d: &mut DataState) {
    // Shared editor bindings use decimal MB/s. Retain the byte/time precision;
    // DataState converts imported widgets to the original binary MiB/s.
    for (key, bytes) in [("network.download", download), ("network.upload", upload)] {
        d.values.insert(
            key.into(),
            (bytes as f64 / seconds / 1_000_000.).to_string(),
        );
    }
}

fn nvidia_readings(line: &str, d: &mut DataState) {
    let readings: Vec<Option<f64>> = line
        .split(',')
        .map(|value| {
            value
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite() && *n >= 0.)
        })
        .collect();
    for (key, reading) in [
        "gpu.usage",
        "gpu.temperature",
        "gpu.memory",
        "gpu.fan",
        "gpu.power",
        "gpu.memory.total",
    ]
    .into_iter()
    .zip(&readings)
    {
        if let Some(n) = reading {
            d.values.insert(key.into(), format!("{n:?}"));
        } else {
            d.values.remove(key);
        }
    }
    // 颧/踭::軂(), RVA 0x46968 IL_1437..14d6: truncate(used / total * 100).
    // 邁/踭::轗(), RVA 0x15b78 IL_028d..02bc,0397..03a2: cap at 99.
    // GpuRam (amount) and GpuRamLoad (percentage) are separate original fields.
    d.values.remove("gpu.memory.usage");
    if let (Some(Some(used)), Some(Some(total))) = (readings.get(2), readings.get(5)) {
        if *total > 0. {
            let percent = crate::original::gpu_memory_percent(*used, *total);
            d.values
                .insert("gpu.memory.usage".into(), format!("{:.0}", percent));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_monitor_does_not_claim_x2_ftdi_adapter() {
        use serialport::{SerialPortInfo, SerialPortType, UsbPortInfo};
        let port = |name: &str, vid, pid| SerialPortInfo {
            port_name: name.into(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid,
                pid,
                serial_number: None,
                manufacturer: None,
                product: None,
            }),
        };
        let ports = vec![port("COM3", 0x1a86, 0x5722), port("COM4", 0x0403, 0x6001)];
        assert_eq!(detect_port(&ports, ""), Some("COM3".into()));
        assert_eq!(detect_port(&ports, "COM4"), None);
    }
    #[test]
    fn byte_rates_reach_original_widgets_without_unit_or_precision_loss() {
        let mut d = DataState::default();
        // Physical byte counters: 1 MiB downloaded and 0.5 MiB uploaded in 0.5 s.
        network_readings(1_048_576, 524_288, 0.5, &mut d);
        for (key, mbps, mibps) in [
            ("network.download", 2.097152, 2.),
            ("network.upload", 1.048576, 1.),
        ] {
            let mut w = Widget::new(1, Kind::Value, 0., 0.);
            w.binding = key.into();
            assert!((d.number(&w).unwrap() - mbps).abs() < 1e-12);
            w.legacy_text = Some(LegacyText {
                cell_width: 10.,
                cell_height: 13.,
                advance: 9.,
                base_size: w.size,
            });
            assert!((d.number(&w).unwrap() - mibps).abs() < 1e-12);
            let restored: Widget =
                serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
            assert_eq!(d.number(&restored), d.number(&w));
            assert!((d.values[key].parse::<f64>().unwrap() - mbps).abs() < 1e-12);
            d.values.remove(key);
            assert_eq!(d.value(&w), "N/A");
        }
    }
    #[test]
    fn alpha_vantage_quotes_have_symbol_scoped_fields_and_reject_empty_quotes() {
        let quote = serde_json::json!({"Global Quote":{"01. symbol":"MSFT","05. price":"209.4000","09. change":"1.2000","10. change percent":"0.6000%","03. high":"210.0000","04. low":"207.0000","02. open":"208.0000","08. previous close":"208.2000","07. latest trading day":"2026-11-03","06. volume":"123456"}});
        let msft = stock_fields(3, "MSFT", &quote).unwrap();
        let aapl = stock_fields(3, "AAPL", &quote).unwrap();
        assert_eq!(msft["source:3:/MSFT/price"], "209.4000");
        assert_eq!(msft["source:3:/MSFT/change_percent"], "0.6000");
        assert_eq!(msft["source:3:/MSFT/latest_trading_day"], "2026-11-03");
        assert_eq!(msft["source:3:/MSFT/volume"], "123456");
        assert!(aapl.contains_key("source:3:/AAPL/price"));
        assert!(stock_fields(3, "BAD", &serde_json::json!({"Global Quote":{}})).is_err());
        assert!(stock_fields(
            3,
            "BAD",
            &serde_json::json!({"Error Message":"Invalid key"})
        )
        .is_err());
        for body in [
            serde_json::json!({"Note":"rate limit"}),
            serde_json::json!({"Information":"premium"}),
        ] {
            assert!(stock_fields(3, "MSFT", &body)
                .unwrap_err()
                .contains("allowance"));
        }
    }
    #[test]
    fn transfer_planner_trims_padding_and_trades_bytes_for_fewer_waits() {
        let (w, h) = (96, 32);
        let previous = vec![0; (w * h * 2) as usize];
        let mut pixels = previous.clone();
        for x in [3, 35, 67] {
            let i = ((7 * w + x) * 2) as usize;
            pixels[i] = 1;
        }
        let separate = transfer_regions(&previous, &pixels, w, h, 6);
        assert_eq!(separate.len(), 3);
        assert!(separate.iter().all(|r| r.w == 1 && r.h == 1 && r.y == 7));
        let combined = transfer_regions(&previous, &pixels, w, h, 236);
        assert_eq!(
            combined,
            vec![render::Region {
                x: 3,
                y: 7,
                w: 65,
                h: 1
            }]
        );
        let cost = |regions: &[render::Region]| {
            regions
                .iter()
                .map(|r| r.w as u64 * r.h as u64 * 2 + 236)
                .sum::<u64>()
        };
        assert!(cost(&combined) < cost(&separate));
        let mut restored = previous.clone();
        for r in combined {
            for y in r.y..r.y + r.h {
                let start = ((y * w + r.x) * 2) as usize;
                let end = start + (r.w * 2) as usize;
                restored[start..end].copy_from_slice(&pixels[start..end]);
            }
        }
        assert_eq!(restored, pixels);
        assert!(transfer_regions(&pixels, &pixels, w, h, 236).is_empty());
    }
    #[test]
    fn incremental_upload_skips_unchanged_frames_and_reconstructs_changed_pixels() {
        for portrait in [false, true] {
            let mut p = Project::blank();
            p.portrait = portrait;
            let mut image = render::frame(&p, &DataState::default());
            let mut previous = UploadedFrame::default();
            let stop = AtomicBool::new(false);
            let mut wire = vec![];
            assert!(
                upload_frame(&mut wire, &p, &image, &mut previous, false, &stop, |_| {}).unwrap()
            );
            let mut display = previous.pixels.clone();
            wire.clear();
            assert!(
                !upload_frame(&mut wire, &p, &image, &mut previous, false, &stop, |_| {}).unwrap()
            );
            assert!(wire.is_empty());
            image.put_pixel(23, 31, image::Rgba([255, 0, 0, 255]));
            image.put_pixel(200, 100, image::Rgba([0, 255, 0, 255]));
            assert!(
                upload_frame(&mut wire, &p, &image, &mut previous, false, &stop, |_| {}).unwrap()
            );
            assert!(wire.len() < 1500, "small edits must not send a full frame");
            let mut offset = 0;
            let width = image.width();
            while offset < wire.len() {
                let b = &wire[offset..offset + 6];
                assert_eq!(b[5], IMAGE_OPCODE);
                let x = (u32::from(b[0]) << 2) | (u32::from(b[1]) >> 6);
                let y = ((u32::from(b[1]) & 63) << 4) | (u32::from(b[2]) >> 4);
                let ex = ((u32::from(b[2]) & 15) << 6) | (u32::from(b[3]) >> 2);
                let ey = ((u32::from(b[3]) & 3) << 8) | u32::from(b[4]);
                offset += 6;
                for row in y..=ey {
                    let count = ((ex - x + 1) * 2) as usize;
                    let start = ((row * width + x) * 2) as usize;
                    display[start..start + count].copy_from_slice(&wire[offset..offset + count]);
                    offset += count;
                }
            }
            assert_eq!(display, render::rgb565(&image));
            wire.clear();
            p.brightness = 10;
            assert!(
                upload_frame(&mut wire, &p, &image, &mut previous, false, &stop, |_| {}).unwrap()
            );
            assert_eq!(
                wire.len(),
                6,
                "brightness changes must not resend the screen"
            );
            assert_eq!(wire[5], BRIGHTNESS_OPCODE);
        }
    }
    #[test]
    fn original_gpu_memory_load_uses_capacity_ratio_truncation_and_99_ceiling() {
        let mut d = DataState::default();
        nvidia_readings("8, 43, 1024.99, 0, 17.5, 4096", &mut d);
        assert_eq!(d.values["gpu.memory.usage"], "25");
        assert_eq!(
            d.number(&Widget {
                binding: "gpu.memory".into(),
                ..Widget::new(1, Kind::Value, 0., 0.)
            }),
            Some(1024.99)
        );
        assert_eq!(d.values["gpu.memory.total"].parse::<f64>().unwrap(), 4096.);
        nvidia_readings("8, 43, 4096, 0, 17.5, 4096", &mut d);
        assert_eq!(d.values["gpu.memory.usage"], "99");
        nvidia_readings("8, 43, 0, 0, 17.5, 4096", &mut d);
        assert_eq!(d.values["gpu.memory.usage"], "0");
        for line in [
            "8, 43, 1024, 0, 17.5, 0",
            "8, 43, 1024, 0, 17.5, [N/A]",
            "8, 43, NaN, 0, 17.5, 4096",
            "8, 43, 1024, 0, 17.5",
        ] {
            d.values.insert("gpu.memory.usage".into(), "50".into());
            nvidia_readings(line, &mut d);
            assert!(!d.values.contains_key("gpu.memory.usage"), "{line}");
        }
    }
    #[test]
    fn packed_screen_bounds() {
        assert_eq!(packet(197, 0, 0, 479, 319), [0, 0, 7, 125, 63, 197]);
    }
    #[test]
    fn brightness_percent_sends_dark_midpoint_and_full_backlight() {
        // Literal device packets: 255 turns the backlight down, 0 turns it up.
        for (percent, expected) in [
            (0, [63, 192, 0, 0, 0, 110]),
            (50, [31, 192, 0, 0, 0, 110]),
            (100, [0, 0, 0, 0, 0, 110]),
            (255, [0, 0, 0, 0, 0, 110]),
        ] {
            let mut project = Project::blank();
            project.brightness = percent;
            let mut wire = Vec::new();
            upload_with_delay(
                &mut wire,
                &project,
                &DataState::default(),
                &AtomicBool::new(false),
                |_| {},
            )
            .unwrap();
            assert_eq!(&wire[32..38], &expected, "brightness {percent}%");
        }
    }
    #[test]
    fn upload_matches_original_commands_pixels_and_wait_boundaries() {
        use std::{cell::RefCell, rc::Rc};
        #[derive(Debug, PartialEq)]
        enum Event {
            Write(usize),
            Wait(u64),
            Flush,
        }
        struct Wire {
            bytes: Vec<u8>,
            events: Rc<RefCell<Vec<Event>>>,
        }
        impl std::io::Write for Wire {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.events
                    .borrow_mut()
                    .push(Event::Write(self.bytes.len()));
                self.bytes.extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.events.borrow_mut().push(Event::Flush);
                Ok(())
            }
        }
        // Literal protocol vectors from the original IL, not from packet().
        for portrait in [false, true] {
            for flipped in [false, true] {
                for v2 in [false, true] {
                    let mut p = Project::blank();
                    p.portrait = portrait;
                    p.rotation = flipped;
                    p.mirror_mode = true;
                    p.display_v2 = v2;
                    p.brightness = 100;
                    p.background = [255, 0, 0, 255];
                    let events = Rc::new(RefCell::new(vec![]));
                    let mut wire = Wire {
                        bytes: vec![],
                        events: events.clone(),
                    };
                    upload_with_delay(
                        &mut wire,
                        &p,
                        &DataState::default(),
                        &AtomicBool::new(false),
                        |d| events.borrow_mut().push(Event::Wait(d.as_millis() as u64)),
                    )
                    .unwrap();
                    assert_eq!(
                        &wire.bytes[..16],
                        &[0, 0, 0, 0, 0, 122, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]
                    );
                    let orientation = 100 + if portrait { 0 } else { 2 } + u8::from(flipped);
                    let dimensions = if portrait {
                        [1, 64, 1, 224]
                    } else {
                        [1, 224, 1, 64]
                    };
                    assert_eq!(&wire.bytes[16..23], &[0, 0, 0, 0, 0, 121, orientation]);
                    assert_eq!(&wire.bytes[23..27], &dimensions);
                    assert!(wire.bytes[27..32].iter().all(|b| *b == 0));
                    assert_eq!(&wire.bytes[32..38], &[0, 0, 0, 0, 0, 110]);
                    assert_eq!(
                        &wire.bytes[38..44],
                        if portrait {
                            &[0, 0, 4, 253, 223, 197]
                        } else {
                            &[0, 0, 7, 125, 63, 197]
                        }
                    );
                    assert_eq!(wire.bytes.len(), 44 + 480 * 320 * 2);
                    assert!(wire.bytes[44..].chunks_exact(2).all(|p| p == [0, 248]));
                    let actual = events.borrow();
                    assert_eq!(
                        &actual[..5],
                        &[
                            Event::Write(0),
                            Event::Wait(10),
                            Event::Write(16),
                            Event::Wait(10),
                            Event::Write(32)
                        ]
                    );
                    assert_eq!(actual[5], Event::Write(38));
                    assert_eq!(
                        actual
                            .iter()
                            .filter(|e| matches!(e, Event::Wait(10)))
                            .count(),
                        if v2 { 2 } else { 4 }
                    );
                    if !v2 {
                        assert_eq!(actual[6], Event::Wait(10));
                        assert_eq!(actual[actual.len() - 2], Event::Wait(10));
                    }
                    assert_eq!(actual.last(), Some(&Event::Flush));
                }
            }
        }
        let mut bytes = vec![];
        assert!(upload_with_delay(
            &mut bytes,
            &Project::blank(),
            &DataState::default(),
            &AtomicBool::new(true),
            |_| panic!("cancelled transfer waited")
        )
        .is_err());
        assert!(bytes.is_empty());
    }
    #[test]
    fn json_paths_escape_keys() {
        let mut m = BTreeMap::new();
        flatten(&serde_json::json!({"a/b":[true,3]}), "", &mut m);
        assert_eq!(m["/a~1b/0"], "true");
    }
    #[test]
    fn http_json_and_home_assistant_discovery() {
        for home in [false, true] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut request = [0; 4096];
                let n = socket.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..n]);
                if home {
                    assert!(request.starts_with("GET /api/states "));
                    assert!(request
                        .to_lowercase()
                        .contains("authorization: bearer test-token"));
                }
                let body = if home {
                    r#"[{"entity_id":"sensor.office","state":"21.5","attributes":{"unit_of_measurement":"°C"}}]"#
                } else {
                    r#"{"temperature":21.5,"online":true}"#
                };
                write!(socket,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
            });
            let s = Source {
                id: 7,
                name: "Test".into(),
                kind: if home {
                    SourceKind::HomeAssistant
                } else {
                    SourceKind::Json
                },
                url: format!("http://{}", address),
                token: "test-token".into(),
                username: String::new(),
                password: String::new(),
                auth: Auth::Bearer,
                credential_id: String::new(),
                interval: 60,
                latitude: String::new(),
                longitude: String::new(),
                symbol: String::new(),
                stocks: vec![],
            };
            let values = fetch(&s, &reqwest::blocking::Client::new()).unwrap();
            assert_eq!(
                values[if home {
                    "source:7:/sensor.office/state"
                } else {
                    "source:7:/temperature"
                }],
                "21.5"
            );
            server.join().unwrap();
            let json = serde_json::to_string(&s).unwrap();
            assert!(!json.contains("test-token"));
        }
    }
}
