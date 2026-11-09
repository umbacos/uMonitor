//! Windows CPU time deltas and Core Audio, independent of performance-counter health.
use crate::model::DataState;

/// sysinfo exposes temperatures, but does not expose fan RPM or CPU power.
/// Keep one component list and reapply its latest sample between scans.
pub struct SysinfoSensors {
    components: sysinfo::Components,
    sampled: Option<std::time::Instant>,
    readings: Vec<(String, Option<f32>)>,
}
impl SysinfoSensors {
    pub fn new() -> Self {
        Self {
            components: sysinfo::Components::new(),
            sampled: None,
            readings: vec![],
        }
    }
    pub fn refresh(&mut self, d: &mut DataState) {
        if self
            .sampled
            .is_none_or(|at| at.elapsed() >= std::time::Duration::from_secs(5))
        {
            self.components.refresh(true);
            self.readings = self
                .components
                .iter()
                .map(|c| (c.label().to_owned(), c.temperature()))
                .collect();
            self.sampled = Some(std::time::Instant::now());
        }
        apply_sysinfo_temperatures(&self.readings, d);
        let valid = self
            .readings
            .iter()
            .filter(|(_, t)| t.is_some_and(|t| t.is_finite() && (0. ..150.).contains(&t)))
            .count();
        d.sensor_status = format!("sysinfo: {valid} temperature components; fan RPM and CPU power are not exposed by sysinfo. {}", d.sensor_status);
    }
}
fn apply_sysinfo_temperatures(readings: &[(String, Option<f32>)], d: &mut DataState) {
    d.values.retain(|k, _| !k.starts_with("sensor:sysinfo:"));
    let mut selected = std::collections::BTreeMap::<&str, (u8, f32)>::new();
    for (name, temperature) in readings {
        let Some(t) = temperature.filter(|t| t.is_finite() && (0. ..150.).contains(t)) else {
            continue;
        };
        d.values
            .insert(format!("sensor:sysinfo:{name}"), format!("{t:?}"));
        let label = name.to_lowercase();
        // Windows' generic "Computer" ACPI thermal zone is not a CPU-package
        // measurement. Leave it available by name without mislabelling it.
        let key = if [
            "cpu", "package", "coretemp", "k10temp", "tctl", "tdie", "core ",
        ]
        .iter()
        .any(|s| label.contains(s))
        {
            "cpu.temperature"
        } else if ["disk", "nvme", "ssd", "hdd", "storage", "drivetemp"]
            .iter()
            .any(|s| label.contains(s))
        {
            "disk.temperature"
        } else {
            continue;
        };
        let priority = if ["package", "tctl", "tdie", "composite"]
            .iter()
            .any(|s| label.contains(s))
        {
            2
        } else {
            1
        };
        if selected.get(key).is_none_or(|(p, _)| priority > *p) {
            selected.insert(key, (priority, t));
        }
    }
    for (key, (_, t)) in selected {
        d.values.insert(key.into(), format!("{t:?}"));
        d.unavailable.remove(key);
    }
}

pub fn cpu_percent(previous: [u64; 3], current: [u64; 3]) -> Option<f64> {
    let idle = current[0].checked_sub(previous[0])?;
    let total = current[1]
        .checked_sub(previous[1])?
        .checked_add(current[2].checked_sub(previous[2])?)?;
    (total > 0 && idle <= total).then(|| (total - idle) as f64 / total as f64 * 100.)
}

#[cfg(windows)]
pub struct LocalSensors {
    previous: Option<[u64; 3]>,
    com_ready: bool,
    child: Option<std::process::Child>,
    updates: std::sync::mpsc::Receiver<serde_json::Value>,
    last: Option<(std::time::Instant, serde_json::Value)>,
}
#[cfg(windows)]
fn cpu_times() -> Option<[u64; 3]> {
    use windows::Win32::{Foundation::FILETIME, System::Threading::GetSystemTimes};
    let mut times = [FILETIME::default(); 3];
    let [idle, kernel, user] = &mut times;
    unsafe {
        GetSystemTimes(Some(idle), Some(kernel), Some(user)).ok()?;
    }
    Some(times.map(|t| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime)))
}
#[cfg(windows)]
fn volume() -> windows::core::Result<f64> {
    use windows::Win32::{
        Media::Audio::{
            eMultimedia, eRender, Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator,
            MMDeviceEnumerator,
        },
        System::Com::{CoCreateInstance, CLSCTX_ALL},
    };
    unsafe {
        // Resolve the current default output each time, including device switches.
        let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = devices.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
        let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        Ok(endpoint.GetMasterVolumeLevelScalar()? as f64 * 100.)
    }
}
#[cfg(windows)]
impl LocalSensors {
    pub fn new() -> Self {
        use std::{
            io::{BufRead, BufReader},
            os::windows::process::CommandExt,
            process::{Command, Stdio},
            sync::mpsc,
        };
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
        let com_ready = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).is_ok() };
        let (tx, updates) = mpsc::channel();
        // One quiet, persistent helper: no PowerShell process launched per sample.
        let mut child = Command::new("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                include_str!("sensors.ps1"),
            ])
            .creation_flags(0x08000000)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .ok();
        if let Some(stdout) = child.as_mut().and_then(|c| c.stdout.take()) {
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if let Ok(value) = serde_json::from_str(&line) {
                        if tx.send(value).is_err() {
                            break;
                        }
                    }
                }
            });
        }
        Self {
            previous: cpu_times(),
            com_ready,
            child,
            updates,
            last: None,
        }
    }
    pub fn refresh(&mut self, d: &mut DataState) {
        if let Some(current) = cpu_times() {
            if let Some(n) = self.previous.and_then(|p| cpu_percent(p, current)) {
                d.values.insert("cpu.usage".into(), format!("{n:.1}"));
                d.unavailable.remove("cpu.usage");
            }
            self.previous = Some(current);
        }
        match self.com_ready.then(volume).transpose() {
            Ok(Some(n)) if n.is_finite() => {
                d.values.insert("volume.level".into(), format!("{n:.0}"));
                d.unavailable.remove("volume.level");
            }
            _ => {
                d.values.remove("volume.level");
                d.unavailable.insert(
                    "volume.level".into(),
                    "No default Windows audio output is available.".into(),
                );
            }
        }
        for value in self.updates.try_iter() {
            self.last = Some((std::time::Instant::now(), value));
        }
        for key in [
            "cpu.temperature",
            "cpu.fan",
            "cpu.power",
            "disk.temperature",
        ] {
            d.values.remove(key);
            d.unavailable.insert(key.into(), "No live reading was found from the bundled HWiNFO library or other providers. Hardware/driver access may require running the app as administrator. LibreHardwareMonitor WMI/web sensors and HWiNFO gadget sensors are also checked. See Sensor scan details.".into());
        }
        d.values.retain(|k, _| !k.starts_with("sensor:"));
        if let Some((at, value)) = &self.last {
            if at.elapsed() < std::time::Duration::from_secs(30) {
                apply_hardware(value, d);
            } else {
                d.sensor_status =
                    "Hardware scan stopped responding. Restart the app to retry.".into();
            }
        } else {
            d.sensor_status = "Checking hardware sensor providers…".into();
        }
    }
}
#[cfg(windows)]
impl Drop for LocalSensors {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        if self.com_ready {
            unsafe {
                windows::Win32::System::Com::CoUninitialize();
            }
        }
    }
}
#[cfg(not(windows))]
pub struct LocalSensors;
#[cfg(not(windows))]
impl LocalSensors {
    pub fn new() -> Self {
        Self
    }
    pub fn refresh(&mut self, d: &mut DataState) {
        for key in [
            "cpu.temperature",
            "disk.temperature",
            "cpu.fan",
            "cpu.power",
        ] {
            d.values.remove(key);
            d.unavailable.insert(
                key.into(),
                "No supported sensor reading. sysinfo does not expose fan RPM or CPU power.".into(),
            );
        }
        d.sensor_status = "Missing readings remain N/A.".into();
    }
}

pub(crate) fn apply_hardware(value: &serde_json::Value, d: &mut DataState) {
    if let Some(status) = value["_diagnostics"].as_str() {
        d.sensor_status = status.to_owned();
    }
    let Some(values) = value.as_object() else {
        return;
    };
    for (key, value) in values {
        if let Some(n) = value.as_f64().filter(|n| n.is_finite() && *n >= 0.) {
            if key.starts_with("sensor:")
                || [
                    "cpu.temperature",
                    "cpu.fan",
                    "cpu.power",
                    "disk.temperature",
                ]
                .contains(&key.as_str())
            {
                d.values.insert(key.clone(), format!("{n:?}"));
                d.unavailable.remove(key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sysinfo_maps_identified_components_and_keeps_generic_zones_separate() {
        let mut d = DataState::default();
        d.values.insert("cpu.power".into(), "45.0".into());
        apply_sysinfo_temperatures(
            &[
                ("Computer".into(), Some(27.)),
                ("CPU Core 0".into(), Some(48.)),
                ("CPU Package".into(), Some(55.)),
                ("NVMe Composite".into(), Some(42.)),
                ("CPU broken".into(), Some(f32::NAN)),
                ("SSD absent".into(), None),
            ],
            &mut d,
        );
        assert_eq!(d.values["cpu.temperature"], "55.0");
        assert_eq!(d.values["disk.temperature"], "42.0");
        assert_eq!(d.values["sensor:sysinfo:Computer"], "27.0");
        assert_eq!(d.values["cpu.power"], "45.0");
        assert!(!d.values.contains_key("cpu.fan"));
        assert!(!d.values.contains_key("sensor:sysinfo:CPU broken"));
        let mut generic = DataState::default();
        apply_sysinfo_temperatures(&[("Computer".into(), Some(30.))], &mut generic);
        assert!(!generic.values.contains_key("cpu.temperature"));
    }
    #[cfg(windows)]
    #[test]
    fn windows_live_cpu_and_audio_readings() {
        std::fs::create_dir_all("evidence").unwrap();
        let mut sensors = LocalSensors::new();
        let mut sysinfo = SysinfoSensors::new();
        let mut d = DataState::default();
        let mut samples = vec![];
        for _ in 0..3 {
            std::thread::sleep(std::time::Duration::from_millis(400));
            sensors.refresh(&mut d);
            sysinfo.refresh(&mut d);
            let cpu = d
                .values
                .get("cpu.usage")
                .expect("Windows CPU clock sample")
                .parse::<f64>()
                .unwrap();
            assert!(cpu.is_finite() && (0.0..=100.0).contains(&cpu));
            samples.push(d.values.clone());
        }
        let result = serde_json::json!({"samples":samples,"unavailable":d.unavailable,"status":d.sensor_status,"sysinfo_components":sysinfo.readings,"capabilities":{"temperature":true,"fan_rpm":false,"cpu_power":false}});
        std::fs::write(
            "evidence/umeter-live-sensors.json",
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        std::fs::write(
            "evidence/live-sensors.json",
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        println!("{}", result);
    }
    #[test]
    fn cpu_deltas_include_idle_once_and_reject_resets_or_empty_samples() {
        assert_eq!(cpu_percent([100, 200, 100], [160, 280, 120]), Some(40.));
        assert_eq!(cpu_percent([0, 0, 0], [100, 100, 0]), Some(0.));
        assert_eq!(cpu_percent([0, 0, 0], [0, 100, 100]), Some(100.));
        assert_eq!(cpu_percent([1, 1, 1], [1, 1, 1]), None);
        assert_eq!(cpu_percent([100, 200, 100], [0, 0, 0]), None);
    }
    #[test]
    fn real_zero_readings_are_kept_and_missing_values_are_not_invented() {
        let mut d = DataState::default();
        apply_hardware(
            &serde_json::json!({"cpu.fan":0,"cpu.temperature":45.5,"cpu.power":null,"disk.temperature":-1}),
            &mut d,
        );
        assert_eq!(d.values["cpu.fan"], "0.0");
        assert_eq!(d.values["cpu.temperature"], "45.5");
        assert!(!d.values.contains_key("cpu.power"));
        assert!(!d.values.contains_key("disk.temperature"));
    }

    #[test]
    fn sensor_samples_keep_fractional_values_until_widget_formatting() {
        let mut d = DataState::default();
        apply_hardware(
            &serde_json::json!({"cpu.power": 125.99, "cpu.temperature": 33.99}),
            &mut d,
        );
        assert_eq!(d.values["cpu.power"].parse::<f64>().unwrap(), 125.99);
        assert_eq!(d.values["cpu.temperature"].parse::<f64>().unwrap(), 33.99);
        apply_sysinfo_temperatures(&[("CPU Package".into(), Some(55.99))], &mut d);
        assert_eq!(d.values["cpu.temperature"].parse::<f32>().unwrap(), 55.99);
    }
}
