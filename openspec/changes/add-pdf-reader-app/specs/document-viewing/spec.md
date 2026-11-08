# Spec Delta

## Purpose

Cho phép người dùng mở và đọc tài liệu PDF: hiển thị trang, cuộn, phóng to/thu nhỏ, duyệt bằng ảnh thu nhỏ và mục lục (bookmark).

## ADDED Requirements

### Requirement: Mở file PDF
Ứng dụng SHALL mở được file PDF phiên bản 1.0 đến 2.0 qua hộp thoại Mở, kéo thả file vào cửa sổ, liên kết loại file của hệ điều hành và danh sách file gần đây. File không mở được MUST hiện thông báo lỗi rõ ràng, không làm ứng dụng bị treo hay thoát.

#### Scenario: Mở bằng hộp thoại
- **WHEN** người dùng chọn Mở và chọn một file PDF hợp lệ
- **THEN** trang 1 của tài liệu hiển thị trong cửa sổ
- **AND** tên file hiện trên thanh tiêu đề hoặc tab

#### Scenario: Mở bằng kéo thả
- **WHEN** người dùng kéo một file PDF từ trình quản lý file thả vào cửa sổ ứng dụng
- **THEN** tài liệu được mở trong một tab mới

#### Scenario: File hỏng
- **WHEN** người dùng mở một file có phần mở rộng `.pdf` nhưng nội dung không phải PDF hợp lệ
- **THEN** ứng dụng hiện thông báo "Không đọc được file PDF" kèm tên file
- **AND** ứng dụng vẫn chạy bình thường, các tab khác không bị ảnh hưởng

#### Scenario: File hỏng một phần
- **WHEN** người dùng mở một PDF có bảng xref bị lỗi nhưng các trang vẫn khôi phục được
- **THEN** ứng dụng hiển thị các trang khôi phục được
- **AND** hiện cảnh báo rằng file đã được sửa tạm khi mở

### Requirement: Mở file có mật khẩu
Khi file được mã hóa bằng mật khẩu người dùng, ứng dụng SHALL hỏi mật khẩu trước khi hiển thị. Nhập sai MUST cho phép nhập lại, không giới hạn số lần.

#### Scenario: Nhập đúng mật khẩu
- **WHEN** người dùng mở file có mật khẩu và nhập đúng mật khẩu
- **THEN** tài liệu hiển thị bình thường

#### Scenario: Nhập sai mật khẩu
- **WHEN** người dùng nhập sai mật khẩu
- **THEN** ứng dụng báo "Mật khẩu không đúng" và cho nhập lại
- **AND** không trang nào được hiển thị

### Requirement: Nhiều tài liệu cùng lúc
Ứng dụng SHALL mở được nhiều tài liệu cùng lúc, mỗi tài liệu trong một tab. Vị trí đọc, mức phóng và trạng thái hoàn tác của mỗi tab MUST độc lập với nhau.

#### Scenario: Chuyển tab giữ vị trí
- **GIVEN** tab A đang ở trang 40 với mức phóng 150%
- **WHEN** người dùng chuyển sang tab B rồi quay lại tab A
- **THEN** tab A vẫn ở trang 40 với mức phóng 150%

### Requirement: Cuộn và điều hướng trang
Ứng dụng SHALL hỗ trợ cuộn liên tục và chế độ từng trang; điều hướng bằng con lăn, thanh cuộn, phím mũi tên, Page Up/Page Down, Home/End và ô nhập số trang. Ô số trang MUST hiện trang hiện tại và tổng số trang.

#### Scenario: Nhảy tới trang
- **WHEN** người dùng nhập "125" vào ô số trang và nhấn Enter
- **THEN** trang 125 hiển thị ở đầu vùng xem
- **AND** ô số trang hiện "125 / <tổng số trang>"

#### Scenario: Số trang không hợp lệ
- **WHEN** người dùng nhập số trang lớn hơn tổng số trang hoặc nhập chữ
- **THEN** vị trí đọc không đổi và ô số trang trở về trang hiện tại

#### Scenario: Cuộn liên tục
- **GIVEN** chế độ cuộn liên tục đang bật
- **WHEN** người dùng cuộn qua cuối một trang
- **THEN** trang kế tiếp hiển thị liền ngay bên dưới, không phải bấm chuyển trang

### Requirement: Phóng to và thu nhỏ
Ứng dụng SHALL cho phóng trong khoảng 10% đến 6400%, có các chế độ Vừa trang, Vừa chiều rộng và Kích thước thật (100%). Phóng bằng Ctrl/Cmd + con lăn hoặc thao tác chụm trên touchpad MUST giữ nguyên điểm dưới con trỏ.

#### Scenario: Vừa chiều rộng
- **WHEN** người dùng chọn Vừa chiều rộng
- **THEN** chiều rộng trang bằng chiều rộng vùng xem
- **AND** chế độ này được giữ khi thay đổi kích thước cửa sổ

#### Scenario: Phóng tại con trỏ
- **WHEN** người dùng giữ Ctrl (Windows) hoặc Cmd (macOS) và lăn chuột lên khi con trỏ đang ở trên một từ
- **THEN** mức phóng tăng và từ đó vẫn nằm dưới con trỏ

#### Scenario: Chữ nét ở mức phóng cao
- **WHEN** người dùng phóng tới 800% trên một trang có chữ vector
- **THEN** chữ hiển thị sắc nét, không bị vỡ điểm ảnh sau khi dựng hình xong

### Requirement: Bảng ảnh thu nhỏ
Ứng dụng SHALL có bảng bên hiện ảnh thu nhỏ của tất cả các trang kèm số trang. Bấm vào ảnh thu nhỏ MUST chuyển tới trang đó; ảnh của trang hiện tại MUST được đánh dấu.

#### Scenario: Chuyển trang bằng ảnh thu nhỏ
- **WHEN** người dùng bấm vào ảnh thu nhỏ của trang 12
- **THEN** vùng xem chuyển tới trang 12
- **AND** ảnh thu nhỏ trang 12 được đánh dấu là trang hiện tại

#### Scenario: Ảnh thu nhỏ tải dần
- **WHEN** người dùng mở file 500 trang và mở bảng ảnh thu nhỏ
- **THEN** các ảnh trong phần đang nhìn thấy hiện trước
- **AND** việc tạo các ảnh còn lại không chặn thao tác cuộn hay đọc

### Requirement: Mục lục (bookmark)
Ứng dụng SHALL hiện cây mục lục (outline) của tài liệu trong bảng bên, mở/thu gọn được từng nhánh. Bấm vào một mục MUST chuyển tới đích của mục đó. Tài liệu không có mục lục MUST hiện thông báo "Tài liệu không có mục lục".

#### Scenario: Đi tới mục
- **WHEN** người dùng bấm vào mục "Chương 3" trong mục lục
- **THEN** vùng xem chuyển tới đúng trang và vị trí đích của mục đó

#### Scenario: Mục lục tiếng Việt
- **WHEN** tài liệu có mục lục lưu dạng UTF-16 với tiêu đề tiếng Việt có dấu
- **THEN** tiêu đề hiển thị đúng dấu, không bị lỗi ký tự

### Requirement: Link trong tài liệu
Ứng dụng SHALL kích hoạt link khi bấm: link nội bộ MUST chuyển tới trang đích; link ra ngoài (URL) MUST hỏi xác nhận trước khi mở bằng trình duyệt mặc định.

#### Scenario: Link nội bộ
- **WHEN** người dùng bấm vào một link trỏ tới trang 30
- **THEN** vùng xem chuyển tới trang 30
- **AND** lệnh Quay lại đưa người dùng về vị trí trước khi bấm link

#### Scenario: Link URL
- **WHEN** người dùng bấm vào link `https://example.com`
- **THEN** ứng dụng hỏi "Mở liên kết này trong trình duyệt?" và chỉ mở khi người dùng đồng ý

### Requirement: Chọn và sao chép chữ
Ứng dụng SHALL cho chọn chữ bằng cách kéo chuột và sao chép vào clipboard theo đúng thứ tự đọc, giữ đúng dấu tiếng Việt (Unicode NFC). Sao chép MUST bị chặn khi tài liệu cấm sao chép và người dùng chưa mở bằng mật khẩu chủ sở hữu.

#### Scenario: Sao chép chữ tiếng Việt
- **WHEN** người dùng chọn đoạn "Cộng hòa xã hội chủ nghĩa Việt Nam" rồi sao chép
- **THEN** clipboard chứa đúng chuỗi đó ở dạng Unicode NFC

#### Scenario: Tài liệu cấm sao chép
- **GIVEN** tài liệu đặt quyền cấm sao chép
- **WHEN** người dùng chọn chữ và nhấn sao chép
- **THEN** clipboard không thay đổi
- **AND** ứng dụng báo "Tài liệu này không cho phép sao chép"

### Requirement: Nhớ vị trí đọc
Ứng dụng SHALL nhớ trang và mức phóng cuối cùng của 50 file mở gần nhất, và khôi phục khi mở lại file đó.

#### Scenario: Mở lại file
- **GIVEN** người dùng đóng file ở trang 87
- **WHEN** người dùng mở lại đúng file đó
- **THEN** tài liệu mở tại trang 87
