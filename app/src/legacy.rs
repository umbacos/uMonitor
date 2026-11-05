//! Independent data-only reader for MS-NRBF theme records. No CLR object activation.
use crate::{model::*, render};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
enum N {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Ref(i32),
    Array(Vec<N>),
    Bytes(Vec<u8>),
    Obj(String, Vec<(String, N)>),
    Nulls(usize),
}
#[derive(Clone)]
struct Meta {
    name: String,
    names: Vec<String>,
    types: Vec<u8>,
    args: Vec<u8>,
}
struct Reader<'a> {
    b: &'a [u8],
    p: usize,
    root: i32,
    objects: BTreeMap<i32, N>,
    meta: BTreeMap<i32, Meta>,
    depth: usize,
}
impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self {
            b,
            p: 0,
            root: 0,
            objects: BTreeMap::new(),
            meta: BTreeMap::new(),
            depth: 0,
        }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if n > 16_000_000 || self.p.checked_add(n).is_none_or(|e| e > self.b.len()) {
            return Err("Truncated or oversized legacy record".into());
        }
        let b = &self.b[self.p..self.p + n];
        self.p += n;
        Ok(b)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn count(&mut self) -> Result<usize, String> {
        let n = self.i32()?;
        if !(0..=4_000_000).contains(&n) {
            Err("Invalid legacy array length".into())
        } else {
            Ok(n as usize)
        }
    }
    fn string(&mut self) -> Result<String, String> {
        let mut n = 0;
        for shift in (0..35).step_by(7) {
            let b = self.u8()?;
            n |= ((b & 127) as usize) << shift;
            if b < 128 {
                return String::from_utf8(self.take(n)?.to_vec()).map_err(|e| e.to_string());
            }
        }
        Err("Bad string length".into())
    }
    fn primitive(&mut self, t: u8) -> Result<N, String> {
        Ok(match t {
            1 => N::Bool(self.u8()? != 0),
            2 => N::Int(self.u8()? as i64),
            7 => N::Int(i16::from_le_bytes(self.take(2)?.try_into().unwrap()) as i64),
            8 => N::Int(self.i32()? as i64),
            9 | 12 | 13 | 16 => N::Int(i64::from_le_bytes(self.take(8)?.try_into().unwrap())),
            10 => N::Int(self.u8()? as i8 as i64),
            11 => N::Float(f32::from_le_bytes(self.take(4)?.try_into().unwrap()) as f64),
            6 => N::Float(f64::from_le_bytes(self.take(8)?.try_into().unwrap())),
            14 => N::Int(u16::from_le_bytes(self.take(2)?.try_into().unwrap()) as i64),
            15 => N::Int(u32::from_le_bytes(self.take(4)?.try_into().unwrap()) as i64),
            18 => N::Str(self.string()?),
            _ => return Err(format!("Unsupported primitive {}", t)),
        })
    }
    fn extra(&mut self, t: u8) -> Result<u8, String> {
        match t {
            0 | 7 => self.u8(),
            3 => {
                self.string()?;
                Ok(0)
            }
            4 => {
                self.string()?;
                self.i32()?;
                Ok(0)
            }
            _ => Ok(0),
        }
    }
    fn value(&mut self, t: u8, a: u8) -> Result<N, String> {
        if t == 0 {
            self.primitive(a)
        } else {
            self.record()
        }
    }
    fn array(&mut self, id: i32, n: usize, t: u8, a: u8) -> Result<N, String> {
        if n > 4_000_000 {
            return Err("Legacy array too large".into());
        }
        if t == 0 && a == 2 {
            let b = self.take(n)?.to_vec();
            self.objects.insert(id, N::Bytes(b));
            return Ok(N::Ref(id));
        }
        let mut v = Vec::new();
        while v.len() < n {
            match self.value(t, a)? {
                N::Nulls(k) => {
                    if k > n - v.len() {
                        return Err("Invalid null run".into());
                    }
                    v.resize(v.len() + k, N::Null)
                }
                x => v.push(x),
            }
        }
        self.objects.insert(id, N::Array(v));
        Ok(N::Ref(id))
    }
    fn class(&mut self, id: i32, m: Meta) -> Result<N, String> {
        let mut f = Vec::new();
        for ((name, t), a) in m.names.iter().zip(&m.types).zip(&m.args) {
            f.push((name.clone(), self.value(*t, *a)?));
        }
        self.objects.insert(id, N::Obj(m.name, f));
        Ok(N::Ref(id))
    }
    fn record(&mut self) -> Result<N, String> {
        self.depth += 1;
        if self.depth > 128 || self.objects.len() > 100000 {
            return Err("Legacy structure exceeds limits".into());
        }
        let tag = self.u8()?;
        let result = match tag {
            0 => {
                self.root = self.i32()?;
                self.take(12)?;
                Ok(N::Null)
            }
            12 => {
                self.i32()?;
                self.string()?;
                self.record()
            }
            4 | 5 => {
                let id = self.i32()?;
                let name = self.string()?;
                let n = self.count()?;
                if n > 1000 {
                    return Err("Too many class fields".into());
                }
                let mut names = vec![];
                for _ in 0..n {
                    names.push(self.string()?);
                }
                let mut types = vec![];
                for _ in 0..n {
                    types.push(self.u8()?);
                }
                let mut args = vec![];
                for t in &types {
                    args.push(self.extra(*t)?);
                }
                if tag == 5 {
                    self.i32()?;
                }
                let m = Meta {
                    name,
                    names,
                    types,
                    args,
                };
                self.meta.insert(id, m.clone());
                self.class(id, m)
            }
            1 => {
                let id = self.i32()?;
                let mid = self.i32()?;
                let m = self.meta.get(&mid).ok_or("Missing class metadata")?.clone();
                self.class(id, m)
            }
            6 => {
                let id = self.i32()?;
                let s = self.string()?;
                self.objects.insert(id, N::Str(s));
                Ok(N::Ref(id))
            }
            7 => {
                let id = self.i32()?;
                let shape = self.u8()?;
                let rank = self.count()?;
                if rank > 8 {
                    return Err("Too many array dimensions".into());
                }
                let mut n = 1usize;
                for _ in 0..rank {
                    n = n.checked_mul(self.count()?).ok_or("Array overflow")?;
                }
                if shape >= 3 {
                    for _ in 0..rank {
                        self.i32()?;
                    }
                }
                let t = self.u8()?;
                let a = self.extra(t)?;
                self.array(id, n, t, a)
            }
            8 => {
                let t = self.u8()?;
                self.primitive(t)
            }
            9 => Ok(N::Ref(self.i32()?)),
            10 => Ok(N::Null),
            11 => Err("END".into()),
            13 => Ok(N::Nulls(self.u8()? as usize)),
            14 => Ok(N::Nulls(self.count()?)),
            15 => {
                let id = self.i32()?;
                let n = self.count()?;
                let t = self.u8()?;
                self.array(id, n, 0, t)
            }
            16 | 17 => {
                let id = self.i32()?;
                let n = self.count()?;
                self.array(id, n, 2, 0)
            }
            _ => Err(format!("Unsupported legacy record {}", tag)),
        };
        self.depth -= 1;
        result
    }
    fn resolve<'b>(&'b self, mut n: &'b N) -> &'b N {
        for _ in 0..100 {
            if let N::Ref(id) = n {
                n = self.objects.get(id).unwrap_or(&N::Null)
            } else {
                return n;
            }
        }
        &N::Null
    }
    fn field<'b>(&'b self, n: &'b N, key: &str) -> &'b N {
        if let N::Obj(_, fields) = self.resolve(n) {
            let suffix = format!("<{}>k__BackingField", key);
            for (k, v) in fields {
                if k == key || k.ends_with(&suffix) {
                    return self.resolve(v);
                }
            }
        }
        &N::Null
    }
    fn number(&self, n: &N, key: &str) -> f32 {
        match self.field(n, key) {
            N::Int(i) => *i as f32,
            N::Float(f) => *f as f32,
            _ => 0.,
        }
    }
    fn text(&self, n: &N, key: &str) -> String {
        if let N::Str(s) = self.field(n, key) {
            s.clone()
        } else {
            String::new()
        }
    }
    fn enabled(&self, n: &N) -> bool {
        matches!(self.field(n, "enabled"), N::Bool(true))
    }
    fn color(&self, n: &N) -> [u8; 4] {
        let mut v = match self.field(n, "value") {
            N::Int(v) => *v as u32,
            _ => 0,
        };
        if v == 0 {
            static COLORS: std::sync::OnceLock<Vec<u32>> = std::sync::OnceLock::new();
            let colors = COLORS.get_or_init(|| {
                serde_json::from_str(
                    include_str!("../known_colors.json").trim_start_matches('\u{feff}'),
                )
                .unwrap_or_default()
            });
            v = colors
                .get(self.number(n, "knownColor") as usize)
                .copied()
                .unwrap_or(0);
        }
        if v != 0 {
            [(v >> 16) as u8, (v >> 8) as u8, v as u8, (v >> 24) as u8]
        } else {
            [235, 235, 235, 255]
        }
    }
    fn bitmap(&self, n: &N) -> Option<String> {
        if let N::Bytes(bytes) = self.field(n, "Data") {
            Some(STANDARD.encode(bytes))
        } else {
            None
        }
    }
    fn list<'b>(&'b self, n: &'b N) -> Vec<&'b N> {
        if let N::Array(items) = self.field(n, "_items") {
            items
                .iter()
                .take(self.number(n, "_size").max(0.) as usize)
                .map(|v| self.resolve(v))
                .collect()
        } else {
            vec![]
        }
    }
}
fn binding(name: &str) -> &'static str {
    match name {
        "CpuLoad" => "cpu.usage",
        "CpuFreq" => "cpu.frequency",
        "RamLoad" => "memory.usage",
        "RamValue" => "memory.used",
        "HDDLoad" => "disk.usage",
        "InternetDown" => "network.download",
        "InternetUp" => "network.upload",
        "GpuLoad" => "gpu.usage",
        "GpuTemp" => "gpu.temperature",
        "CpuTemp" => "cpu.temperature",
        "HDDTemp" => "disk.temperature",
        "GpuRam" => "gpu.memory",
        // 邁::뫾(), RVA 0x12418, IL_0722..076b -> IL_051f..053c.
        "GpuRamLoad" => "gpu.memory.usage",
        "CpuFan" => "cpu.fan",
        "GpuFan" => "gpu.fan",
        "CpuPower" => "cpu.power",
        "VulomeLoad" | "Volume" => "volume.level",
        "Weather" => "weather.summary",
        _ => "",
    }
}
const MARKER: &[u8] = b"\nCOCKPIT_STUDIO_JSON_V1\n";
pub fn import(bytes: &[u8]) -> Result<Project, String> {
    if bytes.len() > 16_000_000 {
        return Err("Theme exceeds 16 MB".into());
    }
    if let Some(i) = bytes.windows(MARKER.len()).rposition(|w| w == MARKER) {
        return serde_json::from_slice(&bytes[i + MARKER.len()..]).map_err(|e| e.to_string());
    }
    let mut r = Reader::new(bytes);
    loop {
        match r.record() {
            Ok(_) => {}
            Err(e) if e == "END" => break,
            Err(e) => return Err(e),
        }
    }
    let root = r.objects.get(&r.root).ok_or("Missing theme")?;
    if !matches!(root,N::Obj(t,_)if t=="usb_hid.ThemeConfig") {
        return Err("This file is not a screen theme".into());
    }
    let mut p = Project::blank();
    p.name = r.text(root, "name");
    p.portrait = !matches!(r.field(root, "isLanscape"), N::Bool(true));
    p.wallpaper = r.bitmap(r.field(root, "backImg"));
    // The original theme editor flattens pictures, text and drawn geometry into
    // osdImg; backImg alone is only the bare wallpaper. Keep this as an editable
    // image layer rather than silently discarding all of that artwork.
    let composite = if matches!(r.field(root, "useOsdWithFontBack"), N::Bool(true)) {
        r.bitmap(r.field(root, "osdWithFontBack"))
            .or_else(|| r.bitmap(r.field(root, "osdImg")))
    } else {
        r.bitmap(r.field(root, "osdImg"))
    };
    if let Some(image) = composite {
        let mut artwork = Widget::new(1, Kind::Image, 0., 0.);
        artwork.name = "Legacy artwork (text, images and shapes)".into();
        let (width, height) = p.dimensions();
        artwork.w = width as f32;
        artwork.h = height as f32;
        artwork.image = Some(image);
        artwork.show_label = false;
        artwork.locked = true;
        p.widgets.push(artwork);
    }
    let mut nodes = Vec::new();
    for key in ["tipList", "barList", "archList"] {
        for node in r.list(r.field(root, key)) {
            nodes.push((node, 0., 0., true));
        }
    }
    for group in r.list(r.field(root, "groupList")) {
        if let Some(image) = r.bitmap(r.field(group, "backImg")) {
            let mut artwork = Widget::new(
                p.widgets.len() as u64 + 1,
                Kind::Image,
                r.number(group, "baseX"),
                r.number(group, "baseY"),
            );
            if let Ok(bytes) = STANDARD.decode(&image) {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    artwork.w = img.width() as f32;
                    artwork.h = img.height() as f32;
                }
            }
            artwork.name = format!("{} artwork", r.text(group, "groupName"));
            artwork.image = Some(image);
            artwork.show_label = false;
            artwork.visible = r.enabled(group);
            artwork.locked = true;
            p.widgets.push(artwork);
        }
        for node in r.list(r.field(group, "itemList")) {
            nodes.push((
                node,
                r.number(group, "baseX"),
                r.number(group, "baseY"),
                r.enabled(group),
            ));
        }
    }
    for (n, base_x, base_y, group_enabled) in nodes {
        if let N::Obj(typ, _) = n {
            if ![
                "usb_hid.TItem",
                "usb_hid.ScreenTip",
                "usb_hid.StatuBar",
                "usb_hid.ArchBar",
            ]
            .contains(&typ.as_str())
            {
                continue;
            }
            let kind = match typ.as_str() {
                "usb_hid.StatuBar" => Kind::Bar,
                "usb_hid.ArchBar" => Kind::Ring,
                "usb_hid.ScreenTip" => Kind::Text,
                _ => Kind::Value,
            };
            let mut w = Widget::new(
                p.widgets.len() as u64 + 1,
                kind,
                r.number(n, "posX") + base_x,
                r.number(n, "posY") + base_y,
            );
            let name = r.text(n, "itemName");
            w.name = name.clone();
            w.label = name.clone();
            w.binding = binding(&name).into();
            w.show_label = false;
            if !w.binding.is_empty() {
                w.decimals = Some(
                    if ["CpuFreq", "InternetUp", "InternetDown", "RamValue"]
                        .contains(&name.as_str())
                    {
                        1
                    } else {
                        0
                    },
                );
            }
            if name.starts_with("userDefine") {
                w.kind = Kind::Text;
            }
            w.visible = if typ == "usb_hid.ScreenTip" {
                true
            } else {
                r.enabled(n) && group_enabled
            };
            w.text = r.text(
                n,
                if typ == "usb_hid.ScreenTip" {
                    "content"
                } else {
                    "valueStr"
                },
            );
            if w.text.is_empty() {
                w.text = "—".into();
            }
            let f = r.field(n, "fontType");
            // The original glyph constructor uses Microsoft YaHei, FontStyle.Bold.
            // FontConfig's numeric name is a size preset, not a font family.
            w.font = "Microsoft YaHei Bold".into();
            w.size = r.number(f, "size").max(6.);
            w.color = r.color(r.field(f, "color"));
            let cell_width = r.number(f, "W").max(1.);
            let cell_height = r.number(f, "H").max(1.);
            // TItem RVA 0x4f80c IL_0744..0751 and 0x500ec IL_01ea..01f7:
            // interval is the full advance, even when cells overlap.
            let advance = r.number(f, "interval").max(1.);
            let stored_count = r.number(n, "strLen");
            let count = if stored_count > 0. {
                stored_count
            } else if name == "Date" {
                10.
            } else if name == "Weather" {
                ((p.dimensions().0 as f32 - w.x) / advance).max(1.)
            } else {
                w.text.chars().count().max(1) as f32
            };
            w.w = cell_width + (count - 1.) * advance;
            w.h = cell_height;
            w.legacy_text = Some(LegacyText {
                cell_width,
                cell_height,
                advance,
                base_size: w.size,
            });
            if w.kind == Kind::Bar {
                // StatuBar RVA0x60590 IL_00d8..0105,0247..02c7: mode0
                // restores the unfilled rectangle from the composed screen.
                // ThemeConfig getter0x62ba0 reads field0x040003cf (barType).
                w.legacy_bar_image_background = r.number(root, "barType") == 0.;
                w.w = r.number(n, "length").max(1.);
                if w.binding == "cpu.frequency" {
                    // System bindings use GHz; original CpuFreq bars use 8000 MHz.
                    w.max = LEGACY_CPU_FREQUENCY_MAX_MHZ as f64 / 1000.;
                }
                let height = r.number(n, "height");
                w.h = if height > 0. {
                    height
                } else {
                    crate::original::BAR_DEFAULT_HEIGHT
                };
                w.color = r.color(r.field(n, "frontColor"));
                w.background = r.color(r.field(n, "backColor"));
            }
            if w.kind == Kind::Ring {
                // ArchBar::駑(), RVA 0x4db44: archR is the stroke's centerline diameter.
                w.ring_thickness = r.number(n, "archWidth").max(1.);
                w.w = r.number(n, "archR").max(1.) + w.ring_thickness;
                w.h = w.w;
                w.x += 30. - r.number(n, "offsetX") - w.ring_thickness / 2.;
                w.y += 30. - r.number(n, "offsetY") - w.ring_thickness / 2.;
                w.ring_start = r.number(n, "startPer") * crate::original::ARC_STEP_DEGREES;
                w.legacy_arc = true;
                // Getter RVA 0x4bb34 -> field0x04000321; mask0x4dfc8
                // omits every fourth segment when useInternal is true.
                w.legacy_arc_gaps = matches!(r.field(n, "useInternal"), N::Bool(true));
                w.color = r.color(r.field(n, "frontArchColor"));
                w.background = r.color(r.field(n, "backArchColor"));
            }
            if name == "Time" || name == "Date" {
                w.kind = Kind::Clock;
                w.text = if name == "Date" { "date" } else { "time" }.into();
                if name == "Time" {
                    // 邁::뫾(), RVA 0x12418, decodes the original format as HH:mm.
                    w.clock_format = "%H:%M".into();
                } else {
                    // TItem RVA 0x507d4 / 0x50ba8 decodes yyyy/M/d.
                    w.clock_format = "%Y/%-m/%-d".into();
                }
            }
            if typ == "usb_hid.ScreenTip" {
                w.name = "Text".into();
                w.size = r.number(n, "size").max(6.);
                w.w = r.number(n, "W").max(1.);
                w.h = r.number(n, "H").max(1.);
                w.legacy_text = None;
                let font = r.field(n, "font");
                let family = r.text(font, "Name");
                if !family.is_empty() {
                    let style = r.number(font, "Style") as u32;
                    w.font = if style & 1 != 0 {
                        format!("{family} Bold")
                    } else {
                        family
                    };
                }
                let stored_size = r.number(font, "Size");
                if stored_size > 0. {
                    // System.Drawing.Font normally stores points (GraphicsUnit.Point=3).
                    w.size = stored_size
                        * if r.number(font, "Unit") == 3. {
                            96. / 72.
                        } else {
                            1.
                        };
                }
                w.color = r.color(r.field(n, "color"));
            }
            p.widgets.push(w);
        }
    }
    Ok(p)
}

// Encoder emits only known data classes, a bitmap, and empty legacy live lists.
// The editable project follows MessageEnd; the original reader ignores trailing data.
// Legacy applications see a pixel-exact snapshot; Cockpit Studio keeps live bindings.
struct Writer {
    b: Vec<u8>,
    id: i32,
}
impl Writer {
    fn new() -> Self {
        Self { b: vec![], id: 20 }
    }
    fn u8(&mut self, x: u8) {
        self.b.push(x)
    }
    fn i32(&mut self, x: i32) {
        self.b.extend(x.to_le_bytes())
    }
    fn str(&mut self, s: &str) {
        let mut n = s.len();
        while n >= 128 {
            self.u8((n as u8) | 128);
            n >>= 7;
        }
        self.u8(n as u8);
        self.b.extend(s.as_bytes());
    }
    fn library(&mut self, id: i32, name: &str) {
        self.u8(12);
        self.i32(id);
        self.str(name)
    }
    fn reference(&mut self, id: i32) {
        self.u8(9);
        self.i32(id)
    }
    fn string(&mut self, s: &str) {
        self.id += 1;
        self.u8(6);
        self.i32(self.id);
        self.str(s)
    }
    fn class(&mut self, id: i32, name: &str, fields: &[(&str, u8, &str, u8)], lib: i32) {
        self.u8(if lib == 0 { 4 } else { 5 });
        self.i32(id);
        self.str(name);
        self.i32(fields.len() as i32);
        for (f, _, _, _) in fields {
            self.str(f);
        }
        for (_, t, _, _) in fields {
            self.u8(*t);
        }
        for (_, t, extra, prim) in fields {
            match t {
                0 | 7 => self.u8(*prim),
                3 => self.str(extra),
                4 => {
                    self.str(extra);
                    self.i32(if extra.starts_with("System.Drawing") {
                        3
                    } else {
                        2
                    })
                }
                _ => {}
            }
        }
        if lib != 0 {
            self.i32(lib);
        }
    }
    fn list(&mut self, id: i32, kind: &str) {
        let full=format!("System.Collections.Generic.List`1[[usb_hid.{}, usb_hid, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null]]",kind);
        let array_type = format!("usb_hid.{}[]", kind);
        self.class(
            id,
            &full,
            &[
                ("_items", 4, &array_type, 0),
                ("_size", 0, "", 8),
                ("_version", 0, "", 8),
            ],
            0,
        );
        self.u8(7);
        self.i32(id + 100);
        self.u8(0);
        self.i32(1);
        self.i32(0);
        self.u8(4);
        self.str(&format!("usb_hid.{}", kind));
        self.i32(2);
        self.i32(0);
        self.i32(0);
    }
    fn bitmap(&mut self, id: i32, png: &[u8]) {
        self.class(id, "System.Drawing.Bitmap", &[("Data", 7, "", 2)], 3);
        self.u8(15);
        self.i32(id + 100);
        self.i32(png.len() as i32);
        self.u8(2);
        self.b.extend(png);
    }
    fn color(&mut self, id: i32, c: [u8; 4]) {
        self.class(
            id,
            "System.Drawing.Color",
            &[
                ("name", 1, "", 0),
                ("value", 0, "", 9),
                ("knownColor", 0, "", 7),
                ("state", 0, "", 7),
            ],
            3,
        );
        self.u8(10);
        let v = ((c[3] as u32) << 24) | ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32;
        self.b.extend((v as i64).to_le_bytes());
        self.b.extend(0i16.to_le_bytes());
        self.b.extend(2i16.to_le_bytes());
    }
}
pub fn export(p: &Project, d: &DataState) -> Result<Vec<u8>, String> {
    let mut w = Writer::new();
    w.u8(0);
    w.i32(1);
    w.i32(-1);
    w.i32(1);
    w.i32(0);
    w.library(
        2,
        "usb_hid, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null",
    );
    w.library(
        3,
        "System.Drawing, Version=4.0.0.0, Culture=neutral, PublicKeyToken=b03f5f7f11d50a3a",
    );
    let list = |k: &str| {
        format!("System.Collections.Generic.List`1[[usb_hid.{}, usb_hid, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null]]",k)
    };
    let kinds = [
        list("ScreenTip"),
        list("TGroup"),
        list("StatuBar"),
        list("ArchBar"),
    ];
    let names = [
        "isLanscape",
        "deviceIndex",
        "tipList",
        "groupList",
        "barList",
        "archList",
        "osdImg",
        "backImg",
        "osdWithFontBack",
        "enbaleOsdWithFontBack",
        "useOsdWithFontBack",
        "needRereshArch",
        "name",
        "userInfoCnt",
        "barType",
        "barFrontIndex",
        "barBackIndex",
        "barFrontColor",
        "barBackColor",
    ]
    .map(|s| format!("<{}>k__BackingField", s));
    let fields: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(i, n)| match i {
            0 | 9 | 10 | 11 => (n.as_str(), 0, "", 1),
            1 | 13 | 14 | 15 | 16 => (n.as_str(), 0, "", 8),
            2..=5 => (n.as_str(), 3, kinds[i - 2].as_str(), 0),
            6..=8 => (n.as_str(), 4, "System.Drawing.Bitmap", 0),
            17 | 18 => (n.as_str(), 4, "System.Drawing.Color", 0),
            _ => (n.as_str(), 1, "", 0),
        })
        .collect();
    w.class(1, "usb_hid.ThemeConfig", &fields, 2);
    w.u8((!p.portrait) as u8);
    w.i32(1);
    for id in 4..=10 {
        w.reference(id);
    }
    w.u8(0);
    w.u8(0);
    w.u8(0);
    w.string(&p.name);
    w.i32(0);
    w.i32(0);
    w.i32(6);
    w.i32(6);
    w.reference(12);
    w.reference(13);
    for (id, kind) in [
        (4, "ScreenTip"),
        (5, "TGroup"),
        (6, "StatuBar"),
        (7, "ArchBar"),
    ] {
        w.list(id, kind);
    }
    let png = render::png(&render::frame(p, d))?;
    for id in 8..=10 {
        w.bitmap(id, &png);
    }
    w.color(12, [98, 225, 187, 255]);
    w.color(13, p.background);
    w.u8(11);
    w.b.extend(MARKER);
    w.b.extend(serde_json::to_vec(p).map_err(|e| e.to_string())?);
    Ok(w.b)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imported_bar_restores_theme_artwork_when_bar_type_is_zero() {
        // Original StatuBar RVA0x60590 uses the saved screen image for the
        // unfilled area when ThemeConfig.barType==0; other modes use backColor.
        for bar_type in [0, 1] {
            let mut file = Writer::new();
            file.u8(0);
            for value in [1, -1, 1, 0] {
                file.i32(value);
            }
            file.library(2, "usb_hid");
            file.class(
                1,
                "usb_hid.ThemeConfig",
                &[("barType", 0, "", 8), ("barList", 2, "", 0)],
                2,
            );
            file.i32(bar_type);
            file.class(2, "List", &[("_size", 0, "", 8), ("_items", 5, "", 0)], 0);
            file.i32(1);
            file.u8(16);
            file.i32(3);
            file.i32(1);
            file.class(
                4,
                "usb_hid.StatuBar",
                &[
                    ("itemName", 1, "", 0),
                    ("length", 0, "", 8),
                    ("height", 0, "", 8),
                    ("enabled", 0, "", 1),
                ],
                2,
            );
            file.string("CpuLoad");
            file.i32(150);
            file.i32(13);
            file.u8(1);
            file.u8(11);
            let mut project = import(&file.b).unwrap();
            let bar = &mut project.widgets[0];
            bar.x = 20.;
            bar.y = 30.;
            bar.color = [250, 20, 30, 255];
            bar.background = [1, 2, 3, 255];
            let mut artwork = Widget::new(2, Kind::Image, 0., 0.);
            artwork.w = 480.;
            artwork.h = 320.;
            let image = image::RgbaImage::from_fn(480, 320, |x, y| {
                image::Rgba([x as u8, y as u8, 80, 255])
            });
            artwork.image = Some(STANDARD.encode(render::png(&image).unwrap()));
            project.widgets.insert(0, artwork);
            let mut data = DataState::default();
            let mut cache = render::FrameCache::default();
            for value in [50, 10, 99, 0] {
                data.values.insert("cpu.usage".into(), value.to_string());
                let frame = render::frame(&project, &data);
                assert_eq!(cache.frame(&project, &data), frame);
                let extent = (value as f64 * 0.01 * 150.).trunc() as u32;
                for local_x in 0..150 {
                    let (x, y) = (20 + local_x, 36);
                    let expected = if local_x < extent {
                        [250, 20, 30, 255]
                    } else if bar_type == 0 {
                        image.get_pixel(x, y).0
                    } else {
                        [1, 2, 3, 255]
                    };
                    assert_eq!(
                        frame.get_pixel(x, y).0,
                        expected,
                        "mode {bar_type}, value {value}, x {local_x}"
                    );
                }
            }
            let restored: Project =
                serde_json::from_slice(&serde_json::to_vec(&project).unwrap()).unwrap();
            assert_eq!(
                restored.widgets[1].legacy_bar_image_background,
                bar_type == 0
            );
            assert_eq!(restored.widgets[1].background, [1, 2, 3, 255]);
            assert_eq!(
                render::frame(&restored, &data),
                render::frame(&project, &data)
            );
            let mut cache = render::FrameCache::default();
            assert_eq!(cache.frame(&project, &data), render::frame(&project, &data));
            data.values.insert("cpu.usage".into(), "50".into());
            assert_eq!(cache.frame(&project, &data), render::frame(&project, &data));
            let snapshot = export(&project, &data).unwrap();
            assert_eq!(
                import(&snapshot).unwrap().widgets[1].legacy_bar_image_background,
                bar_type == 0
            );
            let mut old_json = serde_json::to_value(&project.widgets[1]).unwrap();
            old_json
                .as_object_mut()
                .unwrap()
                .remove("legacy_bar_image_background");
            assert!(
                !serde_json::from_value::<Widget>(old_json)
                    .unwrap()
                    .legacy_bar_image_background
            );
        }
    }
    #[test]
    fn arc_interval_setting_survives_nrbf_and_json_imports() {
        // A real NRBF record graph exercises field names and boolean decoding,
        // without activating CLR objects or depending on installed themes.
        for setting in [Some(true), Some(false), None] {
            let mut file = Writer::new();
            file.u8(0);
            for value in [1, -1, 1, 0] {
                file.i32(value);
            }
            file.library(2, "usb_hid");
            file.class(1, "usb_hid.ThemeConfig", &[("archList", 2, "", 0)], 2);
            file.class(2, "List", &[("_size", 0, "", 8), ("_items", 5, "", 0)], 0);
            file.i32(1);
            file.u8(16);
            file.i32(3);
            file.i32(1);
            let mut fields = vec![("itemName", 1, "", 0), ("archR", 0, "", 8)];
            if setting.is_some() {
                fields.push(("<useInternal>k__BackingField", 0, "", 1));
            }
            file.class(4, "usb_hid.ArchBar", &fields, 2);
            file.string("CpuLoad");
            file.i32(90);
            if let Some(value) = setting {
                file.u8(u8::from(value));
            }
            file.u8(11);
            let imported = import(&file.b).unwrap();
            assert_eq!(imported.widgets.len(), 1);
            let arc = &imported.widgets[0];
            assert!(arc.legacy_arc);
            assert_eq!(arc.legacy_arc_gaps, setting.unwrap_or(false));
            let restored: Project =
                serde_json::from_slice(&serde_json::to_vec(&imported).unwrap()).unwrap();
            assert_eq!(
                restored.widgets[0].legacy_arc_gaps,
                setting.unwrap_or(false)
            );
            let snapshot = export(&imported, &DataState::default()).unwrap();
            assert_eq!(
                import(&snapshot).unwrap().widgets[0].legacy_arc_gaps,
                setting.unwrap_or(false)
            );
        }
    }
    #[test]
    fn legacy_snapshot_contains_valid_graph_and_editable_project() {
        let p = Project::starter();
        let bytes = export(&p, &DataState::default()).unwrap();
        let mut r = Reader::new(&bytes);
        loop {
            match r.record() {
                Ok(_) => {}
                Err(e) if e == "END" => break,
                Err(e) => panic!("{} at {}", e, r.p),
            }
        }
        let root = r.objects.get(&1).unwrap();
        assert_eq!(r.text(root, "name"), p.name);
        assert_eq!(r.list(r.field(root, "groupList")).len(), 0);
        let encoded = r.bitmap(r.field(root, "backImg")).unwrap();
        let decoded = image::load_from_memory(&STANDARD.decode(encoded).unwrap()).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (480, 320));
        let restored = import(&bytes).unwrap();
        assert_eq!(restored.widgets.len(), p.widgets.len());
        assert_eq!(restored.widgets[2].binding, "cpu.usage");
    }
    #[test]
    fn installed_legacy_themes_decode() {
        let Ok(files) = std::fs::read_dir("C:/Program Files/35inchENG/config") else {
            return;
        };
        let mut count = 0;
        for entry in files.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "data")
                && path.file_name().is_some_and(|n| n != "AppConfig.data")
            {
                let p = import(&std::fs::read(&path).unwrap())
                    .unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
                assert!(!p.name.is_empty());
                assert!(!p.widgets.is_empty());
                // All old imports carry FontConfig metrics or the ArchBar flag,
                // so saved JSON also retains the original live-value ceiling.
                let mut data = DataState::default();
                for w in &p.widgets {
                    if matches!(
                        w.name.as_str(),
                        "CpuLoad"
                            | "CpuTemp"
                            | "GpuLoad"
                            | "GpuTemp"
                            | "GpuRamLoad"
                            | "RamLoad"
                            | "Volume"
                            | "VulomeLoad"
                    ) {
                        data.values.insert(w.binding.clone(), "100".into());
                        assert_eq!(data.value(w), "99", "{} / {}", path.display(), w.name);
                        assert_eq!(data.number(w), Some(99.));
                    }
                }
                assert!(
                    p.widgets
                        .iter()
                        .any(|w| w.kind == Kind::Image && w.image.is_some()),
                    "{} missing saved artwork",
                    path.display()
                );
                if path
                    .file_name()
                    .is_some_and(|n| n == "LandscapeTechnologyCh.data")
                {
                    let usage = p
                        .widgets
                        .iter()
                        .find(|w| w.kind == Kind::Value && w.name == "CpuLoad")
                        .unwrap();
                    assert_eq!((usage.w, usage.h), (44., 28.));
                    let temp = p
                        .widgets
                        .iter()
                        .find(|w| w.kind == Kind::Value && w.name == "CpuTemp")
                        .unwrap();
                    assert_eq!((temp.w, temp.h), (21., 13.));
                    assert_eq!(
                        p.widgets
                            .iter()
                            .find(|w| w.name == "userDefine1")
                            .unwrap()
                            .kind,
                        Kind::Text
                    );
                    let mut data = DataState::default();
                    for key in ["cpu.usage", "cpu.temperature", "gpu.usage", "memory.usage"] {
                        data.values.insert(key.into(), "42".into());
                    }
                    let image = render::frame(&p, &data);
                    image.save("evidence/legacy-import-preview.png").unwrap();
                }
                if path.file_name().is_some_and(|n| n == "3.5inchTheme1.data") {
                    let ring = p
                        .widgets
                        .iter()
                        .find(|w| w.name == "CpuLoad" && w.kind == Kind::Ring)
                        .unwrap();
                    assert_eq!((ring.x, ring.y, ring.w, ring.h), (189.5, 102.5, 103., 103.));
                    assert_eq!(ring.ring_thickness, 5.);
                    assert!((ring.ring_start + 54.).abs() < 0.001);
                    assert!(ring.legacy_arc);
                    assert_eq!(ring.background, [0, 128, 192, 255]);
                    let bar = p
                        .widgets
                        .iter()
                        .find(|w| w.name == "CpuLoad" && w.kind == Kind::Bar)
                        .unwrap();
                    assert_eq!((bar.w, bar.h), (150., 13.));
                    let clock = p.widgets.iter().find(|w| w.name == "Time").unwrap();
                    assert_eq!(clock.clock_format, "%H:%M");
                }
                if path.file_name().is_some_and(|n| n == "A cyberpunk.data") {
                    // Literal saved cells overlap: W=22/interval=20 and W=10/interval=9.
                    let frequency = p
                        .widgets
                        .iter()
                        .find(|w| w.name == "CpuFreq" && w.kind == Kind::Value)
                        .unwrap();
                    assert_eq!((frequency.w, frequency.h), (62., 28.));
                    assert_eq!(frequency.legacy_text.as_ref().unwrap().advance, 20.);
                    let download = p
                        .widgets
                        .iter()
                        .find(|w| w.name == "InternetDown" && w.kind == Kind::Value)
                        .unwrap();
                    assert_eq!((download.w, download.h), (37., 13.));
                    assert_eq!(download.legacy_text.as_ref().unwrap().advance, 9.);
                }
                if let Some(date) = p.widgets.iter().find(|w| w.name == "Date") {
                    let fixture = chrono::NaiveDate::from_ymd_opt(2026, 1, 3).unwrap();
                    assert_eq!(fixture.format(&date.clock_format).to_string(), "2026/1/3");
                }
                count += 1;
            }
        }
        assert_eq!(count, 35);
    }
    #[test]
    fn truncated_records_return_errors() {
        for n in [0, 1, 4, 16, 32, 50] {
            assert!(import(&vec![0; n]).is_err());
        }
    }
    #[test]
    fn original_volume_and_gpu_memory_names_use_distinct_live_bindings() {
        assert_eq!(binding("VulomeLoad"), "volume.level");
        assert_eq!(binding("Volume"), "volume.level");
        assert_eq!(binding("GpuRam"), "gpu.memory");
        assert_eq!(binding("GpuRamLoad"), "gpu.memory.usage");
    }
}
