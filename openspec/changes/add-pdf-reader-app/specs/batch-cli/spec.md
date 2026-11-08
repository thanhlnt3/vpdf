# Spec Delta

## Purpose

Cung cấp công cụ dòng lệnh `vpdf` để gộp, tách, nén và OCR PDF, chạy hàng loạt được trong script và hệ thống tự động, cho ra kết quả giống ứng dụng desktop.

## ADDED Requirements

### Requirement: Các lệnh con
Bộ cài SHALL cài công cụ `vpdf` với các lệnh con `merge`, `split`, `compress`, `ocr`, cùng `--help` cho mỗi lệnh và `--version`. Mỗi lệnh MUST cho ra file giống kết quả của chức năng tương ứng trong ứng dụng desktop khi dùng cùng tùy chọn.

#### Scenario: Gộp
- **WHEN** chạy `vpdf merge a.pdf b.pdf c.pdf -o out.pdf`
- **THEN** out.pdf được tạo theo đúng quy tắc của capability `document-merge`

#### Scenario: Tách theo khoảng trang
- **WHEN** chạy `vpdf split report.pdf --ranges "1-3,4-10" -o outdir/`
- **THEN** outdir/ có report_1.pdf và report_2.pdf

#### Scenario: Tách theo mục lục
- **WHEN** chạy `vpdf split report.pdf --by-bookmark --level 1 -o outdir/`
- **THEN** mỗi mục lục cấp 1 thành một file trong outdir/

#### Scenario: Nén
- **WHEN** chạy `vpdf compress scan.pdf --level medium -o scan_nen.pdf`
- **THEN** scan_nen.pdf được nén ở mức Vừa và lệnh in ra dung lượng trước và sau

#### Scenario: OCR
- **WHEN** chạy `vpdf ocr scan.pdf --lang vie+eng -o scan_ocr.pdf`
- **THEN** scan_ocr.pdf có lớp chữ tìm được, đúng như capability `ocr`

#### Scenario: Xem trợ giúp
- **WHEN** chạy `vpdf split --help`
- **THEN** lệnh in ra cú pháp, mọi tùy chọn và ít nhất một ví dụ, rồi thoát với mã 0

### Requirement: Xử lý hàng loạt
Các lệnh `split`, `compress` và `ocr` SHALL nhận nhiều file đầu vào, mẫu glob (`*.pdf`, `**/*.pdf`), thư mục (với `--recursive`) và danh sách đường dẫn từ stdin (`-`), rồi ghi kết quả vào thư mục `-o`. Một file lỗi MUST không làm dừng các file còn lại, trừ khi có `--fail-fast`.

#### Scenario: Nén cả thư mục
- **GIVEN** thư mục `in/` có 200 file PDF, trong đó 1 file hỏng
- **WHEN** chạy `vpdf compress "in/*.pdf" --level high -o out/`
- **THEN** out/ có 199 file đã nén
- **AND** lệnh báo file hỏng và thoát với mã khác 0

#### Scenario: Danh sách từ stdin
- **WHEN** chạy `Get-ChildItem *.pdf | % FullName | vpdf ocr - -o out/` trên PowerShell, hoặc `ls *.pdf | vpdf ocr - -o out/` trên macOS
- **THEN** mọi file trong danh sách được OCR vào out/

#### Scenario: Giữ cấu trúc thư mục
- **WHEN** chạy `vpdf ocr in/ --recursive -o out/` với `in/a/x.pdf`
- **THEN** kết quả nằm ở `out/a/x.pdf`

### Requirement: Chạy song song
Chế độ hàng loạt SHALL xử lý nhiều file song song, mặc định bằng số nhân CPU, đặt được bằng `--jobs N`. Tổng bộ nhớ dùng MUST giữ trong giới hạn của capability `performance`.

#### Scenario: Giới hạn số luồng
- **WHEN** chạy `vpdf ocr "in/*.pdf" --jobs 2 -o out/`
- **THEN** không quá 2 file được xử lý cùng lúc

### Requirement: Mã thoát và đầu ra máy đọc được
Lệnh SHALL thoát với mã 0 khi mọi file thành công, 1 khi có ít nhất một file lỗi, 2 khi sai cú pháp lệnh. Với `--json`, lệnh MUST in ra stdout một kết quả JSON cho mỗi file (đường dẫn vào, đường dẫn ra, trạng thái, thông báo lỗi, dung lượng trước/sau với `compress`), còn tiến độ và nhật ký in ra stderr.

#### Scenario: Sai cú pháp
- **WHEN** chạy `vpdf compress a.pdf --level ultra`
- **THEN** lệnh báo giá trị `--level` hợp lệ là `low`, `medium`, `high` và thoát với mã 2

#### Scenario: Đầu ra JSON
- **WHEN** chạy `vpdf compress a.pdf --level low -o a_nen.pdf --json`
- **THEN** stdout chứa đúng một dòng JSON có các trường `input`, `output`, `status`, `size_before`, `size_after`

### Requirement: An toàn khi ghi file
CLI SHALL không ghi đè file có sẵn nếu không có `--overwrite`, và MUST không bao giờ ghi đè file đầu vào. Với `--dry-run`, lệnh MUST chỉ in ra danh sách việc sẽ làm mà không ghi file nào.

#### Scenario: File đích đã tồn tại
- **GIVEN** out/a.pdf đã tồn tại
- **WHEN** chạy `vpdf compress a.pdf -o out/` không có `--overwrite`
- **THEN** a.pdf bị bỏ qua với lỗi "Đã tồn tại" và out/a.pdf không bị thay đổi

#### Scenario: Chạy thử
- **WHEN** chạy `vpdf split report.pdf --by-bookmark -o out/ --dry-run`
- **THEN** lệnh in ra tên các file sẽ tạo và out/ không có file mới nào

### Requirement: Mật khẩu không lộ trên dòng lệnh
Với file có mật khẩu, CLI SHALL nhận mật khẩu qua `--password-file <file>`, biến môi trường `VPDF_PASSWORD`, hoặc hỏi tương tác khi đang chạy trong terminal. Mật khẩu MUST không được in ra trong nhật ký hay đầu ra JSON.

#### Scenario: Mật khẩu từ biến môi trường
- **GIVEN** biến môi trường `VPDF_PASSWORD` chứa mật khẩu đúng
- **WHEN** chạy `vpdf ocr secret.pdf -o out.pdf`
- **THEN** file được OCR và mật khẩu không xuất hiện trong đầu ra

#### Scenario: Không có terminal
- **WHEN** chạy lệnh trên file có mật khẩu mà không có nguồn mật khẩu nào, và stdin không phải terminal
- **THEN** file đó báo lỗi "Cần mật khẩu" thay vì treo chờ nhập

### Requirement: Chạy không cần giao diện
CLI SHALL chạy được khi không có giao diện đồ họa (phiên SSH, tác vụ theo lịch, máy chủ CI) và không mở cửa sổ nào.

#### Scenario: Tác vụ theo lịch trên Windows
- **WHEN** chạy `vpdf compress` từ Task Scheduler với tài khoản không đăng nhập
- **THEN** lệnh chạy xong và thoát với mã đúng, không hiện cửa sổ nào
