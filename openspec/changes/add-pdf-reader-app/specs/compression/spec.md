# Spec Delta

## Purpose

Cho phép giảm dung lượng file PDF theo 3 mức nén có thể dự đoán được, và báo rõ dung lượng trước và sau khi nén.

## ADDED Requirements

### Requirement: Ba mức nén
Ứng dụng SHALL có 3 mức nén. Nhẹ: không làm giảm chất lượng (nén lại luồng dữ liệu, gộp đối tượng trùng, bỏ đối tượng thừa, dùng object stream). Vừa: như Nhẹ, cộng thêm giảm ảnh màu/xám xuống 150 dpi và nén JPEG chất lượng 80. Mạnh: như Nhẹ, cộng thêm giảm ảnh xuống 96 dpi, JPEG chất lượng 60, ảnh đen trắng xuống 300 dpi.

#### Scenario: Mức Nhẹ không làm giảm chất lượng
- **WHEN** người dùng nén một file ở mức Nhẹ
- **THEN** mọi trang của file kết quả dựng ra ảnh giống hệt từng điểm ảnh với file gốc ở 150 dpi

#### Scenario: Mức Vừa giảm ảnh
- **GIVEN** file chứa một ảnh chụp 300 dpi
- **WHEN** người dùng nén ở mức Vừa
- **THEN** ảnh đó trong file kết quả có độ phân giải hiệu dụng không quá 150 dpi

#### Scenario: Ảnh đã nhỏ hơn ngưỡng
- **GIVEN** file chứa ảnh 72 dpi
- **WHEN** người dùng nén ở mức Mạnh
- **THEN** ảnh đó không bị phóng lên hay nén lại lần nữa

### Requirement: Giữ chữ và cấu trúc khi nén
Ở mọi mức, nén SHALL giữ nguyên chữ dạng vector, lớp chữ tìm được (gồm lớp chữ OCR), mục lục, link, ghi chú và trường form. Font nhúng MUST chỉ bỏ phần glyph không dùng tới (subset), không bỏ cả font.

#### Scenario: Vẫn tìm được chữ sau khi nén
- **GIVEN** file có lớp chữ và tìm được cụm "báo cáo tài chính"
- **WHEN** nén file ở mức Mạnh
- **THEN** file kết quả vẫn tìm và sao chép được cụm "báo cáo tài chính"

#### Scenario: Mục lục còn nguyên
- **WHEN** nén một file có mục lục ở bất kỳ mức nào
- **THEN** file kết quả có cùng mục lục, các mục trỏ đúng trang

### Requirement: Báo dung lượng trước và sau
Sau khi nén, ứng dụng SHALL hiện dung lượng file gốc, dung lượng file kết quả và tỉ lệ giảm theo phần trăm, dùng đơn vị dễ đọc (KB, MB). Trước khi nén, ứng dụng MUST hiện dung lượng hiện tại của file.

#### Scenario: Báo kết quả
- **GIVEN** file gốc 24,6 MB
- **WHEN** nén xong ra file 6,1 MB
- **THEN** ứng dụng hiện "24,6 MB → 6,1 MB (giảm 75%)"

### Requirement: Không làm file lớn hơn
Nếu file kết quả không nhỏ hơn file gốc, ứng dụng SHALL không tạo file lớn hơn, và báo rằng file đã tối ưu sẵn hoặc không nén thêm được.

#### Scenario: File đã được nén tối ưu
- **WHEN** nén một file mà kết quả lớn hơn hoặc bằng file gốc
- **THEN** ứng dụng báo "Không giảm được dung lượng" và không ghi file kết quả

### Requirement: Nơi lưu file nén
Mặc định nén SHALL lưu ra file mới tên `<tên gốc>_nen.pdf` cạnh file gốc. Ghi đè file gốc MUST chỉ xảy ra khi người dùng chọn rõ ràng.

#### Scenario: Lưu mặc định
- **WHEN** người dùng nén hopdong.pdf với tùy chọn mặc định
- **THEN** tạo hopdong_nen.pdf và hopdong.pdf không thay đổi

### Requirement: File có mật khẩu hoặc chữ ký
Nén file đã mã hóa SHALL giữ nguyên mã hóa và quyền của file. Nén file có chữ ký số MUST cảnh báo rằng chữ ký sẽ mất hiệu lực, và chỉ tiếp tục khi người dùng xác nhận.

#### Scenario: Nén file đã ký
- **GIVEN** file có một chữ ký số hợp lệ
- **WHEN** người dùng chọn Nén
- **THEN** ứng dụng cảnh báo "Nén sẽ làm chữ ký số mất hiệu lực" và chờ xác nhận

#### Scenario: Nén file có mật khẩu
- **GIVEN** file có mật khẩu mở và quyền cấm in
- **WHEN** người dùng nén file đó
- **THEN** file kết quả vẫn yêu cầu cùng mật khẩu và vẫn cấm in
