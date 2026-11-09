"""Capture the real application without restoring a session or sending display frames."""
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
EXE = ROOT / "target/release/uMonitor.exe"
SHOTS = ROOT / "docs/screenshots"
SHOTS.mkdir(parents=True, exist_ok=True)

for name, view in [("editor", ""), ("ticker", "ticker-circular"), ("sources", "weather"), ("display-settings", "settings")]:
    env = {k: v for k, v in os.environ.items() if not k.startswith("COCKPIT_SCREENSHOT_")}
    env["COCKPIT_SCREENSHOT_LIGHT"] = "1"
    if view:
        env["COCKPIT_SCREENSHOT_VIEW"] = view
    output = SHOTS / (name + ".png")
    subprocess.run([str(EXE), "--screenshot", str(output)], cwd=ROOT, env=env,
                   check=True, timeout=45, creationflags=subprocess.CREATE_NO_WINDOW)
    assert output.is_file() and output.stat().st_size > 10000, output
    print(f"Captured {name}: {output.stat().st_size} bytes", flush=True)
