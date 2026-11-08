# Spec Delta

## Purpose

Cho phép điền các form tương tác (AcroForm) có sẵn trong PDF và lưu lại, để các trình đọc khác hiển thị đúng giá trị đã điền.

## ADDED Requirements

### Requirement: Nhận diện form
Khi mở tài liệu có AcroForm, ứng dụng SHALL hiện thanh thông báo "Tài liệu có form có thể điền", và có lệnh Tô sáng trường để làm nổi các trường điền được. Tài liệu chỉ có form XFA động MUST báo rõ là không hỗ trợ điền.

#### Scenario: Mở file có form
- **WHEN** người dùng mở một PDF có AcroForm
- **THEN** thanh thông báo hiện ra và các trường được tô sáng

#### Scenario: Form XFA động
- **WHEN** người dùng mở PDF chỉ có form XFA động
- **THEN** ứng dụng báo "Form dạng XFA động chưa được hỗ trợ"

### Requirement: Điền các loại trường
Ứng dụng SHALL hỗ trợ điền ô chữ một dòng và nhiều dòng, ô đánh dấu, nút chọn một, danh sách thả xuống (gồm loại cho nhập giá trị tự do) và hộp danh sách (chọn một hoặc nhiều). Ô chữ MUST nhận tiếng Việt có dấu qua bộ gõ của hệ điều hành (Unikey, EVKey, bộ gõ macOS).

#### Scenario: Ô chữ tiếng Việt
- **WHEN** người dùng gõ "Nguyễn Thị Hằng" vào ô "Họ và tên" bằng bộ gõ Telex
- **THEN** ô hiển thị đúng "Nguyễn Thị Hằng"

#### Scenario: Nút chọn một
- **GIVEN** nhóm nút chọn "Giới tính" có "Nam" và "Nữ"
- **WHEN** người dùng chọn "Nữ"
- **THEN** chỉ "Nữ" được chọn trong nhóm

#### Scenario: Danh sách thả xuống
- **WHEN** người dùng mở danh sách "Tỉnh/Thành" và chọn "Hà Nội"
- **THEN** trường hiển thị "Hà Nội"

### Requirement: Di chuyển giữa các trường
Tab và Shift+Tab SHALL chuyển giữa các trường theo thứ tự tab khai báo trong tài liệu, nếu không khai báo thì theo vị trí trên trang (trên xuống, trái sang phải).

#### Scenario: Chuyển trường bằng Tab
- **WHEN** con trỏ đang ở trường đầu tiên và người dùng nhấn Tab
- **THEN** con trỏ chuyển sang trường kế tiếp theo thứ tự tab

### Requirement: Tôn trọng ràng buộc của trường
Ứng dụng SHALL tôn trọng thuộc tính của trường: chỉ đọc, bắt buộc, độ dài tối đa, ô chia ký tự (comb), định dạng số/ngày khai báo trong tài liệu. Khi lưu mà còn trường bắt buộc chưa điền, ứng dụng MUST cảnh báo và liệt kê các trường đó, nhưng vẫn cho lưu nếu người dùng đồng ý.

#### Scenario: Trường chỉ đọc
- **WHEN** người dùng bấm vào một trường chỉ đọc
- **THEN** giá trị của trường không thay đổi được

#### Scenario: Độ dài tối đa
- **GIVEN** trường "CCCD" có độ dài tối đa 12
- **WHEN** người dùng gõ 13 chữ số
- **THEN** trường chỉ giữ 12 chữ số đầu

#### Scenario: Trường bắt buộc còn trống
- **WHEN** người dùng lưu khi trường bắt buộc "Số điện thoại" còn trống
- **THEN** ứng dụng cảnh báo và liệt kê "Số điện thoại"

### Requirement: Lưu form tương thích
Lưu SHALL ghi giá trị vào trường và tạo lại appearance stream bằng font chứa đủ ký tự tiếng Việt (nhúng một phần font khi font của form thiếu ký tự). Mở file bằng Adobe Acrobat Reader và trình duyệt web MUST thấy đúng giá trị và vẫn sửa tiếp được.

#### Scenario: Mở bằng Acrobat sau khi điền
- **GIVEN** người dùng điền "Trần Văn Đức" vào ô "Họ và tên" và đánh dấu một ô rồi lưu
- **WHEN** mở file đó bằng Adobe Acrobat Reader
- **THEN** ô "Họ và tên" hiển thị đúng "Trần Văn Đức", ô đánh dấu đã được đánh dấu
- **AND** người dùng vẫn sửa được các trường trong Acrobat

#### Scenario: Font của form thiếu ký tự tiếng Việt
- **GIVEN** trường dùng font Helvetica chuẩn không có ký tự "ữ"
- **WHEN** người dùng điền "Nguyễn Hữu" rồi lưu
- **THEN** giá trị hiển thị đúng dấu, không hiện ô vuông hay dấu hỏi

### Requirement: Xóa dữ liệu form
Ứng dụng SHALL có lệnh Xóa form để đưa mọi trường về giá trị mặc định khai báo trong tài liệu; thao tác này MUST hoàn tác được.

#### Scenario: Xóa form
- **WHEN** người dùng chọn Xóa form
- **THEN** mọi trường trở về giá trị mặc định và lệnh hoàn tác khôi phục các giá trị đã điền

### Requirement: Không chạy JavaScript trong form
Ứng dụng SHALL không chạy JavaScript nhúng trong tài liệu. Khi form có JavaScript (tính toán, kiểm tra), ứng dụng MUST báo rằng các phép tính tự động sẽ không chạy.

#### Scenario: Form có script tính toán
- **WHEN** người dùng mở một form có trường "Tổng" tính bằng JavaScript
- **THEN** ứng dụng báo "Form có script tự động; các phép tính sẽ không chạy"
- **AND** không đoạn script nào được thực thi
