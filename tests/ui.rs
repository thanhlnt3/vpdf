//! Kiểm thử giao diện (task 2.2–2.6): chạy giao diện egui thật, không cửa sổ, đưa vào
//! sự kiện bàn phím / chuột như người dùng, rồi đo trạng thái của ứng dụng.
//! Mỗi test ghi rõ scenario trong specs/document-viewing/spec.md mà nó kiểm tra.

use egui::{Context, Event, Key, Modifiers, MouseWheelUnit, Pos2, RawInput, Rect, Vec2};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use vpdf_app::app::{Dialog, VpdfApp};
use vpdf_core::view::{FitMode, LayoutMode};

const W: f32 = 1280.0;
const H: f32 = 800.0;

fn data(name: &str) -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/reference").join(name);
    assert!(p.is_file(), "thiếu {}: chạy `cargo run -p vpdf-testdata --release -- fetch`", p.display());
    p
}

struct Harness {
    ctx: Context,
    app: VpdfApp,
    events: Vec<Event>,
    dropped: Vec<PathBuf>,
    pointer: Pos2,
    copied: Vec<String>,
    t: f64,
}

#[derive(Debug)]
struct DroppedPath(PathBuf);
impl egui::DroppedFile for DroppedPath {
    fn path(&self) -> &Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

impl Harness {
    fn new(recent: Option<PathBuf>) -> Self {
        let ctx = Context::default();
        vpdf_app::fonts::install(&ctx);
        Harness { ctx, app: VpdfApp::new(recent), events: vec![], dropped: vec![], pointer: Pos2::ZERO, copied: vec![], t: 0.0 }
    }

    fn frame(&mut self) {
        self.t += 1.0 / 60.0;
        let mut raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(W, H))),
            time: Some(self.t),
            ..Default::default()
        };
        raw.events = std::mem::take(&mut self.events);
        raw.dropped_files =
            std::mem::take(&mut self.dropped).into_iter().map(|p| Arc::new(DroppedPath(p)) as egui::DroppedFileHandle).collect();
        let app = &mut self.app;
        let out = self.ctx.run_ui(raw, |ui| app.ui(ui));
        for c in out.platform_output.commands {
            if let egui::OutputCommand::CopyText(s) = c {
                self.copied.push(s);
            }
        }
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.frame();
        }
    }

    /// Chạy khung hình cho tới khi điều kiện đúng (tối đa 30 giây).
    fn until(&mut self, what: &str, mut f: impl FnMut(&VpdfApp) -> bool) {
        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_secs(30) {
            self.frame();
            if f(&self.app) {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("hết thời gian chờ: {what}");
    }

    fn open(&mut self, path: &Path) {
        let ctx = self.ctx.clone();
        self.app.open_path(&ctx, path);
        self.until("tài liệu mở xong", |a| a.active().is_some_and(|t| t.is_ready()));
        self.frames(3);
    }

    fn wait_tiles(&mut self) {
        self.until("ô ảnh trang hiện tại", |a| a.active().is_some_and(|t| !t.tiles.is_empty() && t.worker.pending().1 == 0));
        self.frames(2);
    }

    fn key(&mut self, k: Key, m: Modifiers) {
        self.events.push(Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
        self.events.push(Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
        self.frames(2);
    }

    fn move_to(&mut self, p: Pos2) {
        self.pointer = p;
        self.events.push(Event::PointerMoved(p));
        self.frame();
    }

    fn click(&mut self, p: Pos2) {
        self.move_to(p);
        let b = |pressed| Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        self.events.push(b(true));
        self.frame();
        self.events.push(b(false));
        self.frames(2);
    }

    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.move_to(from);
        let b = |pos, pressed| Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        self.events.push(b(from, true));
        self.frame();
        for i in 1..=10 {
            let p = from + (to - from) * (i as f32 / 10.0);
            self.events.push(Event::PointerMoved(p));
            self.frame();
        }
        self.events.push(b(to, false));
        self.frames(2);
    }

    fn type_in_page_box(&mut self, text: &str) {
        let id = self.app.page_box_id();
        self.ctx.memory_mut(|m| m.request_focus(id));
        self.frames(2);
        self.key(Key::A, Modifiers::COMMAND);
        self.events.push(Event::Text(text.into()));
        self.frames(2);
        self.key(Key::Enter, Modifiers::NONE);
        self.frames(2);
    }

    fn viewer_origin(&self) -> Pos2 {
        self.ctx.data(|d| d.get_temp::<Pos2>(egui::Id::new("viewer-origin"))).unwrap()
    }

    fn tab(&self) -> &vpdf_app::tab::DocTab {
        self.app.active().unwrap()
    }
}

/// Scenario "Mở bằng hộp thoại": trang 1 hiển thị, tên file trên tab và thanh tiêu đề.
/// (Hộp thoại hệ điều hành trả về đường dẫn; phần sau đó đi qua `open_path`.)
#[test]
fn open_shows_page_one_and_file_name() {
    let mut h = Harness::new(None);
    h.open(&data("small-vi.pdf"));
    h.wait_tiles();
    assert_eq!(h.tab().view.current_page(), 0);
    assert_eq!(h.tab().view.scroll().1, 0.0, "mở file phải bắt đầu ở đầu trang 1");
    assert_eq!(h.app.hud.page_box, "1 / 40");
    assert_eq!(h.app.hud.tab_titles, vec!["small-vi.pdf".to_string()]);
    assert_eq!(h.app.hud.window_title, "small-vi.pdf — VPdf");
}

/// Scenario "Mở bằng kéo thả": tài liệu mở trong tab mới.
#[test]
fn drop_file_opens_new_tab() {
    let mut h = Harness::new(None);
    h.open(&data("small-vi.pdf"));
    h.dropped.push(data("text-500.pdf"));
    h.frame();
    assert_eq!(h.app.tabs.len(), 2);
    h.until("tab mới mở xong", |a| a.active().is_some_and(|t| t.is_ready() && t.page_count() == 500));
    assert_eq!(h.app.hud.tab_titles.len(), 2);
}

/// Scenario "File hỏng": thông báo kèm tên file, các tab khác không bị ảnh hưởng.
#[test]
fn corrupt_file_shows_error_and_keeps_other_tabs() {
    let mut h = Harness::new(None);
    h.open(&data("small-vi.pdf"));
    h.type_in_page_box("7");
    let ctx = h.ctx.clone();
    h.app.open_path(&ctx, &data("not-a-pdf.pdf"));
    h.until("báo lỗi", |a| a.dialog.is_some());
    assert_eq!(h.app.dialog, Some(Dialog::OpenError { file: "not-a-pdf.pdf".into() }));
    assert_eq!(h.app.tabs.len(), 1);
    assert_eq!(h.tab().view.current_page(), 6, "tab còn lại giữ nguyên vị trí");
    h.key(Key::Enter, Modifiers::NONE);
    assert!(h.app.dialog.is_none());
}

/// Scenario "File hỏng một phần": hiển thị trang và cảnh báo đã sửa tạm.
#[test]
fn partially_broken_file_warns() {
    let mut h = Harness::new(None);
    h.open(&data("broken-xref.pdf"));
    assert_eq!(h.tab().page_count(), 40);
    assert_eq!(h.tab().warning.as_deref(), Some(vpdf_core::messages::REPAIRED_WARNING));
    h.wait_tiles();
}

/// Scenario "Nhập sai mật khẩu" rồi "Nhập đúng mật khẩu".
#[test]
fn password_prompt_in_ui() {
    let mut h = Harness::new(None);
    let ctx = h.ctx.clone();
    h.app.open_path(&ctx, &data("password.pdf"));
    h.until("hỏi mật khẩu", |a| matches!(a.active().unwrap().state, vpdf_app::tab::TabState::NeedPassword { .. }));
    for attempt in ["sai", "cũng sai"] {
        h.events.push(Event::Text(attempt.into()));
        h.frames(2);
        h.key(Key::Enter, Modifiers::NONE);
        h.until("báo sai mật khẩu", |a| {
            matches!(a.active().unwrap().state, vpdf_app::tab::TabState::NeedPassword { error: Some("Mật khẩu không đúng"), .. })
        });
        assert!(h.tab().tiles.is_empty(), "không trang nào được hiển thị");
        assert!(h.app.hud.page_box.is_empty() || h.tab().page_count() == 0);
    }
    h.events.push(Event::Text("matkhau123".into()));
    h.frames(2);
    h.key(Key::Enter, Modifiers::NONE);
    h.until("mở bằng mật khẩu đúng", |a| a.active().unwrap().is_ready());
    h.wait_tiles();
    assert_eq!(h.app.hud.page_box, "1 / 40");
}

/// Scenario "Chuyển tab giữ vị trí": tab A ở trang 40, 150%.
#[test]
fn tabs_keep_independent_position() {
    let mut h = Harness::new(None);
    h.open(&data("text-500.pdf"));
    h.tab_mut_view(|v| v.set_zoom(1.5));
    h.type_in_page_box("40");
    assert_eq!(h.tab().view.current_page(), 39);
    h.open(&data("small-vi.pdf"));
    h.type_in_page_box("3");
    h.key(Key::Minus, Modifiers::COMMAND);
    h.app.tabs.select(0);
    h.frames(3);
    assert_eq!(h.tab().view.current_page(), 39);
    assert_eq!(h.tab().view.zoom_percent(), 150);
    assert_eq!(h.app.hud.page_box, "40 / 500");
    assert!(h.app.hud.zoom.starts_with("150%"));
}

/// Scenario "Nhảy tới trang" và "Số trang không hợp lệ".
#[test]
fn page_box_jump_and_invalid_input() {
    let mut h = Harness::new(None);
    h.open(&data("text-500.pdf"));
    h.type_in_page_box("125");
    assert_eq!(h.tab().view.current_page(), 124);
    assert_eq!(h.app.hud.page_box, "125 / 500");
    let vis = h.tab().view.visible_pages();
    let p = vis.iter().find(|p| p.index == 124).unwrap();
    assert!(p.y.abs() < 0.5, "trang 125 ở đầu vùng xem");
    let scroll = h.tab().view.scroll();
    for bad in ["501", "abc"] {
        h.type_in_page_box(bad);
        assert_eq!(h.tab().view.scroll(), scroll, "vị trí đọc không đổi");
        assert_eq!(h.app.hud.page_box, "125 / 500", "ô trở về trang hiện tại");
    }
}

/// Điều hướng bằng phím: mũi tên, Page Down/Up, Home/End (cuộn liên tục).
#[test]
fn keyboard_navigation() {
    let mut h = Harness::new(None);
    h.open(&data("small-vi.pdf"));
    h.wait_tiles();
    let y0 = h.tab().view.scroll().1;
    h.key(Key::ArrowDown, Modifiers::NONE);
    assert!(h.tab().view.scroll().1 > y0);
    h.key(Key::PageDown, Modifiers::NONE);
    let y1 = h.tab().view.scroll().1;
    assert!(y1 > y0 + 300.0);
    h.key(Key::PageUp, Modifiers::NONE);
    assert!(h.tab().view.scroll().1 < y1);
    h.key(Key::End, Modifiers::NONE);
    assert_eq!(h.tab().view.current_page(), 39);
    assert_eq!(h.app.hud.page_box, "40 / 40");
    h.key(Key::Home, Modifiers::NONE);
    assert_eq!(h.tab().view.current_page(), 0);
}

/// Scenario "Cuộn liên tục": cuộn bằng con lăn qua cuối trang thì trang kế hiện liền bên dưới.
#[test]
fn wheel_scrolls_continuously_into_next_page() {
    let mut h = Harness::new(None);
    h.open(&data("small-vi.pdf"));
    h.wait_tiles();
    let o = h.viewer_origin();
    h.move_to(o + Vec2::new(400.0, 300.0));
    let page_h = h.tab().view.page_bottom(0);
    while h.tab().view.scroll().1 < page_h {
        h.events.push(Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: Vec2::new(0.0, -200.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        h.frames(3);
    }
    let vis = h.tab().view.visible_pages();
    assert!(vis.iter().any(|p| p.index == 1), "trang 2 hiện ngay bên dưới");
    assert_eq!(h.tab().view.layout(), LayoutMode::Continuous);
    h.wait_tiles();
    assert!(h.app.active().unwrap().tiles.len() > 1);
}

/// Chế độ từng trang: Page Down sang trang kế, chỉ một trang được vẽ.
#[test]
fn single_page_mode() {
    let mut h = Harness::new(None);
    h.open(&data("small-vi.pdf"));
    h.tab_mut_view(|v| v.set_layout(LayoutMode::SinglePage));
    h.frames(2);
    h.key(Key::PageDown, Modifiers::NONE);
    assert_eq!(h.tab().view.current_page(), 1);
    assert_eq!(h.tab().view.visible_pages().len(), 1);
}

/// Scenario "Vừa chiều rộng" (giữ khi đổi cỡ cửa sổ) và Vừa trang, 10%–6400%.
#[test]
fn fit_width_page_and_zoom_range() {
    let mut h = Harness::new(None);
    h.open(&data("small-vi.pdf"));
    h.key(Key::Num2, Modifiers::COMMAND);
    let v = &h.tab().view;
    assert_eq!(v.fit(), FitMode::FitWidth);
    let w = v.page_size(0).width * v.scale();
    assert!((w - v.viewport().0).abs() < 0.5, "chiều rộng trang {w} = vùng xem {}", v.viewport().0);
    // Đổi cỡ cửa sổ: thu bảng bên lại làm vùng xem rộng hơn.
    h.app.show_side = false;
    h.frames(3);
    let v = &h.tab().view;
    let w2 = v.page_size(0).width * v.scale();
    assert!(w2 > w && (w2 - v.viewport().0).abs() < 0.5);
    assert_eq!(v.fit(), FitMode::FitWidth);
    h.key(Key::Num0, Modifiers::COMMAND);
    assert_eq!(h.tab().view.fit(), FitMode::FitPage);
    h.key(Key::Num1, Modifiers::COMMAND);
    assert_eq!(h.tab().view.zoom_percent(), 100);
    for _ in 0..40 {
        h.key(Key::Equals, Modifiers::COMMAND);
    }
    assert_eq!(h.tab().view.zoom_percent(), 6400);
    for _ in 0..60 {
        h.key(Key::Minus, Modifiers::COMMAND);
    }
    assert_eq!(h.tab().view.zoom_percent(), 10);
}

/// Scenario "Phóng tại con trỏ": Ctrl + con lăn lên khi con trỏ ở trên một từ.
#[test]
fn ctrl_wheel_zooms_at_cursor() {
    let mut h = Harness::new(None);
    h.open(&data("small-vi.pdf"));
    h.wait_tiles();
    // Đặt con trỏ lên chữ đầu của dòng "Cộng hòa…" (lấy từ lớp chữ của trang 1).
    h.until("lớp chữ trang 1", |a| a.active().unwrap().texts.contains_key(&0));
    let o = h.viewer_origin();
    let (page, x, y) = {
        let t = h.tab();
        let c = t.texts[&0].chars.iter().find(|c| c.text == "C").unwrap().clone();
        let size = t.view.page_size(0);
        (0usize, (c.rect.left + c.rect.right) / 2.0, size.height - (c.rect.top + c.rect.bottom) / 2.0)
    };
    let (sx, sy) = h.tab().view.page_to_screen(page, x, y);
    let cursor = o + Vec2::new(sx, sy);
    h.move_to(cursor);
    let z0 = h.tab().view.zoom();
    h.events.push(Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: Vec2::new(0.0, 2.0),
        modifiers: Modifiers::COMMAND,
        phase: egui::TouchPhase::Move,
    });
    h.frames(3);
    let v = &h.tab().view;
    assert!(v.zoom() > z0 * 1.05, "mức phóng phải tăng: {z0} → {}", v.zoom());
    let (nx, ny) = v.page_to_screen(page, x, y);
    assert!((nx - sx).abs() < 1.0 && (ny - sy).abs() < 1.0, "từ phải còn dưới con trỏ: ({sx},{sy}) → ({nx},{ny})");
}

/// Scenario "Chuyển trang bằng ảnh thu nhỏ": bấm ảnh trang 3 (bảng hiện các trang đầu).
#[test]
fn thumbnail_click_navigates_and_highlights() {
    let mut h = Harness::new(None);
    h.open(&data("small-vi.pdf"));
    h.until("ảnh thu nhỏ", |a| a.hud.thumbnails_drawn >= 3);
    // Ảnh thứ 3 nằm ở hàng thứ 3 của bảng bên (mỗi hàng 196 điểm).
    let side_x = 95.0;
    let top = h.viewer_origin().y + 40.0;
    h.click(Pos2::new(side_x, top + 196.0 * 2.0 + 60.0));
    assert_eq!(h.tab().view.current_page(), 2);
    assert_eq!(h.app.hud.page_box, "3 / 40");
    assert!(h.tab().history.can_go_back());
}

/// Scenario "Ảnh thu nhỏ tải dần": chỉ ảnh của các trang đang thấy được tạo trước.
#[test]
fn thumbnails_load_progressively() {
    let mut h = Harness::new(None);
    h.open(&data("text-500.pdf"));
    h.until("ảnh thu nhỏ đang thấy", |a| a.hud.thumbnails_drawn >= 3);
    let t = h.tab();
    assert!(t.thumbs.len() < 60, "không tạo hết 500 ảnh cùng lúc ({} ảnh)", t.thumbs.len());
    assert!(t.thumbs.contains(&0) && t.thumbs.contains(&1));
}

/// Scenario "Đi tới mục" + "Mục lục tiếng Việt" + tài liệu không có mục lục.
#[test]
fn outline_navigation() {
    let mut h = Harness::new(None);
    h.open(&data("text-500.pdf"));
    h.until("mục lục", |a| a.active().unwrap().outline.is_some());
    let target = {
        let o = h.tab().outline.as_ref().unwrap();
        assert_eq!(o[2].title, "Chương 3: Quyền và nghĩa vụ của các bên");
        o[2].target.clone().unwrap()
    };
    // Bấm mục (qua cùng đường xử lý như bảng mục lục).
    if let vpdf_core::links::LinkTarget::Page { index, y_from_top_pt } = target {
        h.app.active_mut().unwrap().navigate(index, y_from_top_pt);
    }
    h.frames(2);
    let v = &h.tab().view;
    assert_eq!(v.current_page(), 50);
    assert!((v.pos().y_in_page_pt - 100.0).abs() < 1.0, "đúng vị trí đích trên trang");
    h.app.side = vpdf_app::app::SideTab::Outline;
    h.frames(2);
    h.open(&data("scan-500.pdf"));
    h.until("mục lục rỗng", |a| a.active().unwrap().outline.as_ref().is_some_and(|o| o.is_empty()));
}

/// Scenario "Link nội bộ" (kèm Quay lại) và "Link URL" (hỏi xác nhận).
#[test]
fn clicking_links() {
    let mut h = Harness::new(None);
    h.open(&data("small-vi.pdf"));
    h.until("link trang 1", |a| a.active().unwrap().links.contains_key(&0));
    h.wait_tiles();
    let o = h.viewer_origin();
    let links = h.tab().links[&0].clone();
    let size = h.tab().view.page_size(0);
    let center = |r: &vpdf_core::RectPt| ((r.left + r.right) / 2.0, size.height - (r.top + r.bottom) / 2.0);
    let internal = links.iter().find(|l| matches!(l.target, vpdf_core::links::LinkTarget::Page { .. })).unwrap();
    let (x, y) = center(&internal.rect);
    let (sx, sy) = h.tab().view.page_to_screen(0, x, y);
    let before = h.tab().view.pos();
    h.click(o + Vec2::new(sx, sy));
    assert_eq!(h.tab().view.current_page(), 29, "link trỏ tới trang 30");
    h.key(Key::ArrowLeft, Modifiers::ALT);
    assert_eq!(h.tab().view.pos(), before, "Quay lại về vị trí trước khi bấm link");

    let uri = links.iter().find(|l| matches!(l.target, vpdf_core::links::LinkTarget::Uri(_))).unwrap();
    let (x, y) = center(&uri.rect);
    let (sx, sy) = h.tab().view.page_to_screen(0, x, y);
    h.click(o + Vec2::new(sx, sy));
    assert_eq!(h.app.dialog, Some(Dialog::ConfirmUrl("https://example.com".into())));
    assert!(h.app.opened_urls.is_empty(), "chưa được mở khi chưa đồng ý");
    h.key(Key::Escape, Modifiers::NONE);
    assert!(h.app.dialog.is_none() && h.app.opened_urls.is_empty(), "Hủy thì không mở");
}

/// Scenario "Sao chép chữ tiếng Việt": kéo chọn dòng rồi Ctrl+C.
#[test]
fn drag_select_and_copy_nfc() {
    let mut h = Harness::new(None);
    h.open(&data("small-vi.pdf"));
    h.until("lớp chữ trang 1", |a| a.active().unwrap().texts.contains_key(&0));
    h.wait_tiles();
    let want = "Cộng hòa xã hội chủ nghĩa Việt Nam";
    let (first, last) = {
        let t = &h.tab().texts[&0];
        let all: String = t.chars.iter().map(|c| c.text.as_str()).collect();
        let start = all[..all.find(want).unwrap()].chars().count();
        (t.chars[start].clone(), t.chars[start + want.chars().count() - 1].clone())
    };
    let o = h.viewer_origin();
    let size = h.tab().view.page_size(0);
    let pt = |c: &vpdf_core::text::TextChar, right: bool| {
        let x = if right { c.rect.right - 0.5 } else { c.rect.left + 0.5 };
        let (sx, sy) = h.tab().view.page_to_screen(0, x, size.height - (c.rect.top + c.rect.bottom) / 2.0);
        o + Vec2::new(sx, sy)
    };
    let (from, to) = (pt(&first, false), pt(&last, true));
    h.drag(from, to);
    assert!(h.tab().selection.is_some());
    h.events.push(Event::Copy);
    h.frames(2);
    assert_eq!(h.copied.last().map(String::as_str), Some(want));
    assert!(unicode_normalization::is_nfc(h.copied.last().unwrap()));
}

/// Scenario "Tài liệu cấm sao chép": clipboard không đổi và có thông báo.
#[test]
fn copy_blocked_on_protected_document() {
    let mut h = Harness::new(None);
    h.open(&data("no-copy.pdf"));
    h.until("lớp chữ trang 1", |a| a.active().unwrap().texts.contains_key(&0));
    h.app.active_mut().unwrap().selection = Some(vpdf_core::text::Selection { start: (0, 0), end: (0, 20) });
    h.events.push(Event::Copy);
    h.frames(2);
    assert!(h.copied.is_empty(), "clipboard không thay đổi");
    assert_eq!(h.app.toast.as_ref().map(|t| t.0.as_str()), Some("Tài liệu này không cho phép sao chép"));
}

/// Scenario "Mở lại file": đóng ở trang 87 thì mở lại tại trang 87 (task 2.4).
#[test]
fn reopen_restores_reading_position() {
    let dir = std::env::temp_dir().join(format!("vpdf-ui-recent-{}", std::process::id()));
    let recent = dir.join("recent.json");
    let _ = std::fs::remove_dir_all(&dir);
    {
        let mut h = Harness::new(Some(recent.clone()));
        h.open(&data("text-500.pdf"));
        h.type_in_page_box("87");
        h.key(Key::W, Modifiers::COMMAND);
        assert!(h.app.tabs.is_empty());
    }
    // Khởi động lại ứng dụng, mở lại đúng file đó.
    let mut h = Harness::new(Some(recent.clone()));
    assert_eq!(h.app.recent.entries[0].file_name(), "text-500.pdf");
    h.open(&data("text-500.pdf"));
    h.frames(2);
    assert_eq!(h.tab().view.current_page(), 86);
    assert_eq!(h.app.hud.page_box, "87 / 500");
    let _ = std::fs::remove_dir_all(&dir);
}

impl Harness {
    fn tab_mut_view(&mut self, f: impl FnOnce(&mut vpdf_core::view::ViewState)) {
        f(&mut self.app.active_mut().unwrap().view);
        self.frames(2);
    }
}

/// specs/performance "Phóng rất to": ở 6400% chỉ dựng phần đang nhìn thấy, bộ nhớ nhỏ.
#[test]
fn zoom_6400_renders_only_visible_tiles() {
    let mut h = Harness::new(None);
    h.open(&data("small-vi.pdf"));
    h.tab_mut_view(|v| v.set_zoom(64.0));
    let scale = (64.0f32 * vpdf_core::view::PX_PER_PT_AT_100 * 1000.0).round() as u32;
    let at_scale = |a: &VpdfApp| {
        let t = a.active().unwrap();
        (0..64u32).flat_map(|ty| (0..64u32).map(move |tx| (tx, ty))).filter(|&(tx, ty)| {
            t.tiles.contains(&vpdf_render::TileKey { page: t.view.current_page() as u32, scale_milli: scale, tx, ty })
        }).count()
    };
    h.until("ô ảnh ở 6400%", |a| at_scale(a) > 0 && a.active().unwrap().worker.pending().1 == 0);
    h.frames(5);
    h.until("đủ ô đang thấy", |a| a.active().unwrap().worker.pending().1 == 0);
    assert!(at_scale(&h.app) >= 2, "phải có các ô ở 6400% cho vùng đang thấy");
    let t = h.tab();
    assert_eq!(t.view.zoom_percent(), 6400);
    // Trang ở 6400% rộng ~50 000 điểm ảnh; toàn trang sẽ cần ~10 GB. Chỉ vài ô 512×512 được dựng.
    assert!(t.tiles.len() <= 12, "{} ô", t.tiles.len());
    assert!(t.tiles.used_bytes() < 16 * 1024 * 1024, "{} byte", t.tiles.used_bytes());
    if let Some(m) = memory_stats::memory_stats() {
        assert!(m.physical_mem < 1024 * 1024 * 1024, "tiến trình dùng {} MB", m.physical_mem >> 20);
    }
}
