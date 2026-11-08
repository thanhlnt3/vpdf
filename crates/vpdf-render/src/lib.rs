//! Dựng hình PDF bằng PDFium (design.md, D2).
//!
//! PDFium không an toàn khi gọi từ nhiều luồng, nên mỗi tài liệu có **một luồng
//! worker riêng** giữ `PdfDocument`; giao diện chỉ gửi yêu cầu và nhận kết quả qua
//! kênh. `pdfium-render` (tính năng `thread_safe`) còn khóa toàn cục quanh mỗi lời
//! gọi PDFium, nên nhiều worker chạy cùng lúc vẫn an toàn.
//!
//! Hàng đợi ưu tiên: lệnh điều khiển (mở, lấy chữ, lấy link) → ô ảnh trang đang
//! nhìn thấy → ảnh thu nhỏ. Danh sách ô ảnh và ảnh thu nhỏ được giao diện **thay
//! mới** mỗi khi vùng xem đổi, nên việc tạo ảnh thu nhỏ không bao giờ chặn việc đọc.

use pdfium_render::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use vpdf_core::links::{LinkTarget, PageLink};
use vpdf_core::text::{PageText, TextChar};
use vpdf_core::view::PageSize;
use vpdf_core::xref::{self, XrefHealth};
use vpdf_core::RectPt;

/// Cạnh của một ô ảnh (điểm ảnh thiết bị). Trang lớn ở mức phóng cao được chia ô,
/// và chỉ các ô đang nhìn thấy được dựng.
pub const TILE: u32 = 512;

/// Chiều rộng mặc định của ảnh thu nhỏ (điểm ảnh thiết bị).
pub const THUMB_WIDTH: u32 = 120;

// ---------------------------------------------------------------------------
// Nạp PDFium

static PDFIUM: OnceLock<Result<&'static Pdfium, String>> = OnceLock::new();

/// Nạp thư viện PDFium một lần cho cả tiến trình.
pub fn pdfium() -> Result<&'static Pdfium, String> {
    PDFIUM
        .get_or_init(|| {
            let mut errors = Vec::new();
            for dir in vpdf_core::native::pdfium_dirs() {
                let p = Pdfium::pdfium_platform_library_name_at_path(&dir);
                if !p.is_file() {
                    continue;
                }
                match Pdfium::bind_to_library(&p) {
                    Ok(b) => return Ok(&*Box::leak(Box::new(Pdfium::new(b)))),
                    Err(e) => errors.push(format!("{}: {e:?}", p.display())),
                }
            }
            Err(format!("không tìm thấy thư viện PDFium. {}", errors.join("; ")))
        })
        .clone()
}

// ---------------------------------------------------------------------------
// Kiểu dữ liệu trao đổi với giao diện

#[derive(Debug, Clone, PartialEq)]
pub enum OpenError {
    /// Không phải PDF hợp lệ ("Không đọc được file PDF").
    Unreadable(String),
    /// File có mật khẩu, chưa nhập.
    PasswordRequired,
    /// Mật khẩu không đúng ("Mật khẩu không đúng").
    WrongPassword,
    /// Không tìm thấy thư viện PDFium hoặc lỗi hệ thống khác.
    Engine(String),
}

#[derive(Debug, Clone)]
pub struct DocInfo {
    pub path: PathBuf,
    pub page_sizes: Vec<PageSize>,
    /// Được phép sao chép chữ (PDFium báo đủ quyền khi mở bằng mật khẩu chủ sở hữu).
    pub copy_allowed: bool,
    pub encrypted: bool,
    /// Bảng xref hỏng nhưng PDFium đã khôi phục được trang.
    pub repaired: bool,
    pub repair_detail: Option<String>,
    pub open_time: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutlineNode {
    pub title: String,
    pub target: Option<LinkTarget>,
    pub children: Vec<OutlineNode>,
}

/// Khóa của một ô ảnh trang: trang, tỉ lệ (điểm ảnh thiết bị trên point × 1000), ô.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TileKey {
    pub page: u32,
    pub scale_milli: u32,
    pub tx: u32,
    pub ty: u32,
}

impl TileKey {
    pub fn scale(&self) -> f32 {
        self.scale_milli as f32 / 1000.0
    }
}

/// Kích thước (điểm ảnh) của toàn trang ở một tỉ lệ.
pub fn page_pixel_size(size: PageSize, scale: f32) -> (u32, u32) {
    (((size.width * scale).round() as u32).max(1), ((size.height * scale).round() as u32).max(1))
}

/// Số ô theo chiều ngang và dọc.
pub fn tile_grid(size: PageSize, scale: f32) -> (u32, u32) {
    let (w, h) = page_pixel_size(size, scale);
    (w.div_ceil(TILE), h.div_ceil(TILE))
}

/// Ảnh RGBA (8 bit mỗi kênh, không nhân trước alpha).
#[derive(Clone)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl std::fmt::Debug for Rgba {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rgba({}x{})", self.width, self.height)
    }
}

impl Rgba {
    pub fn bytes(&self) -> usize {
        self.pixels.len()
    }
}

#[derive(Debug)]
pub enum Event {
    Opened(Result<DocInfo, OpenError>),
    Outline(Vec<OutlineNode>),
    Tile(TileKey, Rgba),
    Thumb(u32, Rgba),
    Text(u32, PageText),
    Links(u32, Vec<PageLink>),
    Failed(String),
}

enum Control {
    Open { path: PathBuf, password: Option<String> },
    Text(u32),
    Links(u32),
}

#[derive(Default)]
struct Queue {
    control: VecDeque<Control>,
    tiles: Vec<TileKey>,
    thumbs: Vec<u32>,
    thumb_width: u32,
    shutdown: bool,
    /// Ô vừa dựng xong, để không dựng lại khi giao diện chưa kịp nhận kết quả.
    recent_done: HashMap<TileKey, Instant>,
}

type Notify = Arc<dyn Fn() + Send + Sync>;

/// Luồng worker PDFium của một tài liệu.
pub struct DocumentWorker {
    shared: Arc<(Mutex<Queue>, Condvar)>,
    events: Receiver<Event>,
    thread: Option<JoinHandle<()>>,
}

impl DocumentWorker {
    /// Tạo worker và bắt đầu mở file. `notify` được gọi mỗi khi có kết quả mới
    /// (giao diện dùng để yêu cầu vẽ lại).
    pub fn open(path: &Path, password: Option<String>, notify: Notify) -> Self {
        let shared = Arc::new((Mutex::new(Queue { thumb_width: THUMB_WIDTH, ..Default::default() }), Condvar::new()));
        let (tx, rx) = channel();
        let s2 = shared.clone();
        let thread = std::thread::Builder::new()
            .name(format!("pdfium:{}", path.file_name().map(|s| s.to_string_lossy()).unwrap_or_default()))
            .spawn(move || worker_main(s2, tx, notify))
            .expect("không tạo được luồng worker");
        let w = DocumentWorker { shared, events: rx, thread: Some(thread) };
        w.push_control(Control::Open { path: path.to_path_buf(), password });
        w
    }

    fn push_control(&self, c: Control) {
        let (m, cv) = &*self.shared;
        m.lock().unwrap().control.push_back(c);
        cv.notify_one();
    }

    /// Thử mở lại với mật khẩu khác (không giới hạn số lần).
    pub fn retry_password(&self, path: &Path, password: String) {
        self.push_control(Control::Open { path: path.to_path_buf(), password: Some(password) });
    }

    /// Thay danh sách ô ảnh cần dựng (theo thứ tự ưu tiên).
    pub fn set_wanted_tiles(&self, tiles: Vec<TileKey>) {
        let (m, cv) = &*self.shared;
        let mut q = m.lock().unwrap();
        let now = Instant::now();
        q.recent_done.retain(|_, t| now.duration_since(*t) < Duration::from_secs(2));
        let filtered: Vec<TileKey> = tiles.into_iter().filter(|k| !q.recent_done.contains_key(k)).collect();
        if q.tiles != filtered {
            q.tiles = filtered;
            cv.notify_one();
        }
    }

    /// Thay danh sách ảnh thu nhỏ cần tạo (các trang đang thấy trong bảng ảnh thu nhỏ trước).
    pub fn set_wanted_thumbs(&self, pages: Vec<u32>, width_px: u32) {
        let (m, cv) = &*self.shared;
        let mut q = m.lock().unwrap();
        if q.thumbs != pages || q.thumb_width != width_px {
            q.thumbs = pages;
            q.thumb_width = width_px.max(16);
            cv.notify_one();
        }
    }

    pub fn request_text(&self, page: u32) {
        self.push_control(Control::Text(page));
    }

    pub fn request_links(&self, page: u32) {
        self.push_control(Control::Links(page));
    }

    pub fn try_recv(&self) -> Option<Event> {
        self.events.try_recv().ok()
    }

    /// Chờ một sự kiện (dùng trong kiểm thử).
    pub fn recv_timeout(&self, d: Duration) -> Option<Event> {
        self.events.recv_timeout(d).ok()
    }

    /// Số yêu cầu còn chờ: (điều khiển, ô ảnh, ảnh thu nhỏ).
    pub fn pending(&self) -> (usize, usize, usize) {
        let q = self.shared.0.lock().unwrap();
        (q.control.len(), q.tiles.len(), q.thumbs.len())
    }
}

impl Drop for DocumentWorker {
    fn drop(&mut self) {
        {
            let (m, cv) = &*self.shared;
            m.lock().unwrap().shutdown = true;
            cv.notify_all();
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

// ---------------------------------------------------------------------------
// Luồng worker

enum Job {
    Control(Control),
    Tile(TileKey),
    Thumb(u32, u32),
}

fn next_job(shared: &(Mutex<Queue>, Condvar), doc_open: bool) -> Option<Job> {
    let (m, cv) = shared;
    let mut q = m.lock().unwrap();
    loop {
        if q.shutdown {
            return None;
        }
        if let Some(c) = q.control.pop_front() {
            return Some(Job::Control(c));
        }
        if doc_open {
            if !q.tiles.is_empty() {
                let k = q.tiles.remove(0);
                q.recent_done.insert(k, Instant::now());
                return Some(Job::Tile(k));
            }
            if !q.thumbs.is_empty() {
                let p = q.thumbs.remove(0);
                return Some(Job::Thumb(p, q.thumb_width));
            }
        }
        q = cv.wait(q).unwrap();
    }
}

fn worker_main(shared: Arc<(Mutex<Queue>, Condvar)>, tx: Sender<Event>, notify: Notify) {
    let send = |e: Event| {
        let _ = tx.send(e);
        notify();
    };
    let pdfium = match pdfium() {
        Ok(p) => p,
        Err(e) => {
            send(Event::Opened(Err(OpenError::Engine(e))));
            return;
        }
    };
    loop {
        let Some(job) = next_job(&shared, false) else { return };
        let Job::Control(Control::Open { path, password }) = job else { continue };
        match open_document(pdfium, &path, password.as_deref()) {
            Ok((doc, info)) => {
                send(Event::Opened(Ok(info)));
                // Mục lục sâu được trích sau khi trang 1 đã có thể hiển thị.
                send(Event::Outline(read_outline(&doc)));
                serve(&doc, &shared, &send);
                return;
            }
            Err(e) => send(Event::Opened(Err(e))),
        }
    }
}

fn open_document<'a>(
    pdfium: &'a Pdfium,
    path: &Path,
    password: Option<&str>,
) -> Result<(PdfDocument<'a>, DocInfo), OpenError> {
    let t0 = Instant::now();
    let health = xref::check_file(path).map_err(|e| OpenError::Unreadable(e.to_string()))?;
    if health == XrefHealth::NotPdf {
        return Err(OpenError::Unreadable("không có tiêu đề %PDF".into()));
    }
    let doc = pdfium.load_pdf_from_file(path, password).map_err(|e| match e {
        PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError) => {
            if password.is_none() {
                OpenError::PasswordRequired
            } else {
                OpenError::WrongPassword
            }
        }
        PdfiumError::IoError(e) => OpenError::Unreadable(e.to_string()),
        other => OpenError::Unreadable(format!("{other:?}")),
    })?;
    let n = doc.pages().len();
    if n <= 0 {
        return Err(OpenError::Unreadable("tài liệu không có trang".into()));
    }
    let mut page_sizes = Vec::with_capacity(n as usize);
    for i in 0..n {
        let r = doc.pages().page_size(i).unwrap_or(PdfRect::new_from_values(0.0, 0.0, 842.0, 595.0));
        page_sizes.push(PageSize { width: r.width().value.max(1.0), height: r.height().value.max(1.0) });
    }
    let encrypted = xref::has_encrypt_entry(path).unwrap_or(false);
    let copy_allowed = if encrypted { raw_copy_permission(pdfium, path, password) } else { true };
    let (repaired, repair_detail) = match health {
        XrefHealth::Damaged(d) => (true, Some(d)),
        _ => (false, None),
    };
    let info = DocInfo {
        path: path.to_path_buf(),
        page_sizes,
        copy_allowed,
        encrypted,
        repaired,
        repair_detail,
        open_time: t0.elapsed(),
    };
    Ok((doc, info))
}

/// Bit 5 của /P (PDF 32000-1, bảng 22): được trích chữ và đồ họa (sao chép).
const PERM_COPY: u32 = 1 << 4;

/// Đọc quyền sao chép trực tiếp từ PDFium. `pdfium-render` chưa nhận ra bộ bảo mật
/// R6 (AES-256) nên không dùng `PdfPermissions` được. Khi mở bằng mật khẩu chủ sở hữu,
/// PDFium trả về đủ mọi quyền (0xFFFFFFFF).
fn raw_copy_permission(pdfium: &Pdfium, path: &Path, password: Option<&str>) -> bool {
    let b = pdfium.bindings();
    let Some(p) = path.to_str() else { return false };
    // SAFETY: tài liệu được mở và đóng ngay trong hàm, trên luồng worker (bindings có khóa).
    unsafe {
        let h = b.FPDF_LoadDocument(p, password);
        if h.is_null() {
            return false;
        }
        let perms = b.FPDF_GetDocPermissions(h) as u32;
        b.FPDF_CloseDocument(h);
        perms & PERM_COPY != 0
    }
}

/// Giữ vài trang đã nạp gần nhất để không phải nạp lại cho mỗi ô ảnh.
struct PageCache<'d, 'a> {
    doc: &'d PdfDocument<'a>,
    pages: VecDeque<(u32, PdfPage<'a>)>,
}

impl<'d, 'a> PageCache<'d, 'a> {
    const MAX: usize = 6;

    fn get(&mut self, index: u32) -> Result<&PdfPage<'a>, PdfiumError> {
        if let Some(pos) = self.pages.iter().position(|(i, _)| *i == index) {
            let item = self.pages.remove(pos).unwrap();
            self.pages.push_front(item);
        } else {
            let p = self.doc.pages().get(index as PdfPageIndex)?;
            self.pages.push_front((index, p));
            self.pages.truncate(Self::MAX);
        }
        Ok(&self.pages[0].1)
    }
}

fn serve(doc: &PdfDocument<'_>, shared: &(Mutex<Queue>, Condvar), send: &dyn Fn(Event)) {
    let mut cache = PageCache { doc, pages: VecDeque::new() };
    while let Some(job) = next_job(shared, true) {
        let r = match job {
            Job::Control(Control::Open { .. }) => Ok(()), // đã mở rồi
            Job::Control(Control::Text(p)) => cache.get(p).map(|page| send(Event::Text(p, extract_text(page)))),
            Job::Control(Control::Links(p)) => {
                cache.get(p).map(|page| send(Event::Links(p, extract_links(doc, page))))
            }
            Job::Tile(k) => cache.get(k.page).and_then(|page| render_tile(page, k)).map(|img| send(Event::Tile(k, img))),
            Job::Thumb(p, w) => cache.get(p).and_then(|page| render_thumb(page, w)).map(|img| send(Event::Thumb(p, img))),
        };
        if let Err(e) = r {
            send(Event::Failed(format!("{e:?}")));
        }
    }
}

fn bgra_to_rgba(mut b: Vec<u8>) -> Vec<u8> {
    for px in b.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    b
}

/// Dựng một ô ảnh: chỉ phần (tx, ty) của trang ở tỉ lệ `scale` được vẽ.
pub fn render_tile(page: &PdfPage, k: TileKey) -> Result<Rgba, PdfiumError> {
    let size = PageSize { width: page.width().value, height: page.height().value };
    let (pw, ph) = page_pixel_size(size, k.scale());
    let x0 = k.tx * TILE;
    let y0 = k.ty * TILE;
    if x0 >= pw || y0 >= ph {
        return Err(PdfiumError::PageIndexOutOfBounds);
    }
    let w = TILE.min(pw - x0);
    let h = TILE.min(ph - y0);
    let mut bmp = PdfBitmap::empty(w as Pixels, h as Pixels, PdfBitmapFormat::BGRA)?;
    let cfg = PdfRenderConfig::new()
        .set_fixed_size(pw as Pixels, ph as Pixels)
        .set_origin(-(x0 as Pixels), -(y0 as Pixels))
        .render_form_data(true)
        .render_annotations(true)
        .set_clear_color(PdfColor::WHITE);
    page.render_into_bitmap_with_config(&mut bmp, &cfg)?;
    Ok(Rgba { width: w, height: h, pixels: bgra_to_rgba(bmp.as_raw_bytes()) })
}

/// Ảnh thu nhỏ rộng `width` điểm ảnh.
pub fn render_thumb(page: &PdfPage, width: u32) -> Result<Rgba, PdfiumError> {
    let size = PageSize { width: page.width().value, height: page.height().value };
    let scale = width as f32 / size.width.max(1.0);
    let (w, h) = page_pixel_size(size, scale);
    let mut bmp = PdfBitmap::empty(w as Pixels, h as Pixels, PdfBitmapFormat::BGRA)?;
    let cfg = PdfRenderConfig::new().set_fixed_size(w as Pixels, h as Pixels).set_clear_color(PdfColor::WHITE);
    page.render_into_bitmap_with_config(&mut bmp, &cfg)?;
    Ok(Rgba { width: w, height: h, pixels: bgra_to_rgba(bmp.as_raw_bytes()) })
}

fn rect_of(r: PdfRect) -> RectPt {
    RectPt::new(r.left().value, r.bottom().value, r.right().value, r.top().value)
}

/// Lớp chữ của trang theo thứ tự nội dung.
pub fn extract_text(page: &PdfPage) -> PageText {
    let Ok(text) = page.text() else { return PageText::default() };
    let mut chars = Vec::new();
    for c in text.chars().iter() {
        let Some(s) = c.unicode_string() else { continue };
        let generated = c.is_generated().unwrap_or(false);
        let rect = c.loose_bounds().map(rect_of).unwrap_or_default();
        chars.push(TextChar { text: s, rect, generated });
    }
    PageText { chars }
}

fn dest_target(doc: &PdfDocument, dest: &PdfDestination) -> Option<LinkTarget> {
    let index = dest.page_index().ok()? as usize;
    let y = match dest.view_settings() {
        Ok(PdfDestinationViewSettings::SpecificCoordinatesAndZoom(_, Some(y), _)) => Some(y.value),
        Ok(PdfDestinationViewSettings::FitPageHorizontallyToWindow(Some(y))) => Some(y.value),
        Ok(PdfDestinationViewSettings::FitBoundsHorizontallyToWindow(Some(y))) => Some(y.value),
        _ => None,
    };
    // Đổi y (từ đáy trang) sang khoảng cách từ mép trên trang.
    let y_from_top = y.and_then(|y| {
        let h = doc.pages().page_size(index as PdfPageIndex).ok()?.height().value;
        Some((h - y).clamp(0.0, h))
    });
    Some(LinkTarget::Page { index, y_from_top_pt: y_from_top })
}

fn action_target(doc: &PdfDocument, action: &PdfAction) -> Option<LinkTarget> {
    if let Some(u) = action.as_uri_action() {
        return u.uri().ok().map(LinkTarget::Uri);
    }
    if let Some(l) = action.as_local_destination_action() {
        return l.destination().ok().and_then(|d| dest_target(doc, &d));
    }
    Some(LinkTarget::Unsupported)
}

/// Link của trang.
pub fn extract_links(doc: &PdfDocument, page: &PdfPage) -> Vec<PageLink> {
    let mut out = Vec::new();
    for link in page.links().iter() {
        let Ok(rect) = link.rect() else { continue };
        let target = if let Some(d) = link.destination() {
            dest_target(doc, &d)
        } else if let Some(a) = link.action() {
            action_target(doc, &a)
        } else {
            None
        };
        if let Some(target) = target {
            out.push(PageLink { rect: rect_of(rect), target });
        }
    }
    out
}

/// Cây mục lục. Giới hạn độ sâu và số mục để chống mục lục vòng lặp.
pub fn read_outline(doc: &PdfDocument) -> Vec<OutlineNode> {
    let mut budget = 20_000usize;
    fn walk(doc: &PdfDocument, first: Option<PdfBookmark>, depth: usize, budget: &mut usize) -> Vec<OutlineNode> {
        let mut out = Vec::new();
        let mut cur = first;
        while let Some(b) = cur {
            if *budget == 0 {
                break;
            }
            *budget -= 1;
            let title = b.title().unwrap_or_default();
            let target = b.destination().and_then(|d| dest_target(doc, &d)).or_else(|| b.action().and_then(|a| action_target(doc, &a)));
            let children = if depth < 32 { walk(doc, b.first_child(), depth + 1, budget) } else { Vec::new() };
            out.push(OutlineNode { title, target, children });
            cur = b.next_sibling();
        }
        out
    }
    walk(doc, doc.bookmarks().root(), 0, &mut budget)
}
