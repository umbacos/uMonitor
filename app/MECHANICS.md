# Original mechanics and Deep-mode audit

## Current inspection: recovered arc setting and verified original rules (2026-11-04)

Opened the original main program **C:\Program Files\35inchENG\UsbMonitor.exe** with `core_target_open` in **Deep mode**, SHA-256 **a980a69b634fe6cb3a20173293d23f5728855f41b62748cf6a4b5b9406f972ab**. Completed three fresh `native_strings_find` searches for **names, settings and messages**, returning **0 / 1 / 3** matches. Followed every hit at **0x524c78, 0x524a92, 0x524be2, 0x524c0e**, plus controller **0x412418** and command builder **0x42f8a4**, with `native_xrefs_list`. Native references were empty: the hits are manifest/version resources, including the unrelated XML `asm.v2` namespace.

Completed **48 individual `native_function_pseudocode` calls sequentially**, awaiting and reading each full result before advancing. [evidence/deep-arc-audit.json](evidence/deep-arc-audit.json) contains every response, **48 matching valid CLR bodies**, decoded names/settings/messages, addresses, method/field tokens and **1,263 incoming managed call references**. The refreshed managed inventory is **107 types, 974 methods, 946 bodies, 9,045 call sites and 985 fields**, evidence **ev_e367ac3c94a18d03619ef56ae892b942a255907ae9d7c39a85030939e771aa73**. Search evidence: **ev_23505e4cb96193aceb9feb24467d1a5990d8ad3f7cf5d898ceb44ec952d9e6ee**, **ev_621011a0bc155475f73ee1e5b569f2d0562c0473f1ddf49b88d1f915282983b6**, **ev_ad7441cdd5baef8170677fc8de194d47de7b5844e1c36415a3055eab3d5c2399**.

**Decompiler limitation:** Ghidra treats CLR headers/IL as machine instructions and returns invalid pseudo-C, often `halt_baddata()`. Those requested outputs are preserved as an inspection record. Behavioral claims use the valid managed instructions and resolved references; no successful C# decompilation or execution of the original is claimed.

### Newly recovered rule applied to the build

The importer previously discarded **ArchBar.useInternal**, causing saved gapped arcs to render continuously. It now preserves that boolean as `legacy_arc_gaps`, keeps it through JSON/legacy snapshot round trips, and exposes **Gapped segments** in imported arc properties. Earlier JSON without the field retains its previous solid appearance.

| Reference chain | RVA / preferred VA and exact rule |
|---|---|
| UI writes the setting | **0x39fd4 / 0x439fd4 IL_00dd..00e9**: `CheckBox.Checked` -> setter token **0x060002b0**. |
| Stored boolean | Setter **0x4ba84 / 0x44ba84 IL_009b..009d** and getter **0x4bb34 / 0x44bb34 IL_009a..00a0**, token **0x060002b1**, share field **0x04000321**, `<useInternal>k__BackingField`. UI **0x36064 / 0x436064 IL_01a7..01bd** reads it back. |
| Segment masks | **0x4dfc8 / 0x44dfc8**, token **0x060002d9**: read boolean **IL_01ea..01f0**; test `i % 4 == 0` **IL_0249..026e**; skip drawing on that branch. Iterate **i = 0..99**; draw at **(i + startPer) * float32(3.6)** with sweep **float32(3.6)**, **IL_028b..02a4 / 02c7..02e0**. |
| Background and transfer | **0x4dde4 / 0x44dde4 IL_008a..0098** draws the full **360-degree** background mask. **0x4e5c8 / 0x44e5c8** excludes white mask pixels; skipped slices emit no coordinates. **0x4d568 / 0x44d568 IL_01d0..0202 / 02e9..0316** sends changed mask coordinates with front/back RGB565 colors, opcode **195**. |

Recovered segment-visibility formula:

```text
for i = 0 .. 99:
    draw = !useInternal || (i % 4 != 0)
    startDegrees = (i + startPer) * float32(3.6)
    sweepDegrees = float32(3.6)
```

The omitted indices are **0, 4, 8, ..., 96**: **25 gaps**, each **3.6 degrees**, repeating every **14.4 degrees** relative to the saved start. At a live value of **99**, segment **99** also remains in the background because it lies beyond the filled extent. The replacement implements this in [src/original.rs](src/original.rs) `arc_gap`, used by [src/render.rs](src/render.rs); [src/legacy.rs](src/legacy.rs) reads the original saved boolean. The renderer retains the track color in those gaps. Preview and image upload share that rendered frame; the replacement retains its existing image-upload transport rather than adopting opcode195 mask updates. Exact GDI+ edge pixels are not claimed.

Two regressions reproduced the error before the correction and pass afterward: a real NRBF record graph tests true/false/absent settings and JSON persistence; raster samples test gap/filled/unfilled slices at three start angles, integer quantization, old-JSON defaults and cache invalidation. The formulas and addresses below were also rechecked in this pass; all 48 completed native responses have matching managed bodies in the current audit. Build and package results are recorded in `VALIDATION.md` and `evidence/deep-arc-validation.json`.

### Verified values, formulas and rules retained by this build

The existing address-linked serial, packet, RGB565, sensor, glyph, date, network, frequency and percentage rules in the next table remain in use. This update adds the recovered arc-gap rule to those mechanics. Physical display equivalence, native sensor acquisition under this account, original-app acceptance of exported snapshots and exact GDI+ raster identity remain unverified.

## Earlier inspection: original functions and packet builders (2026-11-04)

Opened **C:\Program Files\35inchENG\UsbMonitor.exe** with `core_target_open`; the session reports **Deep mode**. Original SHA-256: **a980a69b634fe6cb3a20173293d23f5728855f41b62748cf6a4b5b9406f972ab**. Ran `native_strings_find` separately for names, settings and messages. The searches returned **0 / 1 / 3** matches; those hits are version-resource/manifest text. Followed all three hit addresses (**0x524a52, 0x524a6e, 0x524c78**) plus controller **0x412418** and packet method **0x42f8a4** with `native_xrefs_list`; native edges were empty. The manifest's `asm.v2` namespace is not the display setting.

Completed **45 individual `native_function_pseudocode` requests sequentially**, awaiting and reading each response. Every full response is preserved in [evidence/deep-protocol-audit.json](evidence/deep-protocol-audit.json), with **45 matching CLR bodies**, decoded names/messages, method tokens, preferred VAs and **1,257 incoming managed call sites**. Fresh managed evidence **ev_c663d787225c57229efdebb6f807cdfcca8c9bae9acde62b00112c8ad9e413f2** contains **107 types, 974 methods, 946 bodies, 9,045 call sites and 985 fields**. Search evidence: **ev_ba31fd920581fcb6a52a35e6555f07282575e5cd31597c959f633cf5f9484839**, **ev_3f29f4b461de5c86a595fcae1966766150594fba8106fad9e7aac7209f675a75**, **ev_869e5c4bfc2396451a01cb5d1ae868cec558879b9fe6bc6ccce167565ae29755**.

Ghidra interprets CLR method headers/IL as native machine instructions, yielding invalid pseudo-C, commonly `halt_baddata()`. Those results are retained as the requested inspection record. Behavioral claims use the valid managed instructions and resolved references. The original executable was not run.

### Verified values, formulas and rules used in this build

Addresses below are **RVAs**; add preferred image base **0x400000** for the native-request VA. Exact instruction ranges and full method bodies are in the audit.

| Rule | Original function(s) and implementation |
|---|---|
| Serial **115200 baud**, **8/N/1**, DTR/RTS true, read timeout **1000 ms** | **0x2ebb0 IL_01d2..0211**; default and timeout in `src/original.rs`, serial configuration in `runtime.rs`. Selectable higher rates remain independent, firmware-dependent options. |
| Landscape **480x320**, portrait **320x480**; orientation portrait **0**, landscape **2**, flip adds **1** | **0x2f090, 0x100e8**; dimensions and orientation helpers. |
| **6-byte** command header, each expression converted to its low byte | **0x2f8a4 IL_0127..0170**, token **0x060001b6**; `command_header` below. |
| Mirror opcode **122**, **16-byte** zero-filled packet, supplied mode at byte **6**, wait **10 ms** | **0x2f1f0 IL_009f..00bd**, token **0x060001ae**; `mirror_packet`. Replacement stores mirror as a boolean. |
| Orientation opcode **121**, **16-byte** zero-filled packet, byte **6 = 100 + orientation**, bytes **7/8 = width high/low**, **9/10 = height high/low**, wait **10 ms** | **0x2f2c0 IL_00a0..00ea**, token **0x060001af**; `orientation_packet`. Dimension bytes are big endian. |
| Brightness opcode **110**, raw brightness in first packed coordinate | **0x10384 IL_00ab..00b3**, token **0x060000c7**; `brightness_header`. Existing percentage mapping `floor((100-clamp(percent,0,100))*255/100)` is a replacement UI rule based on the user's physical-display report. |
| Image opcode **197**, inclusive endpoints **endX=x+width-1**, **endY=y+height-1**; skip image waits for descriptor containing **V2** | **0x2fac0 IL_005a..0092** and later wait sites; `image_header` plus existing runtime waits. Nonempty in-screen rectangles are the replacement caller's contract. |
| RGB565 **((r<<8)&63488) OR ((g<<3)&2016) OR (b>>3)**; **2 little-endian bytes/pixel** | **0x51a0 IL_00b4..00ca**, bitmap transfer **0x2e07c**; `rgb565_pixel`. Pixel endian differs from the orientation dimensions. |
| Network **byte delta / elapsed seconds / 1024 / 1024**; negative -> **0**, strictly positive below **0.1 MiB/s** -> **0.1**; text truncates tenths, above **99** displays **99.9** | Sampler **0x113dc**, controller **0x12418**; `network_mibps`, `network_tenths`. Shared decimal MB/s is converted before applying compatibility rules. |
| CPU-frequency text **truncate(Math.Round(double(float32(MHz))/10000,2)*100)** then **PadLeft(2,'0').Insert(1,".")**; frequency bar **truncate(float32(MHz)/8000*length)** | Controller **0x12418**, float getter/setter **0xdc54/0xdd04**; existing compatibility helpers preserve operation order, including **2900 MHz -> 2.8**. |
| Percentage bar **truncate(double(float32(value))*0.01*length)**; ceiling **99** for CPU/GPU temperature/load, GPU memory load, RAM load and volume | **0x12418, 0x15b78**; existing imported-field rules. Stored integer HDDTemp/CpuFan/GpuFan/Volume getters **0xece8/0xd2b0/0xd574/0xeb84** format integer text. |
| GPU capacity usage **truncate(used/total*100)**, capped **99** | **0x46968 IL_1437..14d6**, getter/setter **0xe49c/0xe54c**, postprocessor **0x15b78**; `gpu_memory_percent`. |
| Arcs: **100** segments of **3.6 degrees**, diameter **archR**, centered stroke **archWidth**, saved start **startPer*3.6**; bar fallback height **13** | **0x4d350, 0x4db44, 0x4dfc8, 0x5fa64**; legacy import and renderer retain these rules. |
| Glyph cells advance by saved **interval**, including overlap; date **yyyy/M/d**, time **HH:mm**, group offsets preserved | **0x4f80c, 0x500ec, 0x5190c, 0x507d4, 0x50ba8, 0x61490, 0x61614**, controller **0x12418**. |
| Theme object serialization/deserialization uses **BinaryFormatter/NRBF** | **0xa894/0xa984**; replacement reads inert records and keeps saved bitmap artwork. Export remains a bitmap snapshot with editable JSON. |
| HWiNFO initialization ordinal **684**, flags **64**; update **216**, temperature **566**, fan **663**, power **579**, cleanup **458**; name buffer **256**, sensor slots **512** | **0x46968, 0x41960, 0x41b08, 0x44cac, 0x46498, 0x44f30**, managed P/Invoke metadata. Existing helper retains these values. |
| String decoder: **k=303400377+argument**, decode UTF-16 bytes with successive XOR keys and swap; increment k by **2** per code unit | **0xcb28**. Controller argument **4**, serial **16**, V2 **10**, date **12**, HDDTemp helper **15**. Serial message decodes to `创建设备失败,疑似占用` (device creation failed, likely occupied). |

For `(opcode,x,y,endX,endY)`, `command_header` returns:

```text
byte0 = low8(x >> 2)
byte1 = low8(((x & 3) << 6) + (y >> 4))
byte2 = low8(((y & 15) << 4) + (endX >> 6))
byte3 = low8(((endX & 63) << 2) + (endY >> 8))
byte4 = low8(endY)
byte5 = opcode
```

Followed managed chain: controller **0x100e8 IL_0158/01a3 -> 0x2f1f0**, **IL_0289 -> 0x2f2c0**; mode **IL_00b5**, orientation **IL_00e2**, brightness **0x10384 IL_00b3** and image **0x2fac0 IL_0070** all call packed-command token **0x060001b6**. Groups **0x61490 IL_00ef / 0x61614 IL_00f1** call item update/preview **0x500ec/0x4f80c**. CPU frequency follows getter/setter field **0x0400005a**; GPU capacity follows **0x04000060**.

### Applied source change and verification

`src/original.rs` now owns the verified packet lengths, orientation bias and command/mirror/orientation/brightness/image builders. `runtime::upload_frame` calls those builders for actual transport; patch-cost estimation also uses the verified header length. This consolidates the already recovered wire behavior without claiming a newly discovered behavioral discrepancy. Other verified formulas remain wired into model/render/import/runtime paths.

All **53 existing tests pass**, including literal full-wire packets in both orientations and flip/V2 combinations, exact waits, brightness endpoints, partial-frame reconstruction, installed themes and sensor/text formula fixtures. `cargo build --offline` and `cargo fmt --check` pass. Final Craft/startup results are recorded in `VALIDATION.md` and `evidence/deep-protocol-validation.json`.

Physical display equivalence, successful native HWiNFO acquisition here, original-app acceptance of exported snapshots, exact GDI+ raster identity and universal RamValue source units remain unverified. Grid, external integrations, rendering optimizations and provider selection remain independently implemented features.

## Earlier audit: complete network sampler path (2026-11-04)

Opened the original main program **C:\Program Files\35inchENG\UsbMonitor.exe** with `core_target_open`, confirmed **Deep mode**, and verified SHA-256 **a980a69b634fe6cb3a20173293d23f5728855f41b62748cf6a4b5b9406f972ab**. Completed three `native_strings_find` searches for names, settings and messages, then followed three `native_xrefs_list` addresses. Names returned no native matches; settings/messages each matched only the manifest at **0x524c78**. Its `asm.v2` namespace is unrelated to the display's V2 setting. Native references were empty. Resolved managed strings, call tokens and backing fields instead.

**45 individual `native_function_pseudocode` requests completed sequentially. Each result was awaited and read.** [evidence/deep-followthrough-audit.json](evidence/deep-followthrough-audit.json) preserves every complete response, all **45 matching CLR bodies**, decoded strings, function addresses/tokens and **1,257 incoming managed references**. The fresh managed request retained evidence **ev_e367ac3c94a18d03619ef56ae892b942a255907ae9d7c39a85030939e771aa73**: **107 types, 974 methods, 946 bodies, 9,045 call sites and 985 fields**. Search evidence: **ev_91d8dc8c78527409c2a6eacd0ffd8cc4690551a3a0f4c5bb6e54597c60069a90**, **ev_21cca2177e873f6a6608ac698425120b52c049cfffa324e5737106c049d3f2a4**, **ev_67a965c1cf2cf147c93161c35726f2d7063be9b6203e95347a3d1e0eb685faf3**.

The Ghidra native decompiler treats this .NET assembly's method headers/IL as machine instructions; its pseudo-C is invalid, often ending in `halt_baddata()`. Those responses prove the requested inspection occurred; the behavioral rules below use valid ECMA-335 instructions and resolved managed references. The original executable was not run. No new ILSpy/C# decompilation is claimed.

### Newly applied rule

The original **network sampler**, RVA **0x113dc**, preferred VA **0x4113dc**, method **0x060000ce**, normalizes its rate before the controller formats it:

```text
rate = byte_delta / elapsed_seconds / 1024 / 1024  // MiB/s
if rate < 0: rate = 0
if 0 < rate < 0.1: rate = 0.1
```

The download negative check is **IL_045c..0489**, with zero assignment **IL_03a9..03b3**. Its positive check **IL_0539..0548** leads to the **0.1** comparison **IL_035b..036c** and assignment **IL_010d..0117**. Upload uses negative check **IL_0154..0181** / zero assignment **IL_00e0..00ea**, positive check **IL_028b..029a**, comparison **IL_04e2..04f3**, assignment **IL_05a9..05b3**. Dispatch constants are consumed as signed low-16-bit state indices; following these branches establishes the positive-only minimum, not a blanket minimum for zero.

Earlier builds recovered the controller's **0.01** text threshold but omitted the sampler's **0.1** floor. The full path therefore displays **0 -> 0.0**, **0.000001 -> 0.1**, **0.009 -> 0.1**, **0.01 -> 0.1**, **1.29 -> 1.2**, **99 -> 99.0**, and **99.0001 -> 99.9**. Negative synthetic/reset rates become **0.0**. The older controller-only examples below are superseded for end-to-end behavior.

Implemented in [src/original.rs](src/original.rs), called by `DataState::value` for imported network fields, including imports restored from older JSON. The compatibility conversion first changes the replacement's decimal MB/s to bytes/s. All imported precision choices see the original sampler result. Shared raw readings and ordinary editor layouts retain their existing values/units; unavailable and nonfinite strings are preserved. Source-provider selection/aggregation and sampling cadence remain replacement differences.

The new data-path and actual-raster regressions failed before the correction and pass afterward. They cover both directions, byte-scale positive readings, zero, negatives, the minimum boundary, values above the minimum, saved JSON, missing/nonfinite readings and unchanged shared/ordinary fields. All **53 tests pass**. Build and startup verification are recorded in `VALIDATION.md`.

### Verified rules wired into this build

All addresses here are **RVAs**; add preferred image base **0x400000** for the VA used in the native requests. Every function listed has a completed native response and matching managed body in this pass. Protocol/dimension constants and the arithmetic helpers now live in `src/original.rs`, called by `model.rs`, `render.rs`, `runtime.rs` and `legacy.rs`.

| Mechanic | Functions and recovered values/formulas |
|---|---|
| Serial | **0x2ebb0**: **115200 baud**, **8/N/1**, DTR/RTS true, timeout **1000 ms**; decoded failure message `创建设备失败,疑似占用` (device creation failed, suspected occupied port). |
| Dimensions/orientation | **0x2f090, 0x100e8**: **480x320** landscape / **320x480** portrait; portrait **0**, landscape **2**, flip adds **1**. |
| Packed commands | **0x2f8a4**: six-byte header `x>>2, ((x&3)<<6)+(y>>4), ((y&15)<<4)+(endX>>6), ((endX&63)<<2)+(endY>>8), endY&255, opcode`. **0x2fac0**: image **197**, inclusive end coordinates `start+size-1`, image waits bypassed when decoded descriptor contains **V2**. |
| Display settings | **0x2f1f0, 0x2f2c0, 0x10384**: mirror **122**, orientation **121** with **100** bias, brightness **110**; mode/orientation packets **16 bytes**, **10 ms** waits. Percentage brightness/inversion is the existing user-confirmed replacement control. |
| RGB565 | **0x51a0, 0x2e07c**: `((r<<8)&63488) | ((g<<3)&2016) | (b>>3)`, two little-endian bytes per pixel. |
| Network | **0x113dc, 0x12418**: sampler formula/minimum above; controller `q=truncate(rate*10)`, zero tenths replaced by **1** only above **0.01**; input above **99** selects **999** tenths. |
| CPU frequency | **0x12418, 0xdc54, 0xdd04**: text `q=truncate(Math.Round(double(float32(MHz))/10000,2)*100)`, midpoint-to-even, then `PadLeft(2,'0').Insert(1,".")`; **2900 MHz -> 2.8**. Bar `truncate(float32(MHz)/8000*length)` with upper bound at length. |
| Percent bars/ceilings | **0x12418, 0x15b78**: `truncate(double(float32(value))*0.01*length)` in that order. Ceiling **99** for CpuTemp/CpuLoad/GpuTemp/GpuLoad/GpuRamLoad/RamLoad/Volume. |
| Stored integer text | Controller **0x12418**: HDDTemp getter/setter **0xece8/0xed9c**, CpuFan **0xd2b0/0xd360**, GpuFan **0xd574/0xd624**, Volume **0xeb84/0xec38** use signed-int storage/ToString; imported zero-decimal fractional provider readings truncate toward zero. CPU/GPU temperatures use two-digit zero padding. |
| GPU memory | **0x46968, 0xe49c, 0xe54c, 0x15b78**: capacity percentage `truncate(used/total*100)`, capped **99**; separate from used memory amount. |
| Arc/bar geometry | **0x4d350, 0x4db44, 0x4dfc8, 0x5fa64**: **100** arc segments of **3.6 degrees**, `archR` is diameter, `archWidth` is centered stroke, saved start `startPer*3.6`; bar height fallback **13**. |
| Glyph/date/group rules | **0x4f80c, 0x500ec, 0x5190c**: saved glyph cells with direct `interval` advance. **0x507d4, 0x50ba8**: date **yyyy/M/d**; controller **0x12418**: time **HH:mm**. **0x61490, 0x61614**: group item placement/update. |
| Theme files | **0xa894, 0xa984**: BinaryFormatter/NRBF object serialization and deserialization; replacement reads inert typed records, preserves bitmap artwork and editable fields. Original-app acceptance of exported snapshots remains untested. |
| Hardware sensors | **0x46968, 0x41960, 0x41b08, 0x44cac, 0x46498, 0x44f30**: HWiNFO init ordinal **684**, flags **64**; refresh **216**, temperature **566**, fan **663**, power **579**, cleanup **458**; **256-byte** name buffers, **512** sensor slots. HDDTemp helper performs `conv.i4`. |
| Obfuscated strings | **0xcb28**: start key **303400377+argument**, XOR successive UTF-16 bytes and swap each pair. Controller argument **4**, serial **16**, V2 **10**, date **12**, HDDTemp helper **15**. |

Original RamValue source units remain configurable/unresolved, GPU fan RPM differs from the NVIDIA percentage fallback, and GDI+ glyph identity, successful native hardware acquisition, higher baud compatibility and physical display equivalence remain unverified. Independent editor features (tickers, external sources, selection controls and rendering optimizations) retain their documented rules.

## Fresh requested inspection: stored integer sensor fields (2026-11-04)

Opened the original main program **C:\Program Files\35inchENG\UsbMonitor.exe** with `core_target_open` and confirmed **Deep mode**. SHA-256 **a980a69b634fe6cb3a20173293d23f5728855f41b62748cf6a4b5b9406f972ab**. Repeated `native_strings_find` for names, settings and messages. The native scanner found zero name matches and one manifest match in each other search; `V2` also matches the manifest's XML namespace, so that hit is not a device-setting reference. Followed native references at **0x524c78**, **0x412418**, **0x42f8a4**; all were empty. Then resolved the managed strings, call tokens and field accesses rather than treating native negative results as absence of behavior.

**47 individual `native_function_pseudocode` requests completed sequentially. Every response was awaited and read before continuing.** Full responses, matching valid managed bodies, decoded names/settings/messages, addresses and **1,260 incoming managed call references** are preserved in [evidence/deep-rules-refresh-audit.json](evidence/deep-rules-refresh-audit.json). The managed inventory contains **107 types, 974 methods, 946 IL bodies, 9,045 call sites and 985 fields**, evidence **ev_e367ac3c94a18d03619ef56ae892b942a255907ae9d7c39a85030939e771aa73**. Search evidence: **ev_87627c809bded9125edde805a48e0e996b762a1815c144fcc02dfd3069f80b2e**, **ev_7a04711b071fcc9755bbf876891ca92d8f5525030f0ca2e3b752b2a53ddf59b7**, **ev_373682da5b5022058b5349fb0949e6de6171d7eea53c1ef18ed1513a3e3b5e4e**.

The native Ghidra decompiler interprets .NET method headers/IL as machine code and returns invalid instructions, usually `halt_baddata()`. These outputs are an inspection record, not behavioral evidence. Rules below come from the ECMA-335 instructions and resolved managed references. The optional ILSpy C# decompilation tool was refused because the environment requires approval for it while its approval policy is never; no C# result is claimed. The original executable was not run.

### Correction applied to this build

The original stores **HDDTemp, CpuFan, GpuFan and Volume as signed integers**, then calls `int.ToString()` before drawing. The replacement's sensor providers can supply fractional values. Its imported default-zero-decimal fields previously rounded those up: **33.9 -> 34**. They now use integer presentation: **33.9 -> 33**, **0.9 -> 0**, **98.99 -> 98**. Conversion uses the supplied double directly, so **0.99999999 -> 0**, without an intermediate float32 rounding to 1. Negative fractional values in the signed-int range truncate toward zero. No original overflow/nonfinite behavior is claimed.

| Field | Original reference chain (RVAs; preferred VA = RVA + 0x400000) |
|---|---|
| HDDTemp | Sensor **0x46968 IL_1e99** -> helper **0x41960 IL_0158..015f**, temperature ordinal **566**, `conv.i4` -> setter **0xed9c**, token **0x060000b0**, field **0x04000066** -> getter **0xece8**, token **0x060000af** -> controller **0x12418 IL_1456..146e**. Pads to **2** characters using the `x` glyph marker. |
| CpuFan | Sensor **0x46968 IL_1e4e..1e58**, `int.Parse` -> setter **0xd360**, token **0x0600008a**, field **0x04000053** -> getter **0xd2b0**, token **0x06000089** -> controller **0x12418 IL_0f83..0f9b**. Pads to **5** characters using `x`. |
| GpuFan | Sensor **0x46968 IL_11f4..11f9**, integer fan ordinal **663** -> setter **0xd624**, token **0x0600008e**, field **0x04000055** -> getter **0xd574**, token **0x0600008d** -> controller **0x12418 IL_18a8..18c0**. Pads to **5** characters using `x`. |
| Volume / VulomeLoad | Setter **0xec38**, token **0x060000ae**, field **0x04000065** -> getter **0xeb84**, token **0x060000ad** -> controller **0x12418 IL_1801..1819**. Pads to **2** characters using `x`; postprocessor **0x15b78** retains its **99** ceiling. |

Implemented in `src/render.rs` for imported fields with `legacy_text` and `decimals = 0`, including imports already saved as JSON. Shared precise readings, fractional chart inputs, explicit nonzero decimal choices and ordinary editor widgets keep their existing behavior. Disk temperature and fan fields have no recovered 99 ceiling. The original GPU fan source is RPM; the replacement's NVIDIA percentage reading remains a provider difference and is not relabeled as RPM. Original `x` padding remains represented by the replacement's existing right alignment, not literal x characters.

A real-raster regression failed before the correction at **disk.temperature 33.9 -> 34** and passes after it. It covers all four bindings, saved JSON, affixes, double precision below 1, zero and negative fractions, values above 99 for uncapped fields, unavailable readings and explicit precision overrides. **52 tests pass**; build and Craft verification are recorded in `VALIDATION.md`.

### Other verified values and formulas used by the build

Each function below also has an awaited native response and matching CLR body in this pass. Detailed instruction offsets and limits remain in the established sections below.

| Mechanic used in the replacement | Function RVAs and real rule |
|---|---|
| Serial connection | **0x2ebb0**: **115200 baud**, **8/N/1**, DTR/RTS true, timeout **1000 ms**. Decoded failure message: device creation failed, likely occupied. |
| Screen dimensions/orientation | **0x2f090, 0x100e8**: **480x320** landscape / **320x480** portrait; portrait **0**, landscape **2**, flip adds **1**. |
| Commands/pixels | **0x2f8a4, 0x2fac0, 0x2f1f0, 0x2f2c0, 0x10384**: six-byte packed header; image **197**, inclusive end coordinates; mirror **122**, orientation **121** with bias **100**, brightness **110**. Mode/orientation wait **10 ms**; image waits skipped when descriptor contains **V2**. Percentage brightness is a replacement control with user-confirmed inversion. |
| RGB565 | **0x51a0, 0x2e07c**: `((r << 8) & 63488) | ((g << 3) & 2016) | (b >> 3)`; two little-endian bytes per pixel. |
| Network rates/text | **0x113dc, 0x12418**: byte delta / elapsed seconds / **1024** / **1024**; `q = truncate(MiBps * 10)`; zero tenths becomes **1** only above **0.01** MiB/s; input above **99** uses **999** tenths (**99.9**). |
| CPU frequency text | **0x12418, 0xdc54, 0xdd04**: `q = truncate(Math.Round(double(float32(MHz)) / 10000, 2) * 100)`; `PadLeft(2,'0').Insert(1,".")`, including **2900 MHz -> 2.8**. |
| Bars/integer float text | **0x12418**: percent width `truncate(double(float32(value)) * 0.01 * length)`; frequency width `truncate(float32(MHz) / 8000 * length)`, upper bounded by length. CpuLoad/CpuTemp/GpuLoad/GpuTemp/RamLoad/HDDLoad text truncates; CPU/GPU temperatures pad with zero to **2** digits. |
| Sensor ceilings/GPU capacity | **0x15b78, 0x46968, 0xe49c, 0xe54c**: upper ceiling **99** for CpuTemp/CpuLoad/GpuTemp/GpuLoad/GpuRamLoad/RamLoad/Volume; GPU capacity usage `truncate(used / total * 100)`. |
| Arcs/bars | **0x4d350, 0x4db44, 0x4dfc8, 0x5fa64**: **100** arc segments, **3.6** degrees/segment; diameter `archR`, centered stroke `archWidth`, saved start `startPer * 3.6`; bar height fallback **13**. |
| Glyphs/date/groups | **0x4f80c, 0x500ec, 0x5190c, 0x507d4, 0x50ba8, 0x61490, 0x61614**: fixed cells with direct `interval` advance, group offsets; date **yyyy/M/d**; controller **0x12418** uses time **HH:mm**. Exact GDI+ glyph identity remains unverified. |
| Theme serialization | **0xa894, 0xa984**: NRBF/BinaryFormatter ThemeConfig and its typed lists/bitmap data; the replacement reads inert records. Export remains a bitmap snapshot plus editable JSON, with original-app acceptance untested. |
| Hardware sensors | **0x46968, 0x41b08, 0x44cac, 0x46498, 0x44f30, 0x41960**: HWiNFO init ordinal **684**, flags **64**; update **216**, temperature **566**, fan **663**, power **579**, cleanup **458**. Live successful acquisition remains unverified. |
| Name/message decoder | **0xcb28**: start key **303400377 + argument**; XOR successive UTF-16 bytes and swap each pair. Controller argument **4**, serial **16**, V2 check **10**, date **12**, HDDTemp helper **15**. |

**Open unit question:** controller **0x12418 IL_11d7..11dc** truncates `RamValue` through getter **0xe760**, field **0x04000062**, setter **0xe810**. Upstream **0x46968 IL_17a7..17e2** reads a configured HWiNFO device/sensor through a generic numeric API, then `conv.i4; conv.r4`. This pass does not establish a universal source unit. The replacement's `memory.used` GiB binding and its one-decimal import default therefore remain a documented provider/presentation difference. No guessed MiB/GiB migration was applied.

Ticker features, external data integrations, selection controls, faster selectable baud rates and rendering optimizations remain independently requested editor features. Physical display equivalence and higher baud compatibility remain unverified.

## Fresh requested inspection: CPU-frequency text (2026-11-04)

Reopened **C:\Program Files\35inchENG\UsbMonitor.exe** with `core_target_open`; session confirmed **Deep mode**, SHA-256 **a980a69b634fe6cb3a20173293d23f5728855f41b62748cf6a4b5b9406f972ab**. Repeated `native_strings_find` separately for names, settings and messages, and followed `native_xrefs_list` at **0x524c78**, **0x412418**, **0x42f8a4**. Names/settings returned zero native matches; messages returned three resource/manifest matches. Native references were empty. These results do not establish absence of managed strings/calls.

**37 individual `native_function_pseudocode` requests completed sequentially, awaiting and reading each full result before continuing.** [evidence/deep-formulas-audit.json](evidence/deep-formulas-audit.json) contains all 37 responses, matching valid managed bodies, decoded strings, addresses/tokens and **1,235 incoming managed call references**. Fresh managed evidence **ev_e367ac3c94a18d03619ef56ae892b942a255907ae9d7c39a85030939e771aa73** contains **107 types, 974 methods, 946 IL bodies, 9,045 call sites and 985 fields**. Search evidence: **ev_bc6714ff1ebaa66df5b18fd8c140641ff0505ea97dfe2b22910b125d032bb268**, **ev_191b623e6f1e5a994f8ce49151b5021019441fc4a041e5ad86c2afdba3e4f944**, **ev_883210ab1727d622f96770cb9b456c1c9c466f756420c760fd5cceb4f1085dde**.

The native Ghidra results misinterpret CLR IL as machine instructions (usually ending in `halt_baddata`). They are preserved as the requested inspection record. Behavioral conclusions use the valid ECMA-335 IL and resolved tokens. The original executable was not run.

### Newly applied original formula

The original CPU-frequency **text** is not a conventional one-decimal GHz conversion. Controller **RVA 0x12418 / preferred VA 0x412418**, method **0x060000d3**, decodes `CpuFreq` at **IL_129d**, branches at **IL_12ea** to **IL_1632**, and performs:

```text
MHz = float32(readingMHz)
rounded = Math.Round(double(MHz) / 10000, 2)  // midpoint to even
q = truncate_toward_zero(rounded * 100)
text = q.ToString().PadLeft(2, '0').Insert(1, ".")
```

Exact instructions: getter **IL_1635**, promotion **IL_163a**, divisor **10000** at **IL_163b..1644**, `Math.Round(double, int)` with **2** digits at **IL_1645..1646**, multiply by **100** at **IL_164b..1654**, `conv.i4` at **IL_1655**, zero padding at **IL_165f..1662**, decimal point insertion at index **1** at **IL_1667..1676**, `TItem.valueStr` write at **IL_167b**. Getter **RVA 0xdc54 / VA 0x40dc54**, method **0x06000097**, reads field **0x0400005a** at **IL_009d**; setter **RVA 0xdd04 / VA 0x40dd04**, method **0x06000098**, writes that field at **IL_009d**. Sensor **RVA 0x46968 / VA 0x446968** supplies the setter at **IL_1dfe**. Decoder **RVA 0xcb28 / VA 0x40cb28** with argument **4** recovers the name and decimal-point literal.

Preserving the operation order matters: **50 MHz -> 0.0**, **1050 -> 1.0**, **1150 -> 1.2**, **2850 -> 2.8**, **2900 -> 2.8**, **2950 -> 3.0**, **4000 -> 4.0**. At 2900 MHz, the rounded intermediate is 0.29; multiplying by 100 produces **28.999999999999996**, which truncates to 28. The fixed string insertion also produces **10000 MHz -> 1.00**. This oddity is documented and reproduced, not silently corrected.

`src/render.rs` now applies that sequence to imported `cpu.frequency` fields with their default **one decimal place**, reconstructing float32 MHz from the shared GHz reading. Existing JSON imports retain their FontConfig marker and receive the same correction. Explicit other decimal choices and ordinary editor fields retain their selected formatting. Missing readings remain N/A. Raw readings remain precise; imported frequency bars retain their separate **8000 MHz** full scale. Negative/nonfinite inputs and values beyond the signed integer conversion range use replacement validation/fallback behavior, not a claim about original overflow handling.

[evidence/cpu-frequency-fixtures.ps1](evidence/cpu-frequency-fixtures.ps1) evaluates only standard .NET math/string APIs on 20 synthetic readings; it never loads the original assembly. [evidence/cpu-frequency-dotnet-fixtures.json](evidence/cpu-frequency-dotnet-fixtures.json) retains every intermediate and expected string. The regression compares actual field rasters with those literals, including affixes, saved JSON restoration, missing readings, explicit precision overrides and ordinary editor fields; it failed before the change at 50 MHz and passed after it.

### Other original functions read in this pass

Addresses below are RVAs; add **0x400000** for the preferred VA. Each has a fully awaited native result and matching managed body in the fresh audit. Detailed formulas and instruction offsets remain in the established sections below.

| Implemented behavior / verified rule | Function RVAs |
|---|---|
| String decode: start key **303400377 + argument**; XOR each UTF-16 byte with successive keys, then swap bytes | **0xcb28** |
| Serial **115200**, **8/N/1**, DTR/RTS true, timeout **1000 ms**; failure message decodes to failed device creation / likely in use | **0x2ebb0** |
| Dimensions **480x320** / **320x480**; portrait **0**, landscape **2**, invert adds **1** | **0x2f090, 0x100e8** |
| Packed six-byte command, image **197** and inclusive endpoints; mirror **122**; orientation **121**, bias **100**; brightness **110**; mode/orientation wait **10 ms**, image waits skipped when descriptor contains V2 | **0x2f8a4, 0x2fac0, 0x2f1f0, 0x2f2c0, 0x10384** |
| RGB565 `((r<<8)&63488) \| ((g<<3)&2016) \| (b>>3)`; little-endian two bytes/pixel | **0x51a0, 0x2e07c** |
| Network byte deltas / elapsed seconds / **1024** / **1024**; tenths truncation, minimum shown 0.1 only above **0.01**, input above **99** shows **99.9** | **0x113dc, 0x12418** |
| Percent bar `truncate(double(float32(value))*0.01*length)`; frequency bar `truncate(float32(MHz)/8000*length)`, bounded by length; integer sensor text truncates; CPU/GPU temperature has two-digit padding | **0x12418, 0xdc54, 0xdd04** |
| CpuTemp/CpuLoad/GpuTemp/GpuLoad/GpuRamLoad/RamLoad/Volume upper ceiling **99**; GPU capacity percent `truncate(used/total*100)` | **0x15b78, 0x46968, 0xe49c, 0xe54c** |
| Arc diameter `archR`, centered stroke `archWidth`, saved start `startPer*3.6` degrees, **100** segments; default bar height **13** | **0x4d350, 0x4db44, 0x4dfc8, 0x5fa64** |
| FontConfig advance `interval`, fixed cells; Date `yyyy/M/d`; group offsets and drawing; Time `HH:mm` via controller | **0x4f80c, 0x500ec, 0x5190c, 0x507d4, 0x50ba8, 0x61490, 0x61614, 0x12418** |
| Theme BinaryFormatter serialization/deserialization, parsed only as data in replacement | **0xa894, 0xa984** |
| HWiNFO initialization **684** flags **64**, update **216**, temperature **566**, fan **663**, power **579**, cleanup **458**, disk-temperature helper | **0x46968, 0x41b08, 0x44cac, 0x46498, 0x44f30, 0x41960** |

Requested ticker features, multi-selection, precision overrides, faster selectable baud rates and rendering optimizations remain independent editor features. No original counterpart is claimed. Physical display timing, higher baud compatibility, successful live HWiNFO acquisition and exact GDI+ glyph identity remain unverified.

## Independent ticker improvements (2026-11-04)

Circular is a saved, default-off ticker option requested for this editor. Moving circular tickers repeat the cached text strip with a small text-sized space, wrapping the sampling coordinates in either direction so the next copy enters immediately behind the previous one. Short strips repeat as many times as needed to fill the field. Sampling interpolates across the wrap with premultiplied alpha; speed 0 retains one stationary copy. The existing scroll speed controls pixels per second, and non-circular passages retain their original behavior. This is an independent editor extension, with no claimed original-app counterpart.

These are editor features, separate from original-theme rules. Per-source `decimals` is optional (Auto preserves the source string), bounded to 0-6 and used only for finite numeric values. Text/missing data retain their strings and affixes. Ticker glyphs are rasterized into a cached transparent strip; positions use the existing 10/speed-second passage with premultiplied-alpha linear sampling instead of rounded pixel jumps or a 30 Hz cache key. Canvas animation requests repaint at the renderer's normal cadence. A separate sampling worker owns blocking sensors/HTTP, and the display worker uses a 60 Hz deadline and latest state, skipping missed animation positions after a slow transfer. RGB565 patches trim tile padding and merge nearby rectangles when payload cost is cheaper than separate commands and their existing waits. Required original command delays and packet/RGB565 semantics are retained. Baud is saved with the design, defaults to 115200 and can be selected up to 2000000; no higher rate is claimed as supported or as the physical maximum. The driver implementation at https://github.com/mathoudebine/turing-smart-screen-python/blob/main/library/lcd/lcd_comm.py also uses 115200, and its Rev A code recognizes VID/PID 1a86:5722; this supports the compatibility default, not a maximum-throughput claim. Display settings reports actual completed transfers/sec. No accessible physical display was available for a throughput measurement.

## Latest requested Deep pass: original network text rules (2026-11-04)

Opened **C:\Program Files\35inchENG\UsbMonitor.exe** with `core_target_open` and confirmed **Deep mode**. SHA-256: `a980a69b634fe6cb3a20173293d23f5728855f41b62748cf6a4b5b9406f972ab`. Ran `native_strings_find` for names, settings and messages, then followed `native_xrefs_list` at **0x524c78** and **0x412418**. Names/settings returned no native matches; messages returned three resource strings. The native reference scanner returned no edges. These negative results do not establish absence of CLR strings or behavior.

**Completed 32 individual `native_function_pseudocode` calls sequentially, awaiting every result and reading every full response.** All responses, all 32 matching valid managed instruction bodies, function addresses, decoded controller strings, resolved field/call references and **1,225 incoming call sites** are saved in [evidence/deep-network-presentation-audit.json](evidence/deep-network-presentation-audit.json). The assembly inventory contains **974 methods and 9,045 call sites**. Fresh IL evidence: `ev_85efc3e4c344639790a45d018ae1860b638137a553252002261e561980d46bd8`, `ev_a983373332c46c5f6cc19396e94c2c46971ca41443cde929fc6d9cc4921de0db`, `ev_184fc5597878f171922d39f8c376026151f06eaca4430293d705b6b3cae416d5`. Reference inventory: `ev_8730383db0a81e1544955e0abbfc8e981ba3ded9fec9801b170868850f889b62`.

The original is a managed .NET assembly. Ghidra misreads CLR headers/IL as machine instructions; these native outputs contain invalid operations and `halt_baddata()`. They record the requested decompilation calls. Behavioral claims below use the valid ECMA-335 instructions and resolved tokens. No original executable code was run.

### Newly applied presentation rule

The existing importer already converts the shared decimal MB/s readings to the original's **MiB/s**. The original controller additionally formats both `InternetDown` and `InternetUp` using this algorithm for finite nonnegative rates:

```text
tenths = truncate_toward_zero(rateMiBps * 10)
if tenths == 0 and rateMiBps > 0.01: tenths = 1
if rateMiBps > 99: tenths = 999
display tenths with one digit after the decimal point
```

| Reference | Exact original instructions |
|---|---|
| Download path | Controller **RVA 0x12418 / preferred VA 0x412418**, token **0x060000d3**. `InternetDown` at **IL_1107**; field **0x0400006b**; multiply by **10** and `conv.i4` at **IL_1157..1167**. Compare padded result with decoded `x00` at **IL_119c..11b1**; if rate **>0.01**, **IL_1855..1885** leads to decoded `x01` at **IL_0d27**. If rate **>99**, **IL_1490..14bd** leads to decoded `999` at **IL_126a**. Insert decoded `.` at index **2**, **IL_0d55..0d6c**. |
| Upload path | Same controller, `InternetUp` at **IL_1521**; field **0x0400006c**. Multiply/truncate **IL_0dbe..0dce**; compare `x00` **IL_0e03..0e18**; **>0.01** at **IL_0da7..0db8** through **IL_1415..1432** selects `x01` at **IL_0c2c**. **>99** at **IL_0ebc..0ee9** selects `999` at **IL_0e3a**. Insert `.` at index **2**, **IL_0ced..0d04**. |
| Name/message decoder | **RVA 0xcb28 / VA 0x40cb28**, token **0x0600007d**; controller argument **4** at **IL_0000..0005**. Decoder XORs the two bytes of each UTF-16 code unit with successive keys beginning at **303400377 + argument**, then swaps the bytes. Static decoding recovers the names and literals above without invoking the original assembly. |
| Upstream units and downstream drawing | Network sampler **RVA 0x113dc / VA 0x4113dc** divides byte/second deltas by **1024 twice**, storing those same fields. Controller writes `TItem.valueStr`; TItem update **RVA 0x500ec / VA 0x4500ec** draws the field using saved FontConfig metrics and advance getter **RVA 0x5190c / VA 0x45190c**. |

That earlier pass applied the controller's numeric presentation rule to imported fields with their default **one decimal place**. Its controller-only examples were **1.29 -> 1.2**, **0.01001 -> 0.1**, **0.01 -> 0.0**, **99 -> 99.0**, **99.0001 -> 99.9**. The current sampler audit above corrects the complete path: every positive rate below **0.1 MiB/s** first becomes **0.1**, so **0.01 now displays 0.1**. Shared raw readings retain their precision; imported numeric inputs use the recovered sampler normalization. Existing JSON imports receive it. Explicit decimal overrides still control formatting. Missing readings remain N/A. Replacement text retains its existing right alignment instead of reproducing the original `x` padding markers; exact GDI+ glyph identity remains unverified.

The regression compares actual field rasters against literal expected strings for both upload and download, including thresholds, truncation, prefix/suffix, missing readings, JSON restoration and decimal overrides. It failed before the implementation change at **0.01001 -> 0.0**, then passed with the correction. Regular editor widgets retain their own formatting.

### Other functions inspected in this pass

Preferred VA is **0x400000 + RVA**. Every function in this table has a full awaited native pseudocode response and matching CLR instructions in the audit above. Detailed formulas, instruction offsets and remaining limits are retained in the sections below.

| Build behavior / original rule | Inspected original function RVAs |
|---|---|
| Serial **115200 baud**, **8/N/1**, DTR/RTS enabled, **1000 ms** read timeout | **0x2ebb0** |
| Six-byte coordinate packing; image **197**; mirror **122**; orientation **121** with bias **100**; brightness **110**; command wait **10 ms** | **0x2f8a4, 0x2fac0, 0x2f1f0, 0x2f2c0, 0x10384, 0x100e8** |
| RGB565 `((r<<8)&63488) \| ((g<<3)&2016) \| (b>>3)`; two little-endian bytes per pixel | **0x51a0, 0x2e07c** |
| Percentage-bar pixels `truncate(double(float32(value))*0.01*length)`; frequency pixels `truncate(float32(MHz)/8000*length)`, upper bound saved length | **0x12418**, frequency getter **0xdc54** |
| CpuTemp/CpuLoad/GpuTemp/GpuLoad/GpuRamLoad/RamLoad/Volume ceiling **99**; integer sensor text truncates; CPU/GPU temperature pads to **2 digits** | **0x15b78, 0x12418** |
| Arc diameter `archR`, stroke `archWidth`, start `startPer*3.6` degrees; **100** segments; bar default height **13** | **0x4d350, 0x4db44, 0x4dfc8, 0x5fa64** |
| Saved glyph advance `interval`; Time **HH:mm**; Date **yyyy/M/d**; group child positioning/drawing | **0x4f80c, 0x500ec, 0x507d4, 0x50ba8, 0x5190c, 0x61490, 0x61614** |
| BinaryFormatter theme serialization/deserialization, independently parsed as data in this build | **0xa894, 0xa984** |
| Sensor acquisition; HWiNFO initialize **684** with flags **64**, update **216**, temperature **566**, fan **663**, power **579**, cleanup **458** | **0x46968, 0x41b08, 0x44cac, 0x46498, 0x44f30** |
| Encoded control names, settings and message literals | **0xcb28** |

The recently requested scrolling ticker, ordered source fields, prefixes/suffixes, multi-selection and Escape deselection are replacement-editor extensions. Their requested rules remain documented below; no original counterpart or invented original function address is claimed. Physical display equivalence, original-app acceptance of legacy snapshots and successful live HWiNFO acquisition remain unverified as previously documented.

## Fresh requested inspection and applied network units (2026-11-04)

Opened the original main program with `core_target_open` in **Deep mode**: `C:\Program Files\35inchENG\UsbMonitor.exe`, SHA-256 `a980a69b634fe6cb3a20173293d23f5728855f41b62748cf6a4b5b9406f972ab`. Searched names, settings and messages separately with `native_strings_find`, followed native references at **0x524c78** and **0x412418**, then followed the actual managed method/field tokens. The names/settings searches found zero native strings; the message search found three resource strings. Native references were empty because this scanner does not resolve CLR metadata or the encrypted strings.

**61 individual `native_function_pseudocode` requests completed sequentially; every complete response was read before proceeding.** [evidence/deep-verified-audit.json](evidence/deep-verified-audit.json) preserves all 61 full responses, the matching 61 valid managed IL bodies, **1,377 incoming managed call references**, decoded names/messages and the exact search results. The complete assembly inventory has 974 methods and 9,045 call sites. Fresh managed IL evidence: **ev_4eec6045a3d2ff20b3548d36ffc685ca8b2b8504cf1b50f84cbd88bec5f921a2**; metadata/reference evidence: **ev_8730383db0a81e1544955e0abbfc8e981ba3ded9fec9801b170868850f889b62**.

Ghidra interprets these CLR headers/IL as native instructions, producing invalid pseudo-C and overlapping function ranges. These native results establish that the requested calls were performed; they do not establish behavior. The rules here come from the valid ECMA-335 IL and resolved references. The original main executable was not run.

### Newly applied rule

| Recovered behavior | Function addresses and references | Build implementation |
|---|---|---|
| Download rate = `(receivedBytes - previousReceivedBytes) / elapsedSeconds / 1024 / 1024`; upload uses the corresponding sent-byte delta. The original rate is **MiB/s**. | Sampler **RVA 0x113dc / VA 0x4113dc**, method **0x060000ce**. Stopwatch elapsed seconds at **IL_03d5..03e9**; receive/send counters and deltas at **IL_03ea..0423**; download divisions at **IL_0425..043b**, field **0x0400006b**; upload divisions at **IL_0440..0457**, field **0x0400006c**. Controller **RVA 0x12418 / VA 0x412418**, method **0x060000d3**, decodes InternetDown/InternetUp with decoder **RVA 0xcb28 / VA 0x40cb28** and reads those same fields. | `runtime::network_readings` preserves the unrounded byte/time rate in the shared MB/s bindings. `DataState::value` converts imported fields with **`MBps * 1_000_000 / 1024 / 1024`**, using their saved FontConfig metrics to recognize both new imports and existing JSON imports. |

Previously, the build used decimal MB/s for imported themes and rounded it to one decimal before widget formatting. A byte-counter fixture of **1,048,576 downloaded bytes in 0.5 seconds** now reaches an imported field as **2 MiB/s**, while a regular editor field retains **2.097152 MB/s**. The corresponding upload fixture is **1 MiB/s**. Missing values remain N/A. The regression checks both directions, retained precision, saved-JSON imports and shared-reading preservation. It failed before the compatibility conversion and passed afterward.

That earlier change copied the proven unit formula. The current pass also applies the original low-rate normalization and previously recovered controller formatting to imports. Network-interface selection/aggregation and polling cadence remain replacement behavior; the full original sampler/controller instructions are retained for inspection. Shared editor bindings continue to use decimal MB/s, so independently designed layouts keep their units and raw precision.

### Other rules reconfirmed in this pass

| Mechanic already used by the build | Inspected original function RVA (preferred VA) |
|---|---|
| Serial **115200**, 8/N/1, DTR/RTS enabled, read timeout **1000 ms** | **0x2ebb0 (0x42ebb0)** |
| Six-byte coordinate packing; image **197**; mirror **122**; orientation **121**, bias **100**; brightness **110**; command wait **10 ms** | **0x2f8a4 (0x42f8a4)**, **0x2fac0 (0x42fac0)**, **0x2f1f0 (0x42f1f0)**, **0x2f2c0 (0x42f2c0)**, **0x10384 (0x410384)** |
| RGB565 = `((r<<8)&63488) | ((g<<3)&2016) | (b>>3)`, two little-endian bytes per pixel | **0x51a0 (0x4051a0)**, **0x2e07c (0x42e07c)** |
| Percentage bars = `truncate(double(float32(value)) * 0.01 * length)`; frequency bars use `truncate(float32(MHz) / 8000 * length)` | **0x12418 (0x412418)** |
| Seven imported sensor ceilings = **99**; integer sensor text truncates, CPU/GPU temperature has at least **2 digits** | **0x15b78 (0x415b78)**, **0x12418 (0x412418)** and getter/setter field traces preserved in the audit |
| Arc diameter = `archR`, stroke = `archWidth`; **100** segments of **3.6 degrees**; start = `startPer * 3.6` | **0x4d350 (0x44d350)**, **0x4db44 (0x44db44)**, **0x4dfc8 (0x44dfc8)** |
| Saved glyph advance = `interval`; width = `W + (n-1)*interval`; clip overflowing text; Time **HH:mm**, Date **yyyy/M/d** | **0x4f80c (0x44f80c)**, **0x500ec (0x4500ec)**, **0x507d4 (0x4507d4)**, **0x50ba8 (0x450ba8)**, **0x5190c (0x45190c)** |
| Original binary theme serialization and the HWiNFO sensor API | **0xa894 (0x40a894)**, **0xa984 (0x40a984)**, **0x41b08 (0x441b08)**, **0x46968 (0x446968)**, **0x44cac (0x444cac)**, **0x46498 (0x446498)**, **0x44f30 (0x444f30)** |

The detailed instruction offsets, formulas and hardware/raster limits for these established behaviors remain below. This inspection does not newly establish physical display identity or successful hardware-sensor initialization.

## Bundled hardware library integration (2026-11-04)

The user authorized copying and running `C:\Program Files\35inchENG\msvcr120d.dll`. Its identical local copy is `app/msvcr120d.dll`, SHA-256 `17b9eaa984d8cde3bcec7787b162f1233a6f354ffb83dc160422892dded4b9c2`; `build.rs` packages it beside the app and test executables. This is the original HWiNFO client library, not Microsoft's similarly named C runtime. Fresh managed API evidence is saved in `evidence/hwinfo-api.json` (`ev_f8db8182dda25e9e17cb3d6edcc334089469e090492249155e38e0917de65ca0`). The original executable was not run.

| Operation | Original managed source | Native ordinal / signature |
|---|---|---|
| Initialize with flags 64 | Method0x06000285 RVA0x44cac; flags64 in static constructor0x0600028b RVA0x46498, IL_00a9 | 684, `int(int)` |
| Device count | Method0x06000281 RVA0x41b08 IL_0351 | 630, `int()` |
| Update device / read its name | RVA0x41b08 IL_0449 / IL_0453 | 216, `void(int)` /531, `void(int,char*,int)` |
| Temperature | RVA0x41b08 IL_0d07 and IL_23f6 | 566, `double(int,int,char*,int)` |
| Fan RPM | RVA0x41b08 IL_2671, integer result converted to double | 663, `int(int,int,char*,int)` |
| Power | RVA0x41b08 IL_145c and IL_1f9a | 579, `double(int,int,char*,int)` |
| Cleanup | Method0x06000287 RVA0x44f30 IL_009e | 458, `int()` |

These are cdecl imports with ANSI name buffers. Acquisition scans all512 indices for each temperature/fan/power family, preserving sparse indices and named zero readings. All valid named measurements can be bound individually. Recognized CPU/package, CPU fan, package power and drive temperature measurements populate the existing four bindings. Automatic label priorities, finite/range checks, the256-device bound, five-second polling, separate helper process and30-second stale timeout are independent replacement choices. Missing or stale native data leaves the other sensor providers active. DLL loading uses only the absolute executable-relative path and Windows system dependencies. Normal helper exit calls the library cleanup function; a stuck scan can be terminated without freezing the editor.

The current restricted, non-administrator live probe successfully loaded the DLL; initialization returned1 and device count was0. The app reports these facts and possible hardware/driver permission requirements, without inventing readings or claiming that administrator mode was tested. `evidence/hwinfo-validation.json` records exact package hashes, executable-relative loading with a different CWD, a missing-DLL check and helper shutdown. Build,37 tests and Craft `rebuild_check` pass. The sensor pipeline now preserves fractional values for widget formatting, including imported CPU power and GPU memory amount text.

## Current Deep inspection (2026-11-04)

Opened the original main program **C:\Program Files\35inchENG\UsbMonitor.exe** with `core_target_open`, confirmed Deep mode, and verified SHA-256 **a980a69b634fe6cb3a20173293d23f5728855f41b62748cf6a4b5b9406f972ab**. Repeated `native_strings_find` separately for names, settings and messages. These three searches returned no native matches; `native_xrefs_list` at controller VA **0x412418** returned no edges. This scanner does not decode CLR metadata or the encrypted strings. Followed the real managed method-token references instead: the complete inventory contains **974 methods / 9,045 call sites**, including **1,315 incoming call sites** for the 50 inspected methods.

**Awaited 50 individual `native_function_pseudocode` calls sequentially and read every complete response.** All 50 outputs, their function addresses/evidence IDs, all 50 corresponding managed IL bodies, incoming references and statically decoded names/messages are preserved in [evidence/deep-current-audit.json](evidence/deep-current-audit.json). Ghidra 12.1.4 interprets these CLR method headers/instructions as native machine code and returns invalid pseudo-C, including `halt_baddata()`. Those outputs are an audit of the requested calls; the behavioral rules use the valid ECMA-335 instructions and resolved tokens. Managed IL evidence: **ev_2022be79ad156870c4caa16164d89db6f6a659951fa15d93ecbe41e09806e560**, **ev_0d091c255f343a07240e526ec62a97fd746908e64b3ee7e8242491cfe1fced5c**. Metadata/reference evidence: **ev_8730383db0a81e1544955e0abbfc8e981ba3ded9fec9801b170868850f889b62**. No original executable code was run.

### Newly applied rule: integer sensor text

Controller **RVA 0x12418 / preferred VA 0x412418**, token **0x060000d3**, converts these float readings with **`conv.i4` before `int.ToString()`**. This means truncation toward zero, rather than rounding to the nearest integer. CPU/GPU temperatures additionally use **`PadLeft(2, '0')`**. The existing postprocessor's 99 ceiling still applies where specified below.

| Original name / binding | Getter RVA (preferred VA) | Formatting instructions in controller 0x12418 |
|---|---|---|
| CpuLoad / cpu.usage | **0xdaf4 (0x40daf4)** | **IL_0fc1..0fd8**, conversion at **0fc6** |
| CpuTemp / cpu.temperature | **0xd994 (0x40d994)** | **IL_14e2..14f9**, conversion at **14e7**, zero padding **14f1..14f4** |
| GpuLoad / gpu.usage | **0xe1d4 (0x40e1d4)** | **IL_1770..1787**, conversion at **1775** |
| GpuTemp / gpu.temperature | **0xe074 (0x40e074)** | **IL_15bc..15d3**, conversion at **15c1**, zero padding **15cb..15ce** |
| RamLoad / memory.usage | **0xea24 (0x40ea24)** | **IL_15f8..160f**, conversion at **15fd** |
| HDDLoad / disk.usage | **0xee4c (0x40ee4c)** | **IL_0c5d..0c74**, conversion at **0c62** |

The renderer now applies `truncate_toward_zero(float32(reading))` to these imported fields when their decimal setting is zero. For example, **33.9 displays 33**, and CPU/GPU temperature **0.9 displays 00**. Default imports already store zero decimals; saved JSON retains its legacy metrics, so existing imports receive the correction. Selecting a nonzero decimal count remains an editor extension and uses the chosen precision. Fresh widgets retain their existing formatting. Numeric suffixes and right alignment are preserved.

Truncation occurs only when formatting the text. Raw readings and percentage-bar multiplication retain their fractions: **33.9 in a 150-pixel legacy bar draws 50 pixels**, rather than the 49 pixels that prematurely truncating to 33 would produce. The actual-raster regression first failed on the previous 33.9 -> 34 rendering, then passed with this correction. It covers all six bindings, small values/temperature padding, suffixes, explicit decimal overrides, fresh-widget behavior and fractional bar widths.

The 50-method inspection also reconfirms the serial settings, packet packing/opcodes, RGB565 conversion, orientation/waits, glyph geometry/date formats, 99 ceilings, 8-GHz frequency bars, sensor paths and serialization documented below. Extensions and hardware/raster verification limits remain identified in their respective sections.

Current user-requested editor changes (2026-11-04): uMeter Studio branding and uM icon; scalable imported font metrics using a persisted base_size; tabular numeric advances, proportional kerned letters, editable tracking and right-aligned numeric fields. Original saved digit advances still define the base-size metrics. These are requested editor behaviors, not newly recovered original rules. Stock data now uses Alpha Vantage SYMBOL_SEARCH/GLOBAL_QUOTE; sysinfo supplies identified component temperatures, with existing hardware-provider fallbacks for unsupported readings. sysinfo does not expose fan RPM or CPU power. See README.md and evidence/umeter-live-sensors.json.

## Previous completed inspection: CPU-frequency bar correction

Reopened **C:\Program Files\35inchENG\UsbMonitor.exe** with `core_target_open` in Deep mode. SHA-256 remains `a980a69b634fe6cb3a20173293d23f5728855f41b62748cf6a4b5b9406f972ab`. Ran `native_strings_find` separately for control names, settings and messages, then followed `native_xrefs_list`. Names/settings returned no native matches; the message search returned the manifest at **VA 0x524c78**, with no native reference edges. These results do not cover CLR metadata or encrypted strings.

**49 individual `native_function_pseudocode` calls were awaited sequentially, and each complete response was read.** Their addresses, evidence IDs and full pseudo-C are saved alongside the valid IL for all 49 methods, **1,311 managed incoming call references**, and statically decoded names/messages in [evidence/deep-frequency-audit.json](evidence/deep-frequency-audit.json). The method inventory covers all 974 methods. Fresh behavioral evidence: **ev_d64d5cad89730b7e6a2004fceea660f285946a5934fb3e23c5abaaf9a5b439b6**; settings IL: **ev_b79e05c4e365d40d1cbb915fdfc4f670c858462d26514643feb72bbd58ffce77**.

The original is managed .NET code. Ghidra 12.1.4 incorrectly interprets CLR headers/IL as machine instructions, producing invalid pseudo-C and `halt_baddata()`. Those outputs are preserved as an audit, **not used to infer mechanics**. The built-in ECMA-335 reader provides the real instructions and resolved method/field tokens used below. The optional ILSpy tool was refused by the session's approval policy; it was not run through another route. No original executable code was run.

### Newly recovered rule applied to this build

**CpuFreq bars use 8000 MHz as full scale**, rather than the percentage formula used by ordinary bars:

    pixels = truncate(float32(frequencyMHz) / float32(8000) * float32(savedLength))
    if pixels > savedLength: pixels = savedLength

| Reference | Original address and exact instructions |
|---|---|
| Name and branch | Controller **RVA 0x12418 / VA 0x412418**, token **0x060000d3**: decoded `CpuFreq` at **IL_088e**; equality **IL_089c**, true branch **IL_08bc..08db → IL_0809**. Decoder **RVA 0xcb28**, argument4. |
| Scale and truncation | **0x12418 IL_0809..0822**: getter → `conv.r4`, constant **8000**, `div`, saved length → `conv.r4`, `mul`, `conv.i4`, bar-width setter. This path uses single precision; percentage bars use double precision after reading a float. |
| Upper bound | **0x12418 IL_08e0..092b → IL_05fb..0604** compares the resulting width with saved length and replaces excess width with length. The rebuild also guards negative/nonfinite or unavailable readings. |
| Frequency field | Getter **RVA 0xdc54 / VA 0x40dc54**, token **0x06000097**, reads **field 0x0400005a** at IL_009d. Setter **RVA 0xdd04 / VA 0x40dd04**, token **0x06000098**, writes the same field at IL_009d. Sensor method **0x46968 IL_1dde..1dfe** supplies this setter through averaging helper **0x44658**, token **0x06000282**. |

The rebuild's `cpu.frequency` binding is in **GHz**, so imported bars convert it back to MHz before this calculation. A **4 GHz reading in a 150-pixel bar now draws 75 pixels**; the previous percentage calculation drew 6. New imports show range0..8 GHz in Properties, and older JSON imports are recognized through their existing legacy metrics. Runtime sampling retains three decimal places in GHz to preserve the source's integer MHz resolution; the widget's decimal setting still controls its text. The replacement retains its independent Windows frequency provider and sampling cadence; original sensor acquisition/averaging is not claimed identical.

Pixel-count regression fixtures cover zero, subpixel widths, 1/4/4.8/7.999/8/9 GHz, negative and unavailable input, restored legacy JSON, and a new widget with an editable range. The existing protocol, imported theme, text and sensor-limit checks remain in place.

### Rules reconfirmed in this inspection

| Mechanic | Verified values/formula | Function RVAs (preferred VA = 0x400000 + RVA) |
|---|---|---|
| Serial | **115200 baud, 8N1, DTR/RTS enabled, read timeout1000 ms** | **0x2ebb0** |
| Commands and pixels | Six-byte packed header; image197, orientation121, mirror122, brightness110; RGB565 little endian with masks63488/2016 and blue shift3 | **0x2f8a4, 0x2fac0, 0x2f2c0, 0x2f1f0, 0x10384, 0x51a0, 0x2e07c** |
| Orientation and waits | Portrait0, landscape2; +1 for invertLcd; independent mirror flag; 10ms mode/orientation waits; image waits skipped for descriptor containing `V2` | **0x100e8, 0x2840, 0x2c68, 0x2f090, 0x2f1f0, 0x2f2c0, 0x2fac0** |
| Arcs | Diameter archR, centered stroke archWidth; bounds archR+archWidth; origin pos+30−offset−stroke/2; startPer×3.6°; integer percentage segments | **0x4d350, 0x4db44, 0x4dfc8** |
| Ordinary bars | truncate(float32(value) promoted to double ×0.01×length); default height13 | **0x12418, 0x5fa64** |
| Text and dates | x(i)=posX+i×interval; fixed cells clipped without shrink; Time `HH:mm`, Date `yyyy/M/d` | **0x4f80c, 0x500ec, 0x5190c, 0x527c, 0x85f0, 0x507d4, 0x50ba8, 0x12418** |
| Live ceilings | Values above99 become99 for CpuTemp/CpuLoad/GpuTemp/GpuLoad/GpuRamLoad/RamLoad/Volume | **0x15b78**, getter/setter pairs listed below and retained in fresh evidence |
| GPU memory percentage | truncate(used/total×100), then ceiling99; GpuRam remains an amount | **0x46968, 0xe49c, 0xe54c, 0x15b78, 0x12418** |
| Files | NRBF/BinaryFormatter ThemeConfig; Serialize/GetBuffer and deserialize from stream start | **0xa894, 0xa984** |
| Disk temperature | Configured disk index+1, decoded HDDTEMP_b selection, bundled HWiNFO temperature import | **0x46968, 0x41960** |

Detailed packet bytes, string-decoding formula, reference chains, hardware-source distinctions, independent extensions and remaining limitations follow. Physical screen behavior and GDI+ raster identity remain unverified.

## Where the example retrieves hardware readings (2026-11-03)

The provided UsbMonitor example uses its bundled **HWiNFO client library**, stored at `C:\Program Files\35inchENG\msvcr120d.dll`. Its version resource identifies **HWiNFO32 Client Library 8.21.0.0**, REALiX / Martin Malik; its manifest identifies HWiNFO64. Despite the filename, this file is the sensor library used by this installation. Resource evidence: `ev_1ab7b99976b71dfea86744cf02ee8be48ee58f870743fd9857b36cc1681cb61f`. Managed native-boundary evidence: `ev_4592c9207f73d8cfdc888ab59deedc4b39306bbfa69c8ea52d0ba3471f1b9501`.

CPU acquisition at **RVA 0x46968** handles `CPUTEMP_b`, `FAN_b` and `CPUPWR` through ordinal imports from that HWiNFO library, caching values in `M_Data` and applying configured sensor selections. The CPU power path calls ordinal **579**, token `0x0600027a`, then stores the value at IL_16d5. Existing complete IL: `evidence/deep-live-rules-audit.json`.

Disk temperature uses the same library: sensor method **0x46968 IL_1e89..1e9e** reads `AppConfig.HDDIndex`, adds one, and calls helper **0x41960**. That helper looks up decoded sensor family **HDDTEMP_b** and the selected index, then calls the temperature import **ordinal566**, token `0x06000276`. Its result reaches the HDDTemp setter **0xed9c**, field `0x04000066`. Helper IL evidence: `ev_e0426d020784329fcd8e7aabe641a6c3893dbc83318a10d8f65123a9653fe36f`. Full 974-method inventory evidence: `ev_8730383db0a81e1544955e0abbfc8e981ba3ded9fec9801b170868850f889b62`.

The original also declares `disk.dll!InitDiskTemp` and `GetDiskTemp`. Initialization is called at **0x1f238 IL_001d**, but `GetDiskTemp` has no managed call sites in the full inventory; the observed active HDDTemp path above uses HWiNFO. The separate disk DLL's internals and any runtime-generated paths are unverified. That earlier inspection ran no executable or DLL; the authorized HWiNFO integration above now runs the copied sensor library, without running UsbMonitor.exe.

The replacement retains its existing independent adapters: LibreHardwareMonitor/OpenHardwareMonitor WMI, their loopback web endpoint `http://127.0.0.1:8085/data.json`, running HWiNFO's gadget registry, and Windows `Get-StorageReliabilityCounter` for disk temperature. It now also loads the copied HWiNFO DLL in an isolated helper, as documented above. Display settings help explains the sources, reading units and N/A. Original source evidence remains in `evidence/sensor-sources.json`; direct integration verification is in `evidence/hwinfo-validation.json`.

Original: C:\Program Files\35inchENG\UsbMonitor.exe. SHA-256: a980a69b634fe6cb3a20173293d23f5728855f41b62748cf6a4b5b9406f972ab. Reopened with core_target_open in Deep mode on 2026-11-03. Addresses below are managed method RVAs; the corresponding preferred virtual address is 0x400000 + RVA. These are method header addresses, not native entry points.

## Fresh live-rule inspection (2026-11-03, latest pass)

Opened the original main program again with `core_target_open`; target `t_a980a69b634f`, SHA-256 as above, Deep mode enabled. Completed the three requested `native_strings_find` searches for names, settings and messages; names/settings had zero native matches and messages had three resource matches. Followed `native_xrefs_list` at **0x524a52**, **0x42f8a4** and **0x415b78**; the native engine returned no edges. Then followed the actual managed method-token call references from all 974 methods (1,286 incoming sites for the selected methods).

Awaited and read **38 individual `native_function_pseudocode` responses sequentially**, covering transport/configuration, controller, pixels, file serialization, strings, arcs/bars, text/date rendering, sensor acquisition/postprocessing and the seven capped getters/setters. The complete responses, complete managed IL for all 38 corresponding method RVAs, decoded names/messages and managed reference sites are saved in [evidence/deep-live-rules-audit.json](evidence/deep-live-rules-audit.json). The selected RVAs are:

```
0x2f8a4 0x2ebb0 0x2f090 0x2f1f0 0x2f2c0 0x2fac0
0x100e8 0x10384 0x51a0 0x2e07c 0xa894 0xa984
0x4d350 0x4db44 0x4dfc8 0x5fa64 0x12418 0xcb28
0x15b78 0x46968 0x4f80c 0x500ec 0x507d4 0x50ba8
0xd994 0xda44 0xdaf4 0xdba4 0xe074 0xe124
0xe1d4 0xe284 0xea24 0xead4 0xeb84 0xec38 0xe49c 0xe54c
```

**Decompiler limitation:** Ghidra treats CLR headers/IL as x86 machine code and yields invalid pseudo-C, often `halt_baddata()`; it even merges the CPU-usage getter into an adjacent bogus native function. These outputs establish that the requested calls completed, not the program's behavior. The rules below come from the built-in ECMA-335 reader's real IL, field tokens and statically decoded strings. Fresh managed IL evidence: **ev_7e5b318e921dfbd905ec69f8790bf4dfb086fd0fddba027c828bef7284482ab8**; metadata/call evidence: **ev_8730383db0a81e1544955e0abbfc8e981ba3ded9fec9801b170868850f889b62**. No original executable code was run.

### Newly applied original rule: live-value ceiling

Postprocessor **RVA 0x15b78 (VA 0x415b78), token 0x060000e2**, applies `v > 99 ? 99 : v` to the following seven fields. This also caps temperatures, not just percentage values. The build previously applied this only to GPU memory usage.

| Original name / replacement binding | Getter RVA / setter RVA; shared field token | Compare / assignment in RVA 0x15b78 |
|---|---|---|
| CpuTemp / cpu.temperature | **0xd994 / 0xda44**; 0x04000058 | IL_03c7..03f5 / IL_058a..059a |
| CpuLoad / cpu.usage | **0xdaf4 / 0xdba4**; 0x04000059 | IL_02de..030c / IL_05b9..05c9 |
| GpuTemp / gpu.temperature | **0xe074 / 0xe124**; 0x0400005d | IL_05e5..0614 / IL_025e..026e |
| GpuLoad / gpu.usage | **0xe1d4 / 0xe284**; 0x0400005e | IL_053d..056b / IL_050e..051e |
| GpuRamLoad / gpu.memory.usage | **0xe49c / 0xe54c**; 0x04000060 | IL_028d..02bc / IL_0397..03a7 |
| RamLoad / memory.usage | **0xea24 / 0xead4**; 0x04000064 | IL_01ab..01d9 / IL_0369..0379 |
| Volume, VulomeLoad / volume.level | **0xeb84 / 0xec38**; 0x04000065 | IL_0499..04c3 / IL_04e2..04ef |

Name-to-field references: controller **RVA 0x12418 (VA 0x412418), token 0x060000d3** decodes names with argument4 to decoder **0xcb28**. Its bar paths call getters at IL_0773 (CpuLoad), IL_05bd (CpuTemp), IL_03e6 (GpuLoad), IL_04e0 (GpuTemp), IL_0522 (GpuRamLoad), IL_034c (RamLoad), IL_0684 (Volume). For RamLoad, decoded literal IL_0390 -> condition IL_03be -> dispatcher index8 -> IL_03de -> IL_0349. Each getter and setter accesses the same field in the table. Postprocessor diagnostic labels decoded with argument15 independently identify CPU temperature/load, GPU temperature/load/memory percentage and volume.

`DataState::value` now applies this compatibility rule to imported FontConfig widgets and ArchBar widgets, before numeric formatting and rendering. Their existing `legacy_text` / `legacy_arc` metadata also preserves the rule in previously saved JSON. The ceiling follows the widget's current binding. Raw shared readings and fresh editor widgets keep their existing behavior; unsupported readings remain N/A. Disk load/temperature, frequency, fans, power, memory amounts and external-source fields are not included in this recovered ceiling.

The same controller computes bar pixels as **truncate(v * 0.01 * length)** (e.g. RamLoad IL_0349..0366; CpuLoad IL_0770..078d). It promotes the float sensor to double before multiplying by0.01, then by length; the renderer now preserves that order and precision for imported bars. With the recovered ceiling, a **150-pixel** bar at input100 becomes **148 pixels**: truncate(99 * 0.01 * 150). Input42 becomes **63 pixels**; the prior float-only ratio incorrectly gave62 because its intermediate was62.999996. Input0.1 becomes **0 pixels**. The raster helper now checks the integer dimensions before creating a rectangle, so a subpixel positive width produces zero foreground pixels instead of a zero-width rectangle assertion.

Fresh regression checks cover all seven capped bindings below/at/above99, source-reading preservation, missing readings, unaffected bindings and ordinary widgets; every matching field across all35 installed themes; and literal bar pixel counts at0,0.1,42,50,58,78,99,99.5,100,150.

## Prior awaited text inspection and corrections (2026-11-03)

Reopened the original with `core_target_open` in Deep mode. Ran `native_strings_find` for names (`ThemeConfig`, `FontConfig`, `ScreenTip`, `ArchBar`, `StatuBar`, `CpuLoad`, `GpuRamLoad`, `TItem`), settings (`brightness`, `interval`, `refresh`, `SerialPort`, `baud`, `mirror`, `invertLcd`, `osdImg`, `useOsdWithFontBack`), and messages (`error`, `failed`, `save`, `COM`, `Landscape`, `UsbMonitor`). The first two searches returned zero; the third returned three resource strings. Followed native references at **0x524a52** and **0x4500ec**; neither had edges. The native scanner does not expose the managed references.

All **34** `native_function_pseudocode` calls were awaited sequentially and their complete outputs read: **0x42ebb0, 0x42f8a4, 0x42fac0, 0x42f1f0, 0x42f2c0, 0x42f090, 0x4100e8, 0x410384, 0x4051a0, 0x42e07c, 0x40a894, 0x40a984, 0x44d350, 0x44db44, 0x44dfc8, 0x45fa64, 0x412418, 0x40cb28, 0x40e49c, 0x40e54c, 0x415b78, 0x446968, 0x44f760, 0x44f80c, 0x4500ec, 0x4507d4, 0x450ba8, 0x40527c, 0x45190c, 0x451a74, 0x451bd4, 0x461490, 0x461614, 0x4085f0**. Ghidra interprets CLR method headers/IL as native instructions, so these pseudo-C results are invalid as behavior. The managed ILSpy tool was also attempted and refused: `MCP tool call requires approval, but approval policy is never`. No external decompiler was executed through another route.

Used `managed_members_list` to read valid IL at all 34 addresses, follow method tokens and field accesses, and decode names/messages/formats with `g::b` at RVA **0xcb28**. [The complete fresh audit](evidence/deep-text-audit.json) preserves every native response, full managed IL, 1,281 managed call references, decoded strings and the refusal. Managed import evidence: **ev_cb53ffbf94b43b60de319af8ae6e17a5aa04a5a3fc368d251494ae1439fb5399**; helper/transport evidence: **ev_d0d5e79c59f57eb5f3b9da0335471ec031bc9ddd84b3a14c12dd5a359c9b7094**; gauge/sensor evidence: **ev_bac693d63c275bd62e52248f3896a8c07d7fac43501f185b267fb4a5ac1fcfe2**. These reconfirm the protocol and gauge rules below and establish these additional corrections:

| Recovered rule used in the rebuild | Function RVA (preferred VA), token/reference and instructions |
|---|---|
| Fixed-cell text uses **x(i) = posX + i × interval**, including overlapping cells when interval < W. For n saved cells, bounds width = W + (n−1) × interval; height = H. Removed the incorrect max(interval,W). | Preview **0x4f80c** (0x44f80c), token0x060002f1, draws at IL_072b..0743 then adds interval at IL_0744..0751. Transfer **0x500ec** (0x4500ec), token0x060002f2, adds interval at IL_01ea..01f7; cell transfer IL_0650..0675 uses W/H. Getter **0x5190c** (0x45190c), token0x06000301, IL_009d reads interval field0x04000346; W/H getters **0x51a74/0x51bd4** read fields0x04000347/348. |
| Keep cell dimensions and advance when content exceeds the layer bounds; clip the raster instead of shrinking the whole string to fit. | **0x4f80c**, IL_073e calls DrawImage(image,int,int) without destination scaling; **0x527c** (0x40527c), token0x06000043, crops saved W/H via token0x06000054 at IL_02d5..02de / IL_03d1..03da. Crop helper **0x85f0** (0x4085f0), IL_00e8..0105 copies equally sized source/destination rectangles. |
| Imported Date is **yyyy/M/d**, with unpadded month and day; e.g. **2026/1/3**. Imported Time remains HH:mm. | Long-text update **0x507d4** (0x4507d4), token0x060002f3, IL_022c..0243; preview **0x50ba8** (0x450ba8), token0x060002f4, IL_00e6..00fd. Both decode the format with argument12 via g::b then call DateTime.ToString. Rust uses `%Y/%-m/%-d`. |

Caller chain: `TGroup` **0x61490** (0x461490), IL_00ef calls TItem token0x060002f2; preview **0x61614** (0x461614), IL_00f1 calls token0x060002f1. In `A cyberpunk.data`, CpuFreq stores W22/H28/interval20/strLen3, giving **62×28**, and InternetDown W10/H13/interval9/strLen4 gives **37×13**. All installed intervals are positive (8..24); malformed nonpositive intervals still receive an independent 1-pixel guard. Glyph rasterization remains an approximation of GDI+; the recovered geometry and date rules do not establish pixel identity. Regression checks use the literal saved dimensions, a January date, and a cropped-versus-full glyph raster.

## Earlier inspection records

Earlier pass: reopened the original with core_target_open in Deep mode, then repeated native_strings_find for names (ThemeConfig, AppConfig, ScreenTip, StatuBar, ArchBar, FontConfig, CpuLoad), settings (brightness, interval, refresh, SerialPort, baud, mirror, invertLcd), and messages (error, failed, save, COM, landscape, UsbMonitor). It returned only three version-resource/manifest matches, with no useful native references. native_xrefs_list was followed at resource address 0x524a52 and command method VA 0x42f8a4; both returned no edges. An earlier native_functions_list returned zero native functions. Absence from this scanner does not mean managed functions or strings are absent.

Every native_function_pseudocode request in this pass was awaited and its complete result read, sequentially, at these 22 VAs:

0x42f8a4, 0x42ebb0, 0x42f090, 0x42f1f0, 0x42f2c0, 0x42fac0, 0x4100e8, 0x410384, 0x4051a0, 0x42e07c, 0x40a894, 0x40a984, 0x44d350, 0x44db44, 0x44dfc8, 0x45fa64, 0x412418, 0x40cb28, 0x40e49c, 0x40e54c, 0x415b78, 0x446968.

Ghidra 12.1.4 made functions on demand over CLR headers/IL and returned x86-64 pseudo-C, usually ending in halt_baddata(). This is a .NET assembly; those outputs are invalid as behavioral evidence. The earlier note that all requests returned function_not_found is superseded by these actual results. An earlier ILSpy attempt was refused with "MCP tool call requires approval, but approval policy is never". No external decompiler was run through another route.

The built-in managed_members_list successfully decoded the real IL, types, method tokens, field accesses and call sites. That evidence supports the rules below. [evidence/deep-mechanics.json](evidence/deep-mechanics.json) includes pseudocode evidence IDs/results, complete IL for 18 relevant methods, caller chains and the static string-decoding formula. The original executable was not run.

That earlier pass is preserved in [evidence/deep-recheck.json](evidence/deep-recheck.json): all three searches, both native reference results, all 22 complete native pseudocode responses, complete managed IL for those same 22 methods, managed call references and decoded controller strings. Managed metadata contains 107 types and 974 methods. Native and managed addresses are kept separate; decompiler warnings are retained. It reconfirms the serial configuration, packet packing, orientations, RGB565, gauge geometry and serialization rules below.

| Evidence key | Ledger record | Contents |
|---|---|---|
| Members | ev_49365b5e6b39602089a5ae3c4f6e95ed5a65253eadff8a62f7873a38e0184859 | All 107 types; 974 methods; metadata references |
| Transport/gauges | ev_5b7e5f2b65b35b84d025c14eca1ed6f993b6f398f17dcbd22f501cc756308da7 | Serial setup, packets, settings fields, arc/bar IL |
| Main/pixels/files | ev_2b82dc18bb04cc875c01fe2b315f27d78ed45a0773c13b8bc1c1f20ab3e170aa | Controller, formatting, RGB565, serialization IL |
| Strings | ev_23cf6a2d197d386f7acd82345da0002a8f6ce0b93518c74f4025330af57baac1 | String decoder IL |
| Pixel memory | ev_6ea97a392452c4f05a87e009c50ab9d16759e0ce0e10a059c63d69477232b38a | Bitmap LockBits and memory copy IL |
| Latest metadata | ev_8730383db0a81e1544955e0abbfc8e981ba3ded9fec9801b170868850f889b62 | Complete managed type/method identities and call/field references |
| Latest managed IL | ev_acb7cb94223faac649dc5664b99e3577bb84d8ca60cb321e9aaf26f127df0744 | Transport, controller, gauges, pixels, files, string decoder and GPU percentage chain |

## Verified display protocol

| Value or rule | Function RVA (preferred VA) and instructions |
|---|---|
| **115200 baud**, eight data bits, one stop bit, no parity; DTR=true, RTS=true; read timeout **1000 ms** | 醫::靽() **0x2ebb0** (0x42ebb0), IL_01d2..0211. IL_01ef explicitly loads 115200 before set_BaudRate; the previous 9600 claim was wrong. |
| Landscape 480×320; portrait 320×480 | 醫::鋕(bool) **0x2f090**; also user-required dimensions. |
| Six-byte command packing; take low eight bits of each expression | 醫::鋕(int,int,int,int,int,byte[],int) **0x2f8a4** (0x42f8a4), IL_0127..0170; formula below. |
| Image opcode **197**; endpoints endX=x+width-1, endY=y+height-1 | 醫::鋕(byte[],int,int,int,int,int) **0x2fac0** (0x42fac0), IL_005a..0070. |
| Mirror-mode opcode **122**; 16-byte packet, byte6 is supplied mode; wait **10 ms** afterward | 醫::鋕(int) **0x2f1f0** (0x42f1f0), IL_009f..00bd. |
| Orientation opcode **121**; 16-byte packet; byte6=100+orientation; bytes7..10 are width/high, width/low, height/high, height/low; wait **10 ms** afterward | 醫::鋕(int,int,int) **0x2f2c0** (0x42f2c0), IL_00a0..00ea. |
| Portrait orientation0, landscape2; add1 for invertLcd; mirror is an independent flag sent before orientation | 邁::렕() **0x100e8** (0x4100e8); AppConfig mirrorMode getter **0x2c68**, invertLcd getter **0x2840**. Current Flip controls the latter; the editor hides Mirror as previously requested. Physical interpretation still needs hardware verification. |
| Brightness opcode **110**; raw brightness occupies the first packed coordinate | 邁::ꊌ(int) **0x10384** (0x410384), IL_00ab..00b3. Rebuild percentage UI maps floor((100-clamp(percent,0,100))*255/100): raw255 is dark, raw0 is full brightness, corrected from the user's physical-display report. That UI mapping is independent of the recovered opcode. |
| Non-V2 waits before and after image pixels; V2 skips those waits | **0x2fac0**, IL_0076..0092, 00cb..00cd, 00ef..0114, 0178..017a. Decoded descriptor test is Contains("V2"). Full-frame caller 醫::颧() **0x2f670**, IL_00af supplies **10 ms**; bar caller **0xfc7c**, IL_0128 does the same. Current editor hides V2 as previously requested; conditional transport behavior remains internally covered by fixtures. |
| RGB565: ((r<<8)&63488) OR ((g<<3)&2016) OR (b>>3) | 踭::趘(Color) **0x51a0** (0x4051a0), IL_00b4..00ca. |
| Two bytes/pixel, little endian in native Windows bitmap transfer | 鄖::邁(int) **0x2e07c** (0x42e07c): Format16bppRgb565 (135174), LockBits, Marshal.Copy. Red/green/blue wire fixtures: 00 f8, e0 07, 1f 00. |

For (opcode,x,y,endX,endY), bytes0..5 are:

    x >> 2
    ((x & 3) << 6) + (y >> 4)
    ((y & 15) << 4) + (endX >> 6)
    ((endX & 63) << 2) + (endY >> 8)
    endY & 255
    opcode

The rebuild uses the verified serial configuration, Flip control, mode/orientation waits and conditional image waits. Mirror and V2 UI options remain removed as previously requested. It sends framebuffer rectangles for changed regions; its dirty-tile selection and 4096-byte write chunks are independent implementation details. Original flash-backed glyph caching is not reproduced. The original sets a read timeout only; the Rust serial API applies the selected 1000 ms timeout to reads and writes.

## Verified names, messages and imported gauges

The string decoder g::b(string,int) is at **RVA 0xcb28** (0x40cb28). Start k=303400377+argument. For each encoded UTF-16 code unit c, decoded=(((c&255)^k)&255)<<8 OR (((c>>8)^(k+1))&255); then k+=2. These strings were decoded statically:

- Transport **0x2fac0**, argument10: V2.
- Serial-open error **0x2ebb0**, argument16: 创建设备失败,疑似占用 (failed to create device; likely in use).
- Controller **0x12418**, argument4: CpuLoad, CpuFreq, CpuTemp, CpuFan, CpuPower, GpuLoad, GpuTemp, GpuRam, GpuRamLoad, GpuFan, RamValue, RamLoad, HDDTemp, HDDLoad, InternetUp, InternetDown, Volume, VulomeLoad, Time, and format HH:mm.

Managed references were followed by method token: controller **0x100e8** calls mode token 0x060001ae at IL_0158/01a3 and orientation token 0x060001af at IL_0289; brightness **0x10384** calls packed-command token 0x060001b6 at IL_00b3; q::頒(object,EventArgs) **0x3908c** creates ArchBar token0x060002d4 at IL_00db; controller **0x12208** calls StatuBar update token0x0600038b at IL_017b. Full caller chains are in the audit JSON.

| Rule | Function address and rebuild behavior |
|---|---|
| archR is a **diameter**, archWidth is centered stroke thickness; centerline rectangle starts at (posX+30-offsetX,posY+30-offsetY) | ArchBar::駑() **0x4db44** (0x44db44), IL_01b9..020b. Imported layer origin subtracts half the stroke; dimensions=archR+archWidth. |
| Saved startPer is already a percentage-of-turn offset; startAngle=startPer*3.6° | ArchBar constructor **0x4d350** (0x44d350), IL_0056..005c stores constructor argument minus25. Do not subtract25 again on import. |
| 100 arc segments, each3.6°; segment i starts at (i+startPer)*3.6° | ArchBar::踭(bool) **0x4dfc8** (0x44dfc8), IL_028b..02e0, 038e..0392. Controller **0x12418** converts sensor values to integer percentages. Imported arcs retain integer-percent steps, saved colors and thickness, without an added numeric center label. |
| Ordinary percentage bars: truncate(value*0.01*length); default height **13**, step width **1** | Controller **0x12418**, e.g. IL_0349..0366; StatuBar::趘(string,string) **0x5fa64**, IL_00ed..00f0 and 017e..0190. Imported positive length/height and both colors are preserved. Rebuild clamps values to visible bounds. |
| Imported Time uses **HH:mm** | Controller **0x12418** decoded literal; mapped to Chrono %H:%M. Fresh editor clocks keep their own formats. |
| Both Volume and misspelled VulomeLoad refer to volume data | Controller **0x12418**; both now bind to volume.level, matching the new template. Unavailable readings show N/A. |

Installed 3.5inchTheme1.data has a CpuLoad arc at pos(190,103), offsets(28,28), diameter98, stroke5, startPer=-15. Expected imported bounds=(189.5,102.5,103,103), start=-54°. Its CpuLoad bar is150×13. Tests assert these literal observed values and decode all35 installed themes. Raster anti-aliasing and text placement are not proven pixel-identical to GDI+.

## GPU memory load: newly reconciled behavior

`GpuRam` is an amount of used video memory; `GpuRamLoad` is its capacity percentage. The importer had omitted the latter binding. It now maps `GpuRamLoad` to `gpu.memory.usage`, keeps `GpuRam` on `gpu.memory`, and exposes Video memory usage in the editor's right-hand Data binding picker.

| Verified rule | Function RVA (preferred VA), references and instructions |
|---|---|
| `GpuRamLoad` selects the memory percentage, then sets bar pixels to truncate(percent × 0.01 × length) | Controller **0x12418** (0x412418), decoded literal at IL_0722; branch IL_076b -> IL_051f..053c calls getter token **0x060000a3** |
| Getter and setter share memory-percentage field token **0x04000060** | Getter **0xe49c** (0x40e49c), token0x060000a3; setter **0xe54c** (0x40e54c), token0x060000a4 |
| Compute **truncate(used memory / total memory × 100)**; direct monitoring input also supports `GPURAMLOAD` | Sensor method **0x46968** (0x446968), IL_1437..14d6, ending with conv.i4, conv.r4, setter0x060000a4. Direct percentage sensor path IL_1261..129d. Sensor names decode with decoder argument3. |
| If GPU memory percentage > **99**, replace it with **99** | Postprocessing **0x15b78** (0x415b78), IL_028d..02bc tests getter0x060000a3 > 99; IL_0397..03a2 writes99 with setter0x060000a4 |

The replacement requests NVIDIA's `memory.used` and `memory.total` together, computes the ratio from the unrounded readings, truncates, and applies the recovered99 ceiling. This is capacity usage, distinct from GPU processor activity. Unsupported, nonfinite or negative inputs and zero/missing capacity produce an unavailable percentage; suppressing such readings is a replacement validation choice. It does not invent sensor values. The existing percentage-bar renderer converts the recovered percentage to a bounded pixel length.

The earlier pass added the missing live binding and formula. The latest pass above also applies the postprocessor's other recovered99 ceilings to imported widgets. Provider selection remains independently implemented. The regression fixture1024.99/4096 produces25%, a full-capacity fixture produces99%, and missing/invalid capacity clears the derived value.

## Files and independent editor choices

轗::趘(object) **0xa894** (0x40a894), IL_0004..001a uses BinaryFormatter.Serialize and MemoryStream.GetBuffer. 轗::趘(byte[]) **0xa984** (0x40a984), IL_00be..00df deserializes from the supplied stream's start. Metadata defines ThemeConfig typed lists of ScreenTip, TGroup, StatuBar and ArchBar, plus TItem/FontConfig and Bitmap Data. AppConfig.data is settings, not a theme. The Rust reader activates no CLR objects and respects group enable flags/base offsets and System.Drawing known colors.

Legacy export remains an independent bitmap snapshot in the original schema, with editable JSON after NRBF MessageEnd. Object-graph and Rust round-trip checks pass; original-app acceptance is untested. Full backward live-widget export, original service behavior, font placement, flash caching, hardware orientation and physical streaming remain unverified.

Explicit extensions/estimates, not recovered original rules: grid8px (range2–64), rounding to nearest grid multiple, system/display refresh1s, HTTP refresh60s (minimum5s), history60 samples, general chart fraction clamp((value-min)/(max-min),0,1) with range0–100, preview125%, percentage brightness UI, new weather/stock/Home Assistant/HTTP integrations, starter layout and font sizing. Screenshot verification waits120 UI updates as previously required. LandscapeTechnologyCh remains independently editable; original-arc rules apply only to imported ArchBar layers.


## Editor, telemetry and legacy import fixes (2026-11-03)

Fresh metadata evidence `ev_33b45a4879e4631cc054b308c4fa2429cb5c6cb0e12ac20ab1514236daeba7f4` confirms ThemeConfig stores three bitmaps (`backImg`, `osdImg`, `osdWithFontBack`) and the composite-selection flag. The installed 35 themes have no separate image/rectangle/line lists: static artwork is baked into these images. Import now preserves the chosen artwork as a locked, screen-sized image layer and preserves group `backImg` images. TItem uses FontConfig W/H/interval for field dimensions and glyph advance; ScreenTip uses its own W/H/font. User-defined text stays editable. Color alpha is preserved. The imported raster font is an approximation, not the original GDI+ glyph bitmap.

Independent implementation choices: Windows CPU percentage uses idle/kernel/user deltas (kernel includes idle). Core Audio reads the current default output's master volume. A persistent, hidden PowerShell helper polls LibreHardwareMonitor/OpenHardwareMonitor WMI and Windows storage reliability; readings absent because of hardware, provider or permissions show N/A and an explanation, without fabricated temperature/fan/power values. The helper is killed on exit.

Text can bind to the complete system field catalog or external fields in Properties. The left library contains widget types and connected source fields. StraightLine is separate from the existing Line chart kind. Shape fill and outline are independently configurable; old JSON defaults retain its filled appearance. Layer labels disable text selection.

Streaming compares RGB565 frames in 16-pixel tiles, merges adjacent tiles, sends opcode197 rectangles with their actual coordinates, and skips unchanged frames. Settings are cached: brightness-only changes send the brightness command without retransmitting pixels; orientation/mirror/connection/explicit-upload changes force a full refresh. Preview textures receive partial updates too. Widget rasters are cached by appearance, bound value/history and displayed clock text. These optimizations retain the existing packet encoding, V2 delays and user-confirmed brightness direction.
