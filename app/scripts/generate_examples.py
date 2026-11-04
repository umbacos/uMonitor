"""Generate credential-free, editable portable examples from the current schema."""
import copy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / "config"
CONFIG.mkdir(exist_ok=True)


def save(name, project):
    (CONFIG / name).write_text(json.dumps(project, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def widget(id, kind, x, y, w, h, **values):
    result = dict(id=id, name=kind, kind=kind, x=x, y=y, w=w, h=h,
                  label="", text="", binding="", unit="", color=[98, 225, 187, 255],
                  background=[20, 42, 48, 255], size=25, min=0, max=100,
                  visible=True, locked=False, show_label=False, image=None, font="Segoe UI")
    result.update(values)
    return result


def project(name, portrait=False):
    return dict(version=2, name=name, portrait=portrait, widgets=[], wallpaper=None,
                background=[10, 25, 31, 255], sources=[], grid=True, grid_size=8,
                rotation=False, brightness=65, serial_baud=115200, mirror_mode=False,
                display_v2=False, wallpaper_rotation=0, wallpaper_mirror=False)


landscape = json.loads((ROOT / "designs/LandscapeTechnologyCh.json").read_text(encoding="utf-8"))
landscape["name"] = "Landscape hardware dashboard"
save("landscape.json", landscape)

portrait = project("Portrait essentials", True)
portrait["widgets"] = [
    widget(1, "Text", 20, 18, 280, 25, text="uMONITOR / SYSTEM", size=17),
    widget(2, "Clock", 20, 55, 280, 50, size=38, clock_format="%H:%M:%S"),
    widget(3, "Text", 20, 125, 126, 22, text="CPU", size=16),
    widget(4, "Text", 174, 125, 126, 22, text="RAM", size=16),
    widget(5, "Ring", 20, 158, 126, 126, binding="cpu.usage", unit="%", ring_label_inside=True, ring_thickness=9, decimals=0),
    widget(6, "Ring", 174, 158, 126, 126, binding="memory.usage", unit="%", ring_label_inside=True, ring_thickness=9, decimals=0, color=[108, 183, 248, 255]),
    widget(7, "Value", 20, 317, 280, 40, binding="network.download", unit_prefix="Down ", unit=" MB/s", size=23, decimals=2),
    widget(8, "Value", 20, 370, 280, 40, binding="network.upload", unit_prefix="Up ", unit=" MB/s", size=23, decimals=2),
    widget(9, "Text", 20, 438, 280, 22, text="Live readings from this PC", size=15, color=[130, 161, 168, 255]),
]
save("portrait.json", portrait)

ticker = project("Circular system ticker")
ticker["widgets"] = [
    widget(1, "Text", 24, 26, 432, 36, text="Live dashboard", size=27),
    widget(2, "Clock", 24, 78, 432, 40, clock_format="%H:%M:%S", size=30),
    widget(3, "Ticker", 24, 150, 432, 42, ticker_bound=True, ticker_circular=True,
           scroll_speed=5, scroll_direction="RightToLeft", ticker_sources=[
               dict(binding="cpu.usage", unit_prefix="CPU ", unit_suffix="%", decimals=0),
               dict(binding="memory.usage", unit_prefix="RAM ", unit_suffix="%", decimals=1),
               dict(binding="network.download", unit_prefix="Down ", unit_suffix=" MB/s", decimals=2),
           ]),
    widget(4, "Text", 24, 238, 432, 46, text="Select the ticker to edit its fields", size=18, color=[130, 161, 168, 255]),
]
save("ticker.json", ticker)

connected = copy.deepcopy(ticker)
connected["name"] = "Connected sources - configure before use"
connected["widgets"][0]["text"] = "Connected dashboard"


def source(id, name, kind, **values):
    result = dict(id=id, name=name, kind=kind, url="", auth="None", credential_id="",
                  interval=60, latitude="", longitude="", symbol="", stocks=[])
    result.update(values)
    return result


connected["sources"] = [
    source(1, "Local JSON example", "Json", url="http://127.0.0.1:8080/metrics"),
    source(2, "Home Assistant example", "HomeAssistant", url="http://homeassistant.local:8123", auth="Bearer"),
    source(3, "Rome weather example", "Weather", latitude="41.9028", longitude="12.4964", interval=900),
    source(4, "Stock example - enter your API key", "Stock", symbol="MSFT", stocks=["MSFT"], interval=86400),
]
save("connected-sources.json", connected)
print("Generated four editable config examples (no credentials or session data).")
