//! Lõi dùng chung của VPdf.
//!
//! Crate này không gọi PDFium và không phụ thuộc giao diện, nên mọi quy tắc
//! đọc tài liệu (bố cục trang, phóng, ô số trang, lịch sử Quay lại, file gần
//! đây, sao chép chữ, nhận diện file hỏng) đều kiểm thử được bằng unit test.
//! Phần sửa cấu trúc PDF (lopdf, qpdf qua C API) cũng nằm ở đây để ứng dụng
//! desktop và CLI dùng chung (design.md, D3).

pub mod history;
pub mod links;
pub mod lru;
pub mod messages;
pub mod native;
pub mod page_input;
pub mod qpdf;
pub mod recent;
pub mod structure;
pub mod tabs;
pub mod text;
pub mod view;
pub mod xref;

/// Tên sản phẩm tạm thời (proposal.md: "tên tạm: VPdf").
pub const PRODUCT_NAME: &str = "VPdf";

/// Phiên bản chung của workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Hình chữ nhật trong hệ tọa độ trang PDF (đơn vị point, gốc ở góc dưới trái).
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct RectPt {
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}

impl RectPt {
    pub fn new(left: f32, bottom: f32, right: f32, top: f32) -> Self {
        Self { left: left.min(right), bottom: bottom.min(top), right: left.max(right), top: bottom.max(top) }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.left && x <= self.right && y >= self.bottom && y <= self.top
    }

    /// Khoảng cách từ điểm tới hình chữ nhật (0 nếu nằm trong).
    pub fn distance(&self, x: f32, y: f32) -> f32 {
        let dx = if x < self.left { self.left - x } else if x > self.right { x - self.right } else { 0.0 };
        let dy = if y < self.bottom { self.bottom - y } else if y > self.top { y - self.top } else { 0.0 };
        (dx * dx + dy * dy).sqrt()
    }

    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.top - self.bottom
    }
}
