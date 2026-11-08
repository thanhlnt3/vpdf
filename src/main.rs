//! VPdf — ứng dụng đọc PDF desktop.
//!
//! Cách chạy:
//!   vpdf-app [file.pdf ...]            mở các file (liên kết loại file của hệ điều hành gọi theo cách này)
//!   vpdf-app --screenshot <file.png>   mở file mẫu, vẽ ~120 khung hình, chụp cửa sổ rồi thoát
//!   vpdf-app --self-test <file.json>   tự kiểm tra các bước đọc tài liệu rồi thoát

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::PathBuf;
use vpdf_app::app::VpdfApp;
use vpdf_app::automation::{Automation, Mode};

struct Shell {
    app: VpdfApp,
    automation: Option<Automation>,
    instance: Option<vpdf_app::single_instance::Listener>,
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // File được chuyển từ lần nhấp đúp khác khi ứng dụng đang chạy: mở thành tab mới.
        if let Some(l) = &self.instance {
            while let Ok(files) = l.files.try_recv() {
                for f in files {
                    self.app.open_path(ui.ctx(), &f);
                }
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }
        self.app.ui(ui);
        if let Some(a) = self.automation.as_mut() {
            let ctx = ui.ctx().clone();
            a.after_frame(&mut self.app, &ctx);
        }
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if let Some(a) = self.automation.as_mut() {
            a.raw_input_hook(raw_input);
        }
    }

    fn on_exit(&mut self) {
        self.app.save_recent();
    }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut mode = None;
    let mut files = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--screenshot" if i + 1 < args.len() => {
                mode = Some(Mode::Screenshot(PathBuf::from(&args[i + 1])));
                i += 1;
            }
            "--self-test" if i + 1 < args.len() => {
                mode = Some(Mode::SelfTest(PathBuf::from(&args[i + 1])));
                i += 1;
            }
            "--version" => {
                println!("{} {}", vpdf_core::PRODUCT_NAME, vpdf_core::VERSION);
                return Ok(());
            }
            other => files.push(PathBuf::from(other)),
        }
        i += 1;
    }
    if mode.is_some() && files.is_empty() {
        files.push(vpdf_app::sample_pdf());
    }
    // Chế độ tự động không ghi vào danh sách gần đây của người dùng.
    let config = vpdf_core::recent::default_config_dir();
    let recent = if mode.is_some() { None } else { Some(config.join("recent.json")) };
    // Ứng dụng đang chạy: chuyển file sang cửa sổ đó rồi thoát.
    if mode.is_none() && !files.is_empty() && vpdf_app::single_instance::forward(&config, &files) {
        return Ok(());
    }
    let single = mode.is_none();

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title(vpdf_core::PRODUCT_NAME)
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 400.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        vpdf_core::PRODUCT_NAME,
        options,
        Box::new(move |cc| {
            vpdf_app::fonts::install(&cc.egui_ctx);
            let mut app = VpdfApp::new(recent);
            for f in &files {
                app.open_path(&cc.egui_ctx, f);
            }
            let ctx = cc.egui_ctx.clone();
            let instance =
                if single { vpdf_app::single_instance::listen(&config, move || ctx.request_repaint()) } else { None };
            Ok(Box::new(Shell { app, automation: mode.map(Automation::new), instance }))
        }),
    )
}
