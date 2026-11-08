# Spec Delta

## Purpose

Cho phép sắp xếp lại các trang của tài liệu (xoay, xóa, chèn, kéo thả đổi thứ tự) và hoàn tác hoặc làm lại nhiều bước trước khi lưu.

## ADDED Requirements

### Requirement: Chọn nhiều trang
Trong bảng ảnh thu nhỏ và chế độ Sắp xếp trang dạng lưới, ứng dụng SHALL cho chọn một trang (bấm), thêm/bớt từng trang (Ctrl/Cmd + bấm), chọn một dải (Shift + bấm) và chọn tất cả (Ctrl/Cmd + A). Mọi thao tác trang MUST áp dụng cho toàn bộ các trang đang chọn.

#### Scenario: Chọn một dải
- **WHEN** người dùng bấm trang 3 rồi Shift + bấm trang 7
- **THEN** trang 3 đến 7 được chọn

### Requirement: Xoay trang
Ứng dụng SHALL xoay các trang đang chọn 90° theo chiều kim đồng hồ, 90° ngược chiều kim đồng hồ, hoặc 180°. Xoay MUST cộng dồn với góc xoay sẵn có của trang và chỉ đổi thuộc tính xoay, không dựng lại nội dung trang.

#### Scenario: Xoay nhiều trang
- **GIVEN** trang 2 và 4 đang được chọn
- **WHEN** người dùng chọn Xoay phải
- **THEN** trang 2 và 4 hiển thị xoay 90° theo chiều kim đồng hồ, các trang khác không đổi

#### Scenario: Xoay cộng dồn
- **GIVEN** trang 1 đã có góc xoay 90°
- **WHEN** người dùng xoay phải thêm một lần
- **THEN** trang 1 có góc xoay 180°

### Requirement: Xóa trang
Ứng dụng SHALL xóa các trang đang chọn bằng lệnh Xóa hoặc phím Delete. Không được xóa hết mọi trang: thao tác xóa tất cả các trang MUST bị từ chối kèm thông báo.

#### Scenario: Xóa trang
- **GIVEN** tài liệu có 10 trang và trang 3, 4 đang được chọn
- **WHEN** người dùng nhấn Delete
- **THEN** tài liệu còn 8 trang và trang 5 cũ trở thành trang 3

#### Scenario: Xóa tất cả
- **WHEN** người dùng chọn tất cả các trang và nhấn Delete
- **THEN** ứng dụng báo "Tài liệu phải còn ít nhất một trang" và không xóa trang nào

### Requirement: Chèn trang
Ứng dụng SHALL chèn vào trước hoặc sau một vị trí chọn: trang trắng (khổ mặc định bằng khổ trang kề bên), các trang của một PDF khác (cả file hoặc một khoảng trang), hoặc file ảnh PNG/JPEG (mỗi ảnh thành một trang). Trang chèn từ PDF khác MUST giữ nguyên nội dung và ghi chú.

#### Scenario: Chèn trang trắng
- **WHEN** người dùng chèn một trang trắng sau trang 5 của tài liệu khổ A4
- **THEN** trang 6 mới là trang trắng khổ A4

#### Scenario: Chèn từ file khác
- **WHEN** người dùng chèn trang 1–2 của phuluc.pdf vào trước trang 1
- **THEN** hai trang đầu của tài liệu là trang 1–2 của phuluc.pdf

#### Scenario: Chèn ảnh
- **WHEN** người dùng chèn 3 file JPEG sau trang cuối
- **THEN** tài liệu có thêm 3 trang, mỗi trang chứa một ảnh theo đúng thứ tự đã chọn

### Requirement: Kéo thả đổi thứ tự trang
Ứng dụng SHALL cho kéo một hoặc nhiều trang đang chọn tới vị trí mới trong bảng ảnh thu nhỏ hoặc lưới Sắp xếp trang, có vạch báo vị trí thả. Các trang kéo đi MUST giữ thứ tự tương đối của chúng.

#### Scenario: Kéo một trang
- **WHEN** người dùng kéo trang 8 thả vào giữa trang 2 và trang 3
- **THEN** trang 8 cũ trở thành trang 3, các trang 3–7 cũ dịch xuống một vị trí

#### Scenario: Kéo nhiều trang không liền nhau
- **GIVEN** trang 2 và trang 6 đang được chọn
- **WHEN** người dùng kéo chúng thả vào cuối tài liệu
- **THEN** hai trang cuối của tài liệu lần lượt là trang 2 cũ rồi trang 6 cũ

#### Scenario: Kéo trang sang tài liệu khác
- **WHEN** người dùng kéo một trang từ bảng ảnh thu nhỏ của tab A thả vào bảng ảnh thu nhỏ của tab B
- **THEN** trang đó được chép sang tài liệu B tại vị trí thả, tài liệu A không đổi

### Requirement: Hoàn tác và làm lại nhiều bước
Mọi thao tác thay đổi tài liệu (thao tác trang, ghi chú, điền form) SHALL hoàn tác được bằng Ctrl/Cmd + Z và làm lại được bằng Ctrl/Cmd + Shift + Z hoặc Ctrl + Y, tối thiểu 100 bước cho mỗi tài liệu. Một thao tác trên nhiều trang MUST tính là một bước. Lịch sử MUST được giữ qua các lần Lưu trong cùng phiên.

#### Scenario: Hoàn tác nhiều bước
- **GIVEN** người dùng lần lượt xoay trang 1, xóa trang 5, rồi chèn một trang trắng
- **WHEN** người dùng nhấn hoàn tác 3 lần
- **THEN** tài liệu trở về đúng trạng thái trước khi xoay trang 1

#### Scenario: Làm lại
- **WHEN** người dùng hoàn tác 2 bước rồi nhấn làm lại 1 lần
- **THEN** chỉ bước bị hoàn tác sau cùng được áp dụng lại

#### Scenario: Thao tác mới xóa nhánh làm lại
- **WHEN** người dùng hoàn tác một bước rồi thực hiện một thao tác mới
- **THEN** không còn bước nào để làm lại

#### Scenario: Hoàn tác sau khi lưu
- **GIVEN** người dùng xóa trang 2 rồi Lưu
- **WHEN** người dùng nhấn hoàn tác
- **THEN** trang 2 trở lại và tài liệu được đánh dấu là có thay đổi chưa lưu

### Requirement: Cảnh báo thay đổi chưa lưu
Ứng dụng SHALL đánh dấu tab có thay đổi chưa lưu, và hỏi Lưu, Không lưu hoặc Hủy khi người dùng đóng tab hay thoát ứng dụng.

#### Scenario: Đóng tab có thay đổi
- **GIVEN** tài liệu có thay đổi chưa lưu
- **WHEN** người dùng đóng tab
- **THEN** ứng dụng hỏi Lưu, Không lưu hoặc Hủy trước khi đóng

### Requirement: Lưu an toàn
Lưu SHALL ghi ra file tạm rồi thay thế file đích một cách nguyên tử, để file gốc không bị hỏng khi việc ghi bị gián đoạn. Lưu thành (Save As) MUST giữ file gốc không đổi.

#### Scenario: Ghi bị gián đoạn
- **WHEN** ứng dụng bị dừng đột ngột trong lúc đang lưu
- **THEN** file đích vẫn là bản trước khi lưu hoặc bản mới đầy đủ, không bao giờ là bản ghi dở
