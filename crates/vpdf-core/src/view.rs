//! Mô hình vùng xem: bố cục trang, cuộn, phóng, Vừa trang / Vừa chiều rộng,
//! phóng tại con trỏ, cuộn liên tục và chế độ từng trang.
//!
//! Đơn vị: kích thước trang tính bằng point PDF (1/72 inch); vị trí trên màn
//! hình tính bằng điểm logic của giao diện (egui "points"). Ở mức phóng 100%,
//! 1 point PDF = 96/72 điểm logic (kích thước thật trên màn hình 96 dpi).

use serde::{Deserialize, Serialize};

/// Mức phóng nhỏ nhất: 10%.
pub const ZOOM_MIN: f32 = 0.10;
/// Mức phóng lớn nhất: 6400%.
pub const ZOOM_MAX: f32 = 64.0;
/// Điểm logic trên mỗi point PDF ở mức 100%.
pub const PX_PER_PT_AT_100: f32 = 96.0 / 72.0;
/// Khoảng cách dọc giữa hai trang (điểm logic).
pub const PAGE_GAP: f32 = 10.0;
/// Lề trên và dưới của toàn bộ tài liệu (điểm logic).
pub const MARGIN_Y: f32 = 10.0;
/// Một nấc cuộn bằng phím mũi tên (điểm logic).
pub const LINE_SCROLL: f32 = 48.0;
/// Hệ số cho một nấc Phóng to / Thu nhỏ.
pub const ZOOM_STEP: f32 = 1.25;
/// Các mức phóng có sẵn trong danh sách chọn.
pub const ZOOM_PRESETS: [f32; 15] =
    [0.10, 0.25, 0.50, 0.75, 1.0, 1.25, 1.50, 2.0, 3.0, 4.0, 8.0, 16.0, 24.0, 32.0, 64.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FitMode {
    /// Mức phóng do người dùng đặt.
    #[default]
    Free,
    /// Vừa trang: cả trang nằm gọn trong vùng xem.
    FitPage,
    /// Vừa chiều rộng: chiều rộng trang bằng chiều rộng vùng xem.
    FitWidth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LayoutMode {
    /// Các trang nối tiếp nhau theo chiều dọc.
    #[default]
    Continuous,
    /// Mỗi lần chỉ hiện một trang.
    SinglePage,
}

/// Kích thước một trang (point PDF, đã tính xoay trang).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageSize {
    pub width: f32,
    pub height: f32,
}

impl PageSize {
    pub const A4: PageSize = PageSize { width: 595.0, height: 842.0 };
}

/// Vị trí đọc có thể lưu lại (cho lệnh Quay lại và để nhớ khi mở lại file).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ViewPos {
    /// Trang ở đầu vùng xem (tính từ 0).
    pub page: usize,
    /// Khoảng cách từ mép trên trang tới mép trên vùng xem, tính bằng point PDF.
    pub y_in_page_pt: f32,
    pub zoom: f32,
    pub fit: FitMode,
}

/// Một trang đang hiện trong vùng xem, với tọa độ màn hình tương đối so với góc
/// trên trái của vùng xem.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisiblePage {
    pub index: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Trạng thái vùng xem của một tab.
#[derive(Debug, Clone)]
pub struct ViewState {
    pages: Vec<PageSize>,
    /// Tổng chiều cao (point) của các trang đứng trước trang i.
    prefix_heights: Vec<f32>,
    max_width_pt: f32,
    zoom: f32,
    fit: FitMode,
    layout: LayoutMode,
    /// Vị trí góc trên trái vùng xem trong không gian nội dung (điểm logic).
    scroll_x: f32,
    scroll_y: f32,
    viewport_w: f32,
    viewport_h: f32,
    /// Trang hiện ở chế độ từng trang.
    single_page: usize,
    /// Trang vừa được nhảy tới; giữ cho ô số trang cho tới khi người dùng cuộn.
    anchor_page: Option<usize>,
}

impl ViewState {
    pub fn new(pages: Vec<PageSize>) -> Self {
        let pages = if pages.is_empty() { vec![PageSize::A4] } else { pages };
        let mut prefix = Vec::with_capacity(pages.len() + 1);
        let mut acc = 0.0;
        let mut max_w: f32 = 0.0;
        for p in &pages {
            prefix.push(acc);
            acc += p.height;
            max_w = max_w.max(p.width);
        }
        prefix.push(acc);
        let mut v = ViewState {
            pages,
            prefix_heights: prefix,
            max_width_pt: max_w,
            zoom: 1.0,
            fit: FitMode::Free,
            layout: LayoutMode::Continuous,
            scroll_x: 0.0,
            scroll_y: 0.0,
            viewport_w: 800.0,
            viewport_h: 600.0,
            single_page: 0,
            anchor_page: None,
        };
        v.center_horizontally();
        v
    }

    // ----- truy vấn -----

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn page_size(&self, index: usize) -> PageSize {
        self.pages[index.min(self.pages.len() - 1)]
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Mức phóng dạng phần trăm, làm tròn (ví dụ 150).
    pub fn zoom_percent(&self) -> u32 {
        (self.zoom * 100.0).round() as u32
    }

    pub fn fit(&self) -> FitMode {
        self.fit
    }

    pub fn layout(&self) -> LayoutMode {
        self.layout
    }

    pub fn scroll(&self) -> (f32, f32) {
        (self.scroll_x, self.scroll_y)
    }

    pub fn viewport(&self) -> (f32, f32) {
        (self.viewport_w, self.viewport_h)
    }

    /// Điểm logic trên mỗi point PDF ở mức phóng hiện tại.
    pub fn scale(&self) -> f32 {
        self.zoom * PX_PER_PT_AT_100
    }

    fn scale_for(zoom: f32) -> f32 {
        zoom * PX_PER_PT_AT_100
    }

    /// Chiều rộng nội dung (điểm logic).
    pub fn content_width(&self) -> f32 {
        match self.layout {
            LayoutMode::Continuous => self.max_width_pt * self.scale(),
            LayoutMode::SinglePage => self.page_size(self.single_page).width * self.scale(),
        }
    }

    /// Chiều cao nội dung (điểm logic).
    pub fn content_height(&self) -> f32 {
        let s = self.scale();
        match self.layout {
            LayoutMode::Continuous => {
                let n = self.pages.len();
                2.0 * MARGIN_Y + self.prefix_heights[n] * s + (n.saturating_sub(1)) as f32 * PAGE_GAP
            }
            LayoutMode::SinglePage => 2.0 * MARGIN_Y + self.page_size(self.single_page).height * s,
        }
    }

    /// Mép trên của trang trong không gian nội dung.
    pub fn page_top(&self, index: usize) -> f32 {
        match self.layout {
            LayoutMode::Continuous => {
                let i = index.min(self.pages.len() - 1);
                MARGIN_Y + self.prefix_heights[i] * self.scale() + i as f32 * PAGE_GAP
            }
            LayoutMode::SinglePage => MARGIN_Y,
        }
    }

    pub fn page_bottom(&self, index: usize) -> f32 {
        self.page_top(index) + self.page_size(index).height * self.scale()
    }

    /// Mép trái của trang trong không gian nội dung (các trang canh giữa).
    pub fn page_left(&self, index: usize) -> f32 {
        (self.content_width() - self.page_size(index).width * self.scale()) / 2.0
    }

    fn scroll_x_range(&self) -> (f32, f32) {
        let d = self.content_width() - self.viewport_w;
        // Nội dung hẹp hơn vùng xem: trang được đặt tự do miễn là nằm trọn trong vùng xem.
        (d.min(0.0), d.max(0.0))
    }

    fn max_scroll_y(&self) -> f32 {
        (self.content_height() - self.viewport_h).max(0.0)
    }

    /// Trang hiện tại (tính từ 0) dùng cho ô số trang và ảnh thu nhỏ được đánh dấu.
    pub fn current_page(&self) -> usize {
        if let Some(a) = self.anchor_page {
            return a;
        }
        match self.layout {
            LayoutMode::SinglePage => self.single_page,
            LayoutMode::Continuous => {
                // Trang đầu tiên còn chiếm ít nhất 10% chiều cao vùng xem tính từ mép trên.
                let probe = self.scroll_y + (self.viewport_h * 0.1).min(PAGE_GAP * 4.0);
                self.page_at_content_y(probe)
            }
        }
    }

    /// Trang chứa (hoặc gần nhất với) tọa độ y trong không gian nội dung.
    fn page_at_content_y(&self, y: f32) -> usize {
        let n = self.pages.len();
        // Tìm nhị phân trang cuối cùng có page_top <= y.
        let (mut lo, mut hi) = (0usize, n);
        while lo + 1 < hi {
            let mid = (lo + hi) / 2;
            if self.page_top(mid) <= y {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        // Nếu y nằm trong khoảng trống sau trang lo thì coi như đã sang trang kế.
        if y > self.page_bottom(lo) && lo + 1 < n {
            lo + 1
        } else {
            lo
        }
    }

    /// Các trang đang nhìn thấy (ít nhất một phần), tọa độ so với góc trên trái vùng xem.
    pub fn visible_pages(&self) -> Vec<VisiblePage> {
        let s = self.scale();
        let make = |i: usize| {
            let size = self.page_size(i);
            VisiblePage {
                index: i,
                x: self.page_left(i) - self.scroll_x,
                y: self.page_top(i) - self.scroll_y,
                width: size.width * s,
                height: size.height * s,
            }
        };
        match self.layout {
            LayoutMode::SinglePage => vec![make(self.single_page)],
            LayoutMode::Continuous => {
                let mut out = Vec::new();
                let first = self.page_at_content_y(self.scroll_y).saturating_sub(1);
                for i in first..self.pages.len() {
                    let top = self.page_top(i);
                    if top > self.scroll_y + self.viewport_h {
                        break;
                    }
                    if self.page_bottom(i) >= self.scroll_y {
                        out.push(make(i));
                    }
                }
                out
            }
        }
    }

    /// Đổi tọa độ trong vùng xem sang (trang, x, y) với x, y là point PDF tính từ
    /// góc trên trái của trang. Điểm nằm ngoài trang vẫn trả về trang gần nhất.
    pub fn screen_to_page(&self, sx: f32, sy: f32) -> (usize, f32, f32) {
        let cy = sy + self.scroll_y;
        let cx = sx + self.scroll_x;
        let i = match self.layout {
            LayoutMode::SinglePage => self.single_page,
            LayoutMode::Continuous => self.page_at_content_y(cy),
        };
        let s = self.scale();
        (i, (cx - self.page_left(i)) / s, (cy - self.page_top(i)) / s)
    }

    /// Ngược lại với [`screen_to_page`].
    pub fn page_to_screen(&self, page: usize, x_pt: f32, y_pt: f32) -> (f32, f32) {
        let s = self.scale();
        (self.page_left(page) + x_pt * s - self.scroll_x, self.page_top(page) + y_pt * s - self.scroll_y)
    }

    // ----- thay đổi -----

    /// Đặt kích thước vùng xem. Nếu đang ở Vừa trang / Vừa chiều rộng thì tính lại mức
    /// phóng và giữ trang hiện tại ở đầu vùng xem.
    pub fn set_viewport(&mut self, w: f32, h: f32) {
        let w = w.max(1.0);
        let h = h.max(1.0);
        if (w - self.viewport_w).abs() < 0.01 && (h - self.viewport_h).abs() < 0.01 {
            return;
        }
        let pos = self.pos();
        let was_centered = self.is_centered();
        self.viewport_w = w;
        self.viewport_h = h;
        if self.fit != FitMode::Free {
            self.apply_fit(pos.page);
            self.restore_y(pos.page, pos.y_in_page_pt);
            self.center_horizontally();
        } else if was_centered {
            self.center_horizontally();
            self.clamp();
        } else {
            // Ví dụ thanh cuộn ngang vừa hiện ra giữa lúc phóng: không giật vị trí.
            let (x, y) = (self.scroll_x, self.scroll_y);
            self.clamp_soft(x, y);
        }
    }

    fn is_centered(&self) -> bool {
        let want = (self.content_width() - self.viewport_w) / 2.0;
        (self.scroll_x - want).abs() < 1.0
    }

    fn center_horizontally(&mut self) {
        self.scroll_x = (self.content_width() - self.viewport_w) / 2.0;
        self.clamp();
    }

    fn clamp(&mut self) {
        let (lo, hi) = self.scroll_x_range();
        self.scroll_x = self.scroll_x.clamp(lo, hi);
        self.scroll_y = self.scroll_y.clamp(0.0, self.max_scroll_y());
    }

    /// Giới hạn "mềm" khi cuộn: nếu vị trí trước đó nằm ngoài khoảng hợp lệ (do phóng
    /// tại con trỏ), cho phép đứng yên hoặc đi về phía khoảng hợp lệ, nhưng không xa thêm.
    fn clamp_soft(&mut self, prev_x: f32, prev_y: f32) {
        let (lo, hi) = self.scroll_x_range();
        self.scroll_x = self.scroll_x.clamp(lo.min(prev_x), hi.max(prev_x));
        let max_y = self.max_scroll_y();
        self.scroll_y = self.scroll_y.clamp(0.0f32.min(prev_y), max_y.max(prev_y));
    }

    fn zoom_for_fit(&self, mode: FitMode, page: usize) -> f32 {
        let size = self.page_size(page);
        let fit_w = self.viewport_w / (size.width * PX_PER_PT_AT_100);
        let fit_h = (self.viewport_h - 2.0 * MARGIN_Y).max(1.0) / (size.height * PX_PER_PT_AT_100);
        let z = match mode {
            FitMode::FitWidth => fit_w,
            FitMode::FitPage => fit_w.min(fit_h),
            FitMode::Free => self.zoom,
        };
        z.clamp(ZOOM_MIN, ZOOM_MAX)
    }

    fn apply_fit(&mut self, page: usize) {
        self.zoom = self.zoom_for_fit(self.fit, page);
    }

    /// Chọn Vừa trang, Vừa chiều rộng hoặc trở về Free.
    pub fn set_fit(&mut self, mode: FitMode) {
        let pos = self.pos();
        self.fit = mode;
        if mode != FitMode::Free {
            self.apply_fit(pos.page);
            // Vừa trang: đưa mép trên trang về đầu vùng xem để thấy trọn trang.
            let y = if mode == FitMode::FitPage { 0.0 } else { pos.y_in_page_pt };
            self.restore_y(pos.page, y);
            self.center_horizontally();
        }
    }

    /// Kích thước thật (100%).
    pub fn set_actual_size(&mut self) {
        let (w, h) = (self.viewport_w / 2.0, self.viewport_h / 2.0);
        self.zoom_at(1.0, w, h);
    }

    /// Đặt mức phóng, giữ nguyên điểm của tài liệu đang nằm tại (ax, ay) trong vùng xem.
    pub fn zoom_at(&mut self, new_zoom: f32, ax: f32, ay: f32) {
        let new_zoom = new_zoom.clamp(ZOOM_MIN, ZOOM_MAX);
        self.fit = FitMode::Free;
        self.anchor_page = None;
        let (page, px, py) = self.screen_to_page(ax, ay);
        let centered = self.is_centered() && self.content_width() <= self.viewport_w;
        self.zoom = new_zoom;
        let s = Self::scale_for(new_zoom);
        self.scroll_x = self.page_left(page) + px * s - ax;
        self.scroll_y = self.page_top(page) + py * s - ay;
        // Trang nhỏ hơn vùng xem trước và sau khi phóng: vẫn được canh giữa nếu
        // con trỏ ở ngoài trang (không có từ nào để giữ dưới con trỏ).
        let size = self.page_size(page);
        let outside = px < 0.0 || py < 0.0 || px > size.width || py > size.height;
        if outside {
            // Con trỏ ở lề hoặc khoảng trống giữa hai trang: không có chữ nào cần giữ.
            if centered && self.content_width() <= self.viewport_w {
                self.scroll_x = (self.content_width() - self.viewport_w) / 2.0;
            }
            self.clamp();
        }
        // Con trỏ ở trên trang: giữ đúng điểm dưới con trỏ, kể cả khi trang tạm lệch
        // khỏi khoảng cuộn thông thường; lần cuộn sau sẽ đưa dần về (clamp_soft).
    }

    /// Phóng theo hệ số (ví dụ từ Ctrl + con lăn hoặc thao tác chụm), tại điểm (ax, ay).
    pub fn zoom_by(&mut self, factor: f32, ax: f32, ay: f32) {
        if factor.is_finite() && factor > 0.0 && (factor - 1.0).abs() > 1e-6 {
            self.zoom_at(self.zoom * factor, ax, ay);
        }
    }

    /// Nấc phóng to / thu nhỏ quanh tâm vùng xem.
    pub fn zoom_step(&mut self, zoom_in: bool) {
        let f = if zoom_in { ZOOM_STEP } else { 1.0 / ZOOM_STEP };
        self.zoom_by(f, self.viewport_w / 2.0, self.viewport_h / 2.0);
    }

    /// Đặt mức phóng quanh tâm vùng xem.
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom_at(zoom, self.viewport_w / 2.0, self.viewport_h / 2.0);
    }

    pub fn set_layout(&mut self, layout: LayoutMode) {
        if layout == self.layout {
            return;
        }
        let pos = self.pos();
        self.layout = layout;
        self.single_page = pos.page;
        if self.fit != FitMode::Free {
            self.apply_fit(pos.page);
        }
        self.restore_y(pos.page, pos.y_in_page_pt);
        self.center_horizontally();
    }

    /// Cuộn theo độ lệch (điểm logic, dương = xuống/phải). Trong chế độ từng trang,
    /// cuộn quá mép trang sẽ chuyển sang trang kế / trang trước.
    pub fn scroll_by(&mut self, dx: f32, dy: f32) {
        self.anchor_page = None;
        if self.layout == LayoutMode::SinglePage && dy != 0.0 {
            let max = self.max_scroll_y();
            if dy > 0.0 && self.scroll_y >= max - 0.5 && self.single_page + 1 < self.pages.len() {
                self.single_page += 1;
                self.after_single_page_change();
                self.scroll_y = 0.0;
                self.scroll_x += dx;
                self.clamp();
                return;
            }
            if dy < 0.0 && self.scroll_y <= 0.5 && self.single_page > 0 {
                self.single_page -= 1;
                self.after_single_page_change();
                self.scroll_y = self.max_scroll_y();
                self.scroll_x += dx;
                self.clamp();
                return;
            }
        }
        let (px, py) = (self.scroll_x, self.scroll_y);
        self.scroll_x += dx;
        self.scroll_y += dy;
        self.clamp_soft(px, py);
    }

    /// Đặt vị trí cuộn tuyệt đối (ví dụ khi kéo thanh cuộn).
    pub fn set_scroll(&mut self, x: f32, y: f32) {
        self.anchor_page = None;
        self.scroll_x = x;
        self.scroll_y = y;
        self.clamp();
    }

    fn after_single_page_change(&mut self) {
        if self.fit != FitMode::Free {
            self.apply_fit(self.single_page);
        }
        self.center_horizontally();
    }

    /// Đưa trang `index` (tính từ 0) lên đầu vùng xem.
    pub fn goto_page(&mut self, index: usize) {
        self.goto_dest(index, None);
    }

    /// Đi tới đích: trang và (tùy chọn) vị trí y tính từ mép trên trang (point).
    pub fn goto_dest(&mut self, index: usize, y_from_top_pt: Option<f32>) {
        let index = index.min(self.pages.len() - 1);
        if self.layout == LayoutMode::SinglePage {
            self.single_page = index;
            self.after_single_page_change();
        } else if self.fit == FitMode::FitWidth || self.fit == FitMode::FitPage {
            // Trang có thể khác kích thước; giữ chế độ khớp cho trang mới.
            let z = self.zoom_for_fit(self.fit, index);
            if (z - self.zoom).abs() > 1e-4 {
                self.zoom = z;
                self.center_horizontally();
            }
        }
        self.restore_y(index, y_from_top_pt.unwrap_or(0.0));
        self.anchor_page = Some(index);
    }

    fn restore_y(&mut self, page: usize, y_in_page_pt: f32) {
        let page = page.min(self.pages.len() - 1);
        if self.layout == LayoutMode::SinglePage {
            self.single_page = page;
        }
        self.scroll_y = self.page_top(page) + y_in_page_pt * self.scale();
        self.clamp();
    }

    pub fn next_page(&mut self) {
        let c = self.current_page();
        if c + 1 < self.pages.len() {
            self.goto_page(c + 1);
        }
    }

    pub fn prev_page(&mut self) {
        let c = self.current_page();
        if c > 0 {
            self.goto_page(c - 1);
        }
    }

    pub fn first_page(&mut self) {
        self.goto_page(0);
    }

    pub fn last_page(&mut self) {
        self.goto_page(self.pages.len() - 1);
    }

    /// Page Down: cuộn liên tục thì cuộn gần một màn hình; từng trang thì sang trang kế.
    pub fn page_down(&mut self) {
        match self.layout {
            LayoutMode::Continuous => self.scroll_by(0.0, (self.viewport_h - LINE_SCROLL).max(LINE_SCROLL)),
            LayoutMode::SinglePage => self.next_page(),
        }
    }

    pub fn page_up(&mut self) {
        match self.layout {
            LayoutMode::Continuous => self.scroll_by(0.0, -(self.viewport_h - LINE_SCROLL).max(LINE_SCROLL)),
            LayoutMode::SinglePage => self.prev_page(),
        }
    }

    /// Có cuộn ngang được không (trang rộng hơn vùng xem).
    pub fn has_horizontal_overflow(&self) -> bool {
        self.content_width() > self.viewport_w + 0.5
    }

    /// Vị trí đọc hiện tại.
    pub fn pos(&self) -> ViewPos {
        let page = self.current_page();
        let y = (self.scroll_y - self.page_top(page)) / self.scale();
        ViewPos { page, y_in_page_pt: y, zoom: self.zoom, fit: self.fit }
    }

    /// Khôi phục một vị trí đọc đã lưu.
    pub fn restore(&mut self, pos: ViewPos) {
        let page = pos.page.min(self.pages.len() - 1);
        self.fit = pos.fit;
        self.zoom = pos.zoom.clamp(ZOOM_MIN, ZOOM_MAX);
        if self.layout == LayoutMode::SinglePage {
            self.single_page = page;
        }
        if self.fit != FitMode::Free {
            self.apply_fit(page);
        }
        self.center_horizontally();
        self.restore_y(page, pos.y_in_page_pt);
        self.anchor_page = if pos.y_in_page_pt.abs() < 0.5 { Some(page) } else { None };
    }

    /// Tỉ lệ thanh cuộn dọc: (vị trí đầu, độ dài) trong khoảng 0..1.
    pub fn vertical_thumb(&self) -> (f32, f32) {
        let ch = self.content_height().max(1.0);
        let len = (self.viewport_h / ch).min(1.0);
        (self.scroll_y / ch, len)
    }

    pub fn horizontal_thumb(&self) -> (f32, f32) {
        let cw = self.content_width().max(1.0);
        let len = (self.viewport_w / cw).min(1.0);
        ((self.scroll_x.max(0.0)) / cw, len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(n: usize) -> ViewState {
        let mut v = ViewState::new(vec![PageSize::A4; n]);
        v.set_viewport(1000.0, 700.0);
        v
    }

    /// Scenario: Nhảy tới trang — trang 125 ở đầu vùng xem.
    #[test]
    fn goto_page_puts_page_at_top() {
        let mut v = doc(500);
        v.goto_page(124);
        assert_eq!(v.current_page(), 124);
        let vis = v.visible_pages();
        let p = vis.iter().find(|p| p.index == 124).unwrap();
        assert!(p.y.abs() < 0.5, "trang 125 phải ở mép trên, y = {}", p.y);
    }

    /// Nhảy tới trang cuối vẫn báo đúng trang dù không cuộn được tới mép trên.
    #[test]
    fn goto_last_page_reports_last_page() {
        let mut v = doc(500);
        v.goto_page(499);
        assert_eq!(v.current_page(), 499);
        v.scroll_by(0.0, -1.0);
        assert!(v.current_page() <= 499);
    }

    /// Scenario: Cuộn liên tục — cuộn qua cuối trang thì trang kế hiện liền bên dưới.
    #[test]
    fn continuous_scroll_shows_next_page_below() {
        let mut v = doc(10);
        v.set_zoom(1.0);
        let bottom0 = v.page_bottom(0);
        v.set_scroll(v.scroll().0, bottom0 - 300.0);
        let vis = v.visible_pages();
        assert!(vis.iter().any(|p| p.index == 0));
        let p1 = vis.iter().find(|p| p.index == 1).expect("trang 2 phải hiện ngay bên dưới");
        let p0 = vis.iter().find(|p| p.index == 0).unwrap();
        assert!((p1.y - (p0.y + p0.height + PAGE_GAP)).abs() < 0.01);
        assert_eq!(v.layout(), LayoutMode::Continuous);
        // Cuộn tiếp bằng con lăn, không cần lệnh chuyển trang.
        v.scroll_by(0.0, 400.0);
        assert_eq!(v.current_page(), 1);
    }

    #[test]
    fn single_page_mode_turns_page_at_edge() {
        let mut v = doc(5);
        v.set_layout(LayoutMode::SinglePage);
        assert_eq!(v.visible_pages().len(), 1);
        for _ in 0..100 {
            v.scroll_by(0.0, 100.0);
        }
        assert!(v.current_page() >= 1);
        v.page_down();
        let c = v.current_page();
        v.page_up();
        assert_eq!(v.current_page(), c - 1);
    }

    /// Scenario: Vừa chiều rộng — chiều rộng trang bằng chiều rộng vùng xem và giữ khi đổi cỡ.
    #[test]
    fn fit_width_tracks_viewport() {
        let mut v = doc(20);
        v.goto_page(7);
        v.set_fit(FitMode::FitWidth);
        let w = v.page_size(7).width * v.scale();
        assert!((w - 1000.0).abs() < 0.01, "w = {w}");
        v.set_viewport(1400.0, 900.0);
        assert_eq!(v.fit(), FitMode::FitWidth);
        let w = v.page_size(7).width * v.scale();
        assert!((w - 1400.0).abs() < 0.01, "w = {w}");
        assert_eq!(v.current_page(), 7);
        let p = v.visible_pages().into_iter().find(|p| p.index == 7).unwrap();
        assert!(p.x.abs() < 0.01, "trang phải sát mép trái, x = {}", p.x);
    }

    #[test]
    fn fit_page_shows_whole_page() {
        let mut v = doc(3);
        v.set_fit(FitMode::FitPage);
        let p = v.visible_pages()[0];
        assert!(p.height <= 700.0 + 0.01 && p.width <= 1000.0 + 0.01);
        assert!(p.y >= 0.0 && p.y + p.height <= 700.0 + 0.01);
    }

    /// Scenario: Phóng tại con trỏ — điểm dưới con trỏ giữ nguyên.
    #[test]
    fn zoom_keeps_point_under_cursor() {
        for &(start, factor) in &[(1.5f32, 1.1f32), (1.0, 1.25), (0.5, 1.1), (3.0, 2.0), (1.0, 0.8)] {
            let mut v = doc(50);
            v.set_zoom(start);
            v.goto_page(10);
            v.scroll_by(13.0, 200.0);
            // Con trỏ ở (520, 330) trong vùng xem, đang nằm trên một "từ" của trang.
            let (sx, sy) = (520.0, 330.0);
            let (page, wx, wy) = v.screen_to_page(sx, sy);
            let size = v.page_size(page);
            assert!(wx > 0.0 && wy > 0.0 && wx < size.width && wy < size.height, "con trỏ phải ở trên trang");
            v.zoom_by(factor, sx, sy);
            let (nx, ny) = v.page_to_screen(page, wx, wy);
            assert!((nx - sx).abs() < 0.5 && (ny - sy).abs() < 0.5,
                "zoom {start}×{factor}: ({sx},{sy}) -> ({nx},{ny})");
            assert!(v.zoom() > start || factor < 1.0);
        }
    }

    #[test]
    fn zoom_is_clamped_to_range() {
        let mut v = doc(3);
        v.set_zoom(1000.0);
        assert_eq!(v.zoom_percent(), 6400);
        v.set_zoom(0.001);
        assert_eq!(v.zoom_percent(), 10);
        v.set_actual_size();
        assert_eq!(v.zoom_percent(), 100);
        assert_eq!(v.fit(), FitMode::Free);
    }

    #[test]
    fn pos_roundtrip() {
        let mut v = doc(100);
        v.set_zoom(1.5);
        v.goto_page(39);
        v.scroll_by(0.0, 123.0);
        let pos = v.pos();
        let mut w = doc(100);
        w.restore(pos);
        assert_eq!(w.current_page(), v.current_page());
        assert_eq!(w.zoom_percent(), 150);
        assert!((w.scroll().1 - v.scroll().1).abs() < 0.5);
    }

    #[test]
    fn screen_page_roundtrip() {
        let mut v = doc(30);
        v.set_zoom(2.0);
        v.goto_page(5);
        v.scroll_by(40.0, 77.0);
        let (p, x, y) = v.screen_to_page(321.0, 222.0);
        let (sx, sy) = v.page_to_screen(p, x, y);
        assert!((sx - 321.0).abs() < 0.01 && (sy - 222.0).abs() < 0.01);
    }

    #[test]
    fn visible_pages_are_virtualised() {
        let mut v = doc(500);
        v.set_zoom(1.0);
        v.goto_page(250);
        let vis = v.visible_pages();
        assert!(vis.len() <= 3, "chỉ vài trang được dựng, có {}", vis.len());
        assert!(vis.iter().any(|p| p.index == 250));
    }
}
