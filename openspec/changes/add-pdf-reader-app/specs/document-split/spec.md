# Spec Delta

## Purpose

Cho phép tách một PDF thành nhiều file theo khoảng trang hoặc theo mục lục (bookmark), và trích các trang chọn ra một file mới.

## ADDED Requirements

### Requirement: Cú pháp khoảng trang
Ứng dụng SHALL hiểu khoảng trang dạng danh sách phân cách bằng dấu phẩy, mỗi phần là một trang ("5"), một đoạn ("3-7"), đoạn mở ("8-" là tới hết, "-4" là từ đầu), hoặc "odd"/"even" (trang lẻ/chẵn). Khoảng trang không hợp lệ hoặc vượt số trang MUST bị báo lỗi chỉ rõ phần sai.

#### Scenario: Khoảng trang hợp lệ
- **GIVEN** tài liệu có 20 trang
- **WHEN** người dùng nhập "1-3,5,18-"
- **THEN** các trang được chọn là 1, 2, 3, 5, 18, 19, 20

#### Scenario: Khoảng trang vượt số trang
- **GIVEN** tài liệu có 20 trang
- **WHEN** người dùng nhập "15-25"
- **THEN** ứng dụng báo "Trang 25 vượt quá số trang (20)" và không tạo file nào

### Requirement: Tách theo khoảng trang
Ứng dụng SHALL tách tài liệu thành nhiều file theo: một danh sách khoảng trang (mỗi khoảng thành một file), mỗi N trang một file, hoặc mỗi trang một file. Tên file MUST theo mẫu đặt được, mặc định là `<tên gốc>_<số thứ tự>.pdf`.

#### Scenario: Mỗi khoảng một file
- **GIVEN** report.pdf có 10 trang
- **WHEN** người dùng tách theo các khoảng "1-3" và "4-10"
- **THEN** tạo report_1.pdf (trang 1–3) và report_2.pdf (trang 4–10)

#### Scenario: Mỗi N trang
- **GIVEN** report.pdf có 10 trang
- **WHEN** người dùng tách mỗi 4 trang
- **THEN** tạo 3 file có lần lượt 4, 4 và 2 trang

### Requirement: Tách theo mục lục
Ứng dụng SHALL tách tại các mục lục ở cấp người dùng chọn (mặc định cấp 1): mỗi mục thành một file, từ trang đích của mục tới trước trang đích của mục kế tiếp cùng cấp. Tên file MUST lấy từ tiêu đề mục, đã bỏ ký tự hệ điều hành không cho phép. Tài liệu không có mục lục MUST báo không tách được theo mục lục.

#### Scenario: Tách theo chương
- **GIVEN** tài liệu 30 trang có mục cấp 1 "Chương 1" (trang 1), "Chương 2" (trang 11), "Chương 3" (trang 21)
- **WHEN** người dùng tách theo mục lục cấp 1
- **THEN** tạo 3 file "Chương 1.pdf" (trang 1–10), "Chương 2.pdf" (trang 11–20), "Chương 3.pdf" (trang 21–30)

#### Scenario: Trang trước mục đầu tiên
- **GIVEN** mục cấp 1 đầu tiên trỏ tới trang 3
- **WHEN** người dùng tách theo mục lục
- **THEN** trang 1–2 được đưa vào một file riêng tên "<tên gốc>_dau.pdf"

#### Scenario: Ký tự không hợp lệ trong tiêu đề
- **WHEN** một mục có tiêu đề "Phần 1: Tổng quan / Mục tiêu"
- **THEN** tên file được tạo không chứa `:` hay `/` và vẫn giữ chữ tiếng Việt có dấu

### Requirement: Trích trang ra file mới
Ứng dụng SHALL trích các trang chọn (từ ô khoảng trang hoặc chọn trong bảng ảnh thu nhỏ) ra một file mới, giữ thứ tự trang. Người dùng MUST chọn được có xóa các trang đã trích khỏi tài liệu đang mở hay không (mặc định là không xóa).

#### Scenario: Trích trang
- **WHEN** người dùng chọn trang 2, 5, 9 trong bảng ảnh thu nhỏ và chọn Trích trang
- **THEN** file mới có 3 trang theo thứ tự 2, 5, 9
- **AND** tài liệu đang mở không thay đổi

#### Scenario: Trích và xóa
- **WHEN** người dùng trích trang 2 và bật "Xóa các trang đã trích"
- **THEN** file mới chứa trang 2 và tài liệu đang mở mất trang 2
- **AND** thao tác xóa có thể hoàn tác

### Requirement: Giữ nội dung khi tách
Mỗi file tạo ra SHALL giữ nguyên nội dung trang, ghi chú, link nội bộ có đích nằm trong chính file đó, và phần mục lục thuộc các trang của nó. Link nội bộ trỏ tới trang không có trong file MUST bị bỏ, link URL MUST giữ nguyên. File nguồn MUST không bị sửa.

#### Scenario: Link trỏ ra ngoài phần đã tách
- **GIVEN** trang 2 có link tới trang 8
- **WHEN** tách trang 1–5 ra một file
- **THEN** file mới không còn link hỏng tới trang 8

#### Scenario: Mục lục con được giữ
- **GIVEN** "Chương 2" có các mục con "2.1" và "2.2"
- **WHEN** tách theo mục lục cấp 1
- **THEN** file "Chương 2.pdf" có mục lục chứa "2.1" và "2.2" trỏ đúng trang

### Requirement: Không ghi đè khi chưa xác nhận
Khi file đích đã tồn tại, ứng dụng SHALL hỏi Ghi đè, Đặt tên khác (thêm hậu tố số) hoặc Hủy, kèm tùy chọn áp dụng cho tất cả các file còn lại.

#### Scenario: File đích đã tồn tại
- **GIVEN** thư mục đích đã có report_1.pdf
- **WHEN** người dùng tách report.pdf
- **THEN** ứng dụng hỏi cách xử lý report_1.pdf trước khi ghi
