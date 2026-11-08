//! task 1.2: `cargo deny check licenses` với deny.toml của dự án phải thất bại khi có
//! thư viện GPL/AGPL (trực tiếp hoặc gián tiếp) hoặc thư viện không khai báo license,
//! và thành công với thư viện Apache-2.0. Mô phỏng "một PR thử" bằng workspace tạm.

use std::path::{Path, PathBuf};
use std::process::Command;

fn app_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn cargo_deny() -> PathBuf {
    let exe = if cfg!(windows) { "cargo-deny.exe" } else { "cargo-deny" };
    let local = app_root().join("tools/bin").join(exe);
    if local.is_file() {
        return local;
    }
    // Trên CI: cài bằng `cargo install cargo-deny` hoặc action EmbarkStudios/cargo-deny-action.
    PathBuf::from(exe)
}

fn write(p: &Path, s: &str) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, s).unwrap();
}

/// Workspace tạm: `app` phụ thuộc `a`; mỗi crate con có license cho trước.
/// `chain`: (tên, license, phụ thuộc).
fn fixture(name: &str, chain: &[(&str, Option<&str>, Option<&str>)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vpdf-deny-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let members: Vec<String> = chain.iter().map(|c| format!("\"{}\"", c.0)).collect();
    write(
        &dir.join("Cargo.toml"),
        &format!(
            "[workspace]\nresolver = \"2\"\nmembers = [\"app\", {}]\n",
            members.join(", ")
        ),
    );
    write(
        &dir.join("app/Cargo.toml"),
        &format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\nlicense = \"Apache-2.0\"\npublish = false\n\n[dependencies]\n{} = {{ path = \"../{}\" }}\n",
            chain[0].0, chain[0].0
        ),
    );
    write(&dir.join("app/src/lib.rs"), "");
    for (n, lic, dep) in chain {
        let lic_line = lic.map(|l| format!("license = \"{l}\"\n")).unwrap_or_default();
        let dep_line = dep.map(|d| format!("{d} = {{ path = \"../{d}\" }}\n")).unwrap_or_default();
        write(
            &dir.join(format!("{n}/Cargo.toml")),
            &format!("[package]\nname = \"{n}\"\nversion = \"1.0.0\"\nedition = \"2021\"\n{lic_line}\n[dependencies]\n{dep_line}"),
        );
        write(&dir.join(format!("{n}/src/lib.rs")), "");
    }
    // deny.toml thật của dự án, chỉ bỏ giới hạn nền tảng.
    std::fs::copy(app_root().join("deny.toml"), dir.join("deny.toml")).unwrap();
    dir
}

fn run(dir: &Path) -> (bool, String) {
    let out = Command::new(cargo_deny())
        .current_dir(dir)
        .args(["--manifest-path", "Cargo.toml", "check", "licenses"])
        .output()
        .expect("không chạy được cargo-deny (cài bằng `cargo install cargo-deny` hoặc đặt vào tools/bin)");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let _ = std::fs::remove_dir_all(dir);
    (out.status.success(), text)
}

/// Scenario "Thư viện AGPL": thất bại, chỉ rõ tên thư viện và license.
#[test]
fn agpl_dependency_fails() {
    let (ok, out) = run(&fixture("agpl", &[("thu-vien-agpl", Some("AGPL-3.0-only"), None)]));
    assert!(!ok, "{out}");
    assert!(out.contains("thu-vien-agpl") && out.contains("AGPL-3.0"), "{out}");
}

/// Scenario "Thư viện cho phép".
#[test]
fn apache_dependency_passes() {
    let (ok, out) = run(&fixture("apache", &[("thu-vien-apache", Some("Apache-2.0"), None)]));
    assert!(ok, "{out}");
}

/// Scenario "Phụ thuộc gián tiếp vi phạm": A (MIT) → B (GPL-3.0), chỉ ra chuỗi A → B.
#[test]
fn transitive_gpl_fails_and_shows_chain() {
    let (ok, out) = run(&fixture(
        "chain",
        &[("thu-vien-a", Some("MIT"), Some("thu-vien-b")), ("thu-vien-b", Some("GPL-3.0-only"), None)],
    ));
    assert!(!ok, "{out}");
    assert!(out.contains("thu-vien-b") && out.contains("GPL-3.0"), "{out}");
    assert!(out.contains("thu-vien-a"), "phải chỉ ra chuỗi phụ thuộc A → B:\n{out}");
}

/// Scenario "Không rõ license".
#[test]
fn missing_license_fails() {
    let (ok, out) = run(&fixture("nolicense", &[("thu-vien-khong-ro", None, None)]));
    assert!(!ok, "{out}");
    assert!(out.contains("thu-vien-khong-ro"), "{out}");
}
