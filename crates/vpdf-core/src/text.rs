//! Lớp chữ của trang: chọn chữ bằng cách kéo chuột và sao chép ra NFC.

use crate::{messages, RectPt};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

/// Một ký tự của lớp chữ, theo thứ tự nội dung (thứ tự đọc) mà PDFium trả về.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextChar {
    /// Ký tự (có thể là dấu tổ hợp hoặc ký tự xuống dòng do PDFium sinh ra).
    pub text: String,
    /// Hộp bao, tọa độ PDF. Ký tự sinh ra (xuống dòng) có hộp rỗng.
    pub rect: RectPt,
    /// Ký tự do PDFium sinh thêm (khoảng trắng, xuống dòng giữa các dòng).
    pub generated: bool,
}

/// Lớp chữ của một trang.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PageText {
    pub chars: Vec<TextChar>,
}

impl PageText {
    pub fn is_empty(&self) -> bool {
        self.chars.iter().all(|c| c.text.trim().is_empty())
    }

    /// Ký tự tại (hoặc gần nhất, trong phạm vi `tolerance` point) điểm (x, y).
    pub fn char_near(&self, x: f32, y: f32, tolerance: f32) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, c) in self.chars.iter().enumerate() {
            if c.rect.width() <= 0.0 && c.rect.height() <= 0.0 {
                continue;
            }
            let d = c.rect.distance(x, y);
            if d == 0.0 {
                return Some(i);
            }
            if d <= tolerance && best.map_or(true, |(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best.map(|b| b.0)
    }

    /// Chuỗi của các ký tự từ `a` tới `b` (gồm cả hai đầu, không cần a <= b).
    pub fn range_text(&self, a: usize, b: usize) -> String {
        if self.chars.is_empty() {
            return String::new();
        }
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let hi = hi.min(self.chars.len() - 1);
        let mut s = String::new();
        for c in &self.chars[lo..=hi] {
            s.push_str(&c.text);
        }
        s
    }

    /// Toàn bộ chữ của trang.
    pub fn all_text(&self) -> String {
        if self.chars.is_empty() {
            String::new()
        } else {
            self.range_text(0, self.chars.len() - 1)
        }
    }
}

/// Chuẩn hóa chuỗi trước khi đưa vào clipboard: NFC, xuống dòng kiểu "\n",
/// bỏ ký tự rỗng (NUL) và khoảng trắng thừa ở hai đầu.
pub fn normalize_for_clipboard(s: &str) -> String {
    let s = s.replace("\r\n", "\n").replace('\r', "\n").replace('\u{0}', "").replace('\u{2}', "-");
    let s: String = s.nfc().collect();
    s.trim_matches(|c: char| c == ' ' || c == '\n').to_string()
}

/// Kết quả của lệnh Sao chép.
#[derive(Debug, Clone, PartialEq)]
pub enum CopyOutcome {
    /// Đưa chuỗi này vào clipboard.
    Copy(String),
    /// Không thay đổi clipboard; hiện thông báo.
    Denied(&'static str),
    /// Chưa chọn gì.
    Nothing,
}

/// Quyết định sao chép: chặn khi tài liệu cấm sao chép (và chưa mở bằng mật khẩu
/// chủ sở hữu — khi đó PDFium báo đủ quyền nên `copy_allowed` là true).
pub fn copy_selection(selected: &str, copy_allowed: bool) -> CopyOutcome {
    if !copy_allowed {
        return CopyOutcome::Denied(messages::COPY_FORBIDDEN);
    }
    let s = normalize_for_clipboard(selected);
    if s.is_empty() {
        CopyOutcome::Nothing
    } else {
        CopyOutcome::Copy(s)
    }
}

/// Vùng chọn chữ có thể trải nhiều trang: (trang, chỉ số ký tự) đầu và cuối.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub start: (usize, usize),
    pub end: (usize, usize),
}

impl Selection {
    /// Đầu và cuối theo thứ tự đọc.
    pub fn ordered(&self) -> ((usize, usize), (usize, usize)) {
        if self.start <= self.end {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }

    /// Các ký tự được chọn trên trang `page` (khoảng gồm hai đầu), nếu có.
    pub fn range_on_page(&self, page: usize, char_count: usize) -> Option<(usize, usize)> {
        if char_count == 0 {
            return None;
        }
        let ((p0, c0), (p1, c1)) = self.ordered();
        if page < p0 || page > p1 {
            return None;
        }
        let lo = if page == p0 { c0 } else { 0 };
        let hi = if page == p1 { c1 } else { char_count - 1 };
        Some((lo.min(char_count - 1), hi.min(char_count - 1)))
    }

    /// Ghép chữ được chọn trên các trang, theo thứ tự đọc.
    pub fn text(&self, page_text: impl Fn(usize) -> Option<PageText>) -> String {
        let ((p0, _), (p1, _)) = self.ordered();
        let mut parts = Vec::new();
        for p in p0..=p1 {
            if let Some(t) = page_text(p) {
                if let Some((a, b)) = self.range_on_page(p, t.chars.len()) {
                    parts.push(t.range_text(a, b));
                }
            }
        }
        parts.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars_of(s: &str) -> PageText {
        let mut x = 0.0;
        PageText {
            chars: s
                .chars()
                .map(|c| {
                    let r = RectPt::new(x, 700.0, x + 6.0, 712.0);
                    x += 6.0;
                    TextChar { text: c.to_string(), rect: r, generated: false }
                })
                .collect(),
        }
    }

    /// Scenario: Sao chép chữ tiếng Việt — clipboard chứa đúng chuỗi ở dạng NFC.
    #[test]
    fn copies_vietnamese_as_nfc() {
        let expected = "Cộng hòa xã hội chủ nghĩa Việt Nam";
        // PDF có thể lưu chữ ở dạng tổ hợp (NFD): "ộ" = o + ̣ + ̂.
        let nfd: String = expected.nfd().collect();
        assert_ne!(nfd, expected);
        let page = chars_of(&nfd);
        let sel = Selection { start: (0, 0), end: (0, page.chars.len() - 1) };
        let text = sel.text(|_| Some(page.clone()));
        match copy_selection(&text, true) {
            CopyOutcome::Copy(s) => {
                assert_eq!(s, expected);
                assert!(unicode_normalization::is_nfc(&s));
            }
            o => panic!("{o:?}"),
        }
    }

    /// Scenario: Tài liệu cấm sao chép — không đổi clipboard, báo lỗi.
    #[test]
    fn copy_denied_when_forbidden() {
        assert_eq!(copy_selection("abc", false), CopyOutcome::Denied("Tài liệu này không cho phép sao chép"));
        assert_eq!(copy_selection("   ", true), CopyOutcome::Nothing);
    }

    #[test]
    fn selection_across_pages_and_reverse_drag() {
        let a = chars_of("Trang một");
        let b = chars_of("Trang hai");
        let sel = Selection { start: (1, 4), end: (0, 6) };
        let t = sel.text(|p| Some(if p == 0 { a.clone() } else { b.clone() }));
        assert_eq!(t, "một\nTrang");
    }

    #[test]
    fn hit_test_finds_nearest_char() {
        let p = chars_of("abc");
        assert_eq!(p.char_near(7.0, 705.0, 2.0), Some(1));
        assert_eq!(p.char_near(19.0, 705.0, 2.0), Some(2));
        assert_eq!(p.char_near(100.0, 100.0, 2.0), None);
    }
}
