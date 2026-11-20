use std::path::{Path, PathBuf};

fn main() {
    if std::env::var("TARGET").as_deref() != Ok("x86_64-pc-windows-msvc") {
        return;
    }

    println!("cargo::rerun-if-changed=web/favicon.png");
    println!("cargo::rerun-if-env-changed=BATTING_WINDOWS_RC");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo provides OUT_DIR"));
    let png = include_bytes!("web/favicon.png");
    // ICO supports PNG-compressed images. This entry uses the project's 64×64 favicon.
    let mut icon = vec![0, 0, 1, 0, 1, 0, 64, 64, 0, 0, 1, 0, 32, 0];
    icon.extend_from_slice(&(png.len() as u32).to_le_bytes());
    icon.extend_from_slice(&22_u32.to_le_bytes());
    icon.extend_from_slice(png);
    std::fs::write(out.join("batting.ico"), icon).expect("write Windows icon");
    std::fs::write(out.join("batting.rc"), "1 ICON \"batting.ico\"\n")
        .expect("write Windows resource script");

    let rc = std::env::var_os("BATTING_WINDOWS_RC")
        .map(PathBuf::from)
        .or_else(windows_sdk_rc)
        .unwrap_or_else(|| PathBuf::from("rc.exe"));
    let status = std::process::Command::new(rc)
        .args(["/nologo", "/fo", "batting.res", "batting.rc"])
        .current_dir(&out)
        .status()
        .expect("run rc.exe from the Windows SDK");
    assert!(status.success(), "Windows icon resource compilation failed");
    println!(
        "cargo::rustc-link-arg-bin=batting={}",
        out.join("batting.res").display()
    );
}

fn windows_sdk_rc() -> Option<PathBuf> {
    let program_files = std::env::var_os("ProgramFiles(x86)")?;
    let kits = Path::new(&program_files).join("Windows Kits/10/bin");
    std::fs::read_dir(kits)
        .ok()?
        .filter_map(Result::ok)
        .map(|dir| dir.path().join("x64/rc.exe"))
        .filter(|rc| rc.is_file())
        .max()
}
