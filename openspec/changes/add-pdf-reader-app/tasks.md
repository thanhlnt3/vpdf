# Tasks

## 1. Nền tảng dự án

- [ ] 1.1 Tạo Cargo workspace gồm `vpdf-core`, `vpdf-render`, `vpdf-search`, `vpdf-ocr`, `vpdf-sign`, `vpdf-cli`, `vpdf-app`; xác minh bằng `cargo build --workspace` chạy được trên Windows và macOS
  - Trạng thái (2026-11-08): workspace đủ 7 crate (+ `tools/vpdf-testdata`, `tools/vpdf-license-check`); `cargo build --workspace` đã chạy được trên **Windows 10 x64**. Chưa xác minh trên macOS (job `build-test` cho macos-14/macos-13 trong `.github/workflows/ci.yml` chưa chạy).
- [ ] 1.2 Thêm `deny.toml` (danh sách license cho phép) và `third_party/native.toml`, chạy `cargo deny check licenses` trong CI; xác minh bằng một PR thử thêm crate GPL thì CI thất bại
  - Trạng thái: có `deny.toml`, `third_party/native.toml`, `third_party/license-exceptions.toml`, công cụ `vpdf-license-check` và job `licenses` trong CI. Đã xác minh cục bộ: `cargo deny check licenses` đạt; test `tools/vpdf-license-check/tests/cargo_deny.rs` cho thấy crate AGPL, GPL gián tiếp (A → B) và crate không rõ license làm kiểm tra thất bại. Chưa xác minh bằng một PR thật trên CI. `vpdf-license-check` hiện **thất bại có chủ ý**: 8 ngoại lệ (FreeType, libpng, IJG…, MSVC runtime) chưa có người duyệt — xem QUESTIONS.md.
- [ ] 1.3 Tích hợp PDFium dựng sẵn cho win-x64, mac-arm64, mac-x64 (universal qua `lipo`); xác minh bằng test dựng trang 1 của file mẫu trên cả ba kiến trúc trong CI
  - Trạng thái: PDFium chromium/8138 (win-x64) đã tích hợp qua `pdfium-render`; test dựng trang 1 (`renders_page_one_of_sample`) đạt trên Windows x64. `scripts/fetch-native.sh` tải mac-arm64, mac-x64 và ghép universal bằng `lipo` (kèm SHA-256), nhưng chưa chạy trên macOS/CI.
- [x] 1.4 Tích hợp qpdf (C API) và lopdf; xác minh bằng test mở, ghi lại và chạy `qpdf --check` trên bộ file mẫu
  - Đã xác minh (Windows x64): qpdf 12.4.2 nạp qua C API (`vpdf_core::qpdf`), lopdf 0.47; `crates/vpdf-core/tests/structure.rs` mở, kiểm tra, ghi lại bằng qpdf và lopdf rồi chạy `qpdf --check` trên bộ file mẫu — đạt.
- [ ] 1.5 Thêm bộ tài liệu tham chiếu (500 trang chữ, 500 trang scan, file có form, có chữ ký, có mật khẩu, file hỏng) cùng script tải về; xác minh mã băm khớp
  - Trạng thái: `tools/vpdf-testdata` tạo (xác định) 500 trang chữ ~58 MB, 500 trang scan ~146 MB, form, file có mật khẩu, file cấm sao chép, file xref hỏng, file không phải PDF; `scripts/fetch-testdata.*` tải từ `VPDF_TESTDATA_URL` hoặc tạo tại chỗ rồi kiểm tra SHA-256 theo `testdata/MANIFEST.json` — đạt. **Còn thiếu file có chữ ký số** (cần bộ công cụ ký của nhóm 10) và chưa có nơi lưu bộ file để tải về — xem QUESTIONS.md.

## 2. Đọc tài liệu (`document-viewing`)

- [x] 2.1 Luồng worker PDFium cho mỗi tài liệu và bộ nhớ đệm ảnh trang LRU 300 MB; xác minh bằng test giới hạn bộ nhớ khi dựng 500 trang
  - Đã xác minh: `DocumentWorker` (một luồng PDFium mỗi tài liệu) + `ByteLru` 300 MB; test `render_500_pages_within_memory_limits` dựng 500 trang: bộ nhớ đệm 299 MB, tiến trình tối đa 392 MB.
- [x] 2.2 Vùng xem cuộn liên tục hoặc từng trang, ô số trang, phóng 10%–6400%, Vừa trang, Vừa chiều rộng, phóng tại con trỏ; xác minh bằng test UI cho từng scenario trong spec
  - Đã xác minh: `tests/ui.rs` chạy giao diện egui thật với sự kiện giả lập cho từng scenario (Nhảy tới trang, Số trang không hợp lệ, Cuộn liên tục, Vừa chiều rộng, Phóng tại con trỏ, 10%–6400%, từng trang) + unit test `vpdf_core::view`; test `text_is_sharp_at_800_percent` cho chữ nét ở 800%.
- [x] 2.3 Mở file bằng hộp thoại, kéo thả, liên kết loại file, danh sách gần đây; hỏi mật khẩu; báo lỗi file hỏng; xác minh với các file mẫu tương ứng
  - Đã xác minh với file mẫu: kéo thả, file hỏng, file hỏng một phần, sai/đúng mật khẩu, danh sách gần đây, mở bằng tham số dòng lệnh và chuyển file sang cửa sổ đang chạy (`single_instance`). Hộp thoại Mở dùng `rfd` (hộp thoại hệ điều hành, không tự động hóa được; phần sau hộp thoại dùng chung `open_path` đã test). Việc **đăng ký** ứng dụng làm trình mở `.pdf` thuộc bộ cài (task 12.1) và cần người dùng đồng ý.
- [x] 2.4 Tab nhiều tài liệu và nhớ vị trí đọc của 50 file gần nhất; xác minh bằng test đóng rồi mở lại file
  - Đã xác minh: `tabs_keep_independent_position` (trang 40, 150%) và `reopen_restores_reading_position` (đóng ở trang 87, khởi động lại, mở lại tại trang 87); giữ 50 file gần nhất (`keeps_only_50_most_recent`).
- [x] 2.5 Bảng ảnh thu nhỏ tải dần và cây mục lục (UTF-16 tiếng Việt); xác minh bằng các scenario trong spec
  - Đã xác minh: ảnh thu nhỏ theo phần đang thấy trước, không chặn trang đang đọc (`thumbnails_visible_first_and_do_not_block_reading`, `thumbnails_load_progressively`); bấm ảnh thu nhỏ; mục lục UTF-16 tiếng Việt, đi tới mục, tài liệu không có mục lục.
- [x] 2.6 Link nội bộ cùng lệnh Quay lại, link URL hỏi xác nhận; chọn và sao chép chữ dạng NFC, chặn sao chép khi tài liệu cấm; xác minh bằng test tự động
  - Đã xác minh: link nội bộ + Quay lại (Alt+←), link URL hỏi "Mở liên kết này trong trình duyệt?", kéo chọn + sao chép NFC, chặn sao chép với file cấm sao chép (mở bằng mật khẩu chủ sở hữu thì được).
- [ ] 2.7 Đo khởi động nguội với file 500 trang trong CI; xác minh dưới 3 giây trên runner tham chiếu
  - Trạng thái: test `open_500_pages_under_3_seconds` đo cục bộ (bộ nhớ đệm ổ đĩa còn nóng): mở + trang 1 = 66 ms (500 trang chữ), 36 ms (500 trang scan). Bước CI đã viết; chưa đo khởi động nguội trên runner tham chiếu.

## 3. Tìm chữ (`text-search`)

- [ ] 3.1 Trích chữ chạy nền theo thứ tự ưu tiên trang, tạo chuỗi chuẩn hóa kèm bảng ánh xạ vị trí; xác minh bằng unit test với "Nghị định", "Đà Nẵng" và chữ có dấu tổ hợp
- [ ] 3.2 Ô tìm khi đang gõ (chờ 150 ms, hủy lượt cũ), bộ đếm cập nhật dần, danh sách kết quả; xác minh kết quả đầu tiên hiện dưới 500 ms trên file 500 trang
- [ ] 3.3 Tùy chọn Phân biệt dấu, Phân biệt hoa thường, Khớp nguyên từ; tìm vượt ngắt dòng; báo trang không có lớp chữ; xác minh bằng các scenario trong spec

## 4. Gộp, tách, trích trang (`document-merge`, `document-split`)

- [ ] 4.1 Gộp: danh sách file, khoảng trang cho mỗi file, mục lục lồng dưới tên file, đổi tên đích có tên và trường form bị trùng; xác minh link và mục lục bằng test trên file mẫu
- [ ] 4.2 Bộ phân tích cú pháp khoảng trang (`1-3,5,8-`, `odd`, `even`) kèm thông báo lỗi; xác minh bằng unit test
- [ ] 4.3 Tách theo khoảng trang, mỗi N trang, theo mục lục (chọn cấp, xử lý ký tự cấm trong tên file); xác minh bằng các scenario trong spec
- [ ] 4.4 Trích trang (có hoặc không xóa khỏi tài liệu gốc); bỏ link hỏng, giữ mục lục con; hỏi trước khi ghi đè; xác minh bằng test tự động

## 5. Sắp xếp trang và hoàn tác (`page-organization`)

- [ ] 5.1 Mô hình lệnh có apply/revert, ngăn xếp 100 bước hoặc 200 MB, giữ lịch sử qua các lần Lưu; xác minh bằng unit test cho chuỗi thao tác hoàn tác và làm lại
- [ ] 5.2 Chọn nhiều trang, xoay, xóa (chặn xóa hết), chèn trang trắng, trang từ PDF khác hoặc ảnh; xác minh bằng các scenario trong spec
- [ ] 5.3 Kéo thả trong bảng ảnh thu nhỏ và lưới, kể cả kéo sang tab khác; xác minh bằng test UI
- [ ] 5.4 Lưu nguyên tử (ghi file tạm rồi đổi tên) và cảnh báo thay đổi chưa lưu; xác minh bằng test dừng tiến trình giữa lúc lưu

## 6. Bảo mật (`document-security`)

- [ ] 6.1 Đặt mật khẩu mở AES-256 R6 (SASLprep, mật khẩu tiếng Việt) qua qpdf; xác minh bằng cách mở lại bằng PDFium và bằng Acrobat
- [ ] 6.2 Đặt quyền cấm in, cấm sao chép kèm bắt buộc mật khẩu chủ sở hữu; tôn trọng quyền khi mở; mở khóa bằng mật khẩu chủ sở hữu; xác minh bằng các scenario trong spec
- [ ] 6.3 Gỡ bảo vệ; cảnh báo mã hóa RC4/AES-128; xác minh nhật ký không chứa mật khẩu bằng test quét chuỗi

## 7. Ghi chú và form (`annotations`, `form-filling`)

- [ ] 7.1 Highlight, gạch chân, gạch ngang nhiều dòng; ghi chú dán; bình luận, trả lời, trạng thái; kèm appearance stream; xác minh mở bằng Acrobat thấy đủ theo checklist
- [ ] 7.2 Bảng Bình luận có lọc và tìm không dấu; sửa, xóa, hoàn tác; giữ annotation lạ; làm phẳng; xác minh bằng test tự động
- [ ] 7.3 Điền mọi loại trường AcroForm, Tab theo thứ tự, ràng buộc của trường, cảnh báo trường bắt buộc; xác minh bằng file form mẫu
- [ ] 7.4 Sinh appearance stream bằng font Noto Sans nhúng một phần khi thiếu ký tự tiếng Việt; báo XFA và JavaScript không chạy; xác minh "Nguyễn Hữu" hiển thị đúng trong Acrobat và trình duyệt

## 8. Nén (`compression`)

- [ ] 8.1 Mức Nhẹ dùng qpdf (object streams, gộp đối tượng trùng, nén lại luồng Flate); xác minh điểm ảnh dựng ra giống hệt bản gốc ở 150 dpi
- [ ] 8.2 Mức Vừa và Mạnh: tính dpi hiệu dụng, giảm độ phân giải, nén JPEG; giữ lớp chữ, mục lục, mã hóa; xác minh bằng các scenario trong spec
- [ ] 8.3 Báo dung lượng trước và sau, không tạo file lớn hơn, cảnh báo file đã ký; xác minh bằng test tự động

## 9. OCR (`ocr`)

- [ ] 9.1 Dựng Tesseract 5 và Leptonica tĩnh cho cả ba kiến trúc, đóng gói dữ liệu `vie` và `eng`; xác minh bằng test OCR chạy khi đã chặn mạng
- [ ] 9.2 Dựng ảnh trang 300 dpi, chỉnh nghiêng, tự xoay, ghi lớp chữ ẩn đúng vị trí từ hOCR; xác minh bằng test tìm và sao chép ra NFC
- [ ] 9.3 Chọn trang và ngôn ngữ, bỏ qua trang đã có chữ, OCR lại thì thay lớp cũ, có tiến độ và hủy được; xác minh bằng các scenario trong spec
- [ ] 9.4 Bộ mẫu kiểm thử độ chính xác và script tính CER; xác minh CER ≤ 3% và trung bình ≤ 4 giây/trang trên máy tham chiếu

## 10. Ký số (`digital-signatures`)

- [ ] 10.1 Nạp `.p12`/`.pfx` qua OpenSSL 3 (có legacy provider), hiện thông tin chứng thư, chặn chứng thư hết hạn; xác minh với file mẫu RSA, ECDSA và loại mã hóa cũ
- [ ] 10.2 Ký PAdES B-B bằng bản cập nhật nối thêm, chữ ký ẩn hoặc hiện, ký vào trường có sẵn, chặn khi tài liệu cấm ký; xác minh Acrobat báo chữ ký toàn vẹn
- [ ] 10.3 Kiểm tra chữ ký có sẵn: toàn vẹn, chuỗi chứng thư (kho của hệ điều hành và CA người dùng thêm), thay đổi sau khi ký, ghi rõ chưa kiểm tra thu hồi; xác minh bằng bộ file mẫu hợp lệ, bị sửa và không rõ nguồn gốc
- [ ] 10.4 Không lưu khóa riêng hay mật khẩu; cảnh báo trước thao tác làm mất hiệu lực chữ ký; xác minh bằng test tự động

## 11. CLI (`batch-cli`)

- [ ] 11.1 Lệnh `vpdf merge|split|compress|ocr` kèm `--help` và `--version`, gọi chung lõi với ứng dụng; xác minh kết quả giống hệt bản desktop trên bộ file mẫu
- [ ] 11.2 Nhận glob, thư mục `--recursive`, stdin `-`; chạy song song bằng tiến trình con với `--jobs`; xác minh với 200 file có 1 file hỏng
- [ ] 11.3 Mã thoát 0/1/2, `--json` dạng JSON Lines, `--overwrite`, `--dry-run`, `--fail-fast`, mật khẩu qua `--password-file` hoặc `VPDF_PASSWORD`; xác minh bằng test CLI cho từng scenario

## 12. Đóng gói và phát hành (`platform-support`, `dependency-licensing`, `performance`)

- [ ] 12.1 MSI ký Authenticode, cài im lặng, thêm `vpdf` vào PATH; xác minh trên máy ảo Windows 10 22H2 và Windows 11 mới cài
- [ ] 12.2 DMG universal ký Developer ID và đã notarize; xác minh chạy native trên M1 và Intel với macOS 13, Gatekeeper không cảnh báo
- [ ] 12.3 Giao diện tiếng Việt và tiếng Anh; xử lý đường dẫn tiếng Việt, đường dẫn dài và tên file NFD; xác minh bằng test tự động trên cả hai hệ điều hành
- [ ] 12.4 Sinh `THIRD_PARTY_NOTICES` (cargo-about) và SBOM (cargo-cyclonedx), mục Giới thiệu → Giấy phép bên thứ ba, `vpdf --licenses`; xác minh danh sách khớp cây phụ thuộc
- [ ] 12.5 Workflow phát hành GitHub Actions tạo release kèm `SHA256SUMS.txt`, có bước chặn khi vượt ngưỡng hiệu năng (3 giây, 1 GB); xác minh bằng một bản phát hành thử
- [ ] 12.6 Kiểm thử tích hợp toàn bộ: chạy checklist tương thích Acrobat (annotation, form, mật khẩu, chữ ký) và bộ đo hiệu năng trên máy tham chiếu thật cho cả hai nền tảng

## Workflow follow-up

- Chạy `openspec archive add-pdf-reader-app` sau khi mọi task hoàn thành và đã được review, để đưa spec vào `openspec/specs/`.
