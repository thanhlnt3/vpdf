# Câu hỏi còn mở (nhóm task 1–3)

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

## Tìm chữ (text-search, nhóm task 3)

15. **Tìm bắt đầu từ đâu?** Spec nói kết quả "hiện dần theo thứ tự trang" và "kết quả đầu tiên được
    tô sáng và cuộn tới". Đã làm đúng chữ: quét từ trang 1 tới trang cuối, tự chọn kết quả đầu tiên
    của tài liệu — nên đang đọc trang 200 mà tìm thì vùng xem nhảy về kết quả ở trang đầu. Nhiều trình
    đọc khác bắt đầu từ trang đang đọc. Cần chọn: giữ như hiện tại, hay chọn kết quả đầu tiên tính
    từ trang đang đọc (danh sách vẫn theo thứ tự trang)?
16. **Enter ở kết quả cuối.** Đã chọn: khi đã quét xong thì quay vòng về kết quả đầu (Shift+Enter ở
    kết quả đầu thì về kết quả cuối); khi chưa quét xong thì đứng yên chờ kết quả mới.
17. **Nút "Chạy OCR?"** OCR thuộc nhóm 9. Hiện nút "Chạy OCR" chỉ báo "Chức năng OCR chưa có trong
    phiên bản này". Khi làm nhóm 9 sẽ nối vào hộp thoại OCR.
18. **Bộ đếm khi chưa có kết quả nào mà vẫn đang quét.** Spec chỉ cho ví dụ "4 / 37…". Đã dùng
    "Đang tìm…". Khi quét xong mà không có kết quả nhưng có trang không có chữ: bộ đếm "0 / 0" và hiện
    câu "N trang không có chữ để tìm. Chạy OCR?" (không hiện "Không tìm thấy").
19. **Cụm nằm vắt qua hai trang** (cuối trang này, đầu trang sau) chưa được tìm thấy; spec chỉ nói ngắt
    dòng. Gạch nối mềm cuối dòng (PDFium đánh dấu `\u{2}`) thì được nối lại.
20. **Chữ đứng cạnh nhau khi bật Khớp nguyên từ**: "từ" là dãy chữ cái/chữ số (kể cả dấu); dấu câu,
    khoảng trắng, "/" và "-" là ranh giới từ.
21. **Mở bảng kết quả tự động.** Khi bắt đầu một lượt tìm, bảng bên chuyển sang thẻ "Kết quả tìm"
    (và hiện bảng bên nếu đang ẩn) để thấy danh sách và thông báo trang không có chữ.

## Nền tảng và CI

12. **Thư mục dự án rất sâu** làm đường dẫn file tạm của build script vượt 260 ký tự, `link.exe` của
    MSVC không mở được. Đã cấu hình `.cargo/config.toml` dùng `rust-lld` cho Windows. Khi chuyển mã sang
    kho chính (đường dẫn ngắn) có thể bỏ cấu hình này.
13. **CI chưa chạy lần nào.** `.github/workflows/ci.yml` đã viết cho Windows, macOS arm64 và Intel;
    các task cần "xác minh trên macOS / trong CI" (1.1, 1.2, 1.3, 2.7) để mở cho tới khi CI chạy.
14. Câu hỏi mở sẵn trong design.md (tên sản phẩm, PdfCraft, TSA) chưa ảnh hưởng nhóm 1–2.
