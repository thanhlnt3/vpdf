//! Tìm chữ trên PDF thật (PDFium + bộ tài liệu tham chiếu), qua đúng đường chạy của
//! ứng dụng: worker PDFium trích chữ nền → luồng tìm → kết quả theo thứ tự trang.
//! Mỗi test ghi scenario tương ứng trong specs/text-search/spec.md.
//!
//! File `search-vi.pdf` và `scan-mix-20.pdf` được tạo bằng
//! `cargo run -p vpdf-testdata --release -- generate-search` (hoặc `fetch`).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use unicode_normalization::UnicodeNormalization;
use vpdf_core::RectPt;
use vpdf_render::{index_order, DocumentWorker, Event};
use vpdf_search::normalize::normalize_query;
use vpdf_search::{Hit, Output, SearchEngine, SearchOptions, SearchSession};

const WAIT: Duration = Duration::from_secs(120);

fn data(name: &str) -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/reference").join(name);
    assert!(p.is_file(), "thiếu {}: chạy `cargo run -p vpdf-testdata --release -- fetch`", p.display());
    p
}

#[allow(dead_code)] // worker phải sống cùng engine
struct Doc {
    worker: DocumentWorker,
    engine: SearchEngine,
    pages: u32,
}

/// Mở tài liệu và nối worker với luồng tìm như ứng dụng làm (tab.rs).
fn open(name: &str) -> Doc {
    let worker = DocumentWorker::open(&data(name), None, Arc::new(|| {}));
    let deadline = Instant::now() + WAIT;
    let info = loop {
        assert!(Instant::now() < deadline, "hết thời gian mở file");
        if let Some(Event::Opened(r)) = worker.recv_timeout(Duration::from_millis(100)) {
            break r.expect("mở được file mẫu");
        }
    };
    let pages = info.page_sizes.len() as u32;
    let handle = worker.index_handle();
    let engine = SearchEngine::new(pages, Arc::new(move |p| handle.prioritize(p)), Arc::new(|| {}));
    worker.start_indexing(engine.page_sink(), index_order(pages, 0));
    Doc { worker, engine, pages }
}

struct Run {
    hits: Vec<Hit>,
    no_text: usize,
}

fn search(d: &Doc, id: u64, q: &str, opts: SearchOptions) -> Run {
    d.engine.search(id, q, opts);
    let mut hits = Vec::new();
    let deadline = Instant::now() + WAIT;
    loop {
        assert!(Instant::now() < deadline, "hết thời gian tìm {q:?}");
        match d.engine.recv_timeout(Duration::from_millis(100)) {
            Some(Output::Hits { id: i, hits: h }) if i == id => hits.extend(h),
            Some(Output::Done { id: i, total, no_text_pages }) if i == id => {
                assert_eq!(total, hits.len());
                return Run { hits, no_text: no_text_pages };
            }
            _ => {}
        }
    }
}

fn dflt() -> SearchOptions {
    SearchOptions::default()
}

fn matched(r: &Run) -> Vec<String> {
    r.hits.iter().map(|h| h.snippet.matched.clone()).collect()
}

/// Lớp chữ của một trang (để so vùng tô sáng với hộp của từng ký tự gốc).
fn page_text(name: &str, page: i32) -> vpdf_core::text::PageText {
    let pdfium = vpdf_render::pdfium().unwrap();
    let doc = pdfium.load_pdf_from_file(&data(name), None).unwrap();
    let p = doc.pages().get(page).unwrap();
    vpdf_render::extract_text(&p)
}

fn union(rs: &[RectPt]) -> RectPt {
    rs.iter().fold(rs[0], |a, r| RectPt::new(a.left.min(r.left), a.bottom.min(r.bottom), a.right.max(r.right), a.top.max(r.top)))
}

/// Hộp bao của các ký tự gốc tạo thành `want` (NFC) trên trang.
fn source_box(t: &vpdf_core::text::PageText, want: &str) -> RectPt {
    let all: Vec<String> = t.chars.iter().map(|c| c.text.nfc().collect()).collect();
    let target: Vec<char> = want.chars().collect();
    // Tìm dãy ký tự gốc mà ghép lại (NFC) bằng `want`.
    for s in 0..all.len() {
        let mut acc = String::new();
        for e in s..all.len() {
            acc.push_str(&t.chars[e].text);
            let n: String = acc.nfc().collect();
            if n.chars().count() >= target.len() {
                if n == want {
                    let rects: Vec<RectPt> = t.chars[s..=e].iter().map(|c| c.rect).filter(|r| r.width() > 0.0).collect();
                    return union(&rects);
                }
                break;
            }
        }
    }
    panic!("không thấy {want:?} trong lớp chữ");
}

fn close(a: RectPt, b: RectPt) -> bool {
    (a.left - b.left).abs() < 0.6 && (a.right - b.right).abs() < 0.6 && (a.top - b.top).abs() < 0.6 && (a.bottom - b.bottom).abs() < 0.6
}

/// Scenario "Gõ không dấu tìm ra có dấu": "nghi dinh" tìm ra "Nghị định", tô sáng đúng vị trí.
#[test]
fn unaccented_query_finds_and_highlights_accented_text() {
    let d = open("search-vi.pdf");
    let r = search(&d, 1, "nghi dinh", dflt());
    assert_eq!(matched(&r), vec!["Nghị định"]);
    let h = &r.hits[0];
    assert_eq!(h.page, 0);
    assert_eq!(h.rects.len(), 1, "một dòng");
    let want = source_box(&page_text("search-vi.pdf", 0), "Nghị định");
    assert!(close(h.rects[0], want), "vùng tô sáng {:?} phải bao đúng chữ gốc {:?}", h.rects[0], want);
    assert!(h.snippet.before.ends_with("Căn cứ "), "đoạn chữ xung quanh: {:?}", h.snippet);
    assert!(h.snippet.after.starts_with(" số 13/2023/NĐ-CP"));
}

/// Scenario "Chữ đ": "da nang" và "duong" tìm ra "Đà Nẵng" và "đường".
#[test]
fn letter_d_with_stroke_matches_d() {
    let d = open("search-vi.pdf");
    assert_eq!(matched(&search(&d, 1, "da nang", dflt())), vec!["Đà Nẵng"]);
    assert_eq!(matched(&search(&d, 2, "duong", dflt())), vec!["đường"]);
}

/// Scenario "Chuỗi tìm có dấu": "hợp đồng" khớp "hợp đồng", "Hợp Đồng" và "hop dong".
#[test]
fn accented_query_matches_all_forms() {
    let d = open("search-vi.pdf");
    let r = search(&d, 1, "hợp đồng", dflt());
    assert_eq!(matched(&r), vec!["hợp đồng", "Hợp Đồng", "hop dong"]);
}

/// Scenario "Dấu dựng sẵn và dấu tổ hợp": "Việt" lưu dạng e + dấu mũ + dấu nặng vẫn tìm được.
#[test]
fn precomposed_query_finds_combining_text() {
    let t = page_text("search-vi.pdf", 0);
    assert!(t.chars.iter().any(|c| c.text == "\u{302}"), "file mẫu phải lưu dấu ở dạng tổ hợp");
    let d = open("search-vi.pdf");
    let r = search(&d, 1, "Việt", dflt());
    assert_eq!(matched(&r), vec!["Việt"]);
    let want = source_box(&t, "Việt");
    assert!(close(r.hits[0].rects[0], want), "{:?} vs {want:?}", r.hits[0].rects[0]);
    // Cả khi bật Phân biệt dấu (so sánh ở dạng NFD có sắp thứ tự dấu).
    let o = SearchOptions { match_diacritics: true, ..dflt() };
    assert_eq!(matched(&search(&d, 2, "Việt", o)), vec!["Việt"]);
}

/// Scenario "Phân biệt dấu": bật Phân biệt dấu, tìm "bán" thì chỉ "bán" khớp.
#[test]
fn match_diacritics_option() {
    let d = open("search-vi.pdf");
    // Mặc định bỏ dấu: khớp cả "ban", "bán", "bàn" (dòng mẫu) và "bản" ("văn bản"), "ban" trong "bang".
    let loose = matched(&search(&d, 1, "bán", dflt()));
    assert_eq!(&loose[..3], &["ban", "bán", "bàn"], "mặc định bỏ dấu");
    assert!(loose.iter().any(|m| m == "bản"));
    let o = SearchOptions { match_diacritics: true, ..dflt() };
    assert_eq!(matched(&search(&d, 2, "bán", o)), vec!["bán"]);
    let ban = matched(&search(&d, 3, "ban", o));
    assert!(!ban.is_empty() && ban.iter().all(|m| m == "ban"), "{ban:?}");
    // Phân biệt dấu giữ chữ đ: "dong" không khớp "đồng".
    assert_eq!(matched(&search(&d, 4, "hop dong", o)), vec!["hop dong"]);
}

/// Tùy chọn Phân biệt hoa thường.
#[test]
fn match_case_option() {
    let d = open("search-vi.pdf");
    let o = SearchOptions { match_case: true, ..dflt() };
    assert_eq!(matched(&search(&d, 1, "Hợp Đồng", o)), vec!["Hợp Đồng"]);
    assert_eq!(matched(&search(&d, 2, "hop dong", o)), vec!["hợp đồng", "hop dong"]);
}

/// Scenario "Khớp nguyên từ": tìm "an" thì "bang" không khớp.
#[test]
fn whole_word_option() {
    let d = open("search-vi.pdf");
    let inside_bang = |r: &Run| r.hits.iter().any(|h| h.snippet.before.ends_with('b') && h.snippet.after.starts_with('g'));
    let loose = search(&d, 1, "an", dflt());
    assert!(inside_bang(&loose), "không bật Khớp nguyên từ thì \"bang\" có khớp");
    let o = SearchOptions { whole_word: true, ..dflt() };
    let r = search(&d, 2, "an", o);
    assert!(!r.hits.is_empty());
    assert!(!inside_bang(&r), "\"bang\" không khớp: {:?}", r.hits.iter().map(|h| &h.snippet).collect::<Vec<_>>());
    assert!(r.hits.iter().all(|h| normalize_query(&h.snippet.matched, dflt()) == "an"));
}

/// Scenario "Cụm bị xuống dòng" (+ nhiều khoảng trắng liên tiếp): tìm thấy và tô sáng trên cả hai dòng.
#[test]
fn phrase_across_line_break() {
    let d = open("search-vi.pdf");
    let r = search(&d, 1, "trach nhiem huu han", dflt());
    assert_eq!(r.hits.len(), 2, "{:?}", matched(&r));
    let broken = &r.hits[0];
    assert_eq!(broken.snippet.matched, "trách nhiệm hữu hạn");
    assert_eq!(broken.rects.len(), 2, "tô sáng trên hai dòng");
    let mid = |r: &RectPt| (r.top + r.bottom) / 2.0;
    assert!(mid(&broken.rects[0]) - mid(&broken.rects[1]) > 10.0, "dòng sau nằm dưới dòng trước: {:?}", broken.rects);
    let t = page_text("search-vi.pdf", 0);
    assert!(close(broken.rects[0], source_box(&t, "trách nhiệm")));
    assert!(close(broken.rects[1], source_box(&t, "hữu hạn")));
    assert_eq!(r.hits[1].rects.len(), 1, "cụm có nhiều khoảng trắng trên một dòng");
}

/// Scenario "Không có kết quả": quét hết tài liệu rồi báo "Không tìm thấy".
#[test]
fn no_results_after_full_scan() {
    let d = open("search-vi.pdf");
    let r = search(&d, 1, "chuỗi không hề có", dflt());
    assert!(r.hits.is_empty());
    let mut s = SearchSession::default();
    s.set_input("chuỗi không hề có", Instant::now());
    s.submit_now(Instant::now());
    s.apply(Output::Done { id: s.id, total: 0, no_text_pages: r.no_text }, Instant::now());
    assert_eq!(s.counter(), "Không tìm thấy");
}

/// Scenario "PDF scan": 20 trang, 15 trang chỉ có ảnh → "15 trang không có chữ để tìm. Chạy OCR?".
#[test]
fn scanned_pages_are_reported() {
    let d = open("scan-mix-20.pdf");
    assert_eq!(d.pages, 20);
    let r = search(&d, 1, "chuỗi bất kỳ", dflt());
    assert_eq!(r.no_text, 15);
    let r2 = search(&d, 2, "hợp đồng", dflt());
    assert_eq!(r2.no_text, 15);
    assert_eq!(r2.hits.len(), 5, "vẫn tìm trên 5 trang có chữ");
    let mut s = SearchSession::default();
    s.set_input("chuỗi bất kỳ", Instant::now());
    s.submit_now(Instant::now());
    s.apply(Output::Done { id: s.id, total: 0, no_text_pages: r.no_text }, Instant::now());
    assert_eq!(s.no_text_message().as_deref(), Some("15 trang không có chữ để tìm. Chạy OCR?"));
    assert_ne!(s.counter(), "Không tìm thấy");
}

/// Scenario "Duyệt kết quả" (dữ liệu): "điều khoản" có đúng 12 kết quả, theo thứ tự trang.
#[test]
fn twelve_results_in_page_order() {
    let d = open("search-vi.pdf");
    let r = search(&d, 1, "điều khoản", dflt());
    assert_eq!(r.hits.len(), 12);
    let pages: Vec<u32> = r.hits.iter().map(|h| h.page).collect();
    assert!(pages.windows(2).all(|w| w[0] <= w[1]), "theo thứ tự trang: {pages:?}");
    assert_eq!(pages[0], 1);
}

/// Scenario "Gõ tiếp hủy lượt cũ": lượt "hợp" chưa xong thì bị thay bằng "hợp đồng";
/// sau khi luồng tìm nhận lượt mới, không còn kết quả nào của lượt cũ.
#[test]
fn new_query_cancels_running_one() {
    let d = open("text-500.pdf");
    d.engine.search(1, "hợp", dflt());
    // Đợi lượt "hợp" có kết quả nhưng chưa quét hết 500 trang.
    let deadline = Instant::now() + WAIT;
    loop {
        assert!(Instant::now() < deadline);
        match d.engine.recv_timeout(Duration::from_millis(50)) {
            Some(Output::Hits { id: 1, .. }) => break,
            Some(Output::Done { id: 1, .. }) => panic!("lượt cũ không được xong trước khi gõ tiếp"),
            _ => {}
        }
    }
    d.engine.search(2, "hợp đồng", dflt());
    let mut seen_new = false;
    let mut new_hits = Vec::new();
    loop {
        assert!(Instant::now() < deadline);
        match d.engine.recv_timeout(Duration::from_millis(100)) {
            Some(Output::Hits { id: 2, hits }) => {
                seen_new = true;
                new_hits.extend(hits);
            }
            Some(Output::Progress { id: 2, .. }) => seen_new = true,
            Some(Output::Done { id: 2, .. }) => break,
            Some(Output::Hits { id: 1, .. }) | Some(Output::Progress { id: 1, .. }) | Some(Output::Done { id: 1, .. }) => {
                assert!(!seen_new, "kết quả của lượt cũ đến sau lượt mới");
            }
            _ => {}
        }
    }
    assert!(!new_hits.is_empty());
    assert!(new_hits.iter().all(|h| normalize_query(&h.snippet.matched, dflt()) == "hop dong"));
}

/// Scenario "Kết quả đầu tiên hiện nhanh" (phần chạy nền): mở file 500 trang rồi tìm
/// ngay "hợp đồng" — kết quả đầu tiên có trong < 350 ms (giao diện còn chờ 150 ms sau lần gõ cuối).
#[test]
fn first_result_fast_on_500_pages() {
    let d = open("text-500.pdf");
    let t0 = Instant::now();
    d.engine.search(1, "hợp đồng", dflt());
    loop {
        match d.engine.recv_timeout(Duration::from_secs(5)) {
            Some(Output::Hits { id: 1, hits }) => {
                assert!(!hits.is_empty());
                break;
            }
            Some(_) => {}
            None => panic!("không có kết quả"),
        }
    }
    let dt = t0.elapsed();
    eprintln!("kết quả đầu tiên sau {} ms", dt.as_millis());
    assert!(dt < Duration::from_millis(350), "{dt:?}");
}

/// Task 3.1: trích chữ chạy nền, ưu tiên trang gần trang đang đọc; khi có lượt tìm
/// đang chờ thì trích theo thứ tự trang từ trang lượt tìm cần.
#[test]
fn background_extraction_priority() {
    assert_eq!(index_order(7, 3), vec![3, 4, 2, 5, 1, 6, 0]);
    assert_eq!(index_order(3, 0), vec![0, 1, 2]);
    let worker = DocumentWorker::open(&data("text-500.pdf"), None, Arc::new(|| {}));
    let deadline = Instant::now() + WAIT;
    while !matches!(worker.recv_timeout(Duration::from_millis(100)), Some(Event::Opened(Ok(_)))) {
        assert!(Instant::now() < deadline);
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = worker.index_handle();
    worker.start_indexing(Box::new(move |p, _t| { let _ = tx.send(p); }), index_order(500, 250));
    let first: Vec<u32> = (0..3).map(|_| rx.recv_timeout(WAIT).unwrap()).collect();
    assert_eq!(first, vec![250, 251, 249], "trang đang đọc và các trang gần nó trước");
    handle.prioritize(Some(10));
    // Có thể còn một trang đang trích dở theo thứ tự cũ.
    let next: Vec<u32> = (0..6).map(|_| rx.recv_timeout(WAIT).unwrap()).collect();
    let tail: Vec<u32> = next.iter().copied().skip_while(|p| *p >= 240).collect();
    assert_eq!(&tail[..4], &[10, 11, 12, 13], "lượt tìm cần trang 10: trích theo thứ tự trang từ 10 ({next:?})");
    assert!(handle.pending() < 500);
}
