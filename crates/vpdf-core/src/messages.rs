//! Chuỗi giao diện tiếng Việt. Những chuỗi có trong spec được giữ đúng từng chữ.
//! (Giao diện tiếng Anh thuộc task 12.3, chưa làm ở lượt này.)

/// document-viewing › File hỏng.
pub const CANNOT_READ_PDF: &str = "Không đọc được file PDF";
/// document-viewing › Nhập sai mật khẩu.
pub const WRONG_PASSWORD: &str = "Mật khẩu không đúng";
/// document-viewing › Mục lục.
pub const NO_OUTLINE: &str = "Tài liệu không có mục lục";
/// document-viewing › Link URL.
pub const CONFIRM_OPEN_URL: &str = "Mở liên kết này trong trình duyệt?";
/// document-viewing › Tài liệu cấm sao chép.
pub const COPY_FORBIDDEN: &str = "Tài liệu này không cho phép sao chép";
/// document-viewing › File hỏng một phần (spec chỉ mô tả ý, câu chữ do dự án chọn).
pub const REPAIRED_WARNING: &str = "File có lỗi cấu trúc và đã được sửa tạm khi mở. Nội dung hiển thị có thể chưa đầy đủ.";

pub const PASSWORD_PROMPT: &str = "Tài liệu này được bảo vệ bằng mật khẩu. Nhập mật khẩu để mở:";
pub const OPEN: &str = "Mở…";
pub const OPEN_FILE_TITLE: &str = "Mở file PDF";
pub const RECENT: &str = "Gần đây";
pub const NO_RECENT: &str = "Chưa có file nào";
pub const CLOSE_TAB: &str = "Đóng tab";
pub const BACK: &str = "Quay lại";
pub const THUMBNAILS: &str = "Ảnh thu nhỏ";
pub const OUTLINE: &str = "Mục lục";
pub const FIT_PAGE: &str = "Vừa trang";
pub const FIT_WIDTH: &str = "Vừa chiều rộng";
pub const ACTUAL_SIZE: &str = "Kích thước thật";
pub const CONTINUOUS: &str = "Cuộn liên tục";
pub const SINGLE_PAGE: &str = "Từng trang";
pub const ZOOM_IN: &str = "Phóng to";
pub const ZOOM_OUT: &str = "Thu nhỏ";
pub const COPY: &str = "Sao chép";
pub const OK: &str = "Đồng ý";
pub const CANCEL: &str = "Hủy";
pub const OPEN_LINK: &str = "Mở";
pub const FILE_MENU: &str = "File";
pub const VIEW_MENU: &str = "Xem";
pub const EMPTY_STATE: &str = "Kéo thả file PDF vào đây hoặc bấm Mở… để bắt đầu";
pub const COPIED: &str = "Đã sao chép";
pub const LOADING: &str = "Đang mở…";
pub const PAGE_LABEL: &str = "Trang";
pub const NO_TEXT_ON_PAGE: &str = "Trang này không có lớp chữ";

// ----- text-search (nhóm task 3) -----

/// text-search › Không có kết quả.
pub const NOT_FOUND: &str = "Không tìm thấy";
/// Bộ đếm khi đang tìm mà chưa có kết quả nào.
pub const SEARCHING: &str = "Đang tìm…";
pub const SEARCH_HINT: &str = "Tìm trong tài liệu (Ctrl+F)";
pub const SEARCH_RESULTS: &str = "Kết quả tìm";
pub const SEARCH_OPTIONS: &str = "Tùy chọn tìm";
/// text-search › Tùy chọn tìm chính xác.
pub const MATCH_DIACRITICS: &str = "Phân biệt dấu";
pub const MATCH_CASE: &str = "Phân biệt hoa thường";
pub const WHOLE_WORD: &str = "Khớp nguyên từ";
pub const PREV_RESULT: &str = "Kết quả trước (Shift+Enter, Shift+F3)";
pub const NEXT_RESULT: &str = "Kết quả sau (Enter, F3)";
pub const RUN_OCR: &str = "Chạy OCR";
/// OCR thuộc nhóm task 9, chưa có ở bản này.
pub const OCR_NOT_YET: &str = "Chức năng OCR chưa có trong phiên bản này";
pub const NO_SEARCH_YET: &str = "Gõ chữ vào ô tìm để xem kết quả";

/// text-search › PDF scan: "15 trang không có chữ để tìm. Chạy OCR?"
pub fn no_text_pages(n: usize) -> String {
    format!("{n} trang không có chữ để tìm. Chạy OCR?")
}
