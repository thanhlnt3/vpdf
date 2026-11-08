# Spec Delta

## Purpose

Cho phép tìm chữ trong tài liệu PDF với kết quả hiện ngay khi đang gõ, và tìm được chữ tiếng Việt có dấu khi người dùng gõ không dấu.

## ADDED Requirements

### Requirement: Tìm khi đang gõ
Ứng dụng SHALL bắt đầu tìm sau mỗi lần người dùng ngừng gõ 150 ms, không cần nhấn Enter. Kết quả MUST hiện dần theo thứ tự trang trong khi việc tìm vẫn tiếp tục. Gõ thêm ký tự MUST hủy lượt tìm cũ và bắt đầu lượt mới.

#### Scenario: Kết quả đầu tiên hiện nhanh
- **GIVEN** đang mở một PDF 500 trang có lớp chữ
- **WHEN** người dùng gõ "hợp đồng" vào ô tìm
- **THEN** kết quả đầu tiên được tô sáng và cuộn tới trong vòng 500 ms sau lần gõ cuối

#### Scenario: Gõ tiếp hủy lượt cũ
- **WHEN** người dùng gõ "hợp" rồi gõ tiếp thành "hợp đồng" trước khi lượt tìm "hợp" xong
- **THEN** danh sách kết quả chỉ chứa kết quả của "hợp đồng"

#### Scenario: Không có kết quả
- **WHEN** người dùng tìm một chuỗi không có trong tài liệu
- **THEN** ô tìm hiện "Không tìm thấy" sau khi đã quét hết tài liệu

### Requirement: Hiển thị và duyệt kết quả
Ứng dụng SHALL tô sáng mọi kết quả trên các trang đang hiển thị, tô đậm hơn kết quả đang chọn, hiện bộ đếm "kết quả hiện tại / tổng số" và danh sách kết quả kèm số trang và đoạn chữ xung quanh. Enter/Shift+Enter (hoặc F3/Shift+F3) MUST chuyển tới kết quả sau/trước.

#### Scenario: Duyệt kết quả
- **GIVEN** tìm "điều khoản" có 12 kết quả
- **WHEN** người dùng nhấn Enter 3 lần
- **THEN** kết quả thứ 4 được chọn và cuộn vào vùng nhìn thấy
- **AND** bộ đếm hiện "4 / 12"

#### Scenario: Bộ đếm cập nhật dần
- **WHEN** việc tìm chưa quét xong toàn bộ tài liệu
- **THEN** bộ đếm hiện số kết quả đã tìm được kèm dấu hiệu đang tìm (ví dụ "4 / 37…")

#### Scenario: Bấm vào danh sách kết quả
- **WHEN** người dùng bấm vào một dòng trong danh sách kết quả ở trang 210
- **THEN** vùng xem chuyển tới trang 210 và tô đậm kết quả đó

### Requirement: Tìm không phân biệt dấu tiếng Việt
Mặc định ứng dụng SHALL so khớp bỏ dấu: chuẩn hóa cả chuỗi tìm và chữ trong tài liệu về Unicode NFD, bỏ dấu thanh và dấu mũ/móc, coi đ/Đ là d/D, không phân biệt hoa thường. Vùng tô sáng MUST bao đúng chữ gốc có dấu trong tài liệu.

#### Scenario: Gõ không dấu tìm ra có dấu
- **GIVEN** tài liệu chứa "Nghị định số 13/2023/NĐ-CP"
- **WHEN** người dùng tìm "nghi dinh"
- **THEN** cụm "Nghị định" được tìm thấy và tô sáng đúng vị trí

#### Scenario: Chữ đ
- **GIVEN** tài liệu chứa "Đà Nẵng" và "đường"
- **WHEN** người dùng tìm "da nang" và "duong"
- **THEN** cả hai cụm đều được tìm thấy

#### Scenario: Chuỗi tìm có dấu
- **WHEN** người dùng tìm "hợp đồng" ở chế độ mặc định
- **THEN** các cụm "hợp đồng", "Hợp Đồng" và "hop dong" (nếu có trong tài liệu) đều khớp

#### Scenario: Dấu dựng sẵn và dấu tổ hợp
- **GIVEN** tài liệu chứa "Việt" lưu ở dạng ký tự tổ hợp (e + dấu mũ + dấu nặng)
- **WHEN** người dùng tìm "Việt" gõ bằng ký tự dựng sẵn
- **THEN** cụm đó được tìm thấy

### Requirement: Tùy chọn tìm chính xác
Ứng dụng SHALL có các tùy chọn bật/tắt: Phân biệt dấu, Phân biệt hoa thường, Khớp nguyên từ. Khi bật Phân biệt dấu, chuỗi tìm MUST chỉ khớp với chữ có đúng các dấu đó.

#### Scenario: Phân biệt dấu
- **GIVEN** tài liệu chứa "ban", "bán" và "bàn"
- **WHEN** người dùng bật Phân biệt dấu và tìm "bán"
- **THEN** chỉ "bán" khớp

#### Scenario: Khớp nguyên từ
- **GIVEN** tài liệu chứa "an" và "bang"
- **WHEN** người dùng bật Khớp nguyên từ và tìm "an"
- **THEN** "bang" không khớp

### Requirement: Tìm vượt ngắt dòng
Ứng dụng SHALL tìm thấy cụm từ bị ngắt sang dòng mới, và cụm có nhiều khoảng trắng liên tiếp, như khi chúng nằm trên một dòng.

#### Scenario: Cụm bị xuống dòng
- **GIVEN** cụm "trách nhiệm hữu hạn" có "trách nhiệm" ở cuối dòng và "hữu hạn" ở đầu dòng sau
- **WHEN** người dùng tìm "trach nhiem huu han"
- **THEN** cụm được tìm thấy và tô sáng trên cả hai dòng

### Requirement: Trang không có lớp chữ
Khi tài liệu có trang chỉ là ảnh (không có lớp chữ), ứng dụng SHALL báo số trang không tìm được và gợi ý chạy OCR, thay vì báo "Không tìm thấy" như thể đã tìm đủ.

#### Scenario: PDF scan
- **GIVEN** tài liệu 20 trang, trong đó 15 trang chỉ có ảnh
- **WHEN** người dùng tìm một chuỗi bất kỳ
- **THEN** ứng dụng báo "15 trang không có chữ để tìm. Chạy OCR?"
