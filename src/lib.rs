//! Ứng dụng desktop VPdf (egui/eframe, backend wgpu — design.md D1).
//!
//! Lượt này hiện thực nhóm task 2 (`document-viewing`): mở file (hộp thoại, kéo thả,
//! tham số dòng lệnh từ liên kết loại file, danh sách gần đây), hỏi mật khẩu, báo
//! lỗi file hỏng, tab, cuộn/phóng, ảnh thu nhỏ, mục lục, link, chọn và sao chép chữ.

pub mod app;
pub mod automation;
pub mod fonts;
pub mod single_instance;
pub mod tab;
pub mod viewer;

use std::path::PathBuf;

/// File PDF mẫu đi kèm (tự tạo bằng `vpdf-testdata`), dùng cho ảnh chụp và tự kiểm tra.
pub fn sample_pdf() -> PathBuf {
    let candidates = [
        std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("assets/sample/mau-tieng-viet.pdf"))),
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/sample/mau-tieng-viet.pdf")),
    ];
    candidates.into_iter().flatten().find(|p| p.is_file()).unwrap_or_else(|| PathBuf::from("assets/sample/mau-tieng-viet.pdf"))
}
