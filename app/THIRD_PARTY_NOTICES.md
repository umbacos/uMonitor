# Third-party components

uMonitor is built with Rust and includes eframe/egui, image/imageproc, reqwest/rustls, serde, sysinfo, serialport, tray-icon and their dependencies. The portable archive's `licenses/dependencies.json` lists the resolved package versions, declared license expressions and upstream repositories. Available license/notice files from the cached crate packages are included under `licenses/`. These notices do not assign a license to uMonitor's own source code.

`msvcr120d.dll` is the pre-existing bundled **HWiNFO client library**, identified by its version resource as version 8.21.0.0. Its internal export-library name is HWiNFO64.dll. It is loaded in a separate helper process for temperature, fan and power readings. It is not Microsoft's Visual C++ debug runtime. Its original vendor terms remain applicable; inclusion here does not grant additional redistribution or usage rights. No original UsbMonitor executable, themes or artwork are included in the portable archive.

Windows fonts, Windows system libraries and hardware drivers are read from the installed system and are not redistributed. Open-Meteo, Alpha Vantage and Home Assistant connections use the user's configured endpoints and credentials; their service terms and limits apply.
