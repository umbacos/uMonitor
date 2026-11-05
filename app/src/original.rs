//! Rules recovered from UsbMonitor.exe (preferred image base 0x400000).
//! Addresses below are method RVAs. Full CLR bodies, references and the 40
//! awaited native responses are in evidence/deep-columns-audit.json.
//! Fresh inspection: evidence/deep-build-audit.json (31 awaited responses).
//! Latest request: evidence/deep-request-audit.json (39 awaited responses).
//! Ghidra native output is invalid for this CLR assembly; use the managed IL.

// Dimensions: 0x2f090; command values: 0x2fac0/0x2f2c0/0x10384/0x2f1f0.
pub const SCREEN_W: u32 = 480;
pub const SCREEN_H: u32 = 320;
pub const IMAGE_OPCODE: u8 = 197;
pub const ORIENTATION_OPCODE: u8 = 121;
pub const BRIGHTNESS_OPCODE: u8 = 110;
pub const MODE_OPCODE: u8 = 122;
// Packed-command 0x2f8a4; settings allocation 0x2f1f0/0x2f2c0.
pub const COMMAND_HEADER_LEN: usize = 6;
pub const SETTINGS_PACKET_LEN: usize = 16;
pub const ORIENTATION_BIAS: u8 = 100;
// Serial open 0x2ebb0 IL_01d2..0211; waits 0x2f1f0/0x2f2c0.
pub const SERIAL_BAUD: u32 = 115200;
pub const SERIAL_TIMEOUT_MS: u64 = 1000;
pub const SERIAL_DATA_BITS: serialport::DataBits = serialport::DataBits::Eight;
pub const SERIAL_STOP_BITS: serialport::StopBits = serialport::StopBits::One;
pub const SERIAL_PARITY: serialport::Parity = serialport::Parity::None;
pub const SERIAL_DTR: bool = true;
pub const SERIAL_RTS: bool = true;
pub const COMMAND_DELAY_MS: u64 = 10;
// Hardware static initializer0x46498 IL_00a9..00ab -> wrapper0x44cac IL_00ad..00b2.
pub const HARDWARE_INIT_FLAGS: i32 = 64;
// Managed P/Invoke declarations, tokens0x0600026f/270/273..278/27a.
// Discovery0x41b08 IL_0351 uses device-count token0x06000273 (#630).
pub const HARDWARE_INIT_ORDINAL: u16 = 684;
pub const HARDWARE_CLOSE_ORDINAL: u16 = 458;
pub const HARDWARE_COUNT_ORDINAL: u16 = 630;
pub const HARDWARE_REFRESH_ORDINAL: u16 = 216;
pub const HARDWARE_NAME_ORDINAL: u16 = 531;
pub const HARDWARE_TEMPERATURE_ORDINAL: u16 = 566;
pub const HARDWARE_FAN_ORDINAL: u16 = 663;
pub const HARDWARE_POWER_ORDINAL: u16 = 579;
// Controller 0x12418 IL_0809..0822; postprocessor 0x15b78.
pub const LEGACY_CPU_FREQUENCY_MAX_MHZ: f32 = 8000.;
pub const SENSOR_CEILING: f64 = 99.;
// ArchBar 0x4db44/0x4dfc8; StatuBar 0x5fa64 IL_00ed..00f0.
pub const ARC_SEGMENTS: f64 = 100.;
pub const ARC_STEP_DEGREES: f32 = 3.6;
// ArchBar 0x4dfc8 IL_0249..026e: useInternal skips i % 4 == 0.
pub const ARC_GAP_PERIOD: u32 = 4;
pub const BAR_DEFAULT_HEIGHT: f32 = 13.;

// 0x4dfc8: i=0..99, start=(i+startPer)*3.6, sweep=3.6 degrees.
// The renderer supplies a normalized turn relative to the saved start angle.
// The gap keeps the full-circle background drawn by 0x4dde4.
pub fn arc_gap(turn: f32) -> bool {
    (turn * ARC_SEGMENTS as f32).floor() as u32 % ARC_GAP_PERIOD == 0
}

// 0x100e8 selects 0/1 portrait or 2/3 landscape; 0x2f2c0 adds100.
pub fn orientation_byte(portrait: bool, flipped: bool) -> u8 {
    ORIENTATION_BIAS + if portrait { 0 } else { 2 } + u8::from(flipped)
}

// 0x2f8a4 IL_0127..0170: conv.u1 keeps the low byte of each expression.
pub fn command_header(op: u8, x: u16, y: u16, ex: u16, ey: u16) -> [u8; COMMAND_HEADER_LEN] {
    [
        (x >> 2) as u8,
        (((x & 3) << 6) + (y >> 4)) as u8,
        (((y & 15) << 4) + (ex >> 6)) as u8,
        (((ex & 63) << 2) + (ey >> 8)) as u8,
        ey as u8,
        op,
    ]
}

// 0x2f1f0 IL_009f..00b5: zero-filled 16-byte packet, raw mode at byte6.
pub fn mirror_packet(mirrored: bool) -> [u8; SETTINGS_PACKET_LEN] {
    let mut bytes = [0; SETTINGS_PACKET_LEN];
    bytes[..COMMAND_HEADER_LEN].copy_from_slice(&command_header(MODE_OPCODE, 0, 0, 0, 0));
    bytes[6] = u8::from(mirrored);
    bytes
}

// 0x2f2c0 IL_00a0..00e2: orientation+bias, then big-endian dimensions.
pub fn orientation_packet(
    portrait: bool,
    flipped: bool,
    width: u16,
    height: u16,
) -> [u8; SETTINGS_PACKET_LEN] {
    let mut bytes = [0; SETTINGS_PACKET_LEN];
    bytes[..COMMAND_HEADER_LEN].copy_from_slice(&command_header(ORIENTATION_OPCODE, 0, 0, 0, 0));
    bytes[6] = orientation_byte(portrait, flipped);
    bytes[7..9].copy_from_slice(&width.to_be_bytes());
    bytes[9..11].copy_from_slice(&height.to_be_bytes());
    bytes
}

// 0x10384 IL_00ab..00b3: raw brightness is the first packed coordinate.
// Percentage-to-raw conversion belongs to the replacement's UI policy.
pub fn brightness_header(raw: u8) -> [u8; COMMAND_HEADER_LEN] {
    command_header(BRIGHTNESS_OPCODE, raw as u16, 0, 0, 0)
}

// 0x2fac0 IL_005a..0070: image rectangle has inclusive end coordinates.
// The caller supplies nonempty rectangles within the 480x320/320x480 screen.
pub fn image_header(x: u16, y: u16, width: u16, height: u16) -> [u8; COMMAND_HEADER_LEN] {
    command_header(IMAGE_OPCODE, x, y, x + width - 1, y + height - 1)
}

// Sampler 0x113dc: convert byte-rate /1024 /1024 (IL_0425..0457).
// Shared replacement readings use decimal MB/s, hence the initial *1e6.
// Down: IL_045c..0489 negative -> IL_03a9..03b3 zero;
// IL_0539..0548 positive -> IL_035b..036c below0.1 -> IL_010d..0117.
// Up: IL_0154..0181 negative -> IL_00e0..00ea zero;
// IL_028b..029a positive -> IL_04e2..04f3 below0.1 -> IL_05a9..05b3.
// Caller supplies finite values only. Do not round shared provider readings.
pub fn network_mibps(decimal_mbps: f64) -> f64 {
    let rate = decimal_mbps * 1_000_000. / 1024. / 1024.;
    if rate < 0. {
        0.
    } else if rate > 0. && rate < 0.1 {
        0.1
    } else {
        rate
    }
}

// Controller0x12418: IL_1157..1167 (down) /IL_0dc4..0dce (up),
// threshold IL_1855..1885 /IL_1415..1432; >99 selects999.
pub fn network_tenths(rate: f64) -> i32 {
    if rate > SENSOR_CEILING {
        999
    } else {
        let tenths = (rate * 10.).trunc() as i32;
        if tenths == 0 && rate > 0.01 {
            1
        } else {
            tenths
        }
    }
}

// Controller0x12418 IL_1632..167b, float getter0xdc54.
// Preserve Round(double,2) midpoint-to-even and the subsequent binary
// multiplication/truncation: 2900MHz really produces2.8 in this path.
pub fn cpu_frequency_text(ghz: f64) -> String {
    let mhz = (ghz * 1000.) as f32 as f64;
    let rounded = (mhz / 10000. * 100.).round_ties_even() / 100.;
    let mut text = format!("{:02}", (rounded * 100.) as i32);
    text.insert(1, '.');
    text
}

// Controller 0x12418: PadLeft uses 'x' (120), a blank glyph in rasterizer
// 0x527c IL_05f0..0640. Spaces retain those cells in the replacement raster.
// These are minimum widths, never a request to truncate longer readings.
pub fn padded_sensor_text(binding: &str, decimals: Option<u8>, text: String) -> String {
    let columns = match (binding, decimals) {
        // IL_0c6c,0fd0,1466,1607,177f,1811.
        (
            "cpu.usage" | "gpu.usage" | "memory.usage" | "disk.usage" | "disk.temperature"
            | "volume.level",
            Some(0),
        ) => 2,
        // IL_0f1d; IL_0f58,0f93,18b8.
        ("cpu.power", Some(0)) => 3,
        ("gpu.memory" | "cpu.fan" | "gpu.fan", Some(0)) => 5,
        // IL_0dd8..0de3 / IL_1171..117c, then Insert(2,".").
        ("network.upload" | "network.download", Some(1)) => 4,
        _ => 0,
    };
    format!("{:>width$}", text, width = columns)
}

// Controller0x12418: frequency IL_0809..0822, percent e.g.IL_0349..0366.
// Clipping to the canvas's nonnegative width is replacement rendering safety.
pub fn bar_pixels(binding: &str, value: f64, width: f32) -> f32 {
    if binding == "cpu.frequency" {
        ((value * 1000.) as f32 / LEGACY_CPU_FREQUENCY_MAX_MHZ * width)
            .trunc()
            .clamp(0., width)
    } else {
        (value as f32 as f64 * 0.01 * width as f64)
            .trunc()
            .clamp(0., width as f64) as f32
    }
}

// Sensor0x46968 IL_1437..14d6 -> setter0xe54c; cap in0x15b78.
pub fn gpu_memory_percent(used: f64, total: f64) -> f64 {
    (used / total * 100.).trunc().min(SENSOR_CEILING)
}

// RGB565 helper0x51a0 IL_00b4..00ca; bitmap byte path0x2e07c.
pub fn rgb565_pixel(red: u8, green: u8, blue: u8) -> [u8; 2] {
    let value =
        (((red as u16) << 8) & 63488) | (((green as u16) << 3) & 2016) | ((blue as u16) >> 3);
    value.to_le_bytes()
}
