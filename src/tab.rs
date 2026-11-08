//! Một tab = một tài liệu, với luồng worker PDFium, vùng xem, lịch sử, bộ nhớ đệm
//! ảnh và lớp chữ riêng. Trạng thái của các tab độc lập với nhau.

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vpdf_core::history::NavHistory;
use vpdf_core::links::PageLink;
use vpdf_core::lru::{ByteLru, PAGE_CACHE_BYTES};
use vpdf_core::page_input::PageBox;
use vpdf_core::text::{PageText, Selection};
use vpdf_core::view::{PageSize, ViewPos, ViewState};
use vpdf_render::{DocInfo, DocumentWorker, Event, OpenError, OutlineNode, Rgba, TileKey};

/// Giới hạn bộ nhớ cho ảnh thu nhỏ (ngoài 300 MB của ảnh trang).
pub const THUMB_CACHE_BYTES: usize = 48 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum TabState {
    Opening,
    /// Cần mật khẩu; `error` = "Mật khẩu không đúng" sau lần nhập sai.
    NeedPassword { input: String, error: Option<&'static str> },
    Ready,
    /// Không mở được (thông báo đã hiện, tab sẽ được đóng).
    Failed(String),
}

pub struct DocTab {
    pub id: u64,
    pub path: PathBuf,
    pub title: String,
    pub worker: DocumentWorker,
    pub state: TabState,
    pub info: Option<DocInfo>,
    pub view: ViewState,
    pub history: NavHistory,
    pub page_box: PageBox,
    pub tiles: ByteLru<TileKey, TextureHandle>,
    /// Tỉ lệ gần nhất mà trang đã có đủ ô ảnh (để vẽ tạm khi đang đổi mức phóng).
    pub last_scale: HashMap<u32, u32>,
    pub thumbs: ByteLru<u32, TextureHandle>,
    pub outline: Option<Vec<OutlineNode>>,
    pub texts: HashMap<u32, PageText>,
    pub links: HashMap<u32, Vec<PageLink>>,
    pub requested_text: HashSet<u32>,
    pub requested_links: HashSet<u32>,
    pub selection: Option<Selection>,
    pub restore_pos: Option<ViewPos>,
    pub warning: Option<String>,
    /// Lần mở đầu tiên đã xong (dùng để khôi phục vị trí đọc đúng một lần).
    pub restored: bool,
    /// Mức phóng ở lần vẽ trước, để biết khi nào ô ảnh cũ không còn dùng.
    pub last_viewport: (f32, f32),
}

impl DocTab {
    pub fn open(id: u64, path: &Path, ctx: &Context, restore: Option<ViewPos>) -> Self {
        let c = ctx.clone();
        let notify: Arc<dyn Fn() + Send + Sync> = Arc::new(move || c.request_repaint());
        let title = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| path.display().to_string());
        DocTab {
            id,
            path: path.to_path_buf(),
            title,
            worker: DocumentWorker::open(path, None, notify),
            state: TabState::Opening,
            info: None,
            view: ViewState::new(vec![PageSize::A4]),
            history: NavHistory::default(),
            page_box: PageBox::default(),
            tiles: ByteLru::new(PAGE_CACHE_BYTES),
            last_scale: HashMap::new(),
            thumbs: ByteLru::new(THUMB_CACHE_BYTES),
            outline: None,
            texts: HashMap::new(),
            links: HashMap::new(),
            requested_text: HashSet::new(),
            requested_links: HashSet::new(),
            selection: None,
            restore_pos: restore,
            warning: None,
            restored: false,
            last_viewport: (0.0, 0.0),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.state == TabState::Ready
    }

    pub fn page_count(&self) -> usize {
        self.info.as_ref().map_or(0, |i| i.page_sizes.len())
    }

    pub fn copy_allowed(&self) -> bool {
        self.info.as_ref().is_some_and(|i| i.copy_allowed)
    }

    pub fn submit_password(&mut self) {
        if let TabState::NeedPassword { input, .. } = &self.state {
            let pw = input.clone();
            self.worker.retry_password(&self.path, pw);
            self.state = TabState::Opening;
        }
    }

    /// Nhận kết quả từ worker. Trả về true nếu có thay đổi.
    pub fn poll(&mut self, ctx: &Context) -> bool {
        let mut changed = false;
        while let Some(ev) = self.worker.try_recv() {
            changed = true;
            match ev {
                Event::Opened(Ok(info)) => {
                    let mut view = ViewState::new(info.page_sizes.clone());
                    let (w, h) = self.last_viewport;
                    if w > 0.0 {
                        view.set_viewport(w, h);
                    }
                    self.view = view;
                    if info.repaired {
                        self.warning = Some(vpdf_core::messages::REPAIRED_WARNING.to_string());
                    }
                    self.info = Some(info);
                    self.state = TabState::Ready;
                }
                Event::Opened(Err(e)) => {
                    self.state = match e {
                        OpenError::PasswordRequired => TabState::NeedPassword { input: String::new(), error: None },
                        OpenError::WrongPassword => TabState::NeedPassword {
                            input: String::new(),
                            error: Some(vpdf_core::messages::WRONG_PASSWORD),
                        },
                        OpenError::Unreadable(d) => TabState::Failed(d),
                        OpenError::Engine(d) => TabState::Failed(d),
                    };
                }
                Event::Outline(o) => self.outline = Some(o),
                Event::Tile(k, img) => {
                    let tex = load_texture(ctx, &format!("tile-{}-{:?}", self.id, k), &img);
                    self.tiles.insert(k, tex, img.bytes());
                }
                Event::Thumb(p, img) => {
                    let tex = load_texture(ctx, &format!("thumb-{}-{p}", self.id), &img);
                    self.thumbs.insert(p, tex, img.bytes());
                }
                Event::Text(p, t) => {
                    self.texts.insert(p, t);
                }
                Event::Links(p, l) => {
                    self.links.insert(p, l);
                }
                Event::Failed(_) => {}
            }
        }
        changed
    }

    /// Lớp chữ và link của các trang đang nhìn thấy (mỗi trang một lần).
    pub fn ensure_page_data(&mut self, page: u32) {
        if self.requested_text.insert(page) {
            self.worker.request_text(page);
        }
        if self.requested_links.insert(page) {
            self.worker.request_links(page);
        }
    }

    /// Ghi vị trí hiện tại vào lịch sử rồi nhảy tới đích.
    pub fn navigate(&mut self, page: usize, y_from_top_pt: Option<f32>) {
        self.history.push(self.view.pos());
        self.view.goto_dest(page, y_from_top_pt);
        self.page_box.cancel();
    }

    pub fn go_back(&mut self) -> bool {
        if let Some(p) = self.history.back(self.view.pos()) {
            self.view.restore(p);
            true
        } else {
            false
        }
    }

    pub fn selected_text(&self) -> String {
        match &self.selection {
            Some(sel) => sel.text(|p| self.texts.get(&(p as u32)).cloned()),
            None => String::new(),
        }
    }
}

pub fn load_texture(ctx: &Context, name: &str, img: &Rgba) -> TextureHandle {
    let ci = ColorImage::from_rgba_unmultiplied([img.width as usize, img.height as usize], &img.pixels);
    ctx.load_texture(name, ci, TextureOptions::LINEAR)
}
