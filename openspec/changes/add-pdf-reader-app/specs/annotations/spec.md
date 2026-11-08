# Spec Delta

## Purpose

Cho phép tạo và quản lý ghi chú trên PDF (highlight, ghi chú dán, bình luận và trả lời), lưu theo chuẩn annotation của PDF để Adobe Acrobat và các trình đọc khác hiển thị được.

## ADDED Requirements

### Requirement: Highlight chữ
Ứng dụng SHALL tạo highlight, gạch chân hoặc gạch ngang trên đoạn chữ đã chọn, kể cả đoạn trải trên nhiều dòng. Mỗi lần đánh dấu MUST thành một annotation với vùng phủ khớp từng dòng chữ, và người dùng MUST đổi được màu cùng độ mờ.

#### Scenario: Highlight nhiều dòng
- **WHEN** người dùng chọn một đoạn kéo dài 3 dòng rồi chọn Highlight màu vàng
- **THEN** cả 3 dòng được tô vàng, chỉ phủ đúng phần chữ đã chọn
- **AND** đoạn highlight đó là một annotation duy nhất

#### Scenario: Đổi màu
- **WHEN** người dùng chọn một highlight có sẵn và đổi sang màu xanh lá
- **THEN** highlight hiển thị màu xanh lá và thao tác này hoàn tác được

### Requirement: Ghi chú dán
Ứng dụng SHALL đặt ghi chú dán (biểu tượng ghi chú) tại vị trí người dùng bấm, kèm nội dung chữ nhiều dòng. Ghi chú MUST di chuyển được bằng kéo thả, và mở/đóng được cửa sổ nội dung.

#### Scenario: Tạo ghi chú dán
- **WHEN** người dùng chọn công cụ Ghi chú, bấm vào lề trang 3 và gõ "Kiểm tra lại số liệu"
- **THEN** biểu tượng ghi chú xuất hiện tại điểm bấm
- **AND** di chuột lên hoặc bấm vào biểu tượng thì hiện nội dung "Kiểm tra lại số liệu"

### Requirement: Bình luận và trả lời
Mọi annotation SHALL có nội dung bình luận, tên tác giả (lấy từ cài đặt, mặc định là tên người dùng của hệ điều hành) và thời điểm tạo/sửa. Người dùng MUST trả lời được một annotation thành một luồng bình luận, và đặt được trạng thái (Chấp nhận, Từ chối, Hoàn thành).

#### Scenario: Trả lời bình luận
- **GIVEN** trang 2 có highlight kèm bình luận "Đoạn này chưa rõ"
- **WHEN** người dùng trả lời "Đã sửa ở bản 2"
- **THEN** câu trả lời hiện dưới bình luận gốc, kèm tên tác giả và thời gian

### Requirement: Danh sách bình luận
Ứng dụng SHALL có bảng Bình luận liệt kê mọi annotation của tài liệu theo trang, lọc được theo loại, tác giả, trạng thái và tìm được theo nội dung (không phân biệt dấu như phần tìm chữ). Bấm một mục MUST chuyển tới annotation đó.

#### Scenario: Lọc theo tác giả
- **GIVEN** tài liệu có bình luận của "Lan" và "Minh"
- **WHEN** người dùng lọc theo tác giả "Lan"
- **THEN** bảng chỉ hiện bình luận của Lan

### Requirement: Sửa và xóa ghi chú
Ứng dụng SHALL cho sửa nội dung, di chuyển, đổi màu và xóa annotation. Mọi thay đổi MUST hoàn tác được.

#### Scenario: Xóa ghi chú
- **WHEN** người dùng chọn một ghi chú dán và nhấn Delete
- **THEN** ghi chú biến mất khỏi trang và khỏi bảng Bình luận
- **AND** nhấn hoàn tác thì ghi chú trở lại

### Requirement: Tương thích với Adobe Acrobat
Annotation SHALL được lưu bằng các loại annotation chuẩn của PDF (Highlight, Underline, StrikeOut, Text, Popup), kèm luồng hình thức (appearance stream), nội dung, tác giả, thời gian, và liên kết trả lời theo chuẩn. Mở file bằng Adobe Acrobat Reader MUST thấy đủ ghi chú, nội dung và luồng trả lời.

#### Scenario: Mở bằng Acrobat
- **GIVEN** người dùng tạo một highlight có bình luận, một ghi chú dán và một câu trả lời, rồi lưu
- **WHEN** mở file đó bằng Adobe Acrobat Reader
- **THEN** highlight hiển thị đúng vị trí và màu, ghi chú dán hiển thị đúng vị trí
- **AND** bảng Comments của Acrobat liệt kê cả 3 mục với đúng nội dung tiếng Việt, tác giả, và câu trả lời nằm dưới bình luận gốc

#### Scenario: Trình đọc không tự dựng hình
- **WHEN** mở file đã lưu bằng một trình đọc chỉ vẽ appearance stream có sẵn
- **THEN** các highlight vẫn hiển thị đúng màu và vị trí

### Requirement: Giữ ghi chú do ứng dụng khác tạo
Ứng dụng SHALL hiển thị annotation do ứng dụng khác tạo (gồm các loại nó không sửa được, như hình vẽ, con dấu, FreeText). Lưu file MUST giữ nguyên những annotation đó và mọi thuộc tính của chúng.

#### Scenario: Annotation lạ được giữ
- **GIVEN** file có một con dấu (Stamp) tạo bằng Acrobat
- **WHEN** người dùng thêm một highlight rồi lưu
- **THEN** con dấu vẫn còn và hiển thị như cũ khi mở lại bằng Acrobat

### Requirement: Ẩn hiện và làm phẳng ghi chú
Ứng dụng SHALL cho ẩn/hiện toàn bộ ghi chú khi xem, và Làm phẳng (flatten) để vẽ ghi chú vào nội dung trang rồi bỏ annotation khi lưu ra file mới.

#### Scenario: Làm phẳng
- **WHEN** người dùng chọn Làm phẳng ghi chú rồi lưu thành file mới
- **THEN** file mới hiển thị các highlight như một phần nội dung trang và không còn annotation nào sửa được
