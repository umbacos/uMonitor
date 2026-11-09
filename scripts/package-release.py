"""Build an embedded-cabinet MSI and a clean source ZIP with Windows tools only.

MSI schema and sequencing follow Microsoft's Windows Installer documentation:
https://learn.microsoft.com/windows/win32/msi/an-installation-example
"""
import argparse
import ctypes as ct
from ctypes import wintypes as wt
import hashlib
from pathlib import Path
import subprocess
import tempfile
import tomllib
import uuid
import zipfile

ROOT = Path(__file__).resolve().parents[1]
VERSION = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
UPGRADE = "{BCE5F85E-B855-42F7-95C2-D3FDE43A8617}"


class Database:
    def __init__(self, path):
        self.api = ct.WinDLL("msi")
        signatures = {
            "MsiOpenDatabaseW": ([wt.LPCWSTR, wt.LPCWSTR, ct.POINTER(wt.UINT)], wt.UINT),
            "MsiDatabaseOpenViewW": ([wt.UINT, wt.LPCWSTR, ct.POINTER(wt.UINT)], wt.UINT),
            "MsiViewExecute": ([wt.UINT, wt.UINT], wt.UINT),
            "MsiViewFetch": ([wt.UINT, ct.POINTER(wt.UINT)], wt.UINT),
            "MsiCreateRecord": ([wt.UINT], wt.UINT),
            "MsiRecordSetStringW": ([wt.UINT, wt.UINT, wt.LPCWSTR], wt.UINT),
            "MsiRecordSetStreamW": ([wt.UINT, wt.UINT, wt.LPCWSTR], wt.UINT),
            "MsiCloseHandle": ([wt.UINT], wt.UINT),
            "MsiDatabaseCommit": ([wt.UINT], wt.UINT),
            "MsiGetSummaryInformationW": ([wt.UINT, wt.LPCWSTR, wt.UINT, ct.POINTER(wt.UINT)], wt.UINT),
            "MsiSummaryInfoSetPropertyW": ([wt.UINT, wt.UINT, wt.UINT, ct.c_int, ct.c_void_p, wt.LPCWSTR], wt.UINT),
            "MsiSummaryInfoPersist": ([wt.UINT], wt.UINT),
        }
        for name, (args, result) in signatures.items():
            function = getattr(self.api, name)
            function.argtypes, function.restype = args, result
        self.handle = wt.UINT()
        self.check(self.api.MsiOpenDatabaseW(str(path), ct.cast(ct.c_void_p(3), wt.LPCWSTR), ct.byref(self.handle)))

    @staticmethod
    def check(code):
        if code:
            raise RuntimeError(f"Windows Installer error {code}")

    def sql(self, query, values=(), stream=None):
        view = wt.UINT()
        self.check(self.api.MsiDatabaseOpenViewW(self.handle, query, ct.byref(view)))
        record = self.api.MsiCreateRecord(len(values)) if values else 0
        try:
            for i, value in enumerate(values, 1):
                function = self.api.MsiRecordSetStreamW if i == stream else self.api.MsiRecordSetStringW
                self.check(function(record, i, str(value)))
            self.check(self.api.MsiViewExecute(view, record))
        finally:
            if record:
                self.api.MsiCloseHandle(record)
            self.api.MsiCloseHandle(view)

    def table(self, name, columns, keys):
        self.sql(f"CREATE TABLE `{name}` ({columns} PRIMARY KEY {keys})")

    def insert(self, table, columns, values):
        cols = ','.join(f'`{c}`' for c in columns.split(','))
        self.sql(f"INSERT INTO `{table}` ({cols}) VALUES ({','.join('?' for _ in values)})", values)

    def summary(self):
        handle = wt.UINT()
        self.check(self.api.MsiGetSummaryInformationW(self.handle, None, 10, ct.byref(handle)))
        try:
            for key, value in {2: "uMonitor 3.5-inch Setup", 3: "uMonitor", 4: "uMonitor", 7: "x64;1033", 9: "{" + str(uuid.uuid4()).upper() + "}", 14: 200, 15: 2, 19: 2}.items():
                self.check(self.api.MsiSummaryInfoSetPropertyW(handle, key, 3 if isinstance(value, int) else 30, value if isinstance(value, int) else 0, None, None if isinstance(value, int) else value))
            self.check(self.api.MsiSummaryInfoPersist(handle))
        finally:
            self.api.MsiCloseHandle(handle)


def make_msi(payload, output, work):
    db = Database(output)
    try:
        schemas = [
            ("Property", "`Property` CHAR(72) NOT NULL, `Value` CHAR(0) NOT NULL", "`Property`"),
            ("Directory", "`Directory` CHAR(72) NOT NULL, `Directory_Parent` CHAR(72), `DefaultDir` CHAR(255) NOT NULL", "`Directory`"),
            ("Component", "`Component` CHAR(72) NOT NULL, `ComponentId` CHAR(38), `Directory_` CHAR(72) NOT NULL, `Attributes` SHORT NOT NULL, `Condition` CHAR(255), `KeyPath` CHAR(72)", "`Component`"),
            ("Feature", "`Feature` CHAR(38) NOT NULL, `Feature_Parent` CHAR(38), `Title` CHAR(64), `Description` CHAR(255), `Display` SHORT, `Level` SHORT NOT NULL, `Directory_` CHAR(72), `Attributes` SHORT NOT NULL", "`Feature`"),
            ("FeatureComponents", "`Feature_` CHAR(38) NOT NULL, `Component_` CHAR(72) NOT NULL", "`Feature_`,`Component_`"),
            ("File", "`File` CHAR(72) NOT NULL, `Component_` CHAR(72) NOT NULL, `FileName` CHAR(255) NOT NULL, `FileSize` LONG NOT NULL, `Version` CHAR(72), `Language` CHAR(20), `Attributes` SHORT, `Sequence` SHORT NOT NULL", "`File`"),
            ("Media", "`DiskId` SHORT NOT NULL, `LastSequence` SHORT NOT NULL, `DiskPrompt` CHAR(64), `Cabinet` CHAR(255), `VolumeLabel` CHAR(32), `Source` CHAR(72)", "`DiskId`"),
            ("InstallExecuteSequence", "`Action` CHAR(72) NOT NULL, `Condition` CHAR(255), `Sequence` SHORT", "`Action`"),
            ("AdminExecuteSequence", "`Action` CHAR(72) NOT NULL, `Condition` CHAR(255), `Sequence` SHORT", "`Action`"),
            ("Shortcut", "`Shortcut` CHAR(72) NOT NULL, `Directory_` CHAR(72) NOT NULL, `Name` CHAR(128) NOT NULL, `Component_` CHAR(72) NOT NULL, `Target` CHAR(255) NOT NULL, `Arguments` CHAR(255), `Description` CHAR(255), `Hotkey` SHORT, `Icon_` CHAR(72), `IconIndex` SHORT, `ShowCmd` SHORT, `WkDir` CHAR(72)", "`Shortcut`"),
            ("Upgrade", "`UpgradeCode` CHAR(38) NOT NULL, `VersionMin` CHAR(20), `VersionMax` CHAR(20), `Language` CHAR(255), `Attributes` LONG NOT NULL, `Remove` CHAR(255), `ActionProperty` CHAR(72) NOT NULL", "`UpgradeCode`,`VersionMin`,`VersionMax`,`Language`,`Attributes`"),
            ("LaunchCondition", "`Condition` CHAR(255) NOT NULL, `Description` CHAR(255) NOT NULL", "`Condition`"),
        ]
        for schema in schemas:
            db.table(*schema)
        product_code = "{" + str(uuid.uuid5(uuid.UUID(UPGRADE.strip('{}')), VERSION)).upper() + "}"
        properties = {"ProductCode": product_code, "ProductName": "uMonitor 3.5-inch", "ProductVersion": VERSION, "ProductLanguage": "1033", "Manufacturer": "uMonitor", "UpgradeCode": UPGRADE, "INSTALLLEVEL": "1", "ARPNOMODIFY": "1", "ARPCOMMENTS": "Dashboard editor for the original 3.5-inch USB display", "SecureCustomProperties": "PREVIOUSVERSIONS;NEWERVERSIONS"}
        for key, value in properties.items():
            db.insert("Property", "Property,Value", (key, value))
        for row in [("TARGETDIR", None, "SourceDir"), ("LocalAppDataFolder", "TARGETDIR", "."), ("ProgramsDir", "LocalAppDataFolder", "Programs"), ("INSTALLDIR", "ProgramsDir", "uMonitor"), ("ProgramMenuFolder", "TARGETDIR", ".")]:
            # Omit null columns instead of turning them into the literal 'None'.
            cols = ["Directory", "Directory_Parent", "DefaultDir"]
            db.insert("Directory", ','.join(c for c, v in zip(cols, row) if v is not None), [v for v in row if v is not None])
        directories = {".": "INSTALLDIR"}
        for folder in sorted((p for p in payload.rglob('*') if p.is_dir()), key=lambda p: (len(p.parts), str(p))):
            relative = folder.relative_to(payload).as_posix()
            identifier = f"D{len(directories):05}"
            directories[relative] = identifier
            db.insert("Directory", "Directory,Directory_Parent,DefaultDir", (identifier, directories[folder.parent.relative_to(payload).as_posix()], identifier + '|' + folder.name))
        db.insert("Feature", "Feature,Title,Display,Level,Directory_,Attributes", ("Main", "uMonitor", 1, 1, "INSTALLDIR", 0))
        files = sorted(p for p in payload.rglob('*') if p.is_file())
        ddf = ['.OPTION EXPLICIT', '.Set Cabinet=on', '.Set Compress=on', '.Set CompressionType=LZX', '.Set MaxDiskSize=0', '.Set CabinetNameTemplate=payload.cab', f'.Set DiskDirectoryTemplate="{work}"']
        for number, path in enumerate(files, 1):
            identifier = f"F{number:05}"
            relative = path.relative_to(payload).as_posix()
            component = "C" + identifier
            guid = "{" + str(uuid.uuid5(uuid.UUID(UPGRADE.strip('{}')), relative)).upper() + "}"
            db.insert("Component", "Component,ComponentId,Directory_,Attributes,KeyPath", (component, guid, directories[path.parent.relative_to(payload).as_posix()], 256, identifier))
            db.insert("FeatureComponents", "Feature_,Component_", ("Main", component))
            db.insert("File", "File,Component_,FileName,FileSize,Attributes,Sequence", (identifier, component, identifier + '|' + path.name, path.stat().st_size, 16384, number))
            ddf.append(f'"{path}" {identifier}')
            if path.name == 'uMonitor.exe':
                db.insert("Shortcut", "Shortcut,Directory_,Name,Component_,Target,Description,ShowCmd,WkDir", ("StartMenu", "ProgramMenuFolder", "UMON35|uMonitor 3.5-inch", component, "[INSTALLDIR]uMonitor.exe", "uMonitor dashboard editor", 1, "INSTALLDIR"))
        ddf_path = work / 'payload.ddf'
        ddf_path.write_text('\n'.join(ddf), encoding='utf-8')
        result = subprocess.run(['makecab.exe', '/F', str(ddf_path)], cwd=work, capture_output=True, text=True)
        if result.returncode:
            raise RuntimeError(result.stdout + result.stderr)
        db.insert("Media", "DiskId,LastSequence,Cabinet", (1, len(files), "#payload.cab"))
        db.sql("INSERT INTO `_Streams` (`Name`,`Data`) VALUES (?,?)", ("payload.cab", work / 'payload.cab'), stream=2)
        db.insert("Upgrade", "UpgradeCode,VersionMax,Attributes,ActionProperty", (UPGRADE, VERSION, 1, "PREVIOUSVERSIONS"))
        db.insert("Upgrade", "UpgradeCode,VersionMin,Attributes,ActionProperty", (UPGRADE, VERSION, 2, "NEWERVERSIONS"))
        db.insert("LaunchCondition", "Condition,Description", ("NOT NEWERVERSIONS", "A newer version of uMonitor is already installed."))
        db.insert("LaunchCondition", "Condition,Description", ("VersionNT64", "uMonitor requires 64-bit Windows 10 or later."))
        actions = [("FindRelatedProducts", 25), ("LaunchConditions", 100), ("CostInitialize", 800), ("FileCost", 900), ("CostFinalize", 1000), ("InstallValidate", 1400), ("InstallInitialize", 1500), ("RemoveExistingProducts", 1510), ("ProcessComponents", 1600), ("UnpublishFeatures", 1800), ("RemoveShortcuts", 3200), ("RemoveFiles", 3500), ("InstallFiles", 4000), ("CreateShortcuts", 4500), ("RegisterUser", 6000), ("RegisterProduct", 6100), ("PublishFeatures", 6300), ("PublishProduct", 6400), ("InstallFinalize", 6600)]
        for action, sequence in actions:
            db.insert("InstallExecuteSequence", "Action,Sequence", (action, sequence))
        for action, sequence in [("CostInitialize", 800), ("FileCost", 900), ("CostFinalize", 1000), ("InstallValidate", 1400), ("InstallInitialize", 1500), ("InstallAdminPackage", 3900), ("InstallFiles", 4000), ("InstallFinalize", 6600)]:
            db.insert("AdminExecuteSequence", "Action,Sequence", (action, sequence))
        db.summary()
        db.check(db.api.MsiDatabaseCommit(db.handle))
    finally:
        db.api.MsiCloseHandle(db.handle)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, default=ROOT / 'dist')
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    portable = output / f'uMonitor-{VERSION}-windows-x64-portable.zip'
    msi = output / f'uMonitor-{VERSION}-windows-x64.msi'
    source = output / f'uMonitor-{VERSION}-source.zip'
    (ROOT / 'target').mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='release-', dir=ROOT / 'target') as temp:
        work = Path(temp)
        with zipfile.ZipFile(portable) as archive:
            archive.extractall(work)
        pending_msi = work / msi.name
        make_msi(work / 'uMonitor', pending_msi, work)
        pending_msi.replace(msi)
        # Explicit allowlist keeps build caches, sessions, credentials and captures out.
        source_files = [ROOT / name for name in ['.gitignore', 'AGENTS.md', 'LICENSE', 'Cargo.toml', 'Cargo.lock', 'build.rs', 'build.ps1', 'README.md', 'PORTABLE.md', 'THIRD_PARTY_NOTICES.md', 'MECHANICS.md', 'mechanics.json', 'msvcr120d.dll']]
        for folder in ['src', 'assets', 'config', 'designs', 'tests', 'docs', 'scripts', '.cargo']:
            source_files.extend(p for p in (ROOT / folder).rglob('*') if p.is_file() and '__pycache__' not in p.parts and p.suffix != '.pyc')
        pending_source = work / source.name
        with zipfile.ZipFile(pending_source, 'w', zipfile.ZIP_DEFLATED) as archive:
            for path in sorted(source_files):
                archive.write(path, 'uMonitor/' + path.relative_to(ROOT).as_posix())
        pending_source.replace(source)
    for path in [msi, source]:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        path.with_name(path.name + '.sha256').write_text(f'{digest}  {path.name}\n', encoding='utf-8')
        print(f'Created {path} ({path.stat().st_size:,} bytes)')


if __name__ == '__main__':
    main()
