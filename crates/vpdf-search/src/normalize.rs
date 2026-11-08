//! Chuẩn hóa chữ để tìm (design.md D4).
//!
//! Chữ của trang (chuỗi gốc, mỗi phần tử là một ký tự PDFium trả về) được đổi thành
//! chuỗi đã chuẩn hóa kèm **bảng ánh xạ** từ mỗi ký tự chuẩn hóa về khoảng ký tự gốc,
//! để tô sáng đúng chữ gốc có dấu.
//!
//! Các bước:
//! 1. Tách mỗi ký tự về dạng NFD (dấu dựng sẵn → chữ cơ sở + dấu tổ hợp).
//! 2. Gom "cụm": chữ cơ sở + các dấu tổ hợp theo sau, rồi sắp các dấu theo thứ tự
//!    chuẩn (combining class). Nhờ vậy "e + dấu mũ + dấu nặng" và "e + dấu nặng + dấu mũ"
//!    giống nhau, và giống "ệ" dựng sẵn.
//! 3. Mặc định: bỏ mọi dấu tổ hợp (dấu thanh, dấu mũ, dấu móc, dấu trăng), đ/Đ → d/D.
//!    Khi bật Phân biệt dấu thì giữ dấu (và giữ đ).
//! 4. Mặc định không phân biệt hoa thường (chuyển về chữ thường).
//! 5. Mọi khoảng trắng liên tiếp (kể cả xuống dòng PDFium sinh ra) gộp thành một dấu
//!    cách, nên cụm bị ngắt dòng vẫn tìm được.

use unicode_normalization::char::{canonical_combining_class, decompose_canonical, is_combining_mark};

/// Tùy chọn tìm chính xác (specs/text-search › Tùy chọn tìm chính xác).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SearchOptions {
    /// Phân biệt dấu.
    pub match_diacritics: bool,
    /// Phân biệt hoa thường.
    pub match_case: bool,
    /// Khớp nguyên từ.
    pub whole_word: bool,
}

impl SearchOptions {
    /// Khóa của dạng chuẩn hóa (Khớp nguyên từ không đổi chuỗi chuẩn hóa).
    pub fn form_key(&self) -> usize {
        (self.match_diacritics as usize) | ((self.match_case as usize) << 1)
    }
}

/// Chuỗi đã chuẩn hóa kèm bảng ánh xạ về chỉ số ký tự gốc.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NormText {
    pub text: String,
    /// Vị trí byte bắt đầu của từng ký tự chuẩn hóa trong `text`.
    starts: Vec<u32>,
    /// Khoảng ký tự gốc (gồm hai đầu) sinh ra từng ký tự chuẩn hóa.
    lo: Vec<u32>,
    hi: Vec<u32>,
}

/// Ký tự PDFium dùng để đánh dấu gạch nối mềm ở cuối dòng.
const PDFIUM_SOFT_HYPHEN: char = '\u{2}';

impl NormText {
    pub fn len_chars(&self) -> usize {
        self.starts.len()
    }

    /// Chỉ số ký tự chuẩn hóa bắt đầu tại byte `b` (b là ranh giới ký tự, hoặc bằng độ dài chuỗi).
    pub fn char_at_byte(&self, b: usize) -> usize {
        self.starts.partition_point(|&s| (s as usize) < b)
    }

    pub fn char(&self, i: usize) -> Option<char> {
        let s = *self.starts.get(i)? as usize;
        self.text[s..].chars().next()
    }

    /// Khoảng ký tự gốc của các ký tự chuẩn hóa `[a, b)`.
    pub fn source_range(&self, a: usize, b: usize) -> (u32, u32) {
        (self.lo[a], self.hi[b - 1])
    }

    fn push(&mut self, c: char, lo: u32, hi: u32) {
        self.starts.push(self.text.len() as u32);
        self.lo.push(lo);
        self.hi.push(hi);
        self.text.push(c);
    }

    /// Bộ nhớ ước lượng (byte).
    pub fn bytes(&self) -> usize {
        self.text.len() + 12 * self.starts.len()
    }
}

/// Chuẩn hóa một dãy ký tự gốc. `source` cho (chỉ số gốc, chuỗi của ký tự đó).
pub fn normalize<'a>(source: impl IntoIterator<Item = (u32, &'a str)>, opts: SearchOptions) -> NormText {
    // 1. NFD, mỗi ký tự NFD nhớ chỉ số gốc.
    let mut units: Vec<(char, u32)> = Vec::new();
    for (i, s) in source {
        for ch in s.chars() {
            decompose_canonical(ch, |d| units.push((d, i)));
        }
    }
    let mut out = NormText::default();
    let mut join_line = false;
    let mut k = 0;
    while k < units.len() {
        // 2. Cụm: một ký tự cơ sở và các dấu tổ hợp theo sau.
        let mut end = k + 1;
        while end < units.len() && canonical_combining_class(units[end].0) != 0 {
            end += 1;
        }
        let cluster = &mut units[k..end];
        cluster[1..].sort_by_key(|u| canonical_combining_class(u.0));
        let lo = cluster.iter().map(|u| u.1).min().unwrap();
        let hi = cluster.iter().map(|u| u.1).max().unwrap();
        for (j, &(c, _)) in cluster.iter().enumerate() {
            emit(&mut out, c, j == 0, lo, hi, opts, &mut join_line);
        }
        k = end;
    }
    out
}

fn emit(out: &mut NormText, c: char, base: bool, lo: u32, hi: u32, opts: SearchOptions, join_line: &mut bool) {
    if c == PDFIUM_SOFT_HYPHEN || c == '\u{AD}' {
        // Gạch nối mềm cuối dòng: nối với chữ ở dòng sau.
        *join_line = true;
        return;
    }
    if c.is_whitespace() || c == '\u{0}' {
        if !*join_line && !out.text.is_empty() && !out.text.ends_with(' ') {
            out.push(' ', lo, hi);
        }
        return;
    }
    if c.is_control() {
        return;
    }
    *join_line = false;
    if !base || is_combining_mark(c) {
        if opts.match_diacritics {
            out.push(c, lo, hi);
        }
        return;
    }
    let c = match c {
        'đ' if !opts.match_diacritics => 'd',
        'Đ' if !opts.match_diacritics => 'D',
        _ => c,
    };
    if opts.match_case {
        out.push(c, lo, hi);
    } else {
        for l in c.to_lowercase() {
            out.push(l, lo, hi);
        }
    }
}

/// Chuẩn hóa chuỗi tìm theo cùng quy tắc, bỏ khoảng trắng ở hai đầu.
pub fn normalize_query(q: &str, opts: SearchOptions) -> String {
    normalize(std::iter::once((0u32, q)), opts).text.trim().to_string()
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || is_combining_mark(c) || c == '_'
}

/// Các vị trí khớp `[a, b)` (theo ký tự chuẩn hóa), không chồng lên nhau.
pub fn find_matches(hay: &NormText, query: &str, opts: SearchOptions) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if query.is_empty() {
        return out;
    }
    let text = hay.text.as_str();
    let mut from = 0;
    while from <= text.len() {
        let Some(off) = text[from..].find(query) else { break };
        let b0 = from + off;
        let b1 = b0 + query.len();
        let a = hay.char_at_byte(b0);
        let b = hay.char_at_byte(b1);
        let before = if a > 0 { hay.char(a - 1) } else { None };
        let after = hay.char(b);
        // Không cắt ngang một cụm chữ + dấu (ví dụ "ba" không khớp phần đầu của "bá").
        let splits_cluster = after.is_some_and(is_combining_mark);
        let word_ok = !opts.whole_word || (!before.is_some_and(is_word_char) && !after.is_some_and(is_word_char));
        if !splits_cluster && word_ok {
            out.push((a, b));
            from = b1;
        } else {
            from = b0 + text[b0..].chars().next().map_or(1, char::len_utf8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str, o: SearchOptions) -> NormText {
        let chars: Vec<String> = s.chars().map(|c| c.to_string()).collect();
        normalize(chars.iter().enumerate().map(|(i, c)| (i as u32, c.as_str())), o)
    }

    const D: SearchOptions = SearchOptions { match_diacritics: false, match_case: false, whole_word: false };

    #[test]
    fn folds_vietnamese() {
        assert_eq!(n("Nghị định số 13/2023/NĐ-CP", D).text, "nghi dinh so 13/2023/nd-cp");
        assert_eq!(n("Đà Nẵng đường", D).text, "da nang duong");
        assert_eq!(normalize_query("  Hợp  Đồng ", D), "hop dong");
    }

    #[test]
    fn whitespace_and_line_breaks_collapse() {
        assert_eq!(n("trách nhiệm\r\nhữu   hạn", D).text, "trach nhiem huu han");
        assert_eq!(n("trách nhi\u{2}\r\nệm", D).text, "trach nhiem");
    }

    #[test]
    fn combining_order_is_canonical() {
        let o = SearchOptions { match_diacritics: true, ..D };
        let a = n("Vi\u{65}\u{302}\u{323}t", o);
        let b = n("Vi\u{65}\u{323}\u{302}t", o);
        let c = n("Việt", o);
        assert_eq!(a.text, b.text);
        assert_eq!(a.text, c.text);
        // Bảng ánh xạ: chữ "ệ" chuẩn hóa trỏ về các ký tự gốc 2..=4.
        assert_eq!(a.source_range(2, 5), (2, 4));
    }

    #[test]
    fn map_points_back_to_source() {
        let t = n("Nghị định", D);
        let m = find_matches(&t, "dinh", D);
        assert_eq!(m, vec![(5, 9)]);
        assert_eq!(t.source_range(5, 9), (5, 8));
    }
}
