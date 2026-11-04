use ab_glyph::FontArc;
use std::{collections::BTreeMap, sync::OnceLock};

pub const LEGACY_SIZES: [f32; 4] = [18., 24., 30., 40.];

pub fn installed() -> &'static BTreeMap<String, std::path::PathBuf> {
    static FONTS: OnceLock<BTreeMap<String, std::path::PathBuf>> = OnceLock::new();
    FONTS.get_or_init(|| {
        let mut fonts = BTreeMap::new();
        #[cfg(windows)]
        {
            use winreg::{enums::*, RegKey};
            for (root, base) in [
                (
                    HKEY_LOCAL_MACHINE,
                    std::path::PathBuf::from("C:/Windows/Fonts"),
                ),
                (
                    HKEY_CURRENT_USER,
                    std::env::var_os("LOCALAPPDATA")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_default()
                        .join("Microsoft/Windows/Fonts"),
                ),
            ] {
                if let Ok(key) = RegKey::predef(root)
                    .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Fonts")
                {
                    for (name, _) in key.enum_values().flatten() {
                        if let Ok(file) = key.get_value::<String, _>(&name) {
                            let path = std::path::PathBuf::from(file);
                            let path = if path.is_absolute() {
                                path
                            } else {
                                base.join(path)
                            };
                            if path.extension().is_some_and(|e| {
                                e.eq_ignore_ascii_case("ttf")
                                    || e.eq_ignore_ascii_case("otf")
                                    || e.eq_ignore_ascii_case("ttc")
                            }) {
                                fonts.insert(
                                    name.trim_end_matches(" (TrueType)")
                                        .trim_end_matches(" (OpenType)")
                                        .to_owned(),
                                    path,
                                );
                            }
                        }
                    }
                }
            }
        }
        for (name, file) in [
            ("Segoe UI", "C:/Windows/Fonts/segoeui.ttf"),
            ("Arial", "C:/Windows/Fonts/arial.ttf"),
            ("Consolas", "C:/Windows/Fonts/consola.ttf"),
            ("Bahnschrift", "C:/Windows/Fonts/bahnschrift.ttf"),
            ("Microsoft YaHei", "C:/Windows/Fonts/msyh.ttc"),
            ("Microsoft YaHei Bold", "C:/Windows/Fonts/msyhbd.ttc"),
            ("Microsoft JhengHei", "C:/Windows/Fonts/msjh.ttc"),
            ("Microsoft JhengHei Bold", "C:/Windows/Fonts/msjhbd.ttc"),
            ("Arial Bold", "C:/Windows/Fonts/arialbd.ttf"),
            ("Segoe UI Bold", "C:/Windows/Fonts/segoeuib.ttf"),
            (
                "DejaVu Sans",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            ),
        ] {
            if std::path::Path::new(file).exists() {
                fonts.insert(name.into(), file.into());
            }
        }
        fonts
    })
}
pub fn resolve(name: &str, embedded: Option<&str>) -> Option<FontArc> {
    use base64::Engine;
    if let Some(data) = embedded {
        if let Ok(font) = base64::engine::general_purpose::STANDARD
            .decode(data)
            .ok()
            .and_then(|b| FontArc::try_from_vec(b).ok())
            .ok_or(())
        {
            return Some(font);
        }
    }
    static CACHE: OnceLock<BTreeMap<String, FontArc>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| {
        installed()
            .iter()
            .filter_map(|(name, path)| {
                std::fs::read(path)
                    .ok()
                    .and_then(|b| FontArc::try_from_vec(b).ok())
                    .map(|f| (name.clone(), f))
            })
            .collect()
    });
    cache
        .get(name)
        .or_else(|| {
            cache
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, font)| font)
        })
        .or_else(|| cache.get("Segoe UI"))
        .or_else(|| cache.values().next())
        .cloned()
}
