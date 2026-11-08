//! Vùng xem trang: vẽ các ô ảnh đang nhìn thấy, cuộn, phóng tại con trỏ, thanh cuộn,
//! chọn chữ bằng cách kéo, bấm link.

use crate::tab::DocTab;
use egui::{Color32, CornerRadius, CursorIcon, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use vpdf_core::links::{link_at, on_click, LinkAction};
use vpdf_core::text::Selection;
use vpdf_render::{page_pixel_size, TileKey, TILE};

/// Bề rộng thanh cuộn (điểm logic).
pub const SCROLLBAR: f32 = 12.0;
pub const VIEW_BG: Color32 = Color32::from_rgb(0x52, 0x56, 0x5c);

#[derive(Default)]
pub struct ViewerOutput {
    /// Trang đang nhìn thấy (để tạo ảnh thu nhỏ làm hình tạm).
    pub visible_pages: Vec<u32>,
    pub link_action: Option<LinkAction>,
    /// Số ô ảnh đang chờ dựng.
    pub missing_tiles: usize,
    /// Vùng xem (để vẽ chú thích nổi).
    pub rect: Option<Rect>,
}

fn quantize_scale(s: f32) -> u32 {
    (s * 1000.0).round().max(1.0) as u32
}

pub fn show(ui: &mut Ui, tab: &mut DocTab, out: &mut ViewerOutput) {
    let full = ui.available_rect_before_wrap();
    let ppp = ui.ctx().pixels_per_point();
    let horizontal_bar = tab.view.has_horizontal_overflow();
    let rect = Rect::from_min_max(
        full.min,
        Pos2::new(full.max.x - SCROLLBAR, full.max.y - if horizontal_bar { SCROLLBAR } else { 0.0 }),
    );
    out.rect = Some(rect);
    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("viewer-origin"), rect.min));
    tab.last_viewport = (rect.width(), rect.height());
    tab.view.set_viewport(rect.width(), rect.height());

    let id = ui.id().with(("viewer", tab.id));
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let pointer = ui.ctx().input(|i| i.pointer.hover_pos());
    let hovered = pointer.is_some_and(|p| rect.contains(p));

    // ----- con lăn, Ctrl/Cmd + con lăn, chụm trên touchpad -----
    if hovered {
        let (zoom, scroll) = ui.ctx().input(|i| (i.zoom_delta(), i.smooth_scroll_delta()));
        if (zoom - 1.0).abs() > 1e-4 {
            let p = pointer.unwrap() - rect.min;
            tab.view.zoom_by(zoom, p.x, p.y);
        } else if scroll != Vec2::ZERO {
            tab.view.scroll_by(-scroll.x, -scroll.y);
        }
    }
    if resp.dragged_by(PointerButton::Middle) {
        let d = resp.drag_delta();
        tab.view.scroll_by(-d.x, -d.y);
    }

    // ----- chọn chữ, bấm link -----
    let to_page = |tab: &DocTab, p: Pos2| {
        let (page, x, y_top) = tab.view.screen_to_page(p.x - rect.min.x, p.y - rect.min.y);
        let h = tab.view.page_size(page).height;
        (page, x, h - y_top)
    };
    if resp.drag_started_by(PointerButton::Primary) {
        // Bắt đầu chọn tại điểm nhấn chuột (không phải điểm egui nhận ra là đang kéo).
        let origin = ui.ctx().input(|i| i.pointer.press_origin()).or(resp.interact_pointer_pos());
        if let Some(p) = origin {
            let (page, x, y) = to_page(tab, p);
            tab.selection = tab
                .texts
                .get(&(page as u32))
                .and_then(|t| t.char_near(x, y, 6.0))
                .map(|c| Selection { start: (page, c), end: (page, c) });
        }
    }
    if resp.dragged_by(PointerButton::Primary) {
        if let (Some(p), Some(sel)) = (resp.interact_pointer_pos(), tab.selection.as_mut()) {
            let (page, x, y) = {
                let (page, x, y_top) = tab.view.screen_to_page(p.x - rect.min.x, p.y - rect.min.y);
                (page, x, tab.view.page_size(page).height - y_top)
            };
            if let Some(c) = tab.texts.get(&(page as u32)).and_then(|t| t.char_near(x, y, 40.0)) {
                sel.end = (page, c);
            }
        }
        // Kéo ra ngoài mép trên/dưới thì cuộn theo.
        if let Some(p) = resp.interact_pointer_pos() {
            if p.y > rect.max.y {
                tab.view.scroll_by(0.0, (p.y - rect.max.y).min(40.0));
            } else if p.y < rect.min.y {
                tab.view.scroll_by(0.0, (p.y - rect.min.y).max(-40.0));
            }
        }
    }
    if resp.clicked_by(PointerButton::Primary) {
        let mut handled = false;
        if let Some(p) = resp.interact_pointer_pos() {
            let (page, x, y) = to_page(tab, p);
            if let Some(l) = tab.links.get(&(page as u32)).and_then(|ls| link_at(ls, x, y)) {
                out.link_action = Some(on_click(&l.target, tab.view.page_count()));
                handled = true;
            }
        }
        if !handled {
            tab.selection = None;
        }
    }
    if let Some(p) = pointer.filter(|_| hovered) {
        let (page, x, y) = to_page(tab, p);
        if tab.links.get(&(page as u32)).and_then(|ls| link_at(ls, x, y)).is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        } else if tab.texts.get(&(page as u32)).and_then(|t| t.char_near(x, y, 0.5)).is_some() {
            ui.ctx().set_cursor_icon(CursorIcon::Text);
        }
    }

    // ----- vẽ -----
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::ZERO, VIEW_BG);
    let ds = tab.view.scale() * ppp;
    let scale_milli = quantize_scale(ds);
    let mut wanted: Vec<TileKey> = Vec::new();
    let current = tab.view.current_page();
    let mut visible = tab.view.visible_pages();
    // Ô của trang hiện tại được dựng trước.
    visible.sort_by_key(|v| (v.index != current, v.index));
    for vp in &visible {
        let page = vp.index as u32;
        out.visible_pages.push(page);
        tab.ensure_page_data(page);
        let prect = Rect::from_min_size(rect.min + Vec2::new(vp.x, vp.y), Vec2::new(vp.width, vp.height));
        painter.rect_filled(prect.translate(Vec2::new(2.0, 2.0)), CornerRadius::ZERO, Color32::from_black_alpha(80));
        painter.rect_filled(prect, CornerRadius::ZERO, Color32::WHITE);
        // 1) ảnh thu nhỏ làm hình tạm
        if let Some(t) = tab.thumbs.peek(&page) {
            painter.image(t.id(), prect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
        let size = tab.view.page_size(vp.index);
        let vis = prect.intersect(rect);
        if vis.width() <= 0.0 || vis.height() <= 0.0 {
            continue;
        }
        // 2) ô ảnh ở mức phóng trước (nếu có) trong lúc chờ mức mới
        if let Some(&old) = tab.last_scale.get(&page) {
            if old != scale_milli {
                draw_tiles(&painter, tab, page, old, size, prect, vis, None);
            }
        }
        // 3) ô ảnh ở mức phóng hiện tại
        let complete = draw_tiles(&painter, tab, page, scale_milli, size, prect, vis, Some(&mut wanted));
        if complete {
            tab.last_scale.insert(page, scale_milli);
        }
        painter.rect_stroke(prect, CornerRadius::ZERO, Stroke::new(1.0, Color32::from_gray(150)), StrokeKind::Outside);

        // vùng chọn chữ
        if let (Some(sel), Some(text)) = (tab.selection, tab.texts.get(&page)) {
            if let Some((a, b)) = sel.range_on_page(vp.index, text.chars.len()) {
                let s = tab.view.scale();
                for c in &text.chars[a..=b] {
                    if c.rect.width() <= 0.0 || c.rect.height() <= 0.0 {
                        continue;
                    }
                    let r = Rect::from_min_max(
                        prect.min + Vec2::new(c.rect.left * s, (size.height - c.rect.top) * s),
                        prect.min + Vec2::new(c.rect.right * s, (size.height - c.rect.bottom) * s),
                    );
                    painter.rect_filled(r, CornerRadius::ZERO, Color32::from_rgba_unmultiplied(30, 100, 220, 80));
                }
            }
        }
    }
    out.missing_tiles = wanted.len();
    tab.worker.set_wanted_tiles(wanted);

    scrollbars(ui, tab, full, rect, horizontal_bar);
}

/// Vẽ các ô ảnh của một trang ở tỉ lệ `scale_milli` trong vùng nhìn thấy `vis`.
/// Nếu `wanted` có giá trị thì ghi các ô còn thiếu vào đó. Trả về true nếu đủ ô.
#[allow(clippy::too_many_arguments)]
fn draw_tiles(
    painter: &egui::Painter,
    tab: &mut DocTab,
    page: u32,
    scale_milli: u32,
    size: vpdf_core::view::PageSize,
    prect: Rect,
    vis: Rect,
    mut wanted: Option<&mut Vec<TileKey>>,
) -> bool {
    let ds = scale_milli as f32 / 1000.0;
    let (pw, ph) = page_pixel_size(size, ds);
    // Điểm ảnh của trang trên mỗi điểm logic hiển thị.
    let kx = pw as f32 / prect.width();
    let ky = ph as f32 / prect.height();
    let px0 = ((vis.min.x - prect.min.x) * kx).max(0.0) as u32;
    let py0 = ((vis.min.y - prect.min.y) * ky).max(0.0) as u32;
    let px1 = (((vis.max.x - prect.min.x) * kx).ceil() as u32).min(pw);
    let py1 = (((vis.max.y - prect.min.y) * ky).ceil() as u32).min(ph);
    if px1 <= px0 || py1 <= py0 {
        return true;
    }
    let mut complete = true;
    for ty in py0 / TILE..=(py1 - 1) / TILE {
        for tx in px0 / TILE..=(px1 - 1) / TILE {
            let key = TileKey { page, scale_milli, tx, ty };
            let tex = if wanted.is_some() { tab.tiles.get(&key) } else { tab.tiles.peek(&key) };
            match tex {
                Some(t) => {
                    let x0 = (tx * TILE) as f32 / kx;
                    let y0 = (ty * TILE) as f32 / ky;
                    let x1 = ((tx * TILE + TILE).min(pw)) as f32 / kx;
                    let y1 = ((ty * TILE + TILE).min(ph)) as f32 / ky;
                    let r = Rect::from_min_max(prect.min + Vec2::new(x0, y0), prect.min + Vec2::new(x1, y1));
                    painter.image(t.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
                None => {
                    complete = false;
                    if let Some(w) = wanted.as_deref_mut() {
                        w.push(key);
                    }
                }
            }
        }
    }
    complete
}

fn scrollbars(ui: &mut Ui, tab: &mut DocTab, full: Rect, view_rect: Rect, horizontal: bool) {
    let painter = ui.painter_at(full);
    let track_col = Color32::from_gray(60);
    let thumb_col = Color32::from_gray(170);
    // dọc
    let track = Rect::from_min_max(Pos2::new(view_rect.max.x, full.min.y), Pos2::new(full.max.x, view_rect.max.y));
    painter.rect_filled(track, CornerRadius::ZERO, track_col);
    let (start, len) = tab.view.vertical_thumb();
    let th = (track.height() * len).max(24.0).min(track.height());
    let free = (track.height() - th).max(0.0);
    let max_scroll = (tab.view.content_height() - tab.view.viewport().1).max(1.0);
    let frac = (tab.view.scroll().1 / max_scroll).clamp(0.0, 1.0);
    let _ = start;
    let thumb = Rect::from_min_size(Pos2::new(track.min.x + 2.0, track.min.y + free * frac), Vec2::new(track.width() - 4.0, th));
    painter.rect_filled(thumb, CornerRadius::same(4), thumb_col);
    let r = ui.interact(track, ui.id().with(("vbar", tab.id)), Sense::click_and_drag());
    if r.dragged() || r.clicked() {
        if let Some(p) = r.interact_pointer_pos() {
            let f = ((p.y - track.min.y - th / 2.0) / free.max(1.0)).clamp(0.0, 1.0);
            let x = tab.view.scroll().0;
            tab.view.set_scroll(x, f * max_scroll);
        }
    }
    // ngang
    if horizontal {
        let track = Rect::from_min_max(Pos2::new(full.min.x, view_rect.max.y), Pos2::new(view_rect.max.x, full.max.y));
        painter.rect_filled(track, CornerRadius::ZERO, track_col);
        let (_, len) = tab.view.horizontal_thumb();
        let tw = (track.width() * len).max(24.0).min(track.width());
        let free = (track.width() - tw).max(0.0);
        let max_x = (tab.view.content_width() - tab.view.viewport().0).max(1.0);
        let frac = (tab.view.scroll().0 / max_x).clamp(0.0, 1.0);
        let thumb = Rect::from_min_size(Pos2::new(track.min.x + free * frac, track.min.y + 2.0), Vec2::new(tw, track.height() - 4.0));
        painter.rect_filled(thumb, CornerRadius::same(4), thumb_col);
        let r = ui.interact(track, ui.id().with(("hbar", tab.id)), Sense::click_and_drag());
        if r.dragged() || r.clicked() {
            if let Some(p) = r.interact_pointer_pos() {
                let f = ((p.x - track.min.x - tw / 2.0) / free.max(1.0)).clamp(0.0, 1.0);
                let y = tab.view.scroll().1;
                tab.view.set_scroll(f * max_x, y);
            }
        }
    }
    ui.allocate_rect(full, Sense::hover());
}
