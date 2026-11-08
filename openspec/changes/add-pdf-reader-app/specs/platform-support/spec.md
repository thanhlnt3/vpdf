# Spec Delta

## Purpose

Xác định các hệ điều hành, kiến trúc CPU và cách cài đặt mà ứng dụng desktop cùng CLI `vpdf` phải hỗ trợ.

## ADDED Requirements

### Requirement: Windows 10 và 11 x64
Ứng dụng và CLI SHALL chạy được trên Windows 10 (bản 22H2) và Windows 11 bản x64, không cần cài thêm runtime nào. Bộ cài MSI MUST được ký mã (Authenticode), cài được cho một người dùng hoặc cho cả máy, và cài im lặng được (`msiexec /i ... /qn`).

#### Scenario: Cài trên máy Windows mới
- **GIVEN** máy Windows 10 22H2 x64 mới cài, chưa có thư viện nào thêm
- **WHEN** người dùng chạy bộ cài MSI rồi mở ứng dụng
- **THEN** ứng dụng mở và đọc được file PDF

#### Scenario: Cài im lặng
- **WHEN** quản trị viên chạy `msiexec /i vpdf.msi /qn`
- **THEN** ứng dụng được cài không hiện hộp thoại nào và `vpdf --version` chạy được trong terminal mới

#### Scenario: Màn hình độ phân giải cao
- **WHEN** chạy trên màn hình đặt tỉ lệ 150% hoặc 200%
- **THEN** giao diện và trang PDF hiển thị nét, không bị mờ do phóng ảnh

### Requirement: macOS 13 trở lên, Apple silicon và Intel
Ứng dụng và CLI SHALL chạy native trên macOS 13 trở lên, cả máy Apple silicon (arm64) và Intel (x86_64), phát hành dưới dạng một bản universal. Bản phát hành MUST được ký bằng Developer ID và notarize để Gatekeeper cho mở mà không cảnh báo.

#### Scenario: Chạy native trên Apple silicon
- **WHEN** chạy ứng dụng trên máy Mac M1 với macOS 13
- **THEN** tiến trình chạy ở kiến trúc arm64, không qua Rosetta

#### Scenario: Chạy trên Mac Intel
- **WHEN** chạy cùng bản phát hành trên Mac Intel với macOS 13
- **THEN** ứng dụng mở và đọc được file PDF

#### Scenario: Gatekeeper
- **WHEN** người dùng tải bản DMG về và mở ứng dụng lần đầu
- **THEN** macOS không hiện cảnh báo "không xác định được nhà phát triển"

### Requirement: Hoạt động đúng chuẩn của từng nền tảng
Ứng dụng SHALL dùng phím tắt, hộp thoại file, menu và clipboard theo chuẩn của từng hệ điều hành (Ctrl trên Windows, Cmd trên macOS). Ứng dụng MUST đăng ký được làm trình mở `.pdf` mặc định, nhưng chỉ khi người dùng đồng ý.

#### Scenario: Phím tắt trên macOS
- **WHEN** người dùng nhấn Cmd+F trên macOS
- **THEN** ô tìm chữ được mở

#### Scenario: Mở bằng nhấp đúp
- **GIVEN** người dùng đã chọn ứng dụng làm trình mở PDF mặc định
- **WHEN** người dùng nhấp đúp một file `.pdf` trong Explorer hoặc Finder
- **THEN** file mở trong ứng dụng, thêm thành tab nếu ứng dụng đang chạy

### Requirement: Đường dẫn và tên file tiếng Việt
Ứng dụng và CLI SHALL mở và lưu được file có đường dẫn chứa ký tự tiếng Việt, dấu cách và đường dẫn dài hơn 260 ký tự trên Windows. Trên macOS, tên file ở dạng NFD MUST được xử lý như tên ở dạng NFC.

#### Scenario: Tên file có dấu
- **WHEN** người dùng mở `D:\Tài liệu\Hợp đồng thuê nhà.pdf`
- **THEN** file mở được và tên hiển thị đúng dấu

### Requirement: Giao diện tiếng Việt và tiếng Anh
Giao diện SHALL có tiếng Việt và tiếng Anh, mặc định theo ngôn ngữ của hệ điều hành, và đổi được trong Cài đặt mà không cần cài lại.

#### Scenario: Đổi ngôn ngữ
- **WHEN** người dùng đổi ngôn ngữ giao diện từ tiếng Anh sang tiếng Việt trong Cài đặt
- **THEN** menu và hộp thoại hiển thị tiếng Việt
