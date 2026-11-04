pub fn app_icon() -> eframe::egui::IconData {
    let icon = image::load_from_memory(include_bytes!("../assets/umeter.png"))
        .expect("bundled uM icon")
        .to_rgba8();
    eframe::egui::IconData {
        width: icon.width(),
        height: icon.height(),
        rgba: icon.into_raw(),
    }
}
#[cfg(windows)]
pub struct Tray {
    icon: tray_icon::TrayIcon,
    open: tray_icon::menu::MenuItem,
    startup: tray_icon::menu::MenuItem,
    exit: tray_icon::menu::MenuItem,
    events: std::sync::mpsc::Receiver<Action>,
    connection: String,
}
pub enum Action {
    Open,
    Startup,
    Exit,
}
#[cfg(windows)]
impl Tray {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use tray_icon::{
            menu::{Menu, MenuItem, PredefinedMenuItem},
            TrayIconBuilder,
        };
        let hwnd = match cc.window_handle().map_err(|e| e.to_string())?.as_raw() {
            RawWindowHandle::Win32(handle) => handle.hwnd.get(),
            _ => return Err("The editor has no Windows window handle".into()),
        };
        let ctx = cc.egui_ctx.clone();
        let menu = Menu::new();
        let open = MenuItem::new("Open uMonitor", true, None);
        let startup = MenuItem::new(
            if startup_enabled() {
                "Remove from Windows startup"
            } else {
                "Add to Windows startup"
            },
            true,
            None,
        );
        let exit = MenuItem::new("Exit", true, None);
        let (tx, events) = std::sync::mpsc::channel();
        let open_id = open.id().clone();
        let startup_id = startup.id().clone();
        let exit_id = exit.id().clone();
        let menu_ctx = ctx.clone();
        let menu_tx = tx.clone();
        tray_icon::menu::MenuEvent::set_event_handler(Some(
            move |e: tray_icon::menu::MenuEvent| {
                let action = if e.id == open_id {
                    Some(Action::Open)
                } else if e.id == startup_id {
                    Some(Action::Startup)
                } else if e.id == exit_id {
                    Some(Action::Exit)
                } else {
                    None
                };
                if let Some(action) = action {
                    if menu_tx.send(action).is_ok() {
                        wake_editor(&menu_ctx, hwnd);
                    }
                }
            },
        ));
        tray_icon::TrayIconEvent::set_event_handler(Some(move |e: tray_icon::TrayIconEvent| {
            if matches!(
                e,
                tray_icon::TrayIconEvent::Click {
                    button: tray_icon::MouseButton::Left,
                    button_state: tray_icon::MouseButtonState::Up,
                    ..
                }
            ) {
                if tx.send(Action::Open).is_ok() {
                    wake_editor(&ctx, hwnd);
                }
            }
        }));
        menu.append_items(&[&open, &startup, &PredefinedMenuItem::separator(), &exit])
            .map_err(|e| e.to_string())?;
        let icon = connection_icon(false)?;
        let icon = TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip("uMonitor • no screen connected")
            .with_menu(Box::new(menu))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            icon: icon,
            open,
            startup,
            exit,
            events,
            connection: String::new(),
        })
    }
    pub fn set_connection(&mut self, connected: bool, port: &str) {
        let label = if connected {
            format!("uMonitor • connected to {}", port)
        } else {
            "uMonitor • no screen connected".into()
        };
        if label != self.connection {
            if let Ok(icon) = connection_icon(connected) {
                let _ = self.icon.set_icon(Some(icon));
            }
            let _ = self.icon.set_tooltip(Some(&label));
            self.connection = label;
        }
    }
    pub fn poll(&self) -> Vec<Action> {
        use tray_icon::{menu::MenuEvent, MouseButton, MouseButtonState, TrayIconEvent};
        let mut out: Vec<_> = self.events.try_iter().collect();
        while let Ok(e) = MenuEvent::receiver().try_recv() {
            if e.id == self.open.id() {
                out.push(Action::Open)
            } else if e.id == self.startup.id() {
                out.push(Action::Startup)
            } else if e.id == self.exit.id() {
                out.push(Action::Exit)
            }
        }
        while let Ok(e) = TrayIconEvent::receiver().try_recv() {
            if matches!(
                e,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                out.push(Action::Open);
            }
        }
        out
    }
    pub fn update_startup(&self) {
        self.startup.set_text(if startup_enabled() {
            "Remove from Windows startup"
        } else {
            "Add to Windows startup"
        });
    }
}
#[cfg(windows)]
fn wake_editor(ctx: &eframe::egui::Context, hwnd: isize) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetForegroundWindow, ShowWindow, SW_MAXIMIZE, SW_RESTORE,
    };
    // Viewport commands are consumed by App::update, which may not run for a
    // hidden/minimized window. Restore the native window from the tray callback
    // so the queued action (including Exit and its save prompt) can be handled.
    // SAFETY: eframe owns this HWND. Callbacks run on its Windows event thread
    // and only call here after sending to the live Tray receiver. Dropping Tray
    // disconnects that receiver before the editor is destroyed, disabling wakeup.
    unsafe {
        ShowWindow(hwnd as _, SW_RESTORE);
        ShowWindow(hwnd as _, SW_MAXIMIZE);
        SetForegroundWindow(hwnd as _);
    }
    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Maximized(true));
    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
    ctx.request_repaint();
}
#[cfg(not(windows))]
pub struct Tray;
#[cfg(not(windows))]
impl Tray {
    pub fn new(_: &eframe::CreationContext<'_>) -> Result<Self, String> {
        Err("Tray controls are available on Windows".into())
    }
    pub fn poll(&self) -> Vec<Action> {
        vec![]
    }
    pub fn update_startup(&self) {}
    pub fn set_connection(&mut self, _: bool, _: &str) {}
}
#[cfg(windows)]
pub fn startup_enabled() -> bool {
    use winreg::{enums::*, RegKey};
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .ok()
        .and_then(|k| k.get_value::<String, _>("CockpitStudio").ok())
        .is_some()
}
#[cfg(windows)]
pub fn set_startup(enabled: bool) -> Result<(), String> {
    use winreg::{enums::*, RegKey};
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .map_err(|e| e.to_string())?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        key.set_value("CockpitStudio", &format!("\"{}\" --tray", exe.display()))
            .map_err(|e| e.to_string())
    } else {
        match key.delete_value("CockpitStudio") {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}
#[cfg(not(windows))]
pub fn startup_enabled() -> bool {
    false
}
#[cfg(not(windows))]
pub fn set_startup(_: bool) -> Result<(), String> {
    Err("Windows only".into())
}

#[cfg(windows)]
fn connection_icon(connected: bool) -> Result<tray_icon::Icon, String> {
    let color = if connected {
        [44, 196, 116, 255]
    } else {
        [231, 68, 68, 255]
    };
    let icon = app_icon();
    let image = image::RgbaImage::from_raw(icon.width, icon.height, icon.rgba).unwrap();
    let mut image = image::imageops::resize(&image, 32, 32, image::imageops::FilterType::Lanczos3);
    imageproc::drawing::draw_filled_circle_mut(
        &mut image,
        (27, 27),
        5,
        image::Rgba([255, 255, 255, 255]),
    );
    imageproc::drawing::draw_filled_circle_mut(&mut image, (27, 27), 4, image::Rgba(color));
    tray_icon::Icon::from_rgba(image.into_raw(), 32, 32).map_err(|e| e.to_string())
}
