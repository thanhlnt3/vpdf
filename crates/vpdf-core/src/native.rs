//! Tìm thư viện native dựng sẵn (PDFium, qpdf) đi kèm ứng dụng.
//!
//! Thứ tự tìm:
//! 1. biến môi trường (`VPDF_PDFIUM_DIR`, `VPDF_QPDF_DIR`),
//! 2. thư mục chứa file chạy (bản phát hành đặt thư viện cạnh file chạy),
//! 3. `third_party/<thư viện>/<nền tảng>/…` trong mã nguồn (khi chạy `cargo run`/`cargo test`).

use std::path::PathBuf;

/// Tên nền tảng dùng trong `third_party/`: `win-x64`, `mac-arm64`, `mac-x64`.
pub fn platform_dir() -> &'static str {
    if cfg!(all(windows, target_arch = "x86_64")) {
        "win-x64"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "mac-arm64"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "mac-x64"
    } else if cfg!(target_os = "linux") {
        "linux-x64"
    } else {
        "unknown"
    }
}

/// Thư mục `third_party/` của mã nguồn.
pub fn third_party_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("third_party")
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(|p| p.to_path_buf())
}

/// Thư mục có thể chứa thư viện PDFium.
pub fn pdfium_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(d) = std::env::var_os("VPDF_PDFIUM_DIR") {
        v.push(PathBuf::from(d));
    }
    if let Some(d) = exe_dir() {
        v.push(d.clone());
        v.push(d.join("..").join("Frameworks")); // macOS .app bundle
    }
    let p = third_party_root().join("pdfium").join(platform_dir());
    v.push(p.join(if cfg!(windows) { "bin" } else { "lib" }));
    if cfg!(target_os = "macos") {
        // Bản universal ghép bằng lipo (scripts/fetch-native.sh mac-universal).
        v.push(third_party_root().join("pdfium").join("mac-universal").join("lib"));
    }
    v
}

/// Tên file thư viện qpdf C API trên nền tảng hiện tại.
pub fn qpdf_library_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["qpdf30.dll", "qpdf29.dll"]
    } else if cfg!(target_os = "macos") {
        &["libqpdf.30.dylib", "libqpdf.dylib"]
    } else {
        &["libqpdf.so.30", "libqpdf.so"]
    }
}

/// Thư mục có thể chứa thư viện qpdf.
pub fn qpdf_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(d) = std::env::var_os("VPDF_QPDF_DIR") {
        v.push(PathBuf::from(d));
    }
    if let Some(d) = exe_dir() {
        v.push(d.clone());
        v.push(d.join("..").join("Frameworks"));
    }
    let root = third_party_root().join("qpdf").join(platform_dir());
    if let Ok(rd) = std::fs::read_dir(&root) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                v.push(p.join(if cfg!(windows) { "bin" } else { "lib" }));
            }
        }
    }
    v.push(root.join("bin"));
    v.push(root.join("lib"));
    v
}

/// Đường dẫn `qpdf` dòng lệnh (dùng trong kiểm thử: `qpdf --check`).
pub fn qpdf_cli() -> Option<PathBuf> {
    let exe = if cfg!(windows) { "qpdf.exe" } else { "qpdf" };
    qpdf_dirs().into_iter().map(|d| d.join(exe)).find(|p| p.is_file())
}
