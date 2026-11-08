# Spec Delta

## Purpose

Đặt ngưỡng đo được cho thời gian mở file, độ mượt khi đọc và bộ nhớ tối đa, để ứng dụng chạy tốt trên máy văn phòng phổ thông 8 GB RAM.

## ADDED Requirements

### Requirement: Máy và tài liệu tham chiếu
Mọi ngưỡng hiệu năng SHALL được đo trên máy tham chiếu: 8 GB RAM, CPU 4 nhân (ví dụ Intel Core i5 thế hệ 8 hoặc Apple M1), ổ SSD; và trên bộ tài liệu tham chiếu lưu trong kho mã, gồm ít nhất một file 500 trang nhiều chữ có ảnh minh họa (khoảng 60 MB) và một file scan 500 trang (khoảng 150 MB).

#### Scenario: Bộ tài liệu tham chiếu có sẵn
- **WHEN** chạy bộ kiểm thử hiệu năng
- **THEN** các file tham chiếu được tải từ kho mã, không phụ thuộc vào file của người dùng

### Requirement: Mở file 500 trang dưới 3 giây
Với file tham chiếu 500 trang, thời gian từ lúc ra lệnh mở tới lúc trang đầu hiển thị nét và vùng xem cuộn được SHALL dưới 3 giây trên máy tham chiếu, đo khi bộ nhớ đệm của ổ đĩa còn trống (khởi động nguội). Phần việc chạy nền như ảnh thu nhỏ và lập chỉ mục tìm kiếm MUST không tính vào và không chặn mốc này.

#### Scenario: Mở file chữ 500 trang
- **GIVEN** ứng dụng đang chạy, chưa mở tài liệu nào
- **WHEN** mở file tham chiếu 500 trang nhiều chữ
- **THEN** trang 1 hiển thị nét và cuộn được trong vòng 3 giây

#### Scenario: Mở file scan 500 trang
- **WHEN** mở file tham chiếu scan 500 trang
- **THEN** trang 1 hiển thị nét và cuộn được trong vòng 3 giây

#### Scenario: Khởi động ứng dụng kèm file
- **GIVEN** ứng dụng chưa chạy
- **WHEN** người dùng nhấp đúp file tham chiếu 500 trang
- **THEN** trang 1 hiển thị nét trong vòng 4 giây kể từ lúc nhấp đúp

### Requirement: Dưới 1 GB RAM
Khi làm việc với một file tham chiếu 500 trang, bộ nhớ tối đa của mọi tiến trình ứng dụng cộng lại SHALL dưới 1 GB (đo bằng private working set trên Windows, physical footprint trên macOS). Việc này gồm mở file, cuộn hết từ đầu đến cuối, mở bảng ảnh thu nhỏ và tìm chữ. Bộ nhớ đệm trang MUST tự giải phóng để không vượt ngưỡng.

#### Scenario: Cuộn hết tài liệu
- **WHEN** mở file tham chiếu 500 trang, cuộn liên tục từ trang 1 tới trang 500 ở mức phóng 100%, rồi tìm một từ có trên mọi trang
- **THEN** bộ nhớ tối đa trong suốt quá trình dưới 1 GB

#### Scenario: Phóng rất to
- **WHEN** phóng trang tới 6400%
- **THEN** ứng dụng chỉ dựng hình phần đang nhìn thấy và bộ nhớ vẫn dưới 1 GB

#### Scenario: Chạy hàng loạt bằng CLI
- **WHEN** chạy `vpdf ocr` trên 50 file scan với số luồng mặc định trên máy tham chiếu
- **THEN** tổng bộ nhớ của tiến trình `vpdf` luôn dưới 1 GB

### Requirement: Giao diện luôn phản hồi
Giao diện SHALL phản hồi thao tác (bấm, gõ, cuộn) trong vòng 100 ms kể cả khi đang chạy gộp, tách, nén, OCR, ký hay tìm kiếm. Những việc kéo dài hơn 1 giây MUST chạy nền, có tiến độ và hủy được.

#### Scenario: Cuộn trong lúc OCR
- **GIVEN** OCR đang chạy nền trên một tài liệu 100 trang
- **WHEN** người dùng cuộn một tài liệu khác
- **THEN** việc cuộn vẫn mượt, không bị khựng quá 100 ms

#### Scenario: Cuộn nhanh
- **WHEN** người dùng kéo thanh cuộn thật nhanh qua 500 trang
- **THEN** vùng xem hiện ngay khung chỗ trang rồi điền nội dung sau, không bị đứng

### Requirement: Kiểm tra hiệu năng tự động
Mỗi lần dựng bản phát hành, hệ thống CI SHALL chạy bộ đo thời gian mở và bộ nhớ trên tài liệu tham chiếu cho cả Windows và macOS. Bản phát hành MUST bị chặn nếu vượt ngưỡng.

#### Scenario: Vượt ngưỡng
- **WHEN** một thay đổi làm thời gian mở file tham chiếu lên 3,2 giây
- **THEN** bước kiểm tra hiệu năng của CI thất bại và bản phát hành không được tạo
