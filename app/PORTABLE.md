# uMonitor portable quick start

1. Extract the entire ZIP into a writable folder, such as `Documents\uMonitor`.
2. Run `uMonitor.exe` and approve the Windows administrator prompt. Canceling the prompt closes the app. No Rust, Python, .NET or separate Visual C++ runtime installation is required.
3. Choose **Open** and load a design from `config`, or start a new design.
4. Choose **Display settings**, select the display's COM port, and use **Upload to screen** or enable continuous streaming.

Keep `msvcr120d.dll` beside the executable. Despite its filename, this is the bundled HWiNFO sensor client, not a Microsoft debug C runtime. All other application libraries are linked into the executable. Windows system libraries, installed fonts and graphics/device drivers are supplied by Windows and the hardware vendor. A USB serial driver may be needed if the display has no COM port; the editor can be used without a connected display.

The portable build targets 64-bit Windows 10/11. It saves `cockpit-session.json` beside `uMonitor.exe`, so use a writable directory. The filename is retained for compatibility with existing sessions. Close the app before copying or moving that file. Designs embed imported images and custom fonts; Windows fonts are read from the local machine. Chinese layouts need a font such as Microsoft YaHei installed on that machine.

Passwords and API keys stay in Windows Credential Manager and must be entered again on another PC. If you move the folder after enabling Windows startup, disable and re-enable that option using the tray menu so the startup path is updated. Launching at sign-in also asks for administrator permission. An already elevated launch continues without another prompt.

Temperature, fan and power availability depends on hardware access and sensor providers. Missing readings show `N/A`; see **Display settings > Hardware sensors > Sensor scan details**. NVIDIA-specific fields use the NVIDIA driver's `nvidia-smi` when available. Other optional providers are described in the README. Physical display compatibility and throughput depend on its firmware; start at the default 115200 baud.

Every completed build creates a fresh portable ZIP and `.sha256` checksum in the application's source directory. The ZIP includes the complete `config` directory with all four editable examples, its instructions and any supporting subdirectories or assets, a README with screenshots, dependency license information and a SHA-256 file manifest. See `THIRD_PARTY_NOTICES.md` for the sensor library and third-party components.
