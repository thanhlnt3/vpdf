# Spec Delta

## Purpose

Cho phép gộp nhiều file PDF thành một file mới mà vẫn giữ mục lục (bookmark), link nội bộ và link ngoài của từng file gốc.

## ADDED Requirements

### Requirement: Chọn và sắp xếp file cần gộp
Ứng dụng SHALL cho thêm từ 2 file PDF trở lên vào danh sách gộp (bằng hộp thoại hoặc kéo thả), sắp xếp lại thứ tự bằng kéo thả, xóa file khỏi danh sách, và với mỗi file chọn được một khoảng trang. File gộp MUST theo đúng thứ tự trong danh sách.

#### Scenario: Gộp theo thứ tự
- **GIVEN** danh sách gộp gồm A.pdf (3 trang), B.pdf (5 trang), C.pdf (2 trang) theo thứ tự đó
- **WHEN** người dùng chọn Gộp và lưu thành out.pdf
- **THEN** out.pdf có 10 trang: trang 1–3 của A, rồi trang 1–5 của B, rồi trang 1–2 của C

#### Scenario: Chỉ gộp một phần
- **WHEN** người dùng đặt khoảng trang "2-4" cho B.pdf
- **THEN** chỉ trang 2, 3, 4 của B có trong file gộp

#### Scenario: Ít hơn 2 file
- **WHEN** danh sách gộp có ít hơn 2 file
- **THEN** nút Gộp bị vô hiệu hóa

### Requirement: Giữ mục lục khi gộp
File gộp SHALL có mục lục, trong đó mỗi file gốc là một mục cấp 1 (mặc định lấy tên file, đổi được trước khi gộp), và toàn bộ mục lục của file gốc nằm bên dưới, giữ nguyên thứ bậc và trỏ đúng trang mới. Mục trỏ tới trang bị loại khỏi khoảng trang MUST bị bỏ.

#### Scenario: Mục lục lồng nhau
- **GIVEN** B.pdf có mục lục "Chương 1 > Mục 1.1" trỏ tới trang 2 của B
- **WHEN** gộp A.pdf (3 trang) rồi B.pdf
- **THEN** mục lục của out.pdf có mục "B" chứa "Chương 1 > Mục 1.1"
- **AND** "Mục 1.1" trỏ tới trang 5 của out.pdf

#### Scenario: File không có mục lục
- **WHEN** gộp một file không có mục lục
- **THEN** file gộp vẫn có một mục cấp 1 mang tên file đó, trỏ tới trang đầu của nó

### Requirement: Giữ link khi gộp
Link nội bộ (gồm đích có tên) của mỗi file gốc SHALL vẫn trỏ đúng trang và vị trí trong file gộp. Đích có tên trùng nhau giữa các file MUST được đổi tên để không trỏ nhầm. Link URL và link mở file khác MUST giữ nguyên.

#### Scenario: Link nội bộ sau khi gộp
- **GIVEN** trang 1 của B.pdf có link tới trang 4 của B
- **WHEN** gộp A.pdf (3 trang) rồi B.pdf
- **THEN** link nằm ở trang 4 của out.pdf và trỏ tới trang 7 của out.pdf

#### Scenario: Đích có tên bị trùng
- **GIVEN** A.pdf và B.pdf đều có đích có tên "intro"
- **WHEN** gộp hai file
- **THEN** link "intro" trong phần của A trỏ tới trang của A, link "intro" trong phần của B trỏ tới trang của B

#### Scenario: Link URL
- **WHEN** gộp một file có link `https://example.com`
- **THEN** link đó trong file gộp vẫn mở `https://example.com`

### Requirement: Giữ nội dung trang khi gộp
Gộp SHALL giữ nguyên nội dung hiển thị, kích thước, chiều xoay của trang, font nhúng, ghi chú và trường form. Trường form trùng tên giữa các file MUST được đổi tên để không bị dính giá trị với nhau.

#### Scenario: Hình thức trang không đổi
- **WHEN** gộp các file có trang khổ A4 dọc, A3 ngang và trang đã xoay 90°
- **THEN** mỗi trang trong file gộp có cùng kích thước, chiều xoay và hình ảnh hiển thị như trong file gốc

#### Scenario: Form trùng tên
- **GIVEN** A.pdf và B.pdf đều có trường form tên "HoTen"
- **WHEN** gộp hai file rồi điền "Nguyễn Văn A" vào trường "HoTen" trong phần của A
- **THEN** trường "HoTen" trong phần của B không bị thay đổi

### Requirement: File nguồn có mật khẩu
Khi một file trong danh sách có mật khẩu, ứng dụng SHALL hỏi mật khẩu của file đó trước khi gộp. File gộp MUST không có mật khẩu, trừ khi người dùng đặt mật khẩu mới cho nó.

#### Scenario: Gộp file có mật khẩu
- **GIVEN** B.pdf có mật khẩu mở
- **WHEN** người dùng thêm B.pdf vào danh sách gộp
- **THEN** ứng dụng hỏi mật khẩu của B.pdf
- **AND** file gộp mở được mà không cần mật khẩu

### Requirement: Không ghi đè file nguồn
Gộp SHALL tạo file mới và không bao giờ sửa file nguồn. Lưu đè lên một file đang nằm trong danh sách gộp MUST bị từ chối.

#### Scenario: Lưu trùng tên file nguồn
- **WHEN** người dùng chọn lưu file gộp đè lên A.pdf trong khi A.pdf đang nằm trong danh sách gộp
- **THEN** ứng dụng báo lỗi và yêu cầu chọn tên khác
