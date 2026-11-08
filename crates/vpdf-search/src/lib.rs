//! Tìm chữ trong tài liệu (`specs/text-search`, design.md D4, nhóm task 3).
//!
//! - [`normalize`]: chuẩn hóa NFD, bỏ dấu, đ→d, chữ thường, gộp khoảng trắng, kèm
//!   bảng ánh xạ về ký tự gốc để tô sáng đúng chữ có dấu.
//! - [`page`]: lớp chữ gọn của một trang, kết quả tìm (vùng tô sáng theo dòng, đoạn chữ xung quanh).
//! - [`engine`]: luồng tìm chạy nền, quét theo thứ tự trang, hủy được.
//! - [`session`]: trạng thái ô tìm (chờ 150 ms, bộ đếm, kết quả sau/trước).

pub mod engine;
pub mod normalize;
pub mod page;
pub mod session;

pub use engine::{Output, SearchEngine};
pub use normalize::SearchOptions;
pub use page::{Hit, PageSource, Snippet};
pub use session::{Action, SearchSession, DEBOUNCE};
