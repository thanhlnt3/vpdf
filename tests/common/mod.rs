//! Bộ điều khiển giao diện dùng chung cho các test giao diện: chạy egui thật, không cửa sổ,
//! đưa vào sự kiện bàn phím / chuột như người dùng.

#![allow(dead_code)]

use egui::{Context, Event, Key, Modifiers, Pos2, RawInput, Rect, Vec2};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use vpdf_app::app::VpdfApp;

pub const W: f32 = 1280.0;
pub const H: f32 = 800.0;

pub fn data(name: &str) -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/reference").join(name);
    assert!(p.is_file(), "thiếu {}: chạy `cargo run -p vpdf-testdata --release -- fetch`", p.display());
    p
}

pub struct Harness {
    pub ctx: Context,
    pub app: VpdfApp,
    pub events: Vec<Event>,
    pub dropped: Vec<PathBuf>,
    pub pointer: Pos2,
    pub copied: Vec<String>,
    pub t: f64,
}

#[derive(Debug)]
pub struct DroppedPath(PathBuf);
impl egui::DroppedFile for DroppedPath {
    fn path(&self) -> &Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

impl Harness {
    pub fn new(recent: Option<PathBuf>) -> Self {
        let ctx = Context::default();
        vpdf_app::fonts::install(&ctx);
        Harness { ctx, app: VpdfApp::new(recent), events: vec![], dropped: vec![], pointer: Pos2::ZERO, copied: vec![], t: 0.0 }
    }

    pub fn frame(&mut self) {
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

    pub fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.frame();
        }
    }

    /// Chạy khung hình cho tới khi điều kiện đúng (tối đa 30 giây).
    pub fn until(&mut self, what: &str, mut f: impl FnMut(&VpdfApp) -> bool) {
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

    pub fn open(&mut self, path: &Path) {
        let ctx = self.ctx.clone();
        self.app.open_path(&ctx, path);
        self.until("tài liệu mở xong", |a| a.active().is_some_and(|t| t.is_ready()));
        self.frames(3);
    }

    pub fn wait_tiles(&mut self) {
        self.until("ô ảnh trang hiện tại", |a| a.active().is_some_and(|t| !t.tiles.is_empty() && t.worker.pending().1 == 0));
        self.frames(2);
    }

    pub fn key(&mut self, k: Key, m: Modifiers) {
        self.events.push(Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
        self.events.push(Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
        self.frames(2);
    }

    pub fn move_to(&mut self, p: Pos2) {
        self.pointer = p;
        self.events.push(Event::PointerMoved(p));
        self.frame();
    }

    pub fn click(&mut self, p: Pos2) {
        self.move_to(p);
        let b = |pressed| Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        self.events.push(b(true));
        self.frame();
        self.events.push(b(false));
        self.frames(2);
    }

    pub fn drag(&mut self, from: Pos2, to: Pos2) {
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

    pub fn type_in_page_box(&mut self, text: &str) {
        let id = self.app.page_box_id();
        self.ctx.memory_mut(|m| m.request_focus(id));
        self.frames(2);
        self.key(Key::A, Modifiers::COMMAND);
        self.events.push(Event::Text(text.into()));
        self.frames(2);
        self.key(Key::Enter, Modifiers::NONE);
        self.frames(2);
    }

    pub fn viewer_origin(&self) -> Pos2 {
        self.ctx.data(|d| d.get_temp::<Pos2>(egui::Id::new("viewer-origin"))).unwrap()
    }

    pub fn tab(&self) -> &vpdf_app::tab::DocTab {
        self.app.active().unwrap()
    }
}
impl Harness {
    pub fn tab_mut_view(&mut self, f: impl FnOnce(&mut vpdf_core::view::ViewState)) {
        f(&mut self.app.active_mut().unwrap().view);
        self.frames(2);
    }
}
