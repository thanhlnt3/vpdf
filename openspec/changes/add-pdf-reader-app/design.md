# Design

## Context

Dự án mới, chưa có mã nguồn (động lực: xem `proposal.md`, mục Why). Có ba ràng buộc chi phối thiết kế:

- **License**: chỉ dùng license cho phép dùng thương mại (`specs/dependency-licensing`). Điều này loại ngay các engine PDF phổ biến dùng copyleft: MuPDF (AGPL), Poppler (GPL), Ghostscript (AGPL), iText (AGPL).
- **Hiệu năng**: mở file 500 trang dưới 3 giây, dưới 1 GB RAM (`specs/performance`).
- **Nền tảng**: Windows x64 và macOS universal, không cần cài runtime (`specs/platform-support`).

Ứng dụng tham khảo phong cách của bộ Crafting Apps (ArtCraft): Rust, native, không dùng Electron hay web view. Lưu ý: bộ ArtCraft đã có sẵn **PdfCraft** ("PDF workbench"), nên cần quyết định là đóng góp vào PdfCraft hay phát triển độc lập (xem Open Questions).

## Goals / Non-Goals

**Goals:**
- Một lõi xử lý PDF dùng chung cho cả ứng dụng desktop và CLI, để hai bên cho ra kết quả giống nhau (`specs/batch-cli`).
- Tách phần dựng hình (đọc) khỏi phần sửa cấu trúc (ghi), để chọn được thư viện tốt nhất cho từng việc mà vẫn đúng license.
- Mọi thao tác sửa tài liệu đi qua một mô hình lệnh chung, để có hoàn tác nhiều bước (`specs/page-organization`).

**Non-Goals:**
- Không tự viết engine dựng hình PDF.
- Không chạy JavaScript trong PDF.
- Không hỗ trợ XFA động, không sửa nội dung chữ/ảnh của trang.
- Không hỗ trợ Linux ở phiên bản đầu, dù kiến trúc không chặn việc này.

## Decisions

### D1. Ngôn ngữ và giao diện: Rust + egui
- **Chọn**: Rust cho toàn bộ dự án; giao diện dùng `egui`/`eframe` (MIT/Apache-2.0) với backend `wgpu`.
- **Lý do**: ra một file chạy duy nhất, không cần runtime, khởi động nhanh. Cùng hệ sinh thái với Crafting Apps, nên dùng lại được `craft-libs`, `craft-i18n` và widget egui của bộ.
- **Phương án đã cân nhắc**: Tauri (dùng web view, đi ngược phong cách "không web view" và tốn RAM hơn); Qt (license LGPL/thương mại, khó phân phối tĩnh); Slint (license royalty-free riêng, không nằm trong danh sách cho phép).

### D2. Dựng hình và trích chữ: PDFium
- **Chọn**: PDFium (BSD-3-Clause, một phần Apache-2.0), gọi qua crate `pdfium-render` (MIT/Apache-2.0), dùng bản dựng sẵn của `bblanchon/pdfium-binaries` cho win-x64, mac-arm64 và mac-x64 (ghép thành universal bằng `lipo`).
- **Lý do**: đây là engine của Chrome, đã chạy với hàng tỷ file PDF thực tế. Có sẵn dựng hình, trích chữ kèm tọa độ, form AcroForm, annotation, mở file mã hóa, và khôi phục file hỏng.
- **Phương án đã cân nhắc**: MuPDF (nhanh nhưng AGPL); pdf.js (Apache-2.0 nhưng cần JS engine); `hayro` (Rust thuần, còn non).
- **Hệ quả**: PDFium không an toàn khi gọi từ nhiều luồng. Vì vậy mọi lệnh gọi PDFium đi qua một luồng worker riêng cho mỗi tài liệu. CLI chạy song song bằng nhiều tiến trình con, không dùng nhiều luồng (xem Risks).

### D3. Sửa cấu trúc PDF: lopdf + qpdf
- **Chọn**:
  - `lopdf` (MIT) cho gộp, tách, chèn và xóa trang, viết lại mục lục và đích của link, đổi tên trường form và đích có tên.
  - `qpdf` (Apache-2.0, gọi qua C API) cho mã hóa và giải mã AES-256 R6, đặt quyền, object streams, nén lại luồng dữ liệu khi lưu, và kiểm tra cấu trúc (`--check`).
- **Lý do**: lopdf thuận tiện để thao tác đồ thị đối tượng ngay trong Rust. qpdf là công cụ tham chiếu cho mã hóa và chuẩn hóa file, đã được kiểm thử rất kỹ.
- **Phương án đã cân nhắc**: chỉ dùng lopdf (phần mã hóa R6 chưa đủ chín, rủi ro tương thích với Acrobat); dùng PDFium để ghi (PDFium không hỗ trợ mã hóa khi ghi).

### D4. Tìm chữ không phân biệt dấu
- **Chọn**: lớp chữ của mỗi trang được trích một lần (chạy nền, ưu tiên các trang gần trang đang đọc) thành chuỗi gốc kèm hộp tọa độ từng ký tự. Từ đó tạo chuỗi đã chuẩn hóa (NFD, bỏ dấu, đ→d, chữ thường) kèm bảng ánh xạ vị trí ngược về chuỗi gốc. Tìm bằng thuật toán tìm chuỗi con trên chuỗi đã chuẩn hóa, rồi dùng bảng ánh xạ để tô sáng đúng ký tự gốc. Thư viện: `unicode-normalization` (MIT/Apache-2.0).
- **Lý do**: tìm khi đang gõ cần nhanh; bảng ánh xạ cho phép tô sáng đúng chữ có dấu (`specs/text-search`). Với 500 trang chữ (khoảng 2 triệu ký tự), tìm tuyến tính chỉ mất vài chục ms, không cần chỉ mục đảo.
- **Phương án đã cân nhắc**: chỉ mục toàn văn kiểu tantivy (thừa với một tài liệu, tốn RAM và thời gian lập chỉ mục).

### D5. Hoàn tác nhiều bước
- **Chọn**: mỗi tab giữ một mô hình tài liệu, gồm danh sách trang tham chiếu tới đối tượng gốc cộng các thay đổi chồng lên (xoay, annotation, giá trị form). Mỗi thao tác là một lệnh có `apply` và `revert`. Ngăn xếp hoàn tác và làm lại giới hạn 100 bước hoặc 200 MB. Chỉ khi Lưu mới dựng lại file PDF từ mô hình.
- **Lý do**: thao tác trang chỉ đổi danh sách tham chiếu nên nhanh, hoàn tác rẻ, và Lưu chạy nguyên tử: ghi file tạm, rồi đổi tên để thay file đích.
- **Phương án đã cân nhắc**: chụp toàn bộ file sau mỗi bước (tốn bộ nhớ, vi phạm ngưỡng 1 GB).

### D6. Annotation và form tương thích Acrobat
- **Chọn**: tạo annotation bằng PDFium (Highlight, Underline, StrikeOut, Text, Popup, trả lời qua `IRT`/`RT`) và tự sinh appearance stream. Giá trị form ghi qua PDFium kèm appearance stream sinh lại. Khi font của form thiếu ký tự tiếng Việt, nhúng một phần font Noto Sans (OFL-1.1) bằng `subsetter` (MIT/Apache-2.0).
- **Kiểm chứng**: bộ file mẫu được mở bằng Adobe Acrobat Reader trong bước kiểm thử thủ công trước mỗi bản phát hành, kèm kiểm tra tự động bằng `qpdf --check` và dựng lại bằng PDFium.

### D7. Nén 3 mức
- **Chọn**: luồng xử lý gồm (1) giải mã ảnh, (2) giảm độ phân giải theo dpi hiệu dụng tính từ ma trận vẽ ảnh trên trang, (3) nén JPEG bằng `jpeg-encoder` (MIT/Apache-2.0) hoặc mozjpeg (IJG/BSD-3-Clause/Zlib), (4) chuẩn hóa bằng qpdf: object streams, gộp đối tượng trùng, nén lại luồng Flate. Mức Nhẹ chỉ chạy bước 4.
- **Kiểm chứng**: so sánh điểm ảnh khi dựng hình ở mức Nhẹ (phải giống hệt, `specs/compression`).

### D8. OCR offline
- **Chọn**: Tesseract 5 (Apache-2.0) cùng Leptonica (license kiểu BSD-2-Clause), liên kết tĩnh, kèm dữ liệu `vie` và `eng` từ `tessdata_best` (Apache-2.0) đi trong bộ cài (khoảng 25 MB). Ảnh trang được dựng bằng PDFium ở 300 dpi. Kết quả từ Tesseract (hOCR/TSV) dùng để ghi lớp chữ ẩn (render mode 3) bằng font GlyphLessFont, ánh xạ ToUnicode để chép ra đúng NFC.
- **Song song**: mỗi trang là một tác vụ độc lập. Số luồng OCR được giới hạn theo bộ nhớ (khoảng 150 MB mỗi luồng) để không vượt 1 GB.
- **Phương án đã cân nhắc**: `ocrs` (Rust thuần, MIT/Apache-2.0, nhưng chưa hỗ trợ tốt dấu tiếng Việt); PaddleOCR với ONNX Runtime (Apache-2.0/MIT, chính xác hơn với ảnh xấu nhưng bộ cài nặng hơn). PaddleOCR được giữ làm phương án dự phòng nếu Tesseract không đạt CER ≤ 3%.

### D9. Ký số và kiểm tra chữ ký
- **Chọn**: OpenSSL 3 (Apache-2.0) qua crate `openssl` (Apache-2.0) để đọc PKCS#12 (bật legacy provider cho file `.pfx` cũ mã hóa RC2/3DES, loại vẫn phổ biến ở các CA tại Việt Nam), tạo và kiểm tra CMS, và kiểm tra chuỗi chứng thư. Phần PDF gồm trường chữ ký, `ByteRange`, vùng giữ chỗ `/Contents`, bản cập nhật nối thêm, và phân tích thay đổi sau khi ký (DocMDP) do dự án tự viết trên lopdf.
- **Kho tin cậy**: kho của hệ điều hành (Windows CertStore, macOS Keychain), cộng một kho riêng của ứng dụng cho CA gốc người dùng thêm vào.
- **Phương án đã cân nhắc**: RustCrypto (`cms`, `pkcs12`, `x509-cert`; Rust thuần nhưng PKCS#12 bản cũ chưa đủ). Có thể chuyển sang RustCrypto sau, khi các crate này chín hơn.

### D10. CLI
- **Chọn**: `clap` (MIT/Apache-2.0) cho phần nhận tham số. Lệnh `vpdf` là một tiến trình điều phối, chạy mỗi file trong một tiến trình con (`vpdf __worker`) để tránh giới hạn luồng của PDFium và cô lập file hỏng. Đầu ra `--json` theo dạng JSON Lines.
- **Phân phối**: `vpdf.exe` cài cạnh ứng dụng và được thêm vào PATH (Windows). Trên macOS, có tùy chọn tạo symlink `/usr/local/bin/vpdf`.

### D11. Kiểm tra license tự động
- **Chọn**:
  - `cargo-deny` với danh sách license cho phép (`deny.toml`) cho các crate Rust.
  - Một manifest tự viết (`third_party/native.toml`) cho thư viện native (PDFium, qpdf, Tesseract, Leptonica, OpenSSL), font và dữ liệu OCR, kiểm tra trong CI.
  - `cargo-about` để sinh `THIRD_PARTY_NOTICES`, `cargo-cyclonedx` để sinh SBOM.

### D12. Đóng gói và phát hành
- **Windows**: MSI tạo bằng WiX Toolset qua `cargo-wix` (WiX chỉ là công cụ dựng, không đi kèm sản phẩm), ký Authenticode.
- **macOS**: dựng riêng cho `aarch64-apple-darwin` và `x86_64-apple-darwin`, ghép bằng `lipo`, đóng gói `.app` vào DMG, ký Developer ID, notarize bằng `notarytool`.
- **CI**: GitHub Actions với runner `windows-latest` và `macos-14`. Mỗi bản phát hành kèm `SHA256SUMS.txt`, cùng định dạng mà ArtCraft Launcher dùng.

## Risks / Trade-offs

- [PDFium không an toàn khi gọi từ nhiều luồng] → Mỗi tài liệu có một luồng worker riêng; CLI chạy song song bằng nhiều tiến trình.
- [Ngưỡng 1 GB RAM khi mở file scan lớn] → Bộ nhớ đệm ảnh trang (LRU) giới hạn 300 MB; chỉ dựng phần đang nhìn thấy ở mức phóng cao; ảnh thu nhỏ lưu ở dạng JPEG nhỏ.
- [Chuẩn 3 giây khi khởi động nguội trên ổ chậm] → Mở file theo kiểu đọc dần (chỉ nạp xref và trang 1). Các việc như ảnh thu nhỏ, trích chữ và mục lục sâu chạy nền sau khi trang 1 đã hiện.
- [Tương thích Acrobat khó kiểm tra tự động] → Danh sách kiểm tra thủ công cố định cho mỗi bản phát hành, cộng bộ file "vàng" đã xác nhận để so sánh cấu trúc tự động.
- [Độ chính xác OCR với ảnh xấu] → Bộ mẫu kiểm thử có cả ảnh chụp điện thoại; nếu Tesseract không đạt CER ≤ 3% thì chuyển sang phương án PaddleOCR + ONNX (D8).
- [File `.pfx` mã hóa kiểu cũ] → Bật legacy provider của OpenSSL; kiểm thử với file mẫu từ nhiều CA.
- [Kích thước bộ cài lớn (PDFium khoảng 10 MB, Tesseract cùng dữ liệu khoảng 30 MB, font khoảng 5 MB)] → Chấp nhận, vì đổi lại được chạy offline; nén bộ cài.
- [Rủi ro pháp lý khi tham khảo sản phẩm thương mại] → Chỉ tham khảo nhóm tính năng; dùng Craft Anything ở chế độ Clean room; không dùng tên, logo hay giao diện của sản phẩm khác (`specs/dependency-licensing`).

## Migration Plan

Không áp dụng, vì đây là dự án mới. Kế hoạch phát hành: alpha nội bộ (6 chức năng cơ bản), rồi beta (thêm 6 chức năng nâng cao), rồi bản 1.0 khi mọi ngưỡng trong `specs/performance` qua CI trên cả hai nền tảng.

## Open Questions

- Tên sản phẩm chính thức thay cho tên tạm "VPdf". Cần tra cứu nhãn hiệu trước khi chọn.
- Phát triển độc lập, hay đóng góp tính năng vào PdfCraft của bộ ArtCraft (dự án Apache-2.0, đã có sẵn)? Câu hỏi này không đổi spec, chỉ đổi kho mã và quy trình phát hành.
- Có cần dấu thời gian (TSA, PAdES B-T) ở bản 1.0 không, hay để bản sau?
