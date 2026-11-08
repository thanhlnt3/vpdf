//! Trạng thái ô tìm của một tab (không phụ thuộc giao diện, nên kiểm thử được):
//! chờ 150 ms sau lần gõ cuối, lượt tìm mới thay lượt cũ, danh sách kết quả theo thứ
//! tự trang, kết quả đang chọn, bộ đếm và thông báo trang không có chữ.

use crate::engine::Output;
use crate::normalize::{normalize_query, SearchOptions};
use crate::page::Hit;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use vpdf_core::messages as m;

/// Thời gian chờ sau lần gõ cuối trước khi tìm (specs/text-search › Tìm khi đang gõ).
pub const DEBOUNCE: Duration = Duration::from_millis(150);

/// Việc cần gửi cho luồng tìm.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Search { id: u64, query: String, opts: SearchOptions },
    Cancel,
}

#[derive(Debug, Default)]
pub struct SearchSession {
    /// Chữ trong ô tìm.
    pub input: String,
    pub opts: SearchOptions,
    edited_at: Option<Instant>,
    submitted: Option<(String, SearchOptions)>,
    next_id: u64,
    /// Mã lượt tìm hiện tại (0 = không có).
    pub id: u64,
    pub hits: Vec<Hit>,
    /// Trang → khoảng chỉ số trong `hits`.
    by_page: BTreeMap<u32, (usize, usize)>,
    pub current: Option<usize>,
    pub done: bool,
    pub scanned: u32,
    pub no_text_pages: usize,
    /// Lúc gửi lượt tìm hiện tại, và thời gian tới khi có kết quả đầu tiên.
    pub submitted_at: Option<Instant>,
    pub first_hit_after: Option<Duration>,
}

impl SearchSession {
    /// Người dùng sửa ô tìm.
    pub fn set_input(&mut self, text: &str, now: Instant) {
        if text != self.input {
            self.input = text.to_string();
            self.edited_at = Some(now);
        }
    }

    /// Có lượt tìm (đã gửi, chưa xóa) hay không.
    pub fn is_active(&self) -> bool {
        self.id != 0
    }

    fn wanted(&self) -> (String, SearchOptions) {
        (self.input.clone(), self.opts)
    }

    /// Gọi mỗi khung hình: đủ 150 ms sau lần gõ cuối (hoặc tùy chọn vừa đổi) thì gửi lượt mới.
    pub fn poll(&mut self, now: Instant) -> Option<Action> {
        if self.submitted.as_ref() == Some(&self.wanted()) {
            return None;
        }
        if self.submitted.is_none() && self.input.trim().is_empty() {
            return None;
        }
        if let Some(t) = self.edited_at {
            if now.duration_since(t) < DEBOUNCE {
                return None;
            }
        }
        Some(self.start(now))
    }

    /// Còn bao lâu nữa thì tới hạn gửi (để giao diện hẹn vẽ lại).
    pub fn due_in(&self, now: Instant) -> Option<Duration> {
        if self.submitted.as_ref() == Some(&self.wanted()) {
            return None;
        }
        let t = self.edited_at?;
        Some(DEBOUNCE.saturating_sub(now.duration_since(t)))
    }

    /// Gửi ngay, không chờ (Enter khi chưa hết 150 ms).
    pub fn submit_now(&mut self, now: Instant) -> Option<Action> {
        if self.submitted.as_ref() == Some(&self.wanted()) {
            return None;
        }
        Some(self.start(now))
    }

    fn start(&mut self, now: Instant) -> Action {
        self.submitted = Some(self.wanted());
        self.hits.clear();
        self.by_page.clear();
        self.current = None;
        self.done = false;
        self.scanned = 0;
        self.no_text_pages = 0;
        self.first_hit_after = None;
        if normalize_query(&self.input, self.opts).is_empty() {
            self.id = 0;
            self.submitted_at = None;
            return Action::Cancel;
        }
        self.next_id += 1;
        self.id = self.next_id;
        self.submitted_at = Some(now);
        Action::Search { id: self.id, query: self.input.clone(), opts: self.opts }
    }

    /// Nhận kết quả từ luồng tìm. Trả về true nếu kết quả đang chọn vừa đổi (kết quả
    /// đầu tiên được chọn tự động), để giao diện cuộn tới.
    pub fn apply(&mut self, out: Output, now: Instant) -> bool {
        match out {
            Output::Hits { id, hits } if id == self.id => {
                let Some(p) = hits.first().map(|h| h.page) else { return false };
                let start = self.hits.len();
                self.hits.extend(hits);
                self.by_page.insert(p, (start, self.hits.len()));
                self.scanned = self.scanned.max(p + 1);
                if self.current.is_none() {
                    self.current = Some(0);
                    self.first_hit_after = self.submitted_at.map(|t| now.duration_since(t));
                    return true;
                }
                false
            }
            Output::Progress { id, scanned } if id == self.id => {
                self.scanned = self.scanned.max(scanned);
                false
            }
            Output::Done { id, no_text_pages, .. } if id == self.id => {
                self.done = true;
                self.no_text_pages = no_text_pages;
                false
            }
            // Kết quả của lượt cũ (đã bị thay) bị bỏ qua.
            _ => false,
        }
    }

    pub fn current_hit(&self) -> Option<&Hit> {
        self.current.and_then(|i| self.hits.get(i))
    }

    /// Kết quả sau (Enter / F3). Ở kết quả cuối: quay lại kết quả đầu nếu đã quét xong.
    pub fn select_next(&mut self) -> Option<&Hit> {
        if self.hits.is_empty() {
            return None;
        }
        let n = self.hits.len();
        self.current = Some(match self.current {
            None => 0,
            Some(i) if i + 1 < n => i + 1,
            Some(i) => {
                if self.done {
                    0
                } else {
                    i
                }
            }
        });
        self.current_hit()
    }

    /// Kết quả trước (Shift+Enter / Shift+F3).
    pub fn select_prev(&mut self) -> Option<&Hit> {
        if self.hits.is_empty() {
            return None;
        }
        let n = self.hits.len();
        self.current = Some(match self.current {
            None | Some(0) => {
                if self.done {
                    n - 1
                } else {
                    0
                }
            }
            Some(i) => i - 1,
        });
        self.current_hit()
    }

    pub fn select(&mut self, index: usize) -> Option<&Hit> {
        if index < self.hits.len() {
            self.current = Some(index);
        }
        self.current_hit()
    }

    /// Kết quả trên trang `page`: (chỉ số của kết quả đầu tiên, các kết quả).
    pub fn hits_on_page(&self, page: u32) -> (usize, &[Hit]) {
        match self.by_page.get(&page) {
            Some(&(a, b)) => (a, &self.hits[a..b]),
            None => (0, &[]),
        }
    }

    /// Bộ đếm cạnh ô tìm: "4 / 12", "4 / 37…" khi chưa quét xong, "Không tìm thấy".
    pub fn counter(&self) -> String {
        if !self.is_active() {
            return String::new();
        }
        let cur = self.current.map_or(0, |i| i + 1);
        let total = self.hits.len();
        if self.done {
            if total == 0 {
                if self.no_text_pages > 0 {
                    // Không báo "Không tìm thấy" như thể đã tìm đủ (specs/text-search › Trang không có lớp chữ).
                    "0 / 0".to_string()
                } else {
                    m::NOT_FOUND.to_string()
                }
            } else {
                format!("{cur} / {total}")
            }
        } else if total == 0 {
            m::SEARCHING.to_string()
        } else {
            format!("{cur} / {total}…")
        }
    }

    /// "15 trang không có chữ để tìm. Chạy OCR?" khi đã quét xong và có trang chỉ là ảnh.
    pub fn no_text_message(&self) -> Option<String> {
        (self.is_active() && self.done && self.no_text_pages > 0).then(|| m::no_text_pages(self.no_text_pages))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::Snippet;

    fn hit(page: u32, i: u32) -> Hit {
        Hit { page, first: i, last: i + 3, rects: vec![], snippet: Snippet::default() }
    }

    fn started(s: &mut SearchSession, q: &str, t: Instant) -> u64 {
        s.set_input(q, t);
        match s.poll(t + DEBOUNCE) {
            Some(Action::Search { id, .. }) => id,
            other => panic!("{other:?}"),
        }
    }

    /// Requirement "Tìm khi đang gõ": tìm sau khi ngừng gõ 150 ms, không cần Enter.
    #[test]
    fn waits_150ms_after_last_keystroke() {
        let t = Instant::now();
        let mut s = SearchSession::default();
        s.set_input("h", t);
        s.set_input("hợ", t + Duration::from_millis(80));
        assert_eq!(s.poll(t + Duration::from_millis(200)), None, "chỉ 120 ms sau lần gõ cuối");
        assert_eq!(s.due_in(t + Duration::from_millis(200)), Some(Duration::from_millis(30)));
        let a = s.poll(t + Duration::from_millis(230));
        assert!(matches!(a, Some(Action::Search { ref query, .. }) if query == "hợ"), "{a:?}");
        assert_eq!(s.poll(t + Duration::from_millis(400)), None, "không gửi lại cùng một lượt");
    }

    /// Scenario "Gõ tiếp hủy lượt cũ": kết quả của "hợp" đến muộn bị bỏ qua.
    #[test]
    fn newer_query_replaces_older_results() {
        let t = Instant::now();
        let mut s = SearchSession::default();
        let old = started(&mut s, "hợp", t);
        s.apply(Output::Hits { id: old, hits: vec![hit(0, 1)] }, t);
        let new = started(&mut s, "hợp đồng", t + Duration::from_millis(300));
        assert_ne!(old, new);
        assert!(s.hits.is_empty(), "danh sách được làm mới");
        s.apply(Output::Hits { id: old, hits: vec![hit(1, 5)] }, t);
        s.apply(Output::Done { id: old, total: 2, no_text_pages: 0 }, t);
        assert!(s.hits.is_empty() && !s.done, "kết quả của lượt cũ không được nhận");
        s.apply(Output::Hits { id: new, hits: vec![hit(2, 7)] }, t);
        assert_eq!(s.hits, vec![hit(2, 7)]);
    }

    /// Scenario "Duyệt kết quả": 12 kết quả, Enter 3 lần → kết quả thứ 4, "4 / 12".
    #[test]
    fn enter_three_times_selects_fourth_of_twelve() {
        let t = Instant::now();
        let mut s = SearchSession::default();
        let id = started(&mut s, "điều khoản", t);
        for p in 0..6 {
            let changed = s.apply(Output::Hits { id, hits: vec![hit(p, 1), hit(p, 20)] }, t);
            assert_eq!(changed, p == 0, "kết quả đầu tiên được chọn tự động");
        }
        s.apply(Output::Done { id, total: 12, no_text_pages: 0 }, t);
        for _ in 0..3 {
            s.select_next();
        }
        assert_eq!(s.current, Some(3));
        assert_eq!(s.counter(), "4 / 12");
        s.select_prev();
        assert_eq!(s.counter(), "3 / 12");
        // Quay vòng khi đã quét xong.
        s.select(11);
        s.select_next();
        assert_eq!(s.counter(), "1 / 12");
        s.select_prev();
        assert_eq!(s.counter(), "12 / 12");
        assert_eq!(s.hits_on_page(3).0, 6);
        assert_eq!(s.hits_on_page(3).1.len(), 2);
    }

    /// Scenario "Bộ đếm cập nhật dần": chưa quét xong thì bộ đếm có dấu "…".
    #[test]
    fn counter_shows_progress_marker_until_done() {
        let t = Instant::now();
        let mut s = SearchSession::default();
        let id = started(&mut s, "hợp đồng", t);
        assert_eq!(s.counter(), "Đang tìm…");
        s.apply(Output::Hits { id, hits: (0..37).map(|i| hit(i / 4, i)).collect::<Vec<_>>()[..4].to_vec() }, t);
        s.apply(Output::Hits { id, hits: (4..37).map(|i| hit(10, i)).collect() }, t);
        for _ in 0..3 {
            s.select_next();
        }
        assert_eq!(s.counter(), "4 / 37…");
        s.apply(Output::Done { id, total: 37, no_text_pages: 0 }, t);
        assert_eq!(s.counter(), "4 / 37");
    }

    /// Scenario "Không có kết quả": "Không tìm thấy" chỉ sau khi quét hết.
    #[test]
    fn not_found_only_after_full_scan() {
        let t = Instant::now();
        let mut s = SearchSession::default();
        let id = started(&mut s, "xyz", t);
        s.apply(Output::Progress { id, scanned: 250 }, t);
        assert_eq!(s.counter(), "Đang tìm…");
        s.apply(Output::Done { id, total: 0, no_text_pages: 0 }, t);
        assert_eq!(s.counter(), "Không tìm thấy");
        assert_eq!(s.no_text_message(), None);
    }

    /// Scenario "PDF scan": báo số trang không có chữ thay vì "Không tìm thấy".
    #[test]
    fn reports_pages_without_text_layer() {
        let t = Instant::now();
        let mut s = SearchSession::default();
        let id = started(&mut s, "bất kỳ", t);
        s.apply(Output::Done { id, total: 0, no_text_pages: 15 }, t);
        assert_ne!(s.counter(), "Không tìm thấy");
        assert_eq!(s.no_text_message().as_deref(), Some("15 trang không có chữ để tìm. Chạy OCR?"));
    }

    /// Đổi tùy chọn thì tìm lại ngay; xóa ô tìm thì hủy.
    #[test]
    fn option_change_reruns_and_empty_query_cancels() {
        let t = Instant::now();
        let mut s = SearchSession::default();
        let id = started(&mut s, "bán", t);
        s.opts.match_diacritics = true;
        match s.poll(t + Duration::from_secs(1)) {
            Some(Action::Search { id: id2, opts, .. }) => assert!(id2 != id && opts.match_diacritics),
            other => panic!("{other:?}"),
        }
        s.set_input("  ", t + Duration::from_secs(2));
        assert_eq!(s.poll(t + Duration::from_secs(3)), Some(Action::Cancel));
        assert!(!s.is_active());
        assert_eq!(s.counter(), "");
    }

    /// Enter khi chưa hết 150 ms: tìm ngay.
    #[test]
    fn enter_submits_immediately() {
        let t = Instant::now();
        let mut s = SearchSession::default();
        s.set_input("hợp đồng", t);
        assert!(matches!(s.submit_now(t), Some(Action::Search { .. })));
        assert_eq!(s.poll(t + DEBOUNCE), None);
    }
}
