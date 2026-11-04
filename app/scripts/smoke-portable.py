"""Launch an extracted portable copy from an unrelated working directory."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid
import zipfile

ROOT = Path(__file__).resolve().parents[1]
archive_path = Path(sys.argv[1]).resolve()
destination = ROOT / "target" / ("portable-smoke-" + uuid.uuid4().hex) / "folder with spaces"
destination.mkdir(parents=True)
with zipfile.ZipFile(archive_path) as archive:
    archive.extractall(destination)
portable = destination / "uMonitor"
exe = portable / "uMonitor.exe"
env = {k: v for k, v in os.environ.items() if not k.startswith("COCKPIT_SCREENSHOT_")}
env["COCKPIT_SCREENSHOT_LIGHT"] = "1"
report = dict(extracted_to=str(portable), executable_sha256=hashlib.sha256(exe.read_bytes()).hexdigest(), screenshots=[])
for name in ["landscape", "portrait", "ticker"]:
    env["COCKPIT_SCREENSHOT_DESIGN"] = str(portable / "config" / (name + ".json"))
    output = ROOT / "evidence" / ("portable-" + name + ".png")
    result = subprocess.run([str(exe), "--screenshot", str(output)], cwd=ROOT.parent,
                            env=env, capture_output=True, check=True, timeout=45,
                            creationflags=subprocess.CREATE_NO_WINDOW)
    assert output.is_file() and output.stat().st_size > 10000, name
    assert not (portable / "cockpit-session.json").exists(), "Screenshot changed session state"
    report["screenshots"].append(dict(design=name, path=str(output), bytes=output.stat().st_size, exit_code=result.returncode))
    print(f"Portable {name} launch passed", flush=True)

result = subprocess.run([str(exe), "--hardware-scan"], cwd=ROOT.parent, env=env,
                        capture_output=True, check=True, timeout=25,
                        creationflags=subprocess.CREATE_NO_WINDOW)
scan = json.loads(result.stdout.decode("utf-8"))
report["sensor_helper"] = scan
assert str(scan.get("_diagnostics", "")).startswith("HWiNFO DLL"), scan
report["passed"] = True
(ROOT / "evidence/portable-smoke.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
print(json.dumps(report, indent=2))
