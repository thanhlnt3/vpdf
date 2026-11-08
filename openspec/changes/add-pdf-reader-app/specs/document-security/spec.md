# Spec Delta

## Purpose

Cho phép bảo vệ tài liệu bằng mật khẩu mã hóa AES-256, đặt quyền cấm in và cấm sao chép, và gỡ các bảo vệ đó khi có mật khẩu.

## ADDED Requirements

### Requirement: Đặt mật khẩu mở tài liệu
Ứng dụng SHALL cho đặt mật khẩu mở (user password) và mã hóa tài liệu bằng AES-256 theo bộ xử lý bảo mật chuẩn của PDF 2.0 (ISO 32000-2, revision 6). Mật khẩu MUST được nhập hai lần để xác nhận; mật khẩu có ký tự tiếng Việt MUST được xử lý theo SASLprep/UTF-8 như chuẩn quy định.

#### Scenario: Đặt mật khẩu
- **WHEN** người dùng đặt mật khẩu mở "Bimat@2026" rồi lưu
- **THEN** file lưu ra yêu cầu đúng mật khẩu đó khi mở bằng ứng dụng này và bằng Adobe Acrobat Reader
- **AND** từ điển mã hóa của file dùng AES-256 (V 5, R 6)

#### Scenario: Hai lần nhập không khớp
- **WHEN** hai ô nhập mật khẩu không giống nhau
- **THEN** nút Áp dụng bị vô hiệu hóa và ứng dụng báo "Mật khẩu không khớp"

#### Scenario: Mật khẩu tiếng Việt
- **WHEN** người dùng đặt mật khẩu "mậtkhẩu" rồi mở lại file bằng đúng mật khẩu đó
- **THEN** file mở thành công

### Requirement: Đặt quyền hạn chế
Ứng dụng SHALL cho đặt quyền: cấm in (hoặc chỉ cho in chất lượng thấp), cấm sao chép chữ và ảnh, kèm các quyền chuẩn khác (sửa, ghi chú, điền form, trích trang). Đặt quyền MUST đi kèm mật khẩu chủ sở hữu (owner password) khác với mật khẩu mở, và file MUST được mã hóa AES-256.

#### Scenario: Cấm in và cấm sao chép
- **WHEN** người dùng bật Cấm in và Cấm sao chép, đặt mật khẩu chủ sở hữu, rồi lưu
- **THEN** khi mở file bằng Adobe Acrobat Reader, lệnh In và sao chép chữ bị vô hiệu hóa
- **AND** file vẫn mở được mà không cần mật khẩu nếu không đặt mật khẩu mở

#### Scenario: Thiếu mật khẩu chủ sở hữu
- **WHEN** người dùng bật một quyền hạn chế nhưng không nhập mật khẩu chủ sở hữu
- **THEN** ứng dụng yêu cầu nhập mật khẩu chủ sở hữu trước khi áp dụng

#### Scenario: Hai mật khẩu giống nhau
- **WHEN** mật khẩu chủ sở hữu giống mật khẩu mở
- **THEN** ứng dụng từ chối và yêu cầu dùng hai mật khẩu khác nhau

### Requirement: Tôn trọng quyền của tài liệu
Khi mở tài liệu có quyền hạn chế mà không dùng mật khẩu chủ sở hữu, ứng dụng SHALL vô hiệu hóa các chức năng bị cấm (in, sao chép, sửa trang, ghi chú, điền form, trích trang) và hiện biểu tượng khóa kèm danh sách quyền. Ứng dụng MUST không cung cấp cách nào để vượt qua các quyền này.

#### Scenario: Mở file cấm in
- **GIVEN** file đặt quyền cấm in
- **WHEN** người dùng mở file không nhập mật khẩu chủ sở hữu
- **THEN** lệnh In bị vô hiệu hóa và thanh trạng thái hiện "Đã hạn chế: không in, không sao chép"

#### Scenario: Mở khóa bằng mật khẩu chủ sở hữu
- **WHEN** người dùng chọn Mở khóa quyền và nhập đúng mật khẩu chủ sở hữu
- **THEN** mọi chức năng bị cấm được bật lại cho phiên làm việc đó

### Requirement: Gỡ mật khẩu và quyền
Ứng dụng SHALL gỡ toàn bộ mã hóa và quyền hạn chế, lưu thành file không mã hóa, chỉ khi người dùng đã nhập mật khẩu chủ sở hữu (hoặc mật khẩu mở nếu file không đặt quyền hạn chế).

#### Scenario: Gỡ bảo vệ
- **GIVEN** file có mật khẩu mở và quyền cấm in
- **WHEN** người dùng nhập mật khẩu chủ sở hữu, chọn Gỡ bảo vệ rồi lưu
- **THEN** file lưu ra mở được không cần mật khẩu và in được

#### Scenario: Không có mật khẩu chủ sở hữu
- **GIVEN** file có quyền hạn chế và người dùng chỉ biết mật khẩu mở
- **WHEN** người dùng chọn Gỡ bảo vệ
- **THEN** ứng dụng yêu cầu mật khẩu chủ sở hữu và không gỡ quyền

### Requirement: Không lưu lộ mật khẩu
Ứng dụng SHALL không ghi mật khẩu ra đĩa, nhật ký hay báo cáo lỗi, và MUST xóa mật khẩu khỏi bộ nhớ khi đóng tài liệu.

#### Scenario: Nhật ký không chứa mật khẩu
- **WHEN** người dùng đặt mật khẩu rồi xuất nhật ký chẩn đoán
- **THEN** nhật ký không chứa mật khẩu dưới bất kỳ dạng nào

### Requirement: Cảnh báo mã hóa cũ
Khi mở file mã hóa bằng thuật toán yếu (RC4 hoặc AES-128), ứng dụng SHALL hiện cảnh báo và gợi ý lưu lại bằng AES-256.

#### Scenario: File RC4
- **WHEN** người dùng mở file mã hóa RC4 40-bit
- **THEN** ứng dụng hiện "Tài liệu dùng mã hóa cũ, không an toàn" kèm nút Nâng cấp lên AES-256
