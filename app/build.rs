use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/umeter.ico");
    println!("cargo:rerun-if-changed=assets/umeter.rc");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    // Keep the original sensor provider beside debug/release executables, including
    // test binaries in deps/. Loading never depends on the working directory.
    println!("cargo:rerun-if-changed=msvcr120d.dll");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let profile_dir = out_dir.ancestors().nth(3).expect("Cargo profile directory");
    for destination in [profile_dir.to_path_buf(), profile_dir.join("deps")] {
        std::fs::create_dir_all(&destination).expect("sensor runtime directory");
        let target = destination.join("msvcr120d.dll");
        let bytes = std::fs::read("msvcr120d.dll").expect("bundled HWiNFO sensor library");
        if std::fs::read(&target).ok().as_deref() != Some(bytes.as_slice()) {
            std::fs::write(&target, bytes).expect("package HWiNFO sensor library");
        }
    }
    let sdk = PathBuf::from(
        env::var_os("ProgramFiles(x86)").unwrap_or_else(|| "C:/Program Files (x86)".into()),
    )
    .join("Windows Kits/10/bin");
    let mut versions: Vec<_> = std::fs::read_dir(sdk)
        .expect("Windows SDK for app icon")
        .flatten()
        .map(|e| e.path())
        .collect();
    versions.sort();
    let rc = versions
        .into_iter()
        .rev()
        .map(|v| v.join("x64/rc.exe"))
        .find(|p| p.exists())
        .expect("Windows resource compiler");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("umeter.res");
    let result = Command::new(rc)
        .arg("/nologo")
        .arg("/fo")
        .arg(&output)
        .arg("assets/umeter.rc")
        .status()
        .expect("compile uMonitor resources");
    assert!(result.success(), "Windows app resources failed to compile");
    println!("cargo:rustc-link-arg-bin=uMonitor={}", output.display());
}
