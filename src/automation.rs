//! Chế độ tự động cho Craft Anything:
//! - `--screenshot <file.png>`: mở file mẫu, vẽ khoảng 120 khung hình, chụp cửa sổ, thoát.
//! - `--self-test <file.json>`: mở file mẫu như người dùng, điều khiển bằng sự kiện bàn
//!   phím / chuột giả lập, đo từ trạng thái của ứng dụng xem từng bước có chạy không.

use crate::app::VpdfApp;
use egui::{Context, Event, Key, Modifiers, MouseWheelUnit, Pos2, RawInput, Vec2};
use serde::Serialize;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum Mode {
    Screenshot(PathBuf),
    SelfTest(PathBuf),
}

#[derive(Serialize, Debug, Clone)]
pub struct Step {
    pub step: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Serialize)]
struct Report {
    version: u32,
    steps: Vec<Step>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    WaitReady,
    Move,
    CheckMove,
    Zoom,
    CheckZoom,
    TypePage,
    CheckPage,
    Search,
    CheckSearch,
    Hud,
    Done,
}

pub struct Automation {
    pub mode: Mode,
    frame: u32,
    start: Instant,
    phase: Phase,
    phase_frame: u32,
    inject: Vec<Event>,
    steps: Vec<Step>,
    before_scroll: f32,
    before_page: usize,
    before_zoom: f32,
    anchor: (f32, f32),
    anchor_page_pt: (usize, f32, f32),
    pointer: Pos2,
    shot_requested: bool,
    pub finished: bool,
}

/// Số khung hình tối thiểu trước khi chụp.
const SCREENSHOT_FRAMES: u32 = 120;

impl Automation {
    pub fn new(mode: Mode) -> Self {
        Automation {
            mode,
            frame: 0,
            start: Instant::now(),
            phase: Phase::WaitReady,
            phase_frame: 0,
            inject: Vec::new(),
            steps: Vec::new(),
            before_scroll: 0.0,
            before_page: 0,
            before_zoom: 1.0,
            anchor: (0.0, 0.0),
            anchor_page_pt: (0, 0.0, 0.0),
            pointer: Pos2::ZERO,
            shot_requested: false,
            finished: false,
        }
    }

    /// Đưa sự kiện giả lập vào khung hình kế tiếp.
    pub fn raw_input_hook(&mut self, raw: &mut RawInput) {
        // Bỏ thao tác thật của chuột / bàn phím để lần chạy tự động lặp lại được.
        raw.events.retain(|e| {
            !matches!(
                e,
                Event::MouseWheel { .. } | Event::Key { .. } | Event::Text(_) | Event::PointerButton { .. } | Event::PointerMoved(_)
                    | Event::Zoom(_) | Event::Copy | Event::Cut | Event::Paste(_)
            )
        });
        raw.dropped_files.clear();
        raw.events.append(&mut self.inject);
    }

    fn viewer_ready(app: &VpdfApp) -> bool {
        app.active().is_some_and(|t| t.is_ready() && !t.tiles.is_empty())
    }

    fn set_phase(&mut self, p: Phase) {
        self.phase = p;
        self.phase_frame = 0;
    }

    pub fn after_frame(&mut self, app: &mut VpdfApp, ctx: &Context) {
        self.frame += 1;
        self.phase_frame += 1;
        ctx.request_repaint();
        match self.mode.clone() {
            Mode::Screenshot(path) => self.screenshot(app, ctx, &path),
            Mode::SelfTest(path) => self.self_test(app, ctx, &path),
        }
    }

    fn screenshot(&mut self, app: &mut VpdfApp, ctx: &Context, path: &PathBuf) {
        // Tùy chọn khi phát triển: VPDF_SCREENSHOT_SEARCH="chuỗi" gõ chuỗi đó vào ô tìm
        // trước khi chụp, để xem cách tô sáng kết quả.
        let query = std::env::var("VPDF_SCREENSHOT_SEARCH").ok().filter(|q| !q.is_empty());
        if let (Some(q), Some(t)) = (&query, app.active_mut()) {
            if t.is_ready() && t.search.input != *q {
                t.search.set_input(q, std::time::Instant::now());
            }
        }
        let searched = query.is_none() || app.active().is_some_and(|t| t.search.done);
        let ready = Self::viewer_ready(app) && searched && app.active().is_some_and(|t| t.worker.pending().1 == 0);
        let timeout = self.start.elapsed() > Duration::from_secs(25);
        if !self.shot_requested && self.frame >= SCREENSHOT_FRAMES && (ready || timeout) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.shot_requested = true;
        }
        let shot = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let [w, h] = img.size;
            let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
            if let Some(buf) = image::RgbaImage::from_raw(w as u32, h as u32, bytes) {
                if let Err(e) = buf.save(path) {
                    eprintln!("không lưu được ảnh chụp: {e}");
                }
            }
            self.finished = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.start.elapsed() > Duration::from_secs(60) {
            eprintln!("hết thời gian chờ chụp màn hình");
            self.finished = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn push(&mut self, step: &str, passed: bool, detail: String) {
        self.steps.push(Step { step: step.into(), passed, detail });
    }

    fn self_test(&mut self, app: &mut VpdfApp, ctx: &Context, path: &PathBuf) {
        let timeout = self.start.elapsed() > Duration::from_secs(45);
        match self.phase {
            Phase::WaitReady => {
                if Self::viewer_ready(app) && self.phase_frame > 20 {
                    let t = app.active().unwrap();
                    self.before_scroll = t.view.scroll().1;
                    self.before_page = t.view.current_page();
                    // Phím Page Down hai lần, như người dùng đọc tiếp.
                    self.inject.push(key(Key::PageDown, Modifiers::NONE, true));
                    self.inject.push(key(Key::PageDown, Modifiers::NONE, false));
                    self.set_phase(Phase::Move);
                } else if timeout {
                    self.push("move", false, "tài liệu mẫu không mở được".into());
                    self.set_phase(Phase::Hud);
                }
            }
            Phase::Move => {
                if self.phase_frame == 2 {
                    self.inject.push(key(Key::PageDown, Modifiers::NONE, true));
                    self.inject.push(key(Key::PageDown, Modifiers::NONE, false));
                }
                if self.phase_frame >= 5 {
                    self.set_phase(Phase::CheckMove);
                }
            }
            Phase::CheckMove => {
                let t = app.active().unwrap();
                let dy = t.view.scroll().1 - self.before_scroll;
                let moved_pt = dy / t.view.scale();
                let passed = dy > 1.0;
                self.push(
                    "move",
                    passed,
                    format!(
                        "Page Down ×2 cuộn {:.0} pt ({:.0} điểm ảnh), trang {} → {}",
                        moved_pt,
                        dy,
                        self.before_page + 1,
                        t.view.current_page() + 1
                    ),
                );
                // Ctrl + con lăn tại một điểm trên trang: phóng to và giữ điểm dưới con trỏ.
                let rect = ctx.content_rect();
                self.pointer = Pos2::new(rect.center().x + 40.0, rect.center().y + 20.0);
                self.inject.push(Event::PointerMoved(self.pointer));
                self.set_phase(Phase::Zoom);
            }
            Phase::Zoom => {
                if self.phase_frame == 2 {
                    let t = app.active().unwrap();
                    self.before_zoom = t.view.zoom();
                    // Vị trí con trỏ trong vùng xem: dùng hàm đổi tọa độ của chính vùng xem.
                    let origin = viewer_origin(ctx);
                    self.anchor = (self.pointer.x - origin.x, self.pointer.y - origin.y);
                    self.anchor_page_pt = t.view.screen_to_page(self.anchor.0, self.anchor.1);
                    self.inject.push(Event::MouseWheel {
                        unit: MouseWheelUnit::Line,
                        delta: Vec2::new(0.0, 3.0),
                        modifiers: Modifiers::COMMAND,
                        phase: egui::TouchPhase::Move,
                    });
                }
                // egui làm mượt thao tác phóng qua vài khung hình: chờ phóng xong.
                let settled = ctx.input(|i| (i.zoom_delta() - 1.0).abs() < 1e-4);
                if self.phase_frame >= 6 && settled || self.phase_frame > 120 {
                    self.set_phase(Phase::CheckZoom);
                }
            }
            Phase::CheckZoom => {
                let t = app.active().unwrap();
                let (p, x, y) = self.anchor_page_pt;
                let (sx, sy) = t.view.page_to_screen(p, x, y);
                let drift = ((sx - self.anchor.0).powi(2) + (sy - self.anchor.1).powi(2)).sqrt();
                let zoomed = t.view.zoom() > self.before_zoom * 1.01;
                self.push(
                    "act",
                    zoomed && drift < 2.0,
                    format!(
                        "Ctrl + con lăn: {}% → {}%, điểm dưới con trỏ lệch {:.2} điểm ảnh",
                        (self.before_zoom * 100.0).round(),
                        t.view.zoom_percent(),
                        drift
                    ),
                );
                self.set_phase(Phase::TypePage);
            }
            Phase::TypePage => {
                // Như người dùng: bấm vào ô số trang, chọn hết, gõ "30", thấy "30" rồi mới nhấn Enter.
                let id = app.page_box_id();
                let focused = ctx.memory(|m| m.has_focus(id));
                let typed = app.hud.page_box == "30";
                if !focused && !typed {
                    ctx.memory_mut(|m| m.request_focus(id));
                } else if focused && !typed && self.phase_frame % 10 == 0 {
                    self.inject.push(key(Key::A, Modifiers::COMMAND, true));
                    self.inject.push(key(Key::A, Modifiers::COMMAND, false));
                    self.inject.push(Event::Text("30".into()));
                } else if typed && self.phase_frame % 10 == 5 {
                    self.inject.push(key(Key::Enter, Modifiers::NONE, true));
                    self.inject.push(key(Key::Enter, Modifiers::NONE, false));
                }
                let arrived = app.active().is_some_and(|t| t.view.current_page() == 29);
                if (arrived && !focused) || self.phase_frame > 240 {
                    self.set_phase(Phase::CheckPage);
                }
            }
            Phase::CheckPage => {
                let t = app.active().unwrap();
                let ok = t.view.current_page() == 29 && app.hud.page_box.starts_with("30 / ");
                self.push(
                    "act",
                    ok,
                    format!("gõ 30 vào ô số trang + Enter: đang ở trang {}, ô hiện \"{}\"", t.view.current_page() + 1, app.hud.page_box),
                );
                self.set_phase(Phase::Search);
            }
            Phase::Search => {
                // Ctrl+F rồi gõ không dấu "hop dong", không nhấn Enter (tìm khi đang gõ).
                if self.phase_frame == 1 {
                    self.inject.push(key(Key::F, Modifiers::COMMAND, true));
                    self.inject.push(key(Key::F, Modifiers::COMMAND, false));
                }
                if self.phase_frame == 4 {
                    self.inject.push(Event::Text("hop dong".into()));
                }
                let found = app.active().is_some_and(|t| t.search.done && t.search.current.is_some());
                if (found && !app.hud.current_hit_rects.is_empty()) || self.phase_frame > 600 {
                    self.set_phase(Phase::CheckSearch);
                }
            }
            Phase::CheckSearch => {
                let t = app.active().unwrap();
                let s = &t.search;
                let first = s.current_hit().map(|h| (h.page + 1, h.snippet.matched.clone()));
                let ok = s.input == "hop dong"
                    && first.as_ref().is_some_and(|(_, m)| m.to_lowercase() == "hợp đồng")
                    && !app.hud.current_hit_rects.is_empty()
                    && app.hud.search_counter == format!("1 / {}", s.hits.len());
                self.push(
                    "act",
                    ok,
                    format!(
                        "gõ \"hop dong\" vào ô tìm (không Enter): {} kết quả, kết quả đầu \"{}\" ở trang {} được tô sáng, bộ đếm \"{}\"",
                        s.hits.len(),
                        first.as_ref().map_or("", |f| f.1.as_str()),
                        first.as_ref().map_or(0, |f| f.0),
                        app.hud.search_counter
                    ),
                );
                self.set_phase(Phase::Hud);
            }
            Phase::Hud => {
                // Chờ ảnh thu nhỏ quanh trang mới được tạo (tối đa ~5 giây).
                if self.phase_frame < 3 || (app.hud.thumbnails_drawn == 0 && self.phase_frame < 300) {
                    return;
                }
                let h = &app.hud;
                let page_ok = {
                    let mut it = h.page_box.split(" / ");
                    matches!((it.next(), it.next()), (Some(a), Some(b)) if a.parse::<u32>().is_ok() && b.parse::<u32>().is_ok())
                };
                let ok = page_ok && h.zoom.contains('%') && !h.tab_titles.is_empty() && !h.search_counter.is_empty();
                self.push(
                    "hud",
                    ok,
                    format!(
                        "ô số trang \"{}\", mức phóng \"{}\", tab {:?}, bộ đếm tìm \"{}\", {} kết quả tô sáng, {} dòng kết quả, {} ảnh thu nhỏ, tiêu đề cửa sổ \"{}\"",
                        h.page_box,
                        h.zoom,
                        h.tab_titles,
                        h.search_counter,
                        h.highlights_drawn,
                        h.search_rows.len(),
                        h.thumbnails_drawn,
                        h.window_title
                    ),
                );
                // Gộp hai lần đo của bước "act".
                let acts: Vec<Step> = self.steps.iter().filter(|s| s.step == "act").cloned().collect();
                if acts.len() > 1 {
                    let passed = acts.iter().all(|s| s.passed);
                    let detail = acts.iter().map(|s| s.detail.clone()).collect::<Vec<_>>().join("; ");
                    self.steps.retain(|s| s.step != "act");
                    self.steps.insert(1.min(self.steps.len()), Step { step: "act".into(), passed, detail });
                }
                let report = Report { version: 1, steps: self.steps.clone() };
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap());
                self.finished = true;
                self.set_phase(Phase::Done);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Phase::Done => {}
        }
        if self.phase != Phase::Done && self.start.elapsed() > Duration::from_secs(90) {
            self.push("timeout", false, format!("dừng ở bước {:?}", self.phase));
            let report = Report { version: 1, steps: self.steps.clone() };
            let _ = std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap());
            self.finished = true;
            self.set_phase(Phase::Done);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn key(k: Key, modifiers: Modifiers, pressed: bool) -> Event {
    Event::Key { key: k, physical_key: Some(k), pressed, repeat: false, modifiers }
}

/// Góc trên trái của vùng xem trang, do vùng xem ghi lại ở khung hình trước.
fn viewer_origin(ctx: &Context) -> Pos2 {
    ctx.data(|d| d.get_temp::<Pos2>(egui::Id::new("viewer-origin"))).unwrap_or(Pos2::ZERO)
}
