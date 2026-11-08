//! Danh sách file gần đây và vị trí đọc (trang, mức phóng) của 50 file mở gần nhất.

use crate::view::{FitMode, ViewPos};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

/// Số file được nhớ.
pub const MAX_RECENT: usize = 50;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentEntry {
    /// Đường dẫn để hiển thị và mở lại (dạng NFC).
    pub path: String,
    pub page: usize,
    pub y_in_page_pt: f32,
    pub zoom: f32,
    pub fit: FitMode,
}

impl RecentEntry {
    pub fn pos(&self) -> ViewPos {
        ViewPos { page: self.page, y_in_page_pt: self.y_in_page_pt, zoom: self.zoom, fit: self.fit }
    }

    pub fn file_name(&self) -> String {
        Path::new(&self.path).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecentFiles {
    /// File mới nhất đứng đầu.
    pub entries: Vec<RecentEntry>,
}

/// Khóa so sánh đường dẫn: NFC (tên file NFD trên macOS được coi như NFC), dấu
/// phân cách thống nhất, và không phân biệt hoa thường trên Windows.
pub fn path_key(path: &str) -> String {
    let s: String = path.nfc().collect();
    let s = s.replace('\\', "/");
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s
    }
}

/// Đường dẫn tuyệt đối dạng NFC để lưu.
pub fn normalize_path(path: &Path) -> String {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map(|d| d.join(path)).unwrap_or_else(|_| path.to_path_buf())
    };
    abs.to_string_lossy().nfc().collect()
}

impl RecentFiles {
    pub fn lookup(&self, path: &Path) -> Option<&RecentEntry> {
        let k = path_key(&normalize_path(path));
        self.entries.iter().find(|e| path_key(&e.path) == k)
    }

    /// Ghi nhớ vị trí đọc của file và đưa file lên đầu danh sách.
    pub fn remember(&mut self, path: &Path, pos: ViewPos) {
        let p = normalize_path(path);
        let k = path_key(&p);
        self.entries.retain(|e| path_key(&e.path) != k);
        self.entries.insert(
            0,
            RecentEntry { path: p, page: pos.page, y_in_page_pt: pos.y_in_page_pt, zoom: pos.zoom, fit: pos.fit },
        );
        self.entries.truncate(MAX_RECENT);
    }

    /// Bỏ một file khỏi danh sách (ví dụ khi file không còn tồn tại).
    pub fn forget(&mut self, path: &Path) {
        let k = path_key(&normalize_path(path));
        self.entries.retain(|e| path_key(&e.path) != k);
    }

    pub fn load(file: &Path) -> Self {
        std::fs::read(file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    /// Lưu nguyên tử: ghi file tạm rồi đổi tên.
    pub fn save(&self, file: &Path) -> std::io::Result<()> {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?)?;
        std::fs::rename(&tmp, file)
    }
}

/// Thư mục cấu hình mặc định: biến môi trường `VPDF_CONFIG_DIR`, nếu không có thì
/// `%APPDATA%\VPdf` (Windows) hoặc `~/Library/Application Support/VPdf` (macOS).
pub fn default_config_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("VPDF_CONFIG_DIR") {
        return PathBuf::from(d);
    }
    if cfg!(windows) {
        if let Some(a) = std::env::var_os("APPDATA") {
            return PathBuf::from(a).join("VPdf");
        }
    }
    if let Some(h) = std::env::var_os("HOME") {
        let h = PathBuf::from(h);
        if cfg!(target_os = "macos") {
            return h.join("Library/Application Support/VPdf");
        }
        return h.join(".config/vpdf");
    }
    PathBuf::from(".vpdf")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(page: usize) -> ViewPos {
        ViewPos { page, y_in_page_pt: 0.0, zoom: 1.25, fit: FitMode::Free }
    }

    /// Scenario: Mở lại file — mở tại trang 87.
    #[test]
    fn reopen_restores_page() {
        let dir = std::env::temp_dir().join(format!("vpdf-recent-{}", std::process::id()));
        let file = dir.join("recent.json");
        let doc = dir.join("Hợp đồng thuê nhà.pdf");
        let mut r = RecentFiles::default();
        r.remember(&doc, pos(86));
        r.save(&file).unwrap();
        let r2 = RecentFiles::load(&file);
        let e = r2.lookup(&doc).expect("phải nhớ file");
        assert_eq!(e.page + 1, 87);
        assert_eq!(e.zoom, 1.25);
        assert_eq!(e.file_name(), "Hợp đồng thuê nhà.pdf");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn keeps_only_50_most_recent() {
        let mut r = RecentFiles::default();
        for i in 0..60 {
            r.remember(Path::new(&format!("/docs/f{i}.pdf")), pos(i));
        }
        assert_eq!(r.entries.len(), MAX_RECENT);
        assert!(r.lookup(Path::new("/docs/f59.pdf")).is_some());
        assert!(r.lookup(Path::new("/docs/f9.pdf")).is_none());
        assert!(r.lookup(Path::new("/docs/f10.pdf")).is_some());
        // Mở lại một file cũ đưa nó lên đầu, không nhân đôi.
        r.remember(Path::new("/docs/f10.pdf"), pos(3));
        assert_eq!(r.entries.len(), MAX_RECENT);
        assert!(r.entries[0].path.ends_with("f10.pdf"));
    }

    #[test]
    fn nfd_and_nfc_names_are_the_same_file() {
        let nfc = "/Tài liệu/Hợp đồng.pdf";
        let nfd: String = nfc.nfd().collect();
        let mut r = RecentFiles::default();
        r.remember(Path::new(&nfd), pos(5));
        assert_eq!(r.lookup(Path::new(nfc)).unwrap().page, 5);
        assert!(unicode_normalization::is_nfc(&r.entries[0].path));
    }
}
