# Spec Delta

## Purpose

Biến PDF scan (chỉ có ảnh) thành PDF tìm và sao chép chữ được, bằng nhận dạng chữ tiếng Việt chạy hoàn toàn trên máy, không cần Internet.

## ADDED Requirements

### Requirement: OCR chạy offline
OCR SHALL chạy hoàn toàn trên máy người dùng, với dữ liệu nhận dạng tiếng Việt và tiếng Anh đi kèm bộ cài. Trong suốt quá trình OCR, ứng dụng MUST không gửi bất kỳ dữ liệu nào ra mạng.

#### Scenario: Máy không có mạng
- **GIVEN** máy đã ngắt toàn bộ kết nối mạng
- **WHEN** người dùng chạy OCR trên một PDF scan tiếng Việt
- **THEN** OCR chạy xong và cho ra file có lớp chữ

#### Scenario: Không có kết nối ra ngoài
- **WHEN** chạy OCR trong lúc theo dõi lưu lượng mạng của tiến trình ứng dụng
- **THEN** không có kết nối mạng nào được mở

### Requirement: Thêm lớp chữ ẩn
OCR SHALL giữ nguyên ảnh gốc của trang và thêm một lớp chữ ẩn (không nhìn thấy) đặt đúng vị trí từng từ. Nhờ đó MUST tìm, chọn và sao chép được chữ, với vùng chọn trùng vị trí chữ trên ảnh.

#### Scenario: Trang trông như cũ
- **WHEN** OCR xong một trang scan
- **THEN** trang hiển thị giống hệt trước khi OCR

#### Scenario: Tìm và sao chép sau OCR
- **GIVEN** trang scan có dòng chữ "QUYẾT ĐỊNH VỀ VIỆC BỔ NHIỆM"
- **WHEN** OCR xong rồi người dùng tìm "quyet dinh"
- **THEN** cụm "QUYẾT ĐỊNH" được tô sáng đúng vị trí trên ảnh
- **AND** chọn và sao chép dòng đó cho ra chuỗi "QUYẾT ĐỊNH VỀ VIỆC BỔ NHIỆM" ở dạng Unicode NFC

#### Scenario: Mở ở trình đọc khác
- **WHEN** mở file đã OCR bằng Adobe Acrobat Reader
- **THEN** tìm và sao chép được chữ tiếng Việt có dấu

### Requirement: Độ chính xác tiếng Việt
Trên bộ mẫu kiểm thử chuẩn của dự án (văn bản hành chính in, scan 300 dpi, nghiêng không quá 5°), OCR SHALL đạt tỉ lệ ký tự đúng (tính cả dấu) từ 97% trở lên, và giữ thứ tự đọc của văn bản nhiều cột.

#### Scenario: Bộ mẫu chuẩn
- **WHEN** chạy OCR trên toàn bộ bộ mẫu kiểm thử chuẩn
- **THEN** tỉ lệ lỗi ký tự (CER) không quá 3%

#### Scenario: Trang hai cột
- **WHEN** OCR một trang có hai cột chữ
- **THEN** chữ sao chép ra theo thứ tự hết cột trái rồi mới tới cột phải

### Requirement: Tiền xử lý ảnh
Trước khi nhận dạng, ứng dụng SHALL tự chỉnh nghiêng (tới ±15°) và tự xoay trang bị lật 90°/180°/270°. Việc chỉnh này MUST chỉ dùng cho nhận dạng, không thay đổi ảnh hiển thị, trừ khi người dùng bật tùy chọn Làm thẳng trang.

#### Scenario: Trang bị xoay ngược
- **GIVEN** một trang scan bị lộn ngược 180°
- **WHEN** chạy OCR
- **THEN** lớp chữ nhận dạng đúng nội dung của trang

### Requirement: Chọn phạm vi và ngôn ngữ
Ứng dụng SHALL cho chọn trang cần OCR (tất cả, khoảng trang, trang hiện tại) và ngôn ngữ (Tiếng Việt, Tiếng Anh, hoặc Tiếng Việt + Tiếng Anh; mặc định là Tiếng Việt + Tiếng Anh). Trang đã có lớp chữ MUST được bỏ qua, trừ khi người dùng chọn OCR lại.

#### Scenario: Bỏ qua trang có chữ
- **GIVEN** tài liệu 10 trang, trong đó trang 1–2 đã có lớp chữ
- **WHEN** người dùng chạy OCR cho tất cả các trang
- **THEN** chỉ trang 3–10 được OCR và báo cáo ghi "Bỏ qua 2 trang đã có chữ"

#### Scenario: OCR lại
- **WHEN** người dùng bật OCR lại trang đã có chữ
- **THEN** lớp chữ OCR cũ bị thay bằng lớp chữ mới, không bị chồng hai lớp

### Requirement: Tiến độ và hủy
OCR SHALL chạy nền, không chặn việc đọc tài liệu khác, có thanh tiến độ theo trang kèm thời gian còn lại ước tính. Người dùng MUST hủy được bất kỳ lúc nào; khi hủy, file gốc MUST không bị thay đổi.

#### Scenario: Hủy giữa chừng
- **WHEN** người dùng bấm Hủy khi OCR đang ở trang 40/100
- **THEN** OCR dừng trong vòng 2 giây và file gốc không bị thay đổi

### Requirement: Tốc độ OCR
Trên máy tham chiếu (xem capability `performance`), OCR SHALL xử lý một trang A4 scan 300 dpi trong trung bình không quá 4 giây, và dùng được nhiều nhân CPU để xử lý song song các trang.

#### Scenario: Tài liệu 50 trang
- **WHEN** OCR một tài liệu scan 50 trang A4 300 dpi trên máy tham chiếu
- **THEN** OCR hoàn tất trong không quá 200 giây
