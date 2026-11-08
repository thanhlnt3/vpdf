//! Kiểm thử phần đọc tài liệu với PDFium thật và bộ tài liệu tham chiếu
//! (`testdata/reference`, tạo bằng `vpdf-testdata fetch`).
//!
//! Mỗi test ứng với một scenario trong specs/document-viewing/spec.md hoặc một
//! task trong tasks.md (ghi ở chú thích).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use unicode_normalization::UnicodeNormalization;
use vpdf_core::links::LinkTarget;
use vpdf_core::lru::{ByteLru, PAGE_CACHE_BYTES};
use vpdf_core::text::{copy_selection, CopyOutcome};
use vpdf_core::view::PX_PER_PT_AT_100;
use vpdf_render::*;

const WAIT: Duration = Duration::from_secs(60);

fn data(name: &str) -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/reference").join(name);
    assert!(p.is_file(), "thiếu {}: chạy `cargo run -p vpdf-testdata --release -- fetch` trước", p.display());
    p
}

fn notify() -> Arc<dyn Fn() + Send + Sync> {
    Arc::new(|| {})
}

fn open(name: &str, pw: Option<&str>) -> (DocumentWorker, Result<DocInfo, OpenError>) {
    let w = DocumentWorker::open(&data(name), pw.map(String::from), notify());
    let r = wait_opened(&w);
    (w, r)
}

fn wait_opened(w: &DocumentWorker) -> Result<DocInfo, OpenError> {
    let deadline = Instant::now() + WAIT;
    while Instant::now() < deadline {
        if let Some(Event::Opened(r)) = w.recv_timeout(Duration::from_millis(100)) {
            return r;
        }
    }
    panic!("hết thời gian chờ mở file");
}

fn wait_for<T>(w: &DocumentWorker, mut f: impl FnMut(Event) -> Option<T>) -> T {
    let deadline = Instant::now() + WAIT;
    while Instant::now() < deadline {
        if let Some(e) = w.recv_timeout(Duration::from_millis(100)) {
            if let Some(t) = f(e) {
                return t;
            }
        }
    }
    panic!("hết thời gian chờ sự kiện");
}

fn tile_key(page: u32, scale: f32, tx: u32, ty: u32) -> TileKey {
    TileKey { page, scale_milli: (scale * 1000.0).round() as u32, tx, ty }
}

fn dark_pixels(img: &Rgba) -> usize {
    img.pixels.chunks_exact(4).filter(|p| (p[0] as u32 + p[1] as u32 + p[2] as u32) < 200).count()
}

/// task 1.3 + Scenario "Mở bằng hộp thoại": trang 1 hiển thị.
#[test]
fn renders_page_one_of_sample() {
    let (w, r) = open("small-vi.pdf", None);
    let info = r.expect("mở được file mẫu");
    assert_eq!(info.page_sizes.len(), 40);
    assert!(!info.repaired && info.copy_allowed);
    w.set_wanted_tiles(vec![tile_key(0, PX_PER_PT_AT_100, 0, 0)]);
    let img = wait_for(&w, |e| if let Event::Tile(_, img) = e { Some(img) } else { None });
    assert_eq!((img.width, img.height), (512, 512));
    assert!(dark_pixels(&img) > 500, "trang 1 phải có chữ được vẽ");
}

/// Scenario "File hỏng": báo lỗi, các tài liệu khác không bị ảnh hưởng.
#[test]
fn not_a_pdf_reports_error_and_other_docs_keep_working() {
    let good = DocumentWorker::open(&data("small-vi.pdf"), None, notify());
    let (_bad, r) = open("not-a-pdf.pdf", None);
    assert!(matches!(r, Err(OpenError::Unreadable(_))), "{r:?}");
    assert!(wait_opened(&good).is_ok());
    good.set_wanted_tiles(vec![tile_key(1, 1.0, 0, 0)]);
    let img = wait_for(&good, |e| if let Event::Tile(_, img) = e { Some(img) } else { None });
    assert!(dark_pixels(&img) > 100);
}

/// Scenario "File hỏng một phần": hiển thị trang khôi phục được và cảnh báo.
#[test]
fn broken_xref_is_repaired_with_warning() {
    let (w, r) = open("broken-xref.pdf", None);
    let info = r.expect("PDFium phải khôi phục được");
    assert!(info.repaired, "phải báo đã sửa tạm");
    assert_eq!(info.page_sizes.len(), 40);
    w.set_wanted_tiles(vec![tile_key(39, 1.0, 0, 0)]);
    let img = wait_for(&w, |e| if let Event::Tile(_, img) = e { Some(img) } else { None });
    assert!(dark_pixels(&img) > 100);
}

/// Scenario "Nhập đúng mật khẩu" và "Nhập sai mật khẩu" (nhập lại không giới hạn).
#[test]
fn password_prompt_wrong_then_right() {
    let (w, r) = open("password.pdf", None);
    assert_eq!(r.unwrap_err(), OpenError::PasswordRequired);
    let path = data("password.pdf");
    for i in 0..5 {
        w.retry_password(&path, format!("sai-{i}"));
        assert_eq!(wait_opened(&w).unwrap_err(), OpenError::WrongPassword);
        // Không có trang nào được hiển thị khi chưa mở được.
        w.set_wanted_tiles(vec![tile_key(0, 1.0, 0, 0)]);
        assert!(w.recv_timeout(Duration::from_millis(200)).is_none());
    }
    w.retry_password(&path, "matkhau123".into());
    let info = wait_opened(&w).expect("mật khẩu đúng");
    assert_eq!(info.page_sizes.len(), 40);
    assert!(info.encrypted);
    w.set_wanted_tiles(vec![tile_key(0, 1.0, 0, 0)]);
    let img = wait_for(&w, |e| if let Event::Tile(_, img) = e { Some(img) } else { None });
    assert!(dark_pixels(&img) > 100);
}

/// Scenario "Mục lục tiếng Việt" + "Đi tới mục".
#[test]
fn outline_has_vietnamese_titles_and_targets() {
    let (w, r) = open("text-500.pdf", None);
    let info = r.unwrap();
    let outline = wait_for(&w, |e| if let Event::Outline(o) = e { Some(o) } else { None });
    assert_eq!(outline.len(), 20);
    let ch3 = &outline[2];
    assert_eq!(ch3.title, "Chương 3: Quyền và nghĩa vụ của các bên");
    assert!(unicode_normalization::is_nfc(&ch3.title));
    match &ch3.target {
        Some(LinkTarget::Page { index, y_from_top_pt }) => {
            assert_eq!(*index, 50);
            let y = y_from_top_pt.expect("đích có tọa độ");
            assert!((y - 100.0).abs() < 1.0, "y = {y}");
            assert!(*index < info.page_sizes.len());
        }
        t => panic!("{t:?}"),
    }
    assert_eq!(ch3.children.len(), 2);
    assert!(ch3.children[0].title.starts_with("Mục 3.1"));
}

#[test]
fn document_without_outline_has_empty_outline() {
    let (w, r) = open("scan-500.pdf", None);
    r.unwrap();
    let outline = wait_for(&w, |e| if let Event::Outline(o) = e { Some(o) } else { None });
    assert!(outline.is_empty());
}

/// Scenario "Link nội bộ" và "Link URL".
#[test]
fn page_one_links() {
    let (w, r) = open("small-vi.pdf", None);
    r.unwrap();
    w.request_links(0);
    let links = wait_for(&w, |e| if let Event::Links(0, l) = e { Some(l) } else { None });
    assert!(links.iter().any(|l| matches!(l.target, LinkTarget::Page { index: 29, .. })), "{links:?}");
    assert!(links.iter().any(|l| l.target == LinkTarget::Uri("https://example.com".into())), "{links:?}");
}

/// Scenario "Sao chép chữ tiếng Việt".
#[test]
fn copy_vietnamese_line_as_nfc() {
    let (w, r) = open("small-vi.pdf", None);
    let info = r.unwrap();
    w.request_text(0);
    let text = wait_for(&w, |e| if let Event::Text(0, t) = e { Some(t) } else { None });
    let want = "Cộng hòa xã hội chủ nghĩa Việt Nam";
    let all: String = text.all_text().nfc().collect();
    let start_byte = all.find(want).expect("trang 1 phải có dòng tiêu đề");
    // Chọn đúng các ký tự của dòng đó (theo chỉ số ký tự của PDFium).
    let start = all[..start_byte].chars().count();
    let end = start + want.chars().count() - 1;
    let sel = vpdf_core::text::Selection { start: (0, start), end: (0, end) };
    let s = sel.text(|_| Some(text.clone()));
    match copy_selection(&s, info.copy_allowed) {
        CopyOutcome::Copy(c) => {
            assert_eq!(c, want);
            assert!(unicode_normalization::is_nfc(&c));
        }
        o => panic!("{o:?}"),
    }
    // Chọn bằng tọa độ: ký tự gần điểm đầu dòng là chữ "C".
    let c0 = &text.chars[start];
    let hit = text.char_near(c0.rect.left + 1.0, (c0.rect.top + c0.rect.bottom) / 2.0, 2.0).unwrap();
    assert_eq!(hit, start);
}

/// Scenario "Tài liệu cấm sao chép"; mở bằng mật khẩu chủ sở hữu thì được sao chép.
#[test]
fn copy_forbidden_unless_owner_password() {
    let (_w, r) = open("no-copy.pdf", None);
    let info = r.unwrap();
    assert!(!info.copy_allowed);
    assert_eq!(copy_selection("Cộng hòa", info.copy_allowed), CopyOutcome::Denied("Tài liệu này không cho phép sao chép"));
    let (_w2, r2) = open("no-copy.pdf", Some("chusohuu456"));
    assert!(r2.unwrap().copy_allowed);
}

/// Scenario "Chữ nét ở mức phóng cao": ở 800% ô ảnh được dựng đúng độ phân giải hiển thị
/// (không phóng ảnh nhỏ lên), nên cạnh chữ sắc.
#[test]
fn text_is_sharp_at_800_percent() {
    let (w, r) = open("small-vi.pdf", None);
    r.unwrap();
    let scale = 8.0 * PX_PER_PT_AT_100;
    // Dòng tiêu đề ở x = 56 pt, cách mép trên ~ 60 pt.
    let tx = (56.0 * scale / TILE as f32) as u32;
    let ty = (52.0 * scale / TILE as f32) as u32;
    w.set_wanted_tiles(vec![tile_key(0, scale, tx, ty)]);
    let img = wait_for(&w, |e| if let Event::Tile(_, img) = e { Some(img) } else { None });
    assert_eq!(img.width, TILE);
    let lum = |x: u32, y: u32| {
        let i = ((y * img.width + x) * 4) as usize;
        (img.pixels[i] as i32 + img.pixels[i + 1] as i32 + img.pixels[i + 2] as i32) / 3
    };
    let mut max_step = 0;
    for y in 0..img.height {
        for x in 1..img.width {
            max_step = max_step.max((lum(x, y) - lum(x - 1, y)).abs());
        }
    }
    assert!(dark_pixels(&img) > 1000, "ô phải chứa nét chữ");
    // Ảnh 100% phóng 8 lần có cạnh trải ~8 điểm ảnh (bước sáng tối đa ~30); chữ dựng vector thì > 120.
    assert!(max_step > 120, "bước sáng tối đa giữa hai điểm ảnh kề nhau = {max_step}");
}

/// task 2.1: dựng 500 trang mà bộ nhớ đệm không vượt 300 MB và tiến trình dưới 1 GB.
#[test]
fn render_500_pages_within_memory_limits() {
    let (w, r) = open("text-500.pdf", None);
    let info = r.unwrap();
    let mut cache: ByteLru<TileKey, Rgba> = ByteLru::new(PAGE_CACHE_BYTES);
    let mut peak = 0usize;
    let scale = PX_PER_PT_AT_100;
    for (i, size) in info.page_sizes.iter().enumerate() {
        let (gx, gy) = tile_grid(*size, scale);
        let keys: Vec<TileKey> = (0..gy).flat_map(|ty| (0..gx).map(move |tx| tile_key(i as u32, scale, tx, ty))).collect();
        let mut remaining = keys.len();
        w.set_wanted_tiles(keys);
        while remaining > 0 {
            if let Event::Tile(k, img) = w.recv_timeout(WAIT).expect("ô ảnh") {
                let b = img.bytes();
                cache.insert(k, img, b);
                remaining -= 1;
            }
        }
        assert!(cache.used_bytes() <= PAGE_CACHE_BYTES);
        if let Some(m) = memory_stats::memory_stats() {
            peak = peak.max(m.physical_mem);
        }
    }
    eprintln!("bộ nhớ đệm {} MB, tiến trình tối đa {} MB", cache.used_bytes() >> 20, peak >> 20);
    assert!(cache.used_bytes() > 200 * 1024 * 1024, "bộ nhớ đệm phải được dùng tới gần giới hạn");
    assert!(peak < 1024 * 1024 * 1024, "tiến trình dùng {} MB", peak >> 20);
}

/// Scenario "Ảnh thu nhỏ tải dần": ảnh đang nhìn thấy hiện trước, phần còn lại không
/// chặn việc dựng trang đang đọc.
#[test]
fn thumbnails_visible_first_and_do_not_block_reading() {
    let (w, r) = open("text-500.pdf", None);
    r.unwrap();
    let visible: Vec<u32> = (200..206).collect();
    let mut wanted = visible.clone();
    wanted.extend((0..500).filter(|p| !visible.contains(p)));
    w.set_wanted_thumbs(wanted, THUMB_WIDTH);
    let mut first = Vec::new();
    while first.len() < visible.len() {
        if let Some(Event::Thumb(p, img)) = w.recv_timeout(WAIT) {
            assert_eq!(img.width, THUMB_WIDTH);
            first.push(p);
        }
    }
    assert_eq!(first, visible, "ảnh của phần đang nhìn thấy phải đến trước");
    // Người dùng đang đọc trang 300: ô ảnh của trang được ưu tiên hơn ảnh thu nhỏ còn lại.
    let t0 = Instant::now();
    w.set_wanted_tiles(vec![tile_key(299, 1.0, 0, 0)]);
    let mut thumbs_between = 0;
    loop {
        match w.recv_timeout(WAIT).expect("sự kiện") {
            Event::Tile(k, _) => {
                assert_eq!(k.page, 299);
                break;
            }
            Event::Thumb(..) => thumbs_between += 1,
            _ => {}
        }
    }
    assert!(thumbs_between <= 1, "chỉ được chờ tối đa ảnh thu nhỏ đang dựng dở, đã chờ {thumbs_between}");
    assert!(t0.elapsed() < Duration::from_millis(500), "trang đang đọc bị chặn {:?}", t0.elapsed());
    assert!(w.pending().2 > 100, "các ảnh thu nhỏ còn lại vẫn đang chờ chạy nền");
}

/// task 2.7 / performance: mở file 500 trang tới lúc trang 1 hiển thị nét dưới 3 giây.
/// (Bộ nhớ đệm ổ đĩa không được xóa trong test này; đo khởi động nguội thật cần CI.)
#[test]
fn open_500_pages_under_3_seconds() {
    for name in ["text-500.pdf", "scan-500.pdf"] {
        let t0 = Instant::now();
        let w = DocumentWorker::open(&data(name), None, notify());
        let info = wait_opened(&w).unwrap();
        let s = PX_PER_PT_AT_100;
        let (gx, gy) = tile_grid(info.page_sizes[0], s);
        let keys: Vec<TileKey> = (0..gy).flat_map(|ty| (0..gx).map(move |tx| tile_key(0, s, tx, ty))).collect();
        let mut n = keys.len();
        w.set_wanted_tiles(keys);
        while n > 0 {
            if let Event::Tile(..) = w.recv_timeout(WAIT).unwrap() {
                n -= 1;
            }
        }
        let dt = t0.elapsed();
        eprintln!("{name}: mở + trang 1 = {dt:?} (riêng mở: {:?})", info.open_time);
        assert!(dt < Duration::from_secs(3), "{name}: {dt:?}");
    }
}
