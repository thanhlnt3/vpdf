# Spec Delta

## Purpose

Cho phép ký số tài liệu PDF bằng chứng thư cá nhân dạng file `.p12`/`.pfx`, và kiểm tra tính toàn vẹn cùng danh tính người ký của các chữ ký số có sẵn trong file.

## ADDED Requirements

### Requirement: Nạp chứng thư cá nhân
Ứng dụng SHALL nạp chứng thư cùng khóa riêng từ file PKCS#12 (`.p12`, `.pfx`) có mật khẩu, hỗ trợ khóa RSA (2048 bit trở lên) và ECDSA (P-256, P-384). Trước khi ký, ứng dụng MUST hiện tên chủ thể, tổ chức cấp và thời hạn của chứng thư.

#### Scenario: Nạp chứng thư
- **WHEN** người dùng chọn file `canhan.pfx` và nhập đúng mật khẩu
- **THEN** ứng dụng hiện tên người ký, tổ chức cấp và ngày hết hạn của chứng thư

#### Scenario: Sai mật khẩu chứng thư
- **WHEN** người dùng nhập sai mật khẩu của file `.pfx`
- **THEN** ứng dụng báo "Mật khẩu chứng thư không đúng" và không nạp khóa

#### Scenario: Chứng thư hết hạn
- **WHEN** người dùng nạp chứng thư đã hết hạn hoặc chưa tới ngày hiệu lực
- **THEN** ứng dụng từ chối ký và báo khoảng thời gian hiệu lực của chứng thư

### Requirement: Ký tài liệu
Ứng dụng SHALL tạo chữ ký PAdES Baseline B-B (CMS tách rời, SHA-256 trở lên) phủ toàn bộ file tại thời điểm ký, lưu thành bản cập nhật nối thêm (incremental update) để giữ nguyên các chữ ký có trước. Chữ ký MUST được Adobe Acrobat Reader nhận ra là còn toàn vẹn.

#### Scenario: Ký và kiểm tra bằng Acrobat
- **WHEN** người dùng ký một tài liệu rồi mở bằng Adobe Acrobat Reader
- **THEN** Acrobat báo tài liệu không bị sửa đổi kể từ khi ký, và hiện đúng tên người ký

#### Scenario: Ký lần thứ hai
- **GIVEN** tài liệu đã có chữ ký hợp lệ của người A
- **WHEN** người B ký thêm
- **THEN** cả chữ ký của A và B đều còn toàn vẹn khi kiểm tra

#### Scenario: Tài liệu cấm ký
- **GIVEN** tài liệu đặt quyền không cho thay đổi hoặc chữ ký chứng nhận không cho thêm chữ ký
- **WHEN** người dùng chọn Ký
- **THEN** ứng dụng báo không thể ký tài liệu này và không thay đổi file

### Requirement: Hình thức chữ ký
Người dùng SHALL chọn được chữ ký ẩn hoặc chữ ký hiện. Với chữ ký hiện, người dùng vẽ một khung trên trang hoặc chọn trường chữ ký có sẵn; khung MUST hiện tên người ký, thời điểm ký, lý do và nơi ký (nếu nhập), với tùy chọn thêm ảnh chữ ký tay.

#### Scenario: Chữ ký hiện
- **WHEN** người dùng vẽ khung ở cuối trang 3, nhập lý do "Phê duyệt" rồi ký
- **THEN** khung chữ ký hiện trên trang 3 với tên người ký, thời điểm ký và lý do "Phê duyệt" bằng tiếng Việt có dấu

#### Scenario: Ký vào trường có sẵn
- **GIVEN** tài liệu có trường chữ ký trống tên "NguoiDuyet"
- **WHEN** người dùng bấm vào trường đó và ký
- **THEN** chữ ký được đặt vào đúng trường "NguoiDuyet"

### Requirement: Không lưu khóa riêng và mật khẩu
Ứng dụng SHALL không chép file `.p12`/`.pfx`, không lưu khóa riêng hay mật khẩu chứng thư ra đĩa hoặc nhật ký, và MUST xóa khóa khỏi bộ nhớ sau khi ký xong. Nếu người dùng chọn nhớ chứng thư, ứng dụng chỉ được nhớ đường dẫn file.

#### Scenario: Sau khi ký
- **WHEN** ký xong và người dùng đóng hộp thoại ký
- **THEN** lần ký sau ứng dụng hỏi lại mật khẩu chứng thư

### Requirement: Kiểm tra chữ ký có sẵn
Khi mở tài liệu có chữ ký số, ứng dụng SHALL tự kiểm tra từng chữ ký và hiện bảng Chữ ký với: tên người ký, thời điểm ký, phần tài liệu mà chữ ký phủ, kết quả kiểm tra toàn vẹn, kết quả kiểm tra chuỗi chứng thư, và các thay đổi sau khi ký. Kết quả tổng của mỗi chữ ký MUST là một trong ba trạng thái: Hợp lệ, Không hợp lệ, hoặc Không xác định.

#### Scenario: Chữ ký hợp lệ
- **GIVEN** tài liệu được ký bằng chứng thư có chuỗi tới một CA gốc tin cậy, và không bị sửa sau khi ký
- **WHEN** người dùng mở tài liệu
- **THEN** bảng Chữ ký hiện trạng thái "Hợp lệ" kèm tên người ký

#### Scenario: Tài liệu bị sửa sau khi ký
- **GIVEN** nội dung một trang bị thay đổi trong vùng đã được ký
- **WHEN** người dùng mở tài liệu
- **THEN** chữ ký có trạng thái "Không hợp lệ" với lý do "Tài liệu đã bị sửa đổi sau khi ký"

#### Scenario: Chứng thư không rõ nguồn gốc
- **GIVEN** chữ ký còn toàn vẹn nhưng chứng thư không có chuỗi tới CA gốc tin cậy nào
- **WHEN** người dùng mở tài liệu
- **THEN** chữ ký có trạng thái "Không xác định" với lý do "Không xác minh được danh tính người ký"

#### Scenario: Có thay đổi được phép sau khi ký
- **GIVEN** sau khi ký, tài liệu được thêm một ghi chú bằng bản cập nhật nối thêm
- **WHEN** người dùng xem chi tiết chữ ký
- **THEN** ứng dụng báo chữ ký còn toàn vẹn cho phiên bản đã ký, và liệt kê "Đã thêm 1 ghi chú sau khi ký"

### Requirement: Nguồn tin cậy cho chứng thư
Ứng dụng SHALL dùng kho chứng thư gốc tin cậy của hệ điều hành, và cho người dùng thêm CA gốc riêng (ví dụ CA gốc quốc gia hoặc CA nội bộ) vào danh sách tin cậy của ứng dụng. Khi không kiểm tra được thu hồi (không có mạng hoặc không có OCSP/CRL), kết quả MUST ghi rõ là chưa kiểm tra thu hồi.

#### Scenario: Thêm CA gốc riêng
- **GIVEN** chữ ký đang ở trạng thái "Không xác định" vì CA gốc chưa được tin cậy
- **WHEN** người dùng nhập CA gốc đó vào danh sách tin cậy
- **THEN** khi kiểm tra lại, chữ ký chuyển sang "Hợp lệ"

#### Scenario: Không kiểm tra được thu hồi
- **WHEN** kiểm tra chữ ký trên máy không có mạng
- **THEN** chi tiết chữ ký ghi "Chưa kiểm tra tình trạng thu hồi chứng thư"

### Requirement: Cảnh báo trước khi sửa tài liệu đã ký
Khi người dùng thực hiện thao tác làm chữ ký mất hiệu lực (sắp xếp trang, nén, gộp, tách, đặt mật khẩu, OCR), ứng dụng SHALL cảnh báo và chỉ tiếp tục khi người dùng xác nhận.

#### Scenario: Xoay trang tài liệu đã ký
- **GIVEN** tài liệu có chữ ký hợp lệ
- **WHEN** người dùng xoay một trang
- **THEN** ứng dụng cảnh báo "Thao tác này sẽ làm chữ ký số mất hiệu lực" trước khi áp dụng
