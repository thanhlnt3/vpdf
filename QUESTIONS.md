# Câu hỏi còn mở (nhóm task 1–2)

AGENTS.md: "Chỗ nào spec chưa rõ thì ghi câu hỏi lại, không tự đoán." Dưới đây là các điểm
spec chưa trả lời, kèm việc đã làm tạm (nếu có) để không chặn công việc.

## Giấy phép (dependency-licensing)

1. **License của thành phần nằm bên trong PDFium và qpdf không có trong danh sách cho phép.**
   PDFium bản dựng sẵn chứa FreeType (FTL), libpng (libpng-2.0), libjpeg-turbo (IJG + BSD-3-Clause + Zlib),
   HarfBuzz ("Old MIT"), Anti-Grain Geometry 2.3 (giấy phép riêng kiểu BSD), LLVM libc
   (Apache-2.0 WITH LLVM-exception). Bản qpdf cho Windows liên kết tĩnh libjpeg (IJG) và kèm
   các DLL runtime Microsoft Visual C++. Tất cả đều là giấy phép cho phép dùng thương mại, nhưng
   **không nằm đúng tên** trong danh sách của spec.
   - Đã làm: ghi vào `third_party/license-exceptions.toml` kèm lý do, **để trống người duyệt và
     ngày duyệt**. Vì vậy `cargo run -p vpdf-license-check` hiện báo 8 vi phạm (đúng theo
     scenario "Ngoại lệ thiếu người duyệt").
   - Cần quyết định: ai là người duyệt; có chấp nhận các license này không (hay mở rộng danh sách
     cho phép); với MSVC runtime, chấp nhận DLL kèm theo hay dựng lại qpdf liên kết tĩnh runtime.
2. **Font giao diện.** Font mặc định của egui (Ubuntu-Light, Ubuntu Font Licence) không có trong
   danh sách, nên giao diện dùng Noto Sans (OFL-1.1). Không cần quyết định, chỉ ghi lại.

## Bộ tài liệu tham chiếu (task 1.5)

3. **File có chữ ký số.** Task 1.5 yêu cầu file mẫu "có chữ ký", nhưng việc ký (OpenSSL, PKCS#12, CMS)
   thuộc nhóm 10. Chưa tạo file này. Cần chọn: tạo ở nhóm 10 bằng công cụ ký của dự án, hay dùng
   một file đã ký có sẵn (cần nguồn có license cho phép)?
4. **Nơi lưu bộ tài liệu để "tải về".** Spec nói tài liệu tham chiếu "lưu trong kho mã" và có "script
   tải về". Bộ file nặng ~210 MB. Đã làm: script tải từ `VPDF_TESTDATA_URL` nếu có, nếu không thì tạo
   tại chỗ (xác định, cùng mã băm). Cần chọn nơi lưu (Git LFS, GitHub Release, máy chủ nội bộ).
5. **File 500 trang "khoảng 60 MB" và scan "khoảng 150 MB".** Bộ tự tạo: 57,7 MB và 146 MB. Nội dung
   là chữ tiếng Việt và ảnh do dự án tự sinh, không phải tài liệu thật. Có cần thay bằng tài liệu thật
   (ví dụ văn bản pháp luật công khai) không?

## Đọc tài liệu (document-viewing)

6. **"Kích thước thật (100%)" theo đơn vị nào?** Đã chọn: 100% = 1 point PDF hiển thị bằng 96/72
   điểm logic của hệ điều hành (kích thước giấy thật trên màn hình 96 dpi; trên màn hình 150%/200%
   ảnh vẫn được dựng đủ độ phân giải). Cần xác nhận.
7. **Phím tắt Vừa trang / Kích thước thật / Vừa chiều rộng.** Spec không quy định. Đã chọn Ctrl/Cmd+0,
   Ctrl/Cmd+1, Ctrl/Cmd+2; Ctrl/Cmd+= và Ctrl/Cmd+- để phóng từng nấc 1,25×; Alt+← để Quay lại.
8. **Câu chữ cảnh báo "file đã được sửa tạm khi mở".** Spec chỉ mô tả ý. Đã dùng: "File có lỗi cấu
   trúc và đã được sửa tạm khi mở. Nội dung hiển thị có thể chưa đầy đủ."
9. **Phát hiện "bảng xref bị lỗi".** PDFium tự sửa mà không báo, nên ứng dụng tự kiểm tra nhanh bảng
   xref cuối (startxref, lấy mẫu 24 mục). Lỗi ở các bản cập nhật cũ hơn (/Prev) chưa được phát hiện.
10. **Chế độ từng trang**: cuộn quá mép trang thì sang trang kế/trước. Spec không nói; cần xác nhận.
11. **Giao diện tiếng Anh** (task 12.3) chưa làm; chuỗi giao diện hiện là tiếng Việt, chuỗi trong spec
    được giữ đúng từng chữ.

## Nền tảng và CI

12. **Thư mục dự án rất sâu** làm đường dẫn file tạm của build script vượt 260 ký tự, `link.exe` của
    MSVC không mở được. Đã cấu hình `.cargo/config.toml` dùng `rust-lld` cho Windows. Khi chuyển mã sang
    kho chính (đường dẫn ngắn) có thể bỏ cấu hình này.
13. **CI chưa chạy lần nào.** `.github/workflows/ci.yml` đã viết cho Windows, macOS arm64 và Intel;
    các task cần "xác minh trên macOS / trong CI" (1.1, 1.2, 1.3, 2.7) để mở cho tới khi CI chạy.
14. Câu hỏi mở sẵn trong design.md (tên sản phẩm, PdfCraft, TSA) chưa ảnh hưởng nhóm 1–2.
