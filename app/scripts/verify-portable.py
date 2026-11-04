"""Check archive integrity and Windows runtime imports using only Python's stdlib."""
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import struct
import sys
import zipfile


def pe_info(data):
    def u16(offset):
        return struct.unpack_from("<H", data, offset)[0]

    def u32(offset):
        return struct.unpack_from("<I", data, offset)[0]

    pe = u32(0x3C)
    assert data[:2] == b"MZ" and data[pe:pe + 4] == b"PE\0\0", "Invalid PE"
    header = pe + 24
    assert u16(header) == 0x20B, "Expected a 64-bit executable"
    section_start = header + u16(pe + 20)
    sections = []
    for i in range(u16(pe + 6)):
        section = section_start + i * 40
        sections.append((u32(section + 12), max(u32(section + 8), u32(section + 16)), u32(section + 20)))

    def offset(rva):
        for start, length, raw in sections:
            if start <= rva < start + length:
                return raw + rva - start
        raise ValueError(f"RVA not mapped: {rva:#x}")

    imports = []
    for directory, stride, name_offset in [(1, 20, 12), (13, 32, 4)]:
        rva = u32(header + 112 + directory * 8)
        if not rva:
            continue
        cursor = offset(rva)
        while any(data[cursor:cursor + stride]):
            name_rva = u32(cursor + name_offset)
            if name_rva:
                start = offset(name_rva)
                imports.append(data[start:data.index(b"\0", start)].decode("ascii"))
            cursor += stride
    return dict(machine=u16(pe + 4), subsystem=u16(header + 68), imports=sorted(set(imports)))


def verify(path):
    with zipfile.ZipFile(path) as archive:
        assert archive.testzip() is None, "ZIP CRC failure"
        names = archive.namelist()
        assert len(names) == len(set(names)), "Duplicate ZIP entries"
        assert all(".." not in PurePosixPath(n).parts and not n.startswith(("/", "\\")) for n in names)
        files = {n: archive.read(n) for n in names if not n.endswith("/")}
    prefix = "uMonitor/"
    assert all(n.startswith(prefix) for n in files), "Unexpected archive root"
    files = {n[len(prefix):]: content for n, content in files.items()}
    required = {"uMonitor.exe", "msvcr120d.dll", "manifest.json", "README.md", "PORTABLE.md", "THIRD_PARTY_NOTICES.md", "MECHANICS.md", "licenses/dependencies.json"}
    required |= {"config/" + n + ".json" for n in ["landscape", "portrait", "ticker", "connected-sources"]}
    required |= {"docs/screenshots/" + n + ".png" for n in ["editor", "ticker", "sources", "display-settings"]}
    assert required <= files.keys(), f"Missing files: {required - files.keys()}"
    assert not any("cockpit-session" in n or n.endswith((".pdb", ".rs")) for n in files), "Development/personal file included"
    manifest = json.loads(files["manifest.json"])
    assert {e["path"] for e in manifest["files"]} == files.keys() - {"manifest.json"}
    for entry in manifest["files"]:
        data = files[entry["path"]]
        assert len(data) == entry["bytes"], entry["path"]
        assert hashlib.sha256(data).hexdigest() == entry["sha256"], entry["path"]
    for name, data in files.items():
        if name.startswith("config/") and name.endswith(".json"):
            project = json.loads(data)
            assert project["version"] == 2 and project["widgets"], name
            for source in project["sources"]:
                assert all(not source.get(key) for key in ["token", "username", "password", "credential_id"]), name
    binaries = {}
    system_dir = Path(os.environ.get("SystemRoot", "C:/Windows")) / "System32"
    for name in ["uMonitor.exe", "msvcr120d.dll"]:
        info = pe_info(files[name])
        assert info["machine"] == 0x8664, name
        if name.endswith(".exe"):
            assert info["subsystem"] == 2, "Expected GUI subsystem, with no console window"
        for dependency in info["imports"]:
            lowered = dependency.lower()
            assert not lowered.startswith(("vcruntime", "msvcp", "msvcr")), f"External VC runtime dependency: {dependency}"
            assert dependency in files or lowered.startswith(("api-ms-", "ext-ms-")) or (system_dir / dependency).is_file(), f"Missing dependency: {dependency}"
        binaries[name] = info
    checksum = hashlib.sha256(path.read_bytes()).hexdigest()
    checksum_file = path.with_name(path.name + ".sha256")
    assert checksum_file.read_text(encoding="utf-8").split()[0] == checksum, "ZIP checksum mismatch"
    return dict(passed=True, archive=path.name, bytes=path.stat().st_size, file_count=len(files), sha256=checksum, binaries=binaries)


if __name__ == "__main__":
    report = verify(Path(sys.argv[1]).resolve())
    if len(sys.argv) > 2:
        Path(sys.argv[2]).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
