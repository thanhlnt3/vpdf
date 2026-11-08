//! Cửa sổ chính: thanh tab, thanh công cụ (ô số trang, phóng), bảng bên (ảnh thu
//! nhỏ, mục lục), vùng xem, hộp thoại (mật khẩu, lỗi, xác nhận mở link).

use crate::tab::{DocTab, TabState};
use crate::viewer::{self, ViewerOutput};
use egui::{
    Align, Align2, Color32, Context, CornerRadius, FontId, Id, Key, Layout, Modifiers, Panel, Pos2, Rect, RichText,
    ScrollArea, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2,
};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use vpdf_core::links::LinkAction;
use vpdf_core::messages as m;
use vpdf_core::recent::RecentFiles;
use vpdf_core::tabs::TabSet;
use vpdf_core::text::{copy_selection, CopyOutcome};
use vpdf_core::view::{FitMode, LayoutMode, ZOOM_PRESETS};
use vpdf_render::{OutlineNode, THUMB_WIDTH};
use vpdf_core::links::LinkTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideTab {
    Thumbnails,
    Outline,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Dialog {
    /// "Không đọc được file PDF" kèm tên file.
    OpenError { file: String },
    /// "Mở liên kết này trong trình duyệt?"
    ConfirmUrl(String),
}

/// Những gì thanh công cụ và thanh tab đã vẽ ở khung hình gần nhất (dùng cho tự kiểm tra).
#[derive(Debug, Clone, Default)]
pub struct Hud {
    pub page_box: String,
    pub zoom: String,
    pub tab_titles: Vec<String>,
    pub thumbnails_drawn: usize,
    pub window_title: String,
}

pub struct VpdfApp {
    pub tabs: TabSet<DocTab>,
    pub recent: RecentFiles,
    pub recent_path: Option<PathBuf>,
    pub side: SideTab,
    pub show_side: bool,
    pub dialog: Option<Dialog>,
    pub toast: Option<(String, Instant)>,
    pub hud: Hud,
    /// Link URL người dùng đã đồng ý mở (để kiểm thử).
    pub opened_urls: Vec<String>,
    next_id: u64,
    last_current_page: Option<(u64, usize)>,
    scroll_thumbs_to_current: bool,
    page_box_id: Id,
}

impl VpdfApp {
    /// `recent_path = None`: không đọc/ghi danh sách gần đây (kiểm thử).
    pub fn new(recent_path: Option<PathBuf>) -> Self {
        let recent = recent_path.as_deref().map(RecentFiles::load).unwrap_or_default();
        VpdfApp {
            tabs: TabSet::default(),
            recent,
            recent_path,
            side: SideTab::Thumbnails,
            show_side: true,
            dialog: None,
            toast: None,
            hud: Hud::default(),
            opened_urls: Vec::new(),
            next_id: 1,
            last_current_page: None,
            scroll_thumbs_to_current: false,
            page_box_id: Id::new("page-box"),
        }
    }

    pub fn page_box_id(&self) -> Id {
        self.page_box_id
    }

    /// Mở file trong tab mới (hoặc chuyển sang tab đang mở file đó).
    pub fn open_path(&mut self, ctx: &Context, path: &Path) {
        let key = vpdf_core::recent::path_key(&vpdf_core::recent::normalize_path(path));
        if let Some(i) = self.tabs.position(|t| vpdf_core::recent::path_key(&vpdf_core::recent::normalize_path(&t.path)) == key) {
            self.tabs.select(i);
            return;
        }
        let restore = self.recent.lookup(path).map(|e| e.pos());
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(DocTab::open(id, path, ctx, restore));
    }

    pub fn open_dialog(&mut self, ctx: &Context) {
        if let Some(files) = rfd::FileDialog::new().set_title(m::OPEN_FILE_TITLE).add_filter("PDF", &["pdf", "PDF"]).pick_files() {
            for f in files {
                self.open_path(ctx, &f);
            }
        }
    }

    pub fn active(&self) -> Option<&DocTab> {
        self.tabs.active()
    }

    pub fn active_mut(&mut self) -> Option<&mut DocTab> {
        self.tabs.active_mut()
    }

    fn remember(&mut self, index: usize) {
        if let Some(t) = self.tabs.get(index) {
            if t.is_ready() {
                let (p, pos) = (t.path.clone(), t.view.pos());
                self.recent.remember(&p, pos);
            }
        }
    }

    pub fn save_recent(&mut self) {
        for i in 0..self.tabs.len() {
            self.remember(i);
        }
        if let Some(p) = &self.recent_path {
            let _ = self.recent.save(p);
        }
    }

    pub fn close_tab(&mut self, index: usize) {
        self.remember(index);
        self.tabs.close(index);
        if let Some(p) = &self.recent_path {
            let _ = self.recent.save(p);
        }
    }

    pub fn toast(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), Instant::now()));
    }

    /// Sao chép vùng chọn của tab hiện tại.
    pub fn copy(&mut self, ctx: &Context) {
        let Some(tab) = self.tabs.active() else { return };
        let outcome = copy_selection(&tab.selected_text(), tab.copy_allowed());
        match outcome {
            CopyOutcome::Copy(s) => {
                ctx.copy_text(s);
                self.toast(m::COPIED);
            }
            CopyOutcome::Denied(msg) => self.toast(msg),
            CopyOutcome::Nothing => {}
        }
    }

    // -------------------------------------------------------------------
    // Khung hình

    pub fn ui(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        self.poll_tabs(&ctx);
        self.handle_dropped_files(&ctx);
        self.handle_keys(&ctx);
        self.update_title(&ctx);

        Panel::top("menu").show(ui, |ui| self.menu_bar(ui));
        Panel::top("tabs").show(ui, |ui| self.tab_bar(ui));
        Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        let mut thumbs_wanted: Vec<u32> = Vec::new();
        if self.show_side && self.tabs.active().is_some_and(|t| t.is_ready()) {
            Panel::left("side").default_size(190.0).size_range(140.0..=420.0).show(ui, |ui| {
                self.side_panel(ui, &mut thumbs_wanted);
            });
        }
        let mut vout = ViewerOutput::default();
        egui::CentralPanel::no_frame().show(ui, |ui| self.central(ui, &mut vout));

        // Ảnh thu nhỏ: trang đang hiện trong bảng bên trước, rồi trang đang đọc (làm hình tạm).
        if let Some(tab) = self.tabs.active_mut() {
            if tab.is_ready() {
                let mut list = thumbs_wanted;
                for p in &vout.visible_pages {
                    if !list.contains(p) {
                        list.push(*p);
                    }
                }
                list.retain(|p| !tab.thumbs.contains(p));
                tab.worker.set_wanted_thumbs(list, THUMB_WIDTH);
            }
        }
        self.link_action(&ctx, vout.link_action.take());
        self.dialogs(&ctx);
        self.draw_toast(&ctx, vout.rect);
        self.drop_overlay(&ctx);
    }

    fn poll_tabs(&mut self, ctx: &Context) {
        let mut failed = Vec::new();
        for (i, tab) in self.tabs.iter_mut().enumerate() {
            tab.poll(ctx);
            if tab.is_ready() && !tab.restored {
                tab.restored = true;
                if let Some(pos) = tab.restore_pos.take() {
                    if pos.page < tab.page_count() {
                        tab.view.restore(pos);
                    }
                }
            }
            if let TabState::Failed(_) = tab.state {
                failed.push((i, tab.title.clone()));
            }
        }
        // Tab không mở được: đóng tab đó, các tab khác giữ nguyên.
        for (i, title) in failed.into_iter().rev() {
            self.tabs.close(i);
            self.dialog = Some(Dialog::OpenError { file: title });
        }
    }

    fn handle_dropped_files(&mut self, ctx: &Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        for p in dropped {
            self.open_path(ctx, &p);
        }
    }

    fn drop_overlay(&self, ctx: &Context) {
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        if hovering {
            let r = ctx.content_rect();
            let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, Id::new("drop")));
            p.rect_filled(r, CornerRadius::ZERO, Color32::from_black_alpha(140));
            p.text(r.center(), Align2::CENTER_CENTER, "Thả file PDF để mở", FontId::proportional(24.0), Color32::WHITE);
        }
    }

    fn handle_keys(&mut self, ctx: &Context) {
        if self.dialog.is_some() {
            return;
        }
        let typing = ctx.egui_wants_keyboard_input();
        let cmd = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, k));
        if cmd(Key::O) {
            self.open_dialog(ctx);
        }
        if cmd(Key::W) {
            if let Some(i) = self.tabs.active_index() {
                self.close_tab(i);
            }
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Tab)) && !self.tabs.is_empty() {
            let n = self.tabs.len();
            let next = (self.tabs.active_index().unwrap_or(0) + 1) % n;
            self.tabs.select(next);
        }
        if !typing && cmd(Key::C) {
            self.copy(ctx);
        }
        // egui chuyển Ctrl+C thành sự kiện Copy khi không có ô chữ nào giữ tiêu điểm.
        if !typing && ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy))) {
            self.copy(ctx);
        }
        let Some(tab) = self.tabs.active_mut() else { return };
        if !tab.is_ready() {
            return;
        }
        let v = &mut tab.view;
        if cmd(Key::Equals) || cmd(Key::Plus) {
            v.zoom_step(true);
        }
        if cmd(Key::Minus) {
            v.zoom_step(false);
        }
        if cmd(Key::Num0) {
            v.set_fit(FitMode::FitPage);
        }
        if cmd(Key::Num1) {
            v.set_actual_size();
        }
        if cmd(Key::Num2) {
            v.set_fit(FitMode::FitWidth);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::ALT, Key::ArrowLeft)) {
            tab.go_back();
        }
        if typing {
            return;
        }
        let pressed = |k: Key| ctx.input(|i| i.key_pressed(k) && !i.modifiers.alt && !i.modifiers.command);
        let line = vpdf_core::view::LINE_SCROLL;
        let v = &mut tab.view;
        if pressed(Key::ArrowDown) {
            v.scroll_by(0.0, line);
        }
        if pressed(Key::ArrowUp) {
            v.scroll_by(0.0, -line);
        }
        if pressed(Key::ArrowRight) {
            if v.has_horizontal_overflow() {
                v.scroll_by(line, 0.0);
            } else {
                v.next_page();
            }
        }
        if pressed(Key::ArrowLeft) {
            if v.has_horizontal_overflow() {
                v.scroll_by(-line, 0.0);
            } else {
                v.prev_page();
            }
        }
        if pressed(Key::PageDown) || pressed(Key::Space) {
            v.page_down();
        }
        if pressed(Key::PageUp) {
            v.page_up();
        }
        if pressed(Key::Home) {
            let before = v.pos();
            tab.history.push(before);
            tab.view.first_page();
        }
        if pressed(Key::End) {
            let before = tab.view.pos();
            tab.history.push(before);
            tab.view.last_page();
        }
    }

    fn update_title(&mut self, ctx: &Context) {
        let title = match self.tabs.active() {
            Some(t) => format!("{} — {}", t.title, vpdf_core::PRODUCT_NAME),
            None => vpdf_core::PRODUCT_NAME.to_string(),
        };
        if title != self.hud.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.hud.window_title = title;
        }
    }

    fn menu_bar(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(m::FILE_MENU, |ui| {
                if ui.button(format!("{}    Ctrl+O", m::OPEN)).clicked() {
                    ui.close();
                    self.open_dialog(&ctx);
                }
                ui.menu_button(m::RECENT, |ui| {
                    if self.recent.entries.is_empty() {
                        ui.label(m::NO_RECENT);
                    }
                    let entries: Vec<(String, String)> =
                        self.recent.entries.iter().map(|e| (e.file_name(), e.path.clone())).collect();
                    for (name, path) in entries {
                        if ui.button(&name).on_hover_text(&path).clicked() {
                            ui.close();
                            let p = PathBuf::from(&path);
                            if p.is_file() {
                                self.open_path(&ctx, &p);
                            } else {
                                self.recent.forget(&p);
                                self.dialog = Some(Dialog::OpenError { file: name });
                            }
                        }
                    }
                });
                ui.separator();
                let can_close = self.tabs.active_index();
                if ui.add_enabled(can_close.is_some(), egui::Button::new(format!("{}    Ctrl+W", m::CLOSE_TAB))).clicked() {
                    ui.close();
                    if let Some(i) = can_close {
                        self.close_tab(i);
                    }
                }
            });
            ui.menu_button(m::VIEW_MENU, |ui| {
                ui.checkbox(&mut self.show_side, "Bảng bên");
                let Some(tab) = self.tabs.active_mut() else { return };
                let mut layout = tab.view.layout();
                ui.radio_value(&mut layout, LayoutMode::Continuous, m::CONTINUOUS);
                ui.radio_value(&mut layout, LayoutMode::SinglePage, m::SINGLE_PAGE);
                if layout != tab.view.layout() {
                    tab.view.set_layout(layout);
                }
            });
        });
    }

    fn tab_bar(&mut self, ui: &mut Ui) {
        self.hud.tab_titles.clear();
        let mut close = None;
        let mut select = None;
        ui.horizontal(|ui| {
            let active = self.tabs.active_index();
            for (i, t) in self.tabs.iter().enumerate() {
                self.hud.tab_titles.push(t.title.clone());
                let selected = Some(i) == active;
                ui.scope(|ui| {
                    let label = if t.state == TabState::Opening { format!("{} …", t.title) } else { t.title.clone() };
                    let r = ui.selectable_label(selected, label).on_hover_text(t.path.display().to_string());
                    if r.clicked() {
                        select = Some(i);
                    }
                    if ui.small_button("×").on_hover_text(m::CLOSE_TAB).clicked() {
                        close = Some(i);
                    }
                });
                ui.separator();
            }
            if ui.button(format!("+ {}", m::OPEN)).clicked() {
                let ctx = ui.ctx().clone();
                self.open_dialog(&ctx);
            }
        });
        if let Some(i) = select {
            // Ghi vị trí đọc của tab đang rời đi (vào danh sách gần đây).
            if let Some(a) = self.tabs.active_index() {
                self.remember(a);
            }
            self.tabs.select(i);
        }
        if let Some(i) = close {
            self.close_tab(i);
        }
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        let page_box_id = self.page_box_id;
        let ctx = ui.ctx().clone();
        ui.horizontal(|ui| {
            if ui.button(m::OPEN).clicked() {
                self.open_dialog(&ctx);
            }
            let Some(tab) = self.tabs.active_mut() else {
                self.hud.page_box.clear();
                self.hud.zoom.clear();
                return;
            };
            if !tab.is_ready() {
                return;
            }
            ui.separator();
            if ui.add_enabled(tab.history.can_go_back(), egui::Button::new(format!("‹ {}", m::BACK))).clicked() {
                tab.go_back();
            }
            ui.separator();
            let total = tab.page_count();
            let current = tab.view.current_page();
            if ui.button("‹").on_hover_text("Trang trước").clicked() {
                tab.view.prev_page();
            }
            // Ô số trang: "trang / tổng".
            let mut text = tab.page_box.text(current, total);
            let r = ui.add(TextEdit::singleline(&mut text).id(page_box_id).desired_width(90.0).horizontal_align(Align::Center));
            if r.gained_focus() || r.has_focus() {
                if tab.page_box.editing.is_none() {
                    tab.page_box.editing = Some(text.clone());
                } else {
                    tab.page_box.editing = Some(text);
                }
            }
            if r.lost_focus() {
                if ui.input(|i| i.key_pressed(Key::Enter)) {
                    match tab.page_box.commit(total) {
                        Some(p) => tab.navigate(p, None),
                        None => tab.page_box.cancel(),
                    }
                } else {
                    tab.page_box.cancel();
                }
            }
            self.hud.page_box = tab.page_box.text(tab.view.current_page(), total);
            if ui.button("›").on_hover_text("Trang sau").clicked() {
                tab.view.next_page();
            }
            ui.separator();
            if ui.button("−").on_hover_text(format!("{}  Ctrl+-", m::ZOOM_OUT)).clicked() {
                tab.view.zoom_step(false);
            }
            let zoom_label = match tab.view.fit() {
                FitMode::FitPage => format!("{} ({}%)", m::FIT_PAGE, tab.view.zoom_percent()),
                FitMode::FitWidth => format!("{} ({}%)", m::FIT_WIDTH, tab.view.zoom_percent()),
                FitMode::Free => format!("{}%", tab.view.zoom_percent()),
            };
            self.hud.zoom = zoom_label.clone();
            egui::ComboBox::from_id_salt("zoom").selected_text(zoom_label).width(150.0).show_ui(ui, |ui| {
                if ui.selectable_label(tab.view.fit() == FitMode::FitPage, format!("{}    Ctrl+0", m::FIT_PAGE)).clicked() {
                    tab.view.set_fit(FitMode::FitPage);
                }
                if ui.selectable_label(tab.view.fit() == FitMode::FitWidth, format!("{}    Ctrl+2", m::FIT_WIDTH)).clicked() {
                    tab.view.set_fit(FitMode::FitWidth);
                }
                if ui.selectable_label(false, format!("{} (100%)    Ctrl+1", m::ACTUAL_SIZE)).clicked() {
                    tab.view.set_actual_size();
                }
                ui.separator();
                for z in ZOOM_PRESETS {
                    let pct = (z * 100.0).round() as u32;
                    let sel = tab.view.fit() == FitMode::Free && tab.view.zoom_percent() == pct;
                    if ui.selectable_label(sel, format!("{pct}%")).clicked() {
                        tab.view.set_zoom(z);
                    }
                }
            });
            if ui.button("+").on_hover_text(format!("{}  Ctrl+=", m::ZOOM_IN)).clicked() {
                tab.view.zoom_step(true);
            }
            ui.separator();
            let mut layout = tab.view.layout();
            ui.selectable_value(&mut layout, LayoutMode::Continuous, m::CONTINUOUS);
            ui.selectable_value(&mut layout, LayoutMode::SinglePage, m::SINGLE_PAGE);
            if layout != tab.view.layout() {
                tab.view.set_layout(layout);
            }
        });
    }

    fn side_panel(&mut self, ui: &mut Ui, thumbs_wanted: &mut Vec<u32>) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.side, SideTab::Thumbnails, m::THUMBNAILS);
            ui.selectable_value(&mut self.side, SideTab::Outline, m::OUTLINE);
        });
        ui.separator();
        let Some(tab) = self.tabs.active_mut() else { return };
        let current = tab.view.current_page();
        if self.last_current_page != Some((tab.id, current)) {
            self.last_current_page = Some((tab.id, current));
            self.scroll_thumbs_to_current = true;
        }
        match self.side {
            SideTab::Thumbnails => {
                let n = tab.page_count();
                let row_h = 196.0;
                let mut clicked = None;
                let scroll_to = std::mem::take(&mut self.scroll_thumbs_to_current);
                let mut drawn = 0;
                let mut sa = ScrollArea::vertical().id_salt(("thumbs", tab.id)).auto_shrink([false, false]);
                if scroll_to {
                    // Giữ ảnh của trang hiện tại trong tầm nhìn của bảng.
                    sa = sa.vertical_scroll_offset((current as f32 * row_h - 2.0 * row_h).max(0.0));
                }
                sa.show_rows(ui, row_h, n, |ui, range| {
                    for p in range.clone() {
                        thumbs_wanted.push(p as u32);
                    }
                    // Tải trước một ít ảnh ngay dưới phần đang thấy.
                    for p in range.end..(range.end + 6).min(n) {
                        thumbs_wanted.push(p as u32);
                    }
                    for p in range {
                        let (resp, painter) = ui.allocate_painter(Vec2::new(ui.available_width(), row_h - 4.0), Sense::click());
                        let size = tab.view.page_size(p);
                        let box_h = row_h - 26.0;
                        let w = (box_h * size.width / size.height).min(resp.rect.width() - 16.0);
                        let h = w * size.height / size.width;
                        let r = Rect::from_center_size(
                            Pos2::new(resp.rect.center().x, resp.rect.min.y + 4.0 + h / 2.0),
                            Vec2::new(w, h),
                        );
                        if p == current {
                            painter.rect_filled(r.expand(4.0), CornerRadius::same(3), Color32::from_rgb(40, 110, 220));
                        }
                        painter.rect_filled(r, CornerRadius::ZERO, Color32::WHITE);
                        if let Some(t) = tab.thumbs.peek(&(p as u32)) {
                            painter.image(t.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                            drawn += 1;
                        }
                        painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, Color32::from_gray(120)), StrokeKind::Outside);
                        let label_col = if p == current { Color32::from_rgb(40, 110, 220) } else { ui.visuals().text_color() };
                        painter.text(
                            Pos2::new(resp.rect.center().x, r.max.y + 12.0),
                            Align2::CENTER_CENTER,
                            (p + 1).to_string(),
                            FontId::proportional(13.0),
                            label_col,
                        );
                        if resp.clicked() {
                            clicked = Some(p);
                        }
                    }
                });
                self.hud.thumbnails_drawn = drawn;
                if let Some(p) = clicked {
                    tab.navigate(p, None);
                }
            }
            SideTab::Outline => match &tab.outline {
                None => {
                    ui.spinner();
                }
                Some(o) if o.is_empty() => {
                    ui.label(m::NO_OUTLINE);
                }
                Some(o) => {
                    let o = o.clone();
                    let mut target = None;
                    ScrollArea::both().id_salt(("outline", tab.id)).auto_shrink([false, false]).show(ui, |ui| {
                        outline_tree(ui, &o, &mut target, 0);
                    });
                    if let Some(LinkTarget::Page { index, y_from_top_pt }) = target {
                        tab.navigate(index, y_from_top_pt);
                    }
                }
            },
        }
    }

    fn central(&mut self, ui: &mut Ui, vout: &mut ViewerOutput) {
        {
            let Some(tab) = self.tabs.active_mut() else {
                empty_state(ui);
                return;
            };
            match tab.state.clone() {
                TabState::Opening => {
                    ui.centered_and_justified(|ui| ui.label(m::LOADING));
                    return;
                }
                TabState::Failed(_) => return,
                TabState::NeedPassword { mut input, error } => {
                    let mut submit = false;
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.3);
                        ui.label(RichText::new(&tab.title).strong());
                        ui.label(m::PASSWORD_PROMPT);
                        let r = ui.add(TextEdit::singleline(&mut input).password(true).desired_width(260.0).id(Id::new(("pw", tab.id))));
                        if !r.has_focus() && !r.lost_focus() {
                            r.request_focus();
                        }
                        if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                            submit = true;
                        }
                        if let Some(e) = error {
                            ui.colored_label(Color32::from_rgb(220, 60, 60), e);
                        }
                        if ui.button(m::OK).clicked() {
                            submit = true;
                        }
                    });
                    tab.state = TabState::NeedPassword { input, error };
                    if submit {
                        tab.submit_password();
                    }
                    return;
                }
                TabState::Ready => {}
            }
            if let Some(w) = tab.warning.clone() {
                let mut dismiss = false;
                egui::Frame::new().fill(Color32::from_rgb(255, 236, 170)).inner_margin(6.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::from_rgb(90, 60, 0), w);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.small_button("×").clicked() {
                                dismiss = true;
                            }
                        });
                    });
                });
                if dismiss {
                    tab.warning = None;
                }
            }
            viewer::show(ui, tab, vout);
        }
    }

    fn link_action(&mut self, _ctx: &Context, action: Option<LinkAction>) {
        match action {
            Some(LinkAction::Navigate { page, y_from_top_pt }) => {
                if let Some(t) = self.tabs.active_mut() {
                    t.navigate(page, y_from_top_pt);
                }
            }
            Some(LinkAction::ConfirmOpenUrl(u)) => self.dialog = Some(Dialog::ConfirmUrl(u)),
            _ => {}
        }
    }

    fn dialogs(&mut self, ctx: &Context) {
        let Some(d) = self.dialog.clone() else { return };
        let mut close = false;
        egui::Modal::new(Id::new("dialog")).show(ctx, |ui| {
            ui.set_width(380.0);
            match &d {
                Dialog::OpenError { file } => {
                    ui.heading(m::CANNOT_READ_PDF);
                    ui.label(file);
                    ui.add_space(8.0);
                    if ui.button(m::OK).clicked() || ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Escape)) {
                        close = true;
                    }
                }
                Dialog::ConfirmUrl(url) => {
                    ui.heading(m::CONFIRM_OPEN_URL);
                    ui.label(RichText::new(url).monospace());
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(m::OPEN_LINK).clicked() {
                            ctx.open_url(egui::OpenUrl::new_tab(url));
                            self.opened_urls.push(url.clone());
                            close = true;
                        }
                        if ui.button(m::CANCEL).clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                            close = true;
                        }
                    });
                }
            }
        });
        if close {
            self.dialog = None;
        }
    }

    fn draw_toast(&mut self, ctx: &Context, area: Option<Rect>) {
        let Some((text, t0)) = &self.toast else { return };
        if t0.elapsed() > Duration::from_secs(3) {
            self.toast = None;
            return;
        }
        let r = area.unwrap_or_else(|| ctx.content_rect());
        let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, Id::new("toast")));
        let galley = p.layout_no_wrap(text.clone(), FontId::proportional(15.0), Color32::WHITE);
        let size = galley.size() + Vec2::new(24.0, 14.0);
        let rect = Rect::from_center_size(Pos2::new(r.center().x, r.max.y - 40.0), size);
        p.rect_filled(rect, CornerRadius::same(6), Color32::from_rgba_unmultiplied(20, 20, 20, 220));
        p.galley(rect.min + Vec2::new(12.0, 7.0), galley, Color32::WHITE);
        ctx.request_repaint_after(Duration::from_millis(200));
    }
}

fn outline_tree(ui: &mut Ui, nodes: &[OutlineNode], target: &mut Option<LinkTarget>, depth: usize) {
    for (i, n) in nodes.iter().enumerate() {
        let id = ui.id().with((depth, i, &n.title));
        if n.children.is_empty() {
            if ui.selectable_label(false, &n.title).clicked() {
                *target = n.target.clone();
            }
        } else {
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
                .show_header(ui, |ui| {
                    if ui.selectable_label(false, &n.title).clicked() {
                        *target = n.target.clone();
                    }
                })
                .body(|ui| outline_tree(ui, &n.children, target, depth + 1));
        }
    }
}

fn empty_state(ui: &mut Ui) {
    let r = ui.available_rect_before_wrap();
    ui.painter().rect_filled(r, CornerRadius::ZERO, viewer::VIEW_BG);
    ui.painter().text(r.center(), Align2::CENTER_CENTER, m::EMPTY_STATE, FontId::proportional(18.0), Color32::from_gray(230));
}
