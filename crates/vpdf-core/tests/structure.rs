//! task 1.4: tích hợp qpdf (C API) và lopdf — mở, ghi lại và chạy `qpdf --check`
//! trên bộ file mẫu (`testdata/reference`).

use std::path::PathBuf;
use std::process::Command;
use vpdf_core::qpdf::{self, ObjectStreams, Qpdf};
use vpdf_core::structure;

fn data(name: &str) -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/reference").join(name);
    assert!(p.is_file(), "thiếu {}: chạy `cargo run -p vpdf-testdata --release -- fetch`", p.display());
    p
}

fn out_dir(test: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vpdf-structure-{}-{test}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// (file, mật khẩu, số trang)
const SAMPLES: &[(&str, Option<&str>, usize)] = &[
    ("small-vi.pdf", None, 40),
    ("text-500.pdf", None, 500),
    ("scan-500.pdf", None, 500),
    ("form.pdf", None, 1),
    ("password.pdf", Some("matkhau123"), 40),
    ("no-copy.pdf", None, 40),
];

#[test]
fn qpdf_library_loads() {
    assert!(qpdf::available(), "không nạp được thư viện qpdf");
}

/// Mở bằng qpdf, kiểm tra cấu trúc, ghi lại, rồi kiểm tra lại file vừa ghi.
#[test]
fn qpdf_open_check_rewrite() {
    let dir = out_dir("qpdf");
    for &(name, pw, pages) in SAMPLES {
        let q = Qpdf::open(&data(name), pw).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(q.page_count(), pages, "{name}");
        let r = q.check();
        assert!(r.ok, "{name}: qpdf check: {:?} {:?}", r.warnings, r.error);
        let out = dir.join(format!("qpdf-{name}"));
        q.write(&out, ObjectStreams::Generate, None).unwrap_or_else(|e| panic!("{name}: {e}"));
        let r2 = qpdf::check_file(&out, pw).unwrap();
        assert!(r2.ok && r2.pages == pages, "{name} sau khi ghi lại: {r2:?}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// Quyền đọc được bằng qpdf khớp với file mẫu.
#[test]
fn qpdf_reads_encryption_and_permissions() {
    let q = Qpdf::open(&data("no-copy.pdf"), None).unwrap();
    assert!(q.is_encrypted());
    assert!(!q.allows_extract());
    assert!(!q.allows_print());
    assert!(Qpdf::open(&data("password.pdf"), None).is_err(), "thiếu mật khẩu thì không mở được");
    assert!(Qpdf::open(&data("password.pdf"), Some("sai")).is_err());
    let q = Qpdf::open(&data("password.pdf"), Some("matkhau123")).unwrap();
    assert!(q.is_encrypted() && q.allows_extract());
}

/// File xref hỏng: qpdf báo cảnh báo; ghi lại bằng qpdf thì được file sạch.
#[test]
fn qpdf_repairs_broken_xref() {
    let q = Qpdf::open(&data("broken-xref.pdf"), None).unwrap();
    let r = q.check();
    assert!(!r.warnings.is_empty(), "qpdf phải báo bảng xref hỏng");
    let dir = out_dir("repair");
    let out = dir.join("repaired.pdf");
    Qpdf::open(&data("broken-xref.pdf"), None).unwrap().write(&out, ObjectStreams::Preserve, None).unwrap();
    let r = qpdf::check_file(&out, None).unwrap();
    assert!(r.ok && r.pages == 40, "{r:?}");
    let _ = std::fs::remove_dir_all(dir);
}

/// Mở bằng lopdf, ghi lại, rồi `qpdf --check` (dòng lệnh) trên file vừa ghi.
#[test]
fn lopdf_rewrite_passes_qpdf_check() {
    let dir = out_dir("lopdf");
    let cli = vpdf_core::native::qpdf_cli().expect("không tìm thấy qpdf dòng lệnh");
    for &(name, pw, pages) in SAMPLES {
        if pw.is_some() || name == "no-copy.pdf" {
            continue; // file mã hóa do qpdf xử lý (design.md D3)
        }
        let out = dir.join(format!("lopdf-{name}"));
        let n = structure::rewrite_with_lopdf(&data(name), &out).unwrap();
        assert_eq!(n, pages, "{name}");
        for f in [data(name), out.clone()] {
            let o = Command::new(&cli).arg("--check").arg(&f).output().unwrap();
            assert!(
                o.status.success(),
                "qpdf --check {}: {}{}",
                f.display(),
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}
