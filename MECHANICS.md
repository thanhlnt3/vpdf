# Các con số quyết định cách ứng dụng hoạt động

Nguồn: `spec` = openspec/changes/add-pdf-reader-app/specs/…; `design` = design.md;
"ước lượng" = spec không nói, dự án tự chọn (cũng ghi trong QUESTIONS.md).
Cùng các giá trị nằm trong `mechanics.json`; trong mã chúng là hằng số ở
`crates/vpdf-core/src/view.rs`, `lru.rs`, `recent.rs`, `history.rs`, `crates/vpdf-render/src/lib.rs`.

## Phóng và bố cục

| Giá trị | Số | Nguồn |
|---|---|---|
| Mức phóng nhỏ nhất | 10% | spec document-viewing "Phóng to và thu nhỏ" |
| Mức phóng lớn nhất | 6400% | spec document-viewing |
| 100% (Kích thước thật) | 1 pt = 96/72 điểm logic | ước lượng |
| Nấc Phóng to / Thu nhỏ | ×1,25 | ước lượng |
| Mức có sẵn trong danh sách | 10, 25, 50, 75, 100, 125, 150, 200, 300, 400, 800, 1600, 2400, 3200, 6400% | ước lượng |
| Phóng bằng Ctrl/Cmd + con lăn | giữ điểm dưới con trỏ (lệch đo được 0,00 điểm ảnh) | spec "Phóng tại con trỏ" |
| Vừa chiều rộng | chiều rộng trang = chiều rộng vùng xem, giữ khi đổi cỡ cửa sổ | spec "Vừa chiều rộng" |
| Khoảng cách giữa hai trang | 10 điểm | ước lượng |
| Lề trên/dưới tài liệu | 10 điểm | ước lượng |
| Bề rộng thanh cuộn | 12 điểm | ước lượng |

## Cuộn và điều hướng

| Giá trị | Số | Nguồn |
|---|---|---|
| Chế độ mặc định | cuộn liên tục | spec (cuộn liên tục và từng trang) |
| Mũi tên lên/xuống | 48 điểm | ước lượng |
| Page Down/Up (cuộn liên tục) | chiều cao vùng xem − 48 điểm | ước lượng |
| Page Down/Up (từng trang) | sang trang kế / trước | ước lượng |
| Ô số trang | "trang / tổng", số ngoài 1…tổng hoặc chữ thì trở về trang hiện tại | spec "Nhảy tới trang", "Số trang không hợp lệ" |
| Trang hiện tại (cuộn liên tục) | trang đầu tiên còn ≥ min(10% vùng xem, 40 điểm) ở mép trên | ước lượng |
| Lịch sử Quay lại | 100 vị trí | ước lượng |

## Bộ nhớ và hiệu năng

| Giá trị | Số | Nguồn |
|---|---|---|
| Bộ nhớ đệm ảnh trang (LRU) | 300 MB | design.md Risks |
| Bộ nhớ đệm ảnh thu nhỏ | 48 MB | ước lượng |
| Ô ảnh trang | 512 × 512 điểm ảnh, chỉ dựng ô đang thấy | design.md Risks ("chỉ dựng phần đang nhìn thấy") |
| Ảnh thu nhỏ | rộng 120 điểm ảnh, phần đang thấy trước + 6 ảnh kế | spec "Ảnh thu nhỏ tải dần"; số là ước lượng |
| Trang PDFium giữ sẵn mỗi tài liệu | 6 | ước lượng |
| Ngưỡng mở file 500 trang | < 3 giây (đo được 66 ms, bộ nhớ đệm ổ đĩa nóng) | spec performance |
| Ngưỡng RAM | < 1 GB (đo được tối đa 392 MB khi dựng 500 trang) | spec performance |

## File gần đây

| Giá trị | Số | Nguồn |
|---|---|---|
| Số file nhớ vị trí đọc | 50 | spec "Nhớ vị trí đọc" |
| Nơi lưu | `%APPDATA%\VPdf\recent.json` (macOS: `~/Library/Application Support/VPdf`) | ước lượng |
| So khớp tên file | NFC, không phân biệt hoa thường trên Windows | spec platform-support (NFD = NFC) |

## Chuỗi giao diện lấy đúng từ spec

"Không đọc được file PDF" (kèm tên file) · "Mật khẩu không đúng" · "Tài liệu không có mục lục" ·
"Mở liên kết này trong trình duyệt?" · "Tài liệu này không cho phép sao chép".
