//! Kiểm tra license cho phần không phải crate Rust (task 1.2; specs/dependency-licensing).
//!
//! - Mỗi thành phần trong `third_party/native.toml` phải có license thuộc danh sách cho
//!   phép, hoặc có ngoại lệ **đã duyệt** trong `third_party/license-exceptions.toml`.
//! - Thành phần không ghi license (hoặc ghi UNKNOWN / NOASSERTION) bị coi là không được phép.
//! - Ngoại lệ thiếu lý do, người duyệt hoặc ngày duyệt làm kiểm tra thất bại.
//! - Mỗi crate trong `exceptions` của `deny.toml` phải có ngoại lệ đã duyệt tương ứng.
//!
//! Mã thoát: 0 = đạt, 1 = vi phạm.

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Danh sách cho phép, đúng như specs/dependency-licensing.
pub const ALLOWED: &[&str] = &[
    "MIT", "BSD-2-Clause", "BSD-3-Clause", "Apache-2.0", "ISC", "Zlib", "BSL-1.0", "Unicode-3.0", "OFL-1.1", "CC0-1.0",
    "MPL-2.0",
];

#[derive(Deserialize, Debug, Clone)]
pub struct Component {
    pub name: String,
    #[serde(default)]
    pub license: String,
    #[serde(default = "yes")]
    pub shipped: bool,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize, Debug, Default)]
pub struct Native {
    #[serde(default)]
    pub component: Vec<Component>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Exception {
    #[serde(default)]
    pub component: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub reviewer: String,
    #[serde(default)]
    pub date: String,
}

#[derive(Deserialize, Debug, Default)]
pub struct Exceptions {
    #[serde(default)]
    pub exception: Vec<Exception>,
}

/// Các trường còn thiếu của một ngoại lệ (rỗng nghĩa là đủ).
pub fn missing_fields(e: &Exception) -> Vec<&'static str> {
    let mut v = Vec::new();
    if e.component.trim().is_empty() {
        v.push("component");
    }
    if e.license.trim().is_empty() {
        v.push("license");
    }
    if e.reason.trim().is_empty() {
        v.push("lý do (reason)");
    }
    if e.reviewer.trim().is_empty() {
        v.push("người duyệt (reviewer)");
    }
    let d = e.date.trim();
    let date_ok = d.len() == 10
        && d.as_bytes()[4] == b'-'
        && d.as_bytes()[7] == b'-'
        && d.chars().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !date_ok {
        v.push("ngày duyệt (date, YYYY-MM-DD)");
    }
    v
}

fn term_ok(term: &str, component: &str, exceptions: &[Exception], problems: &mut Vec<String>) -> bool {
    let term = term.trim().trim_matches(|c| c == '(' || c == ')').trim();
    if term.is_empty() || term.eq_ignore_ascii_case("UNKNOWN") || term.eq_ignore_ascii_case("NOASSERTION") {
        problems.push("không xác định được license".into());
        return false;
    }
    if ALLOWED.contains(&term) {
        return true;
    }
    match exceptions.iter().find(|e| e.component == component && e.license == term) {
        Some(e) => {
            let miss = missing_fields(e);
            if miss.is_empty() {
                true
            } else {
                problems.push(format!("ngoại lệ cho {term} thiếu {}", miss.join(", ")));
                false
            }
        }
        None => {
            problems.push(format!("license {term} không nằm trong danh sách cho phép và không có ngoại lệ đã duyệt"));
            false
        }
    }
}

/// "A OR B": đạt nếu một nhánh đạt. "A AND B": mọi phần phải đạt.
pub fn license_ok(expr: &str, component: &str, exceptions: &[Exception]) -> Result<(), String> {
    if expr.trim().is_empty() {
        return Err("không khai báo license".into());
    }
    let mut problems = Vec::new();
    let mut any = false;
    for alt in expr.split(" OR ") {
        let mut p = Vec::new();
        let mut ok = true;
        for t in alt.split(" AND ") {
            ok &= term_ok(t, component, exceptions, &mut p);
        }
        if ok {
            any = true;
        }
        problems.extend(p);
    }
    if any {
        Ok(())
    } else {
        problems.dedup();
        Err(problems.join("; "))
    }
}

/// Danh sách crate trong `[licenses] exceptions = [...]` của deny.toml.
pub fn deny_exception_crates(deny_toml: &str) -> Vec<String> {
    #[derive(Deserialize, Default)]
    struct Deny {
        #[serde(default)]
        licenses: Licenses,
    }
    #[derive(Deserialize, Default)]
    struct Licenses {
        #[serde(default)]
        exceptions: Vec<DenyException>,
    }
    #[derive(Deserialize)]
    struct DenyException {
        #[serde(rename = "crate", alias = "name")]
        krate: String,
    }
    toml::from_str::<Deny>(deny_toml).map(|d| d.licenses.exceptions.into_iter().map(|e| e.krate).collect()).unwrap_or_default()
}

pub fn check(native: &Native, exceptions: &Exceptions, deny_crates: &[String]) -> Vec<String> {
    let mut errors = Vec::new();
    for c in native.component.iter().filter(|c| c.shipped) {
        if let Err(e) = license_ok(&c.license, &c.name, &exceptions.exception) {
            errors.push(format!("{} [{}]: {e}", c.name, if c.license.is_empty() { "?" } else { &c.license }));
        }
    }
    for e in &exceptions.exception {
        let miss = missing_fields(e);
        if !miss.is_empty() && !errors.iter().any(|x| x.contains(&e.component)) {
            errors.push(format!("ngoại lệ \"{}\" ({}) thiếu {}", e.component, e.license, miss.join(", ")));
        }
    }
    for k in deny_crates {
        match exceptions.exception.iter().find(|e| &e.component == k) {
            None => errors.push(format!("crate {k} có ngoại lệ trong deny.toml nhưng chưa có mục đã duyệt trong license-exceptions.toml")),
            Some(e) if !missing_fields(e).is_empty() => {
                errors.push(format!("crate {k}: ngoại lệ thiếu {}", missing_fields(e).join(", ")))
            }
            _ => {}
        }
    }
    errors
}

fn default_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| {
        eprintln!("không đọc được {}: {e}", p.display());
        std::process::exit(1)
    })
}

fn main() {
    let root = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(default_root);
    let native: Native = toml::from_str(&read(&root.join("third_party/native.toml"))).unwrap_or_else(|e| {
        eprintln!("native.toml sai cú pháp: {e}");
        std::process::exit(1)
    });
    let exceptions: Exceptions =
        toml::from_str(&read(&root.join("third_party/license-exceptions.toml"))).unwrap_or_else(|e| {
            eprintln!("license-exceptions.toml sai cú pháp: {e}");
            std::process::exit(1)
        });
    let deny = deny_exception_crates(&read(&root.join("deny.toml")));
    let errors = check(&native, &exceptions, &deny);
    let shipped = native.component.iter().filter(|c| c.shipped).count();
    if errors.is_empty() {
        println!("license native: đạt ({shipped} thành phần đi kèm)");
    } else {
        eprintln!("license native: {} vi phạm trong {shipped} thành phần đi kèm:", errors.len());
        for e in &errors {
            eprintln!("  - {e}");
        }
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comp(name: &str, license: &str) -> Native {
        Native { component: vec![Component { name: name.into(), license: license.into(), shipped: true }] }
    }

    fn approved(component: &str, license: &str) -> Exception {
        Exception {
            component: component.into(),
            license: license.into(),
            reason: "đã đọc toàn văn".into(),
            reviewer: "Người duyệt mẫu".into(),
            date: "2026-11-08".into(),
        }
    }

    /// Scenario "Thư viện AGPL": thất bại và chỉ rõ tên cùng license.
    #[test]
    fn agpl_component_fails_with_name_and_license() {
        let errs = check(&comp("MuPDF", "AGPL-3.0-only"), &Exceptions::default(), &[]);
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("MuPDF") && errs[0].contains("AGPL-3.0-only"), "{errs:?}");
    }

    /// Scenario "Thư viện cho phép".
    #[test]
    fn apache_component_passes() {
        assert!(check(&comp("qpdf", "Apache-2.0"), &Exceptions::default(), &[]).is_empty());
        assert!(check(&comp("x", "MIT OR GPL-3.0-only"), &Exceptions::default(), &[]).is_empty());
        assert!(!check(&comp("x", "MIT AND GPL-3.0-only"), &Exceptions::default(), &[]).is_empty());
    }

    /// Scenario "Không rõ license": thất bại cho tới khi có ngoại lệ đã duyệt.
    #[test]
    fn unknown_license_fails() {
        for l in ["", "UNKNOWN", "NOASSERTION"] {
            assert!(!check(&comp("lạ", l), &Exceptions::default(), &[]).is_empty(), "{l:?}");
        }
        let ok = Exceptions { exception: vec![approved("lạ", "LicenseRef-Rieng")] };
        assert!(check(&comp("lạ", "LicenseRef-Rieng"), &ok, &[]).is_empty());
    }

    /// Scenario "Ngoại lệ thiếu người duyệt".
    #[test]
    fn exception_without_reviewer_fails() {
        let mut e = approved("FreeType", "FTL");
        e.reviewer = String::new();
        let errs = check(&comp("FreeType", "FTL"), &Exceptions { exception: vec![e.clone()] }, &[]);
        assert!(errs.iter().any(|x| x.contains("người duyệt")), "{errs:?}");
        e.reviewer = "Người duyệt mẫu".into();
        e.date = "hôm qua".into();
        let errs = check(&comp("FreeType", "FTL"), &Exceptions { exception: vec![e] }, &[]);
        assert!(errs.iter().any(|x| x.contains("ngày duyệt")), "{errs:?}");
    }

    #[test]
    fn deny_exceptions_need_approval() {
        let deny = "[licenses]\nexceptions = [{ crate = \"ring\", allow = [\"OpenSSL\"] }]\n";
        let crates = deny_exception_crates(deny);
        assert_eq!(crates, vec!["ring".to_string()]);
        assert!(!check(&Native::default(), &Exceptions::default(), &crates).is_empty());
        let ok = Exceptions { exception: vec![approved("ring", "OpenSSL")] };
        assert!(check(&Native::default(), &ok, &crates).is_empty());
    }

    #[test]
    fn build_only_tools_are_not_checked() {
        let n = Native { component: vec![Component { name: "WiX".into(), license: "MS-RL".into(), shipped: false }] };
        assert!(check(&n, &Exceptions::default(), &[]).is_empty());
    }
}
