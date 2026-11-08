# Proposal

## Why

Người dùng văn phòng tại Việt Nam cần một ứng dụng PDF desktop đầy đủ (đọc, chỉnh trang, bảo mật, ghi chú, form, OCR, ký số) chạy offline, tìm kiếm tốt với tiếng Việt, và có thể phân phối thương mại mà không vướng license copyleft. Ứng dụng tham khảo nhóm tính năng của các trình đọc PDF thương mại phổ biến, nhưng được viết lại hoàn toàn từ đầu (clean-room), không dùng tên, logo, giao diện hay mã nguồn của sản phẩm nào khác.

## What Changes

- Tạo mới ứng dụng desktop đọc và xử lý PDF (tên tạm: **VPdf**, CLI là `vpdf`), gồm:
  - **6 chức năng cơ bản**: đọc PDF (cuộn, phóng to, ảnh thu nhỏ, mục lục), tìm chữ hiện kết quả khi đang gõ; tìm không dấu ra chữ có dấu; gộp PDF giữ bookmark và link; tách theo khoảng trang hoặc bookmark, trích trang; sắp xếp trang (xoay, xóa, chèn, kéo thả) với hoàn tác nhiều bước; đặt/gỡ mật khẩu AES-256 và quyền cấm in, cấm sao chép.
  - **6 chức năng nâng cao**: ghi chú (highlight, ghi chú dán, bình luận) tương thích Adobe Acrobat; điền và lưu form PDF có sẵn; nén PDF 3 mức kèm báo dung lượng; OCR tiếng Việt offline; ký số bằng chứng thư `.p12`/`.pfx` và kiểm tra chữ ký có sẵn; CLI chạy hàng loạt cho gộp, tách, nén, OCR.
  - **Yêu cầu chung**: chạy trên Windows 10/11 x64 và macOS 13+ (Apple silicon và Intel); file 500 trang mở dưới 3 giây, dùng dưới 1 GB RAM trên máy 8 GB; chỉ dùng thư viện có license cho phép dùng thương mại.

Ánh xạ yêu cầu gốc sang capability:

| Yêu cầu gốc | Capability |
|---|---|
| CB1. Mở và đọc PDF, tìm khi đang gõ | `document-viewing`, `text-search` |
| CB2. Gõ không dấu tìm ra có dấu | `text-search` |
| CB3. Gộp PDF | `document-merge` |
| CB4. Tách, trích trang | `document-split` |
| CB5. Sắp xếp trang, hoàn tác | `page-organization` |
| CB6. Mật khẩu AES-256, quyền | `document-security` |
| NC1. Ghi chú | `annotations` |
| NC2. Điền form | `form-filling` |
| NC3. Nén 3 mức | `compression` |
| NC4. OCR tiếng Việt offline | `ocr` |
| NC5. Ký số, kiểm tra chữ ký | `digital-signatures` |
| NC6. CLI hàng loạt | `batch-cli` |
| Chung 1. Nền tảng | `platform-support` |
| Chung 2. Hiệu năng | `performance` |
| Chung 3. License | `dependency-licensing` |

## Capabilities

### New Capabilities

- `document-viewing`: Mở file PDF, hiển thị trang, cuộn, phóng to/thu nhỏ, bảng ảnh thu nhỏ, mục lục (bookmark), mở file có mật khẩu.
- `text-search`: Tìm chữ trong tài liệu, hiện kết quả tăng dần khi đang gõ, tìm không phân biệt dấu tiếng Việt.
- `document-merge`: Gộp nhiều file PDF thành một, giữ bookmark, link nội bộ và link ngoài.
- `document-split`: Tách PDF theo khoảng trang hoặc theo bookmark, trích các trang chọn ra file mới.
- `page-organization`: Xoay, xóa, chèn, kéo thả sắp xếp trang; hoàn tác và làm lại nhiều bước.
- `document-security`: Đặt và gỡ mật khẩu mã hóa AES-256; đặt quyền cấm in, cấm sao chép.
- `annotations`: Tạo, sửa, xóa highlight, ghi chú dán, bình luận theo chuẩn PDF để trình đọc khác (gồm Adobe Acrobat) hiển thị được.
- `form-filling`: Điền form AcroForm có sẵn trong file và lưu lại.
- `compression`: Nén PDF theo 3 mức và báo dung lượng trước/sau.
- `ocr`: Nhận dạng chữ tiếng Việt offline, thêm lớp chữ ẩn để PDF scan tìm và chép chữ được.
- `digital-signatures`: Ký số bằng chứng thư cá nhân `.p12`/`.pfx` và kiểm tra chữ ký số có sẵn trong file.
- `batch-cli`: Công cụ dòng lệnh cho gộp, tách, nén, OCR, chạy được hàng loạt và trong script.
- `platform-support`: Các hệ điều hành và kiến trúc CPU được hỗ trợ, cách cài đặt.
- `performance`: Ngưỡng thời gian mở file và bộ nhớ tối đa.
- `dependency-licensing`: Quy định license của thư viện bên thứ ba và nghĩa vụ ghi công.

### Modified Capabilities

(Không có. Đây là dự án mới.)

## Impact

- **Mã nguồn**: dự án mới, gồm lõi xử lý PDF dùng chung, ứng dụng desktop và CLI cùng dùng lõi đó (xem `design.md`).
- **Phụ thuộc**: engine dựng hình PDF, thư viện ghi/sửa cấu trúc PDF, engine OCR kèm dữ liệu huấn luyện tiếng Việt, thư viện mật mã (AES, RSA/ECDSA, CMS/PKCS#7, PKCS#12). Tất cả phải qua kiểm tra license theo `dependency-licensing`.
- **Phân phối**: bộ cài Windows (MSI) và macOS (DMG universal) có ký mã và notarize; dữ liệu OCR tiếng Việt đi kèm bộ cài để chạy offline.
- **Ngoài phạm vi phiên bản đầu**: sửa nội dung chữ/ảnh trong trang, form XFA động, tạo form mới, chuyển PDF sang Word/Excel, đồng bộ đám mây, Linux, ký bằng USB token/HSM, dấu thời gian (TSA) bắt buộc.
