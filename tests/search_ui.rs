//! Kiểm thử giao diện của chức năng tìm chữ (nhóm task 3): gõ vào ô tìm như người dùng
//! (Ctrl+F, từng ký tự), nhấn Enter / F3, bấm danh sách kết quả, rồi đo trạng thái và
//! những gì được vẽ. Mỗi test ghi scenario tương ứng trong specs/text-search/spec.md.

mod common;
use common::*;
use egui::{Event, Key, Modifiers, Rect};
use std::time::{Duration, Instant};
use vpdf_app::app::SideTab;
use vpdf_search::normalize::normalize_query;
use vpdf_search::SearchOptions;

impl Harness {
    /// Ctrl+F rồi gõ từng ký tự. Trả về thời điểm gõ ký tự cuối.
    fn type_search(&mut self, text: &str) -> Instant {
        self.key(Key::F, Modifiers::COMMAND);
        assert!(self.ctx.memory(|m| m.has_focus(self.app.search_box_id())), "Ctrl+F đưa con trỏ vào ô tìm");
        self.key(Key::A, Modifiers::COMMAND);
        let mut last = Instant::now();
        for c in text.chars() {
            self.events.push(Event::Text(c.to_string()));
            last = Instant::now();
            self.frame();
        }
        last
    }

    fn search_done(&mut self) {
        self.until("tìm xong", |a| a.active().unwrap().search.done);
        self.frames(2);
    }

    fn counter(&self) -> &str {
        &self.app.hud.search_counter
    }

    /// Kết quả đang chọn được vẽ (tô đậm) và nằm trọn trong vùng xem.
    fn current_hit_visible(&self) -> bool {
        let (Some(view), rects) = (self.app.hud.viewer_rect, &self.app.hud.current_hit_rects) else { return false };
        !rects.is_empty() && rects.iter().all(|r: &Rect| view.contains_rect(*r))
    }
}

/// Scenario "Kết quả đầu tiên hiện nhanh": PDF 500 trang, gõ "hợp đồng" — kết quả đầu
/// tiên được tô sáng và cuộn tới trong vòng 500 ms sau lần gõ cuối.
#[test]
fn first_result_highlighted_within_500ms() {
    let mut h = Harness::new(None);
    h.open(&data("text-500.pdf"));
    let last_key = h.type_search("hợp đồng");
    assert!(h.tab().search.current.is_none(), "chưa tìm khi chưa hết 150 ms");
    h.until("kết quả đầu tiên được tô sáng", |a| a.active().unwrap().search.current.is_some());
    h.frame();
    let dt = last_key.elapsed();
    assert!(h.current_hit_visible(), "kết quả đầu tiên được cuộn tới và tô đậm");
    eprintln!("kết quả đầu tiên hiện sau {} ms kể từ lần gõ cuối", dt.as_millis());
    assert!(dt < Duration::from_millis(500), "{dt:?}");
    let first = h.tab().search.hits[0].clone();
    assert_eq!(normalize_query(&first.snippet.matched, SearchOptions::default()), "hop dong");
    assert_eq!(h.app.side, SideTab::Search, "bảng Kết quả tìm được mở");
}

/// Scenario "Bộ đếm cập nhật dần": chưa quét xong thì bộ đếm có dấu "…".
#[test]
fn counter_updates_progressively() {
    let mut h = Harness::new(None);
    h.open(&data("text-500.pdf"));
    h.type_search("hợp đồng");
    h.until("có kết quả", |a| !a.active().unwrap().search.hits.is_empty());
    h.frame();
    let s = &h.tab().search;
    assert!(!s.done, "500 trang chưa quét xong ngay");
    let c = h.counter().to_string();
    assert!(c.starts_with("1 / ") && c.ends_with('…'), "bộ đếm khi đang tìm: {c:?}");
    h.search_done();
    let n = h.tab().search.hits.len();
    assert_eq!(h.counter(), format!("1 / {n}"));
}

/// Scenario "Gõ tiếp hủy lượt cũ": gõ "hợp", rồi gõ tiếp thành "hợp đồng" trước khi
/// lượt "hợp" xong — danh sách chỉ có kết quả của "hợp đồng".
#[test]
fn typing_more_replaces_running_search() {
    let mut h = Harness::new(None);
    h.open(&data("text-500.pdf"));
    h.type_search("hợp");
    h.until("lượt \"hợp\" bắt đầu", |a| !a.active().unwrap().search.hits.is_empty());
    let old_id = h.tab().search.id;
    assert!(!h.tab().search.done, "lượt \"hợp\" chưa xong");
    for c in " đồng".chars() {
        h.events.push(Event::Text(c.to_string()));
        h.frame();
    }
    h.search_done();
    let s = &h.tab().search;
    assert_ne!(s.id, old_id);
    assert!(!s.hits.is_empty());
    for hit in &s.hits {
        assert_eq!(normalize_query(&hit.snippet.matched, SearchOptions::default()), "hop dong", "{:?}", hit.snippet);
    }
}

/// Scenario "Không có kết quả": ô tìm hiện "Không tìm thấy" sau khi quét hết.
#[test]
fn not_found_message() {
    let mut h = Harness::new(None);
    h.open(&data("search-vi.pdf"));
    h.type_search("không hề có chuỗi này");
    h.search_done();
    assert_eq!(h.counter(), "Không tìm thấy");
    assert_eq!(h.app.hud.search_message, None);
}

/// Scenario "Duyệt kết quả": "điều khoản" có 12 kết quả; Enter 3 lần → kết quả thứ 4
/// được chọn và cuộn vào vùng nhìn thấy, bộ đếm "4 / 12". Shift+Enter, F3, Shift+F3.
#[test]
fn enter_moves_through_results() {
    let mut h = Harness::new(None);
    h.open(&data("search-vi.pdf"));
    h.type_search("điều khoản");
    h.search_done();
    assert_eq!(h.tab().search.hits.len(), 12);
    assert_eq!(h.counter(), "1 / 12");
    for _ in 0..3 {
        h.key(Key::Enter, Modifiers::NONE);
    }
    assert_eq!(h.tab().search.current, Some(3));
    assert_eq!(h.counter(), "4 / 12");
    assert!(h.current_hit_visible(), "kết quả thứ 4 nằm trong vùng nhìn thấy");
    let p = h.tab().search.hits[3].page as usize;
    assert!(h.tab().view.visible_pages().iter().any(|v| v.index == p), "trang của kết quả thứ 4 đang hiển thị");
    assert!(h.ctx.memory(|m| m.has_focus(h.app.search_box_id())), "ô tìm vẫn giữ con trỏ để nhấn Enter tiếp");
    h.key(Key::Enter, Modifiers::SHIFT);
    assert_eq!(h.counter(), "3 / 12");
    h.key(Key::F3, Modifiers::NONE);
    h.key(Key::F3, Modifiers::NONE);
    assert_eq!(h.counter(), "5 / 12");
    h.key(Key::F3, Modifiers::SHIFT);
    assert_eq!(h.counter(), "4 / 12");
    assert!(h.current_hit_visible());
    // Mọi kết quả trên trang đang hiển thị đều được tô sáng (không chỉ kết quả đang chọn).
    assert!(h.app.hud.highlights_drawn >= 2, "{} kết quả được tô", h.app.hud.highlights_drawn);
}

/// Scenario "Bấm vào danh sách kết quả": bấm dòng ở trang 210 → vùng xem chuyển tới
/// trang 210 và tô đậm kết quả đó.
#[test]
fn clicking_result_row_jumps_to_page_210() {
    let mut h = Harness::new(None);
    h.open(&data("text-500.pdf"));
    // Chân trang "Trang 21 / 500", "Trang 210 / 500"… "Trang 219 / 500".
    h.type_search("Trang 21");
    h.until("kết quả ở trang 210", |a| a.active().unwrap().search.hits.iter().any(|x| x.page == 209));
    h.frames(2);
    let idx = h.tab().search.hits.iter().position(|x| x.page == 209).unwrap();
    let row = h.app.hud.search_rows.iter().find(|(i, _)| *i == idx).map(|r| r.1).expect("dòng của trang 210 đang hiện trong danh sách");
    assert_eq!(h.tab().view.current_page(), 20, "đang ở kết quả đầu tiên (trang 21)");
    h.click(row.center());
    assert_eq!(h.tab().search.current, Some(idx));
    assert_eq!(h.tab().view.current_page(), 209);
    assert_eq!(h.app.hud.page_box, "210 / 500");
    assert!(h.current_hit_visible(), "kết quả ở trang 210 được tô đậm");
    assert!(h.tab().history.can_go_back(), "Quay lại về chỗ cũ được");
}

/// Scenario "Cụm bị xuống dòng" trong giao diện: tô sáng trên cả hai dòng.
#[test]
fn line_broken_phrase_highlighted_on_two_lines() {
    let mut h = Harness::new(None);
    h.open(&data("search-vi.pdf"));
    h.type_search("trach nhiem huu han");
    h.search_done();
    assert_eq!(h.tab().search.hits.len(), 2);
    let rects = h.app.hud.current_hit_rects.clone();
    assert_eq!(rects.len(), 2, "hai vùng tô: {rects:?}");
    assert!(rects[1].center().y > rects[0].center().y + 10.0, "dòng sau ở dưới");
}

/// Tùy chọn trong giao diện: bật Phân biệt dấu thì tìm lại ngay, chỉ "bán" khớp.
#[test]
fn toggling_option_reruns_search() {
    let mut h = Harness::new(None);
    h.open(&data("search-vi.pdf"));
    h.type_search("bán");
    h.search_done();
    assert!(h.tab().search.hits.len() > 3);
    // Ô đánh dấu "Phân biệt dấu" trong menu Tùy chọn tìm sửa đúng trường này.
    h.app.active_mut().unwrap().search.opts.match_diacritics = true;
    h.frames(2);
    h.search_done();
    let m: Vec<String> = h.tab().search.hits.iter().map(|x| x.snippet.matched.clone()).collect();
    assert_eq!(m, vec!["bán"]);
    assert_eq!(h.counter(), "1 / 1");
}

/// Scenario "PDF scan": 20 trang, 15 trang chỉ có ảnh → báo "15 trang không có chữ để tìm. Chạy OCR?".
#[test]
fn scanned_pages_message_in_ui() {
    let mut h = Harness::new(None);
    h.open(&data("scan-mix-20.pdf"));
    h.type_search("chuỗi bất kỳ");
    h.search_done();
    assert_eq!(h.app.hud.search_message.as_deref(), Some("15 trang không có chữ để tìm. Chạy OCR?"));
    assert_ne!(h.counter(), "Không tìm thấy", "không báo như thể đã tìm đủ");
}

/// Mỗi tab có ô tìm riêng.
#[test]
fn search_state_is_per_tab() {
    let mut h = Harness::new(None);
    h.open(&data("search-vi.pdf"));
    h.type_search("điều khoản");
    h.search_done();
    h.open(&data("small-vi.pdf"));
    h.frames(2);
    assert_eq!(h.tab().search.input, "");
    assert_eq!(h.counter(), "");
    h.app.tabs.select(0);
    h.frames(2);
    assert_eq!(h.counter(), "1 / 12");
}
