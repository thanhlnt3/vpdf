# VPdf (tên tạm)

Ứng dụng đọc PDF desktop viết bằng Rust + egui, dựng hình bằng PDFium, theo OpenSpec trong
`openspec/changes/add-pdf-reader-app` (bản sao trong thư mục này; `tasks.md` ở đây được đánh dấu
tiến độ). Lượt này làm **nhóm task 1 (nền tảng)** và **nhóm 2 (đọc tài liệu)**.

## Cấu trúc

| Đường dẫn | Nội dung |
|---|---|
| `src/` (crate `vpdf-app`) | Ứng dụng desktop: tab, thanh công cụ, ô số trang, phóng, bảng ảnh thu nhỏ / mục lục, vùng xem, hộp thoại |
| `crates/vpdf-core` | Lõi dùng chung không phụ thuộc giao diện: vùng xem, ô số trang, lịch sử Quay lại, file gần đây, chọn/sao chép chữ, kiểm tra xref, LRU, qpdf (C API), lopdf |
| `crates/vpdf-render` | PDFium: một luồng worker mỗi tài liệu, ô ảnh 512×512, ảnh thu nhỏ, lớp chữ, link, mục lục |
| `crates/vpdf-search`, `vpdf-ocr`, `vpdf-sign` | Chỗ giữ chỗ cho nhóm 3, 9, 10 |
| `crates/vpdf-cli` | CLI `vpdf` (lượt này chỉ có `--version`, `--help`) |
| `tools/vpdf-testdata` | Tạo / tải / kiểm tra mã băm bộ tài liệu tham chiếu |
| `tools/vpdf-license-check` | Kiểm tra license thư viện native, font, dữ liệu và ngoại lệ đã duyệt |
| `third_party/` | `native.toml`, `license-exceptions.toml`; PDFium và qpdf tải về bằng `scripts/fetch-native.*` |
| `deny.toml` | Danh sách license cho phép cho `cargo deny check licenses` |
| `QUESTIONS.md` | Điểm spec chưa rõ, chờ quyết định |

## Dựng và chạy (Windows)

```powershell
powershell -ExecutionPolicy Bypass -File scripts\fetch-native.ps1   # PDFium + qpdf (kiểm tra SHA-256)
cargo build --workspace
cargo run -- path\to\file.pdf                                       # hoặc kéo thả file vào cửa sổ
```

macOS: `scripts/fetch-native.sh` (hoặc `mac-universal` để ghép bằng `lipo`).

## Kiểm thử

```powershell
scripts\fetch-testdata.ps1          # tạo hoặc tải bộ tài liệu tham chiếu rồi kiểm tra mã băm
cargo test --workspace --release    # 80+ test: lõi, PDFium, giao diện, qpdf/lopdf, license
tools\bin\cargo-deny check licenses # hoặc cargo deny check licenses
cargo run -p vpdf-license-check     # hiện thất bại có chủ ý: ngoại lệ chưa có người duyệt (QUESTIONS.md)
```

Chế độ tự động: `vpdf-app --screenshot shots/now.png` và `vpdf-app --self-test shots/selftest.json`.

## Phím tắt

| Phím | Việc |
|---|---|
| Ctrl/Cmd+O | Mở file |
| Ctrl/Cmd+W | Đóng tab · Ctrl+Tab: tab kế |
| ↑ ↓ / Page Up, Page Down, Space / Home, End | Cuộn dòng / cuộn màn hình (từng trang: sang trang) / trang đầu, cuối |
| ← → | Trang trước / sau (hoặc cuộn ngang khi trang rộng hơn cửa sổ) |
| Ctrl/Cmd + con lăn, chụm touchpad | Phóng tại con trỏ |
| Ctrl/Cmd + = / − | Phóng to / thu nhỏ một nấc (×1,25) |
| Ctrl/Cmd + 0 / 1 / 2 | Vừa trang / Kích thước thật / Vừa chiều rộng |
| Alt+← | Quay lại (sau khi bấm link, mục lục, ảnh thu nhỏ, nhảy trang) |
| Ctrl/Cmd+C | Sao chép chữ đã chọn (kéo chuột trái trên trang để chọn) |
| Chuột giữa kéo | Kéo trang |
