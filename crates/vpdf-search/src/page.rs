//! Lớp chữ của một trang ở dạng gọn để lập chỉ mục, và kết quả tìm trên trang.

use crate::normalize::{find_matches, NormText, SearchOptions};
use unicode_normalization::UnicodeNormalization;
use vpdf_core::text::PageText;
use vpdf_core::RectPt;

/// Chữ gốc của trang: các ký tự nối liền trong `text`, `offsets[i]..offsets[i+1]` là ký tự i.
#[derive(Debug, Clone, Default)]
pub struct PageSource {
    text: String,
    offsets: Vec<u32>,
    rects: Vec<RectPt>,
}

impl PageSource {
    pub fn from_page_text(t: &PageText) -> Self {
        let mut s = PageSource { text: String::new(), offsets: Vec::with_capacity(t.chars.len() + 1), rects: Vec::with_capacity(t.chars.len()) };
        for c in &t.chars {
            s.offsets.push(s.text.len() as u32);
            s.text.push_str(&c.text);
            s.rects.push(c.rect);
        }
        s.offsets.push(s.text.len() as u32);
        s
    }

    pub fn char_count(&self) -> usize {
        self.rects.len()
    }

    pub fn char_str(&self, i: usize) -> &str {
        &self.text[self.offsets[i] as usize..self.offsets[i + 1] as usize]
    }

    /// Trang không có lớp chữ (chỉ là ảnh) hoặc chỉ có khoảng trắng.
    pub fn has_text(&self) -> bool {
        self.text.chars().any(|c| !c.is_whitespace() && !c.is_control())
    }

    pub fn chars(&self) -> impl Iterator<Item = (u32, &str)> {
        (0..self.char_count()).map(move |i| (i as u32, self.char_str(i)))
    }

    pub fn normalize(&self, opts: SearchOptions) -> NormText {
        crate::normalize::normalize(self.chars(), opts)
    }

    /// Chữ gốc của các ký tự `[a, b)`, khoảng trắng gộp lại, dạng NFC.
    pub fn text_of(&self, a: usize, b: usize) -> String {
        let b = b.min(self.char_count());
        if a >= b {
            return String::new();
        }
        let raw = &self.text[self.offsets[a] as usize..self.offsets[b] as usize];
        let mut out = String::with_capacity(raw.len());
        for c in raw.nfc() {
            if c.is_whitespace() || c.is_control() {
                if !out.ends_with(' ') {
                    out.push(' ');
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    /// Hộp bao của các ký tự `first..=last`, gộp theo dòng (mỗi dòng một hình chữ nhật).
    pub fn line_rects(&self, first: usize, last: usize) -> Vec<RectPt> {
        let mut out: Vec<RectPt> = Vec::new();
        for r in &self.rects[first..=last.min(self.char_count().saturating_sub(1))] {
            if r.width() <= 0.0 || r.height() <= 0.0 {
                continue;
            }
            if let Some(cur) = out.last_mut() {
                let overlap = cur.top.min(r.top) - cur.bottom.max(r.bottom);
                if overlap >= 0.5 * cur.height().min(r.height()) && r.left >= cur.left - 1.0 {
                    *cur = RectPt::new(cur.left.min(r.left), cur.bottom.min(r.bottom), cur.right.max(r.right), cur.top.max(r.top));
                    continue;
                }
            }
            out.push(*r);
        }
        out
    }

    pub fn bytes(&self) -> usize {
        self.text.len() + 4 * self.offsets.len() + std::mem::size_of::<RectPt>() * self.rects.len()
    }
}

/// Đoạn chữ quanh kết quả, hiện trong danh sách kết quả.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snippet {
    pub before: String,
    pub matched: String,
    pub after: String,
}

/// Một kết quả tìm.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    /// Trang (tính từ 0).
    pub page: u32,
    /// Ký tự gốc đầu và cuối (gồm cả hai) trong lớp chữ của trang.
    pub first: u32,
    pub last: u32,
    /// Vùng tô sáng (tọa độ PDF), mỗi dòng một hình chữ nhật.
    pub rects: Vec<RectPt>,
    pub snippet: Snippet,
}

/// Số ký tự gốc lấy thêm mỗi bên cho đoạn chữ xung quanh.
pub const SNIPPET_CONTEXT: usize = 32;

/// Tìm `query` (đã chuẩn hóa) trên một trang.
pub fn hits_on_page(page: u32, src: &PageSource, norm: &NormText, query: &str, opts: SearchOptions) -> Vec<Hit> {
    find_matches(norm, query, opts)
        .into_iter()
        .map(|(a, b)| {
            let (first, last) = norm.source_range(a, b);
            let (f, l) = (first as usize, last as usize);
            let start = f.saturating_sub(SNIPPET_CONTEXT);
            let end = (l + 1 + SNIPPET_CONTEXT).min(src.char_count());
            let mut before = src.text_of(start, f);
            if start > 0 {
                before.insert(0, '…');
            }
            let mut after = src.text_of(l + 1, end);
            if end < src.char_count() {
                after.push('…');
            }
            Hit {
                page,
                first,
                last,
                rects: src.line_rects(f, l),
                snippet: Snippet { before: before.trim_start().to_string(), matched: src.text_of(f, l + 1), after: after.trim_end().to_string() },
            }
        })
        .collect()
}
