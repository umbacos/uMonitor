//! Original HWiNFO client ABI, verified against UsbMonitor managed P/Invokes.
//! Native acquisition runs in a separate process so a driver hang/crash cannot
//! block the editor or its other sensor providers. See evidence/hwinfo-api.json.
use crate::{model::DataState, original};
use serde_json::{Map, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    os::windows::{ffi::OsStrExt, process::CommandExt},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{FreeLibrary, HMODULE},
    Globalization::{MultiByteToWideChar, CP_ACP},
    System::LibraryLoader::{
        GetProcAddress, LoadLibraryExW, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
        LOAD_LIBRARY_SEARCH_SYSTEM32,
    },
};

const MAX_AGE: Duration = Duration::from_secs(30);
type Init = unsafe extern "C" fn(i32) -> i32;
type Count = unsafe extern "C" fn() -> i32;
type Refresh = unsafe extern "C" fn(i32);
type DeviceName = unsafe extern "C" fn(i32, *mut u8, i32);
type Reading = unsafe extern "C" fn(i32, i32, *mut u8, i32) -> f64;
type Fan = unsafe extern "C" fn(i32, i32, *mut u8, i32) -> i32;

struct Library {
    module: HMODULE,
    init: Init,
    close: Count,
    count: Count,
    refresh: Refresh,
    name: DeviceName,
    temperature: Reading,
    fan: Fan,
    power: Reading,
    initialized: bool,
}
impl Library {
    fn open() -> Result<Self, String> {
        let path = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .with_file_name("msvcr120d.dll");
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            // Absolute, executable-relative path; never search PATH or the CWD.
            let module = LoadLibraryExW(
                wide.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
            );
            if module.is_null() {
                return Err(format!(
                    "Cannot load {}: {}",
                    path.display(),
                    std::io::Error::last_os_error()
                ));
            }
            macro_rules! ordinal {
                ($n:expr, $ty:ty) => {{
                    match GetProcAddress(module, $n as usize as *const u8) {
                        Some(address) => std::mem::transmute::<
                            unsafe extern "system" fn() -> isize,
                            $ty,
                        >(address),
                        None => {
                            FreeLibrary(module);
                            return Err(format!("Sensor library is missing ordinal {}", $n));
                        }
                    }
                }};
            }
            Ok(Self {
                module,
                init: ordinal!(original::HARDWARE_INIT_ORDINAL, Init),
                close: ordinal!(original::HARDWARE_CLOSE_ORDINAL, Count),
                count: ordinal!(original::HARDWARE_COUNT_ORDINAL, Count),
                refresh: ordinal!(original::HARDWARE_REFRESH_ORDINAL, Refresh),
                name: ordinal!(original::HARDWARE_NAME_ORDINAL, DeviceName),
                temperature: ordinal!(original::HARDWARE_TEMPERATURE_ORDINAL, Reading),
                fan: ordinal!(original::HARDWARE_FAN_ORDINAL, Fan),
                power: ordinal!(original::HARDWARE_POWER_ORDINAL, Reading),
                initialized: false,
            })
        }
    }
    fn sample(&self, init_code: i32) -> Value {
        let mut values = Map::new();
        let mut selected = std::collections::BTreeMap::new();
        let count = unsafe { (self.count)() };
        if !(0..=256).contains(&count) {
            return serde_json::json!({"_diagnostics": format!("HWiNFO DLL returned invalid device count {count} (initialization code {init_code}).")});
        }
        let mut readings = 0;
        for device in 0..count {
            let mut buffer = [0u8; 256];
            unsafe {
                (self.refresh)(device);
                (self.name)(device, buffer.as_mut_ptr(), buffer.len() as i32);
            }
            let parent = ansi_name(&buffer);
            for kind in ["Temperature", "Fan", "Power"] {
                // The original scans all 512 indices per family. Preserve sparse
                // indices; zero with a nonempty label is a real reading.
                for index in 0..512 {
                    let mut label = [0u8; 256];
                    let value = unsafe {
                        match kind {
                            "Temperature" => {
                                (self.temperature)(device, index, label.as_mut_ptr(), 256)
                            }
                            "Fan" => (self.fan)(device, index, label.as_mut_ptr(), 256) as f64,
                            _ => (self.power)(device, index, label.as_mut_ptr(), 256),
                        }
                    };
                    let label = ansi_name(&label);
                    if label.is_empty() {
                        continue;
                    }
                    if add_reading(
                        &mut values,
                        &mut selected,
                        device,
                        index,
                        &parent,
                        kind,
                        &label,
                        value,
                    ) {
                        readings += 1;
                    }
                }
            }
        }
        for (key, (_, value)) in selected {
            values.insert(key.into(), Value::from(value));
        }
        let status = if count == 0 {
            format!("HWiNFO DLL loaded; no sensor devices found (initialization code {init_code}). Hardware/driver access may require running the app as administrator. Other sensor providers remain active.")
        } else {
            format!("HWiNFO DLL: {readings} live temperature, fan and power readings across {count} devices (initialization code {init_code}).")
        };
        values.insert("_diagnostics".into(), Value::from(status));
        Value::Object(values)
    }
}
impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            if self.initialized {
                (self.close)();
            }
            FreeLibrary(self.module);
        }
    }
}
fn ansi_name(bytes: &[u8]) -> String {
    let len = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    if len == 0 {
        return String::new();
    }
    let mut wide = [0u16; 256];
    let size = unsafe {
        MultiByteToWideChar(
            CP_ACP,
            0,
            bytes.as_ptr(),
            len as i32,
            wide.as_mut_ptr(),
            256,
        )
    };
    String::from_utf16_lossy(&wide[..size.max(0) as usize])
        .trim()
        .to_owned()
}

#[allow(clippy::too_many_arguments)]
fn add_reading(
    values: &mut Map<String, Value>,
    selected: &mut std::collections::BTreeMap<&'static str, (u8, f64)>,
    device: i32,
    index: i32,
    parent: &str,
    kind: &str,
    label: &str,
    value: f64,
) -> bool {
    if !value.is_finite()
        || value < 0.
        || label.is_empty()
        || (kind == "Temperature" && value >= 150.)
        || (kind == "Fan" && value > 100_000.)
        || (kind == "Power" && value > 10_000.)
    {
        return false;
    }
    values.insert(
        format!("sensor:HWiNFO DLL:{device}:{kind}:{index} {parent} / {label}"),
        Value::from(value),
    );
    let label = label.to_lowercase();
    let context = format!("{parent} {label}").to_lowercase();
    let cpu = ["cpu", "ryzen", "core i", "core ultra"]
        .iter()
        .any(|s| context.contains(s));
    let disk = ["drive", "disk", "nvme", "ssd", "hdd", "s.m.a.r.t."]
        .iter()
        .any(|s| context.contains(s));
    let candidate = match kind {
        "Temperature" if cpu && !disk => Some((
            "cpu.temperature",
            if ["package", "tctl", "tdie", "core average"]
                .iter()
                .any(|s| label.contains(s))
            {
                3
            } else {
                1
            },
        )),
        "Temperature" if disk => Some((
            "disk.temperature",
            if label.contains("composite") || label.contains("drive temperature") {
                3
            } else {
                1
            },
        )),
        "Fan" if cpu && !context.contains("gpu") && !context.contains("pump") => {
            Some(("cpu.fan", 2))
        }
        "Power"
            if cpu
                && ["package", "cpu total", "ppt", "cpu power", "cpu cores"]
                    .iter()
                    .any(|s| label.contains(s)) =>
        {
            Some((
                "cpu.power",
                if label.contains("package") || label.contains("ppt") {
                    3
                } else {
                    1
                },
            ))
        }
        _ => None,
    };
    if let Some((key, priority)) = candidate {
        if selected.get(key).is_none_or(|(p, _)| priority > *p) {
            selected.insert(key, (priority, value));
        }
    }
    true
}

fn emit(value: &Value) -> bool {
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    writeln!(output, "{value}")
        .and_then(|_| output.flush())
        .is_ok()
}
pub fn run_helper(once: bool) {
    if !once
        && !emit(&serde_json::json!({"_diagnostics":"HWiNFO DLL: initializing hardware access…"}))
    {
        return;
    }
    let mut library = match Library::open() {
        Ok(library) => library,
        Err(error) => {
            emit(&serde_json::json!({"_diagnostics":error}));
            return;
        }
    };
    // Same flags as the original's static initializer and code.ini default.
    let init_code = unsafe { (library.init)(original::HARDWARE_INIT_FLAGS) };
    library.initialized = true;
    let (stop, stopped) = mpsc::channel();
    if !once {
        std::thread::spawn(move || {
            let _ = std::io::stdin().read(&mut [0u8; 1]);
            let _ = stop.send(());
        });
    }
    loop {
        if !emit(&library.sample(init_code)) || once {
            break;
        }
        if stopped.recv_timeout(Duration::from_secs(5)) != Err(mpsc::RecvTimeoutError::Timeout) {
            break;
        }
    }
}

pub struct DirectSensors {
    child: Option<Child>,
    updates: Receiver<Value>,
    last: Option<(Instant, Value)>,
    started: Instant,
    error: Option<String>,
}
impl DirectSensors {
    pub fn new() -> Self {
        let (sender, updates) = mpsc::sync_channel(1);
        let mut error = None;
        let mut child = std::env::current_exe()
            .map_err(|e| e.to_string())
            .and_then(|exe| {
                Command::new(exe)
                    .arg("--hardware-sensors")
                    .creation_flags(0x08000000)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|e| e.to_string())
            })
            .map_err(|e| {
                error = Some(format!("HWiNFO DLL helper could not start: {e}"));
            })
            .ok();
        if let Some(stdout) = child.as_mut().and_then(|c| c.stdout.take()) {
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if let Ok(value) = serde_json::from_str(&line) {
                        if sender.send(value).is_err() {
                            break;
                        }
                    }
                }
            });
        }
        Self {
            child,
            updates,
            last: None,
            started: Instant::now(),
            error,
        }
    }
    pub fn refresh(&mut self, d: &mut DataState) {
        for value in self.updates.try_iter() {
            self.last = Some((Instant::now(), value));
        }
        let age = self
            .last
            .as_ref()
            .map_or(self.started, |(at, _)| *at)
            .elapsed();
        if self.error.is_none() {
            if let Some(child) = &mut self.child {
                if let Ok(Some(status)) = child.try_wait() {
                    self.error = Some(format!(
                        "HWiNFO DLL helper stopped ({status}). {}",
                        self.last
                            .as_ref()
                            .and_then(|(_, v)| v["_diagnostics"].as_str())
                            .unwrap_or("No readings were returned.")
                    ));
                }
            }
            if self.error.is_none() && age >= MAX_AGE {
                if let Some(child) = &mut self.child {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                self.error = Some("HWiNFO DLL scan timed out. Restart the app to retry; other sensor providers remain active.".into());
            }
        }
        if let Some(error) = &self.error {
            d.sensor_status = format!("{error} {}", d.sensor_status);
        } else if let Some((_, value)) = &self.last {
            if age < MAX_AGE {
                let fallback_status = d.sensor_status.clone();
                crate::sensors::apply_hardware(value, d);
                d.sensor_status = format!("{} {fallback_status}", d.sensor_status);
            }
        } else {
            d.sensor_status = format!("HWiNFO DLL: starting sensor scan… {}", d.sensor_status);
        }
    }
}
impl Drop for DirectSensors {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            // Give the DLL its shutdown call; kill only if native code is stuck.
            if let Some(mut input) = child.stdin.take() {
                let _ = input.write_all(b"q");
            }
            let deadline = Instant::now() + Duration::from_millis(300);
            while Instant::now() < deadline {
                if matches!(child.try_wait(), Ok(Some(_))) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merges_fresh_readings_and_drops_stale_samples_without_losing_fallbacks() {
        let (_sender, updates) = mpsc::channel();
        let mut direct = DirectSensors {
            child: None,
            updates,
            last: Some((
                Instant::now(),
                serde_json::json!({"cpu.temperature":55.99,"cpu.power":125.99,"_diagnostics":"HWiNFO DLL: 2 readings"}),
            )),
            started: Instant::now(),
            error: None,
        };
        let fallback = || {
            let mut data = DataState::default();
            data.values.insert("cpu.temperature".into(), "44.0".into());
            data.values.insert("disk.temperature".into(), "42.0".into());
            data.sensor_status = "Windows storage: available".into();
            data
        };
        let mut fresh = fallback();
        direct.refresh(&mut fresh);
        assert_eq!(fresh.values["cpu.temperature"], "55.99");
        assert_eq!(fresh.values["cpu.power"], "125.99");
        assert_eq!(fresh.values["disk.temperature"], "42.0");
        assert!(fresh.sensor_status.contains("HWiNFO DLL: 2 readings"));
        assert!(fresh.sensor_status.contains("Windows storage: available"));
        direct.last.as_mut().unwrap().0 = Instant::now() - MAX_AGE - Duration::from_secs(1);
        let mut stale = fallback();
        direct.refresh(&mut stale);
        assert_eq!(stale.values["cpu.temperature"], "44.0");
        assert!(!stale.values.contains_key("cpu.power"));
        assert!(stale.sensor_status.contains("timed out"));
        assert!(stale.sensor_status.contains("Windows storage: available"));
    }
    #[test]
    fn maps_real_labels_without_confusing_gpu_pumps_and_generic_thermal_zones() {
        let mut values = Map::new();
        let mut selected = std::collections::BTreeMap::new();
        for (device, parent, kind, label, value) in [
            (0, "CPU [#0]: AMD Ryzen", "Temperature", "Core 0", 48.25),
            (
                0,
                "CPU [#0]: AMD Ryzen",
                "Temperature",
                "CPU (Tctl/Tdie)",
                55.99,
            ),
            (
                0,
                "CPU [#0]: AMD Ryzen",
                "Power",
                "CPU Package Power",
                125.99,
            ),
            (1, "Motherboard", "Fan", "CPU Fan", 0.),
            (1, "Motherboard", "Fan", "CPU Pump", 2400.),
            (2, "GPU NVIDIA", "Temperature", "GPU Temperature", 70.),
            (3, "Drive: NVMe", "Temperature", "Drive Temperature", 42.5),
            (4, "Computer", "Temperature", "Thermal Zone", 30.),
        ] {
            add_reading(
                &mut values,
                &mut selected,
                device,
                0,
                parent,
                kind,
                label,
                value,
            );
        }
        assert_eq!(selected["cpu.temperature"].1, 55.99);
        assert_eq!(selected["cpu.power"].1, 125.99);
        assert_eq!(selected["cpu.fan"].1, 0.);
        assert_eq!(selected["disk.temperature"].1, 42.5);
        assert_eq!(values.len(), 8); // All named readings remain individually bindable.
        assert!(!add_reading(
            &mut values,
            &mut selected,
            0,
            1,
            "CPU",
            "Temperature",
            "broken",
            f64::NAN
        ));
        assert!(!add_reading(
            &mut values,
            &mut selected,
            0,
            1,
            "CPU",
            "Power",
            "absent",
            -1.
        ));
    }
}
