# Spec Delta

## Purpose

Bảo đảm mọi thư viện, font và dữ liệu đi kèm ứng dụng có license cho phép dùng và phân phối thương mại mà không buộc công bố mã nguồn của ứng dụng, đồng thời ghi công đầy đủ.

## ADDED Requirements

### Requirement: Chỉ dùng license cho phép dùng thương mại
Mọi thành phần bên thứ ba đi kèm bản phát hành (thư viện, font, mô hình OCR, dữ liệu) SHALL có license nằm trong danh sách cho phép: MIT, BSD-2-Clause, BSD-3-Clause, Apache-2.0, ISC, Zlib, BSL-1.0, Unicode-3.0, OFL-1.1, CC0-1.0, MPL-2.0 (chỉ khi dùng nguyên vẹn, không sửa file). Thành phần GPL, AGPL, LGPL liên kết tĩnh, SSPL, hay license cấm dùng thương mại MUST không được đưa vào.

#### Scenario: Thư viện AGPL
- **WHEN** một thay đổi thêm thư viện có license AGPL-3.0 vào phụ thuộc
- **THEN** bước kiểm tra license của CI thất bại và chỉ rõ tên thư viện cùng license của nó

#### Scenario: Thư viện cho phép
- **WHEN** một thay đổi thêm thư viện có license Apache-2.0
- **THEN** bước kiểm tra license của CI thành công

### Requirement: Kiểm tra cả phụ thuộc gián tiếp
Việc kiểm tra license SHALL áp dụng cho toàn bộ cây phụ thuộc, gồm phụ thuộc gián tiếp, thư viện native dựng kèm, và tài nguyên tải về lúc dựng (ví dụ dữ liệu OCR, font). Thành phần không xác định được license MUST bị coi là không được phép.

#### Scenario: Phụ thuộc gián tiếp vi phạm
- **GIVEN** thư viện A (MIT) phụ thuộc vào thư viện B (GPL-3.0)
- **WHEN** thêm A vào dự án
- **THEN** CI thất bại và chỉ ra chuỗi phụ thuộc A → B

#### Scenario: Không rõ license
- **WHEN** một phụ thuộc không khai báo license
- **THEN** CI thất bại cho tới khi license được xác minh và ghi vào danh sách ngoại lệ đã duyệt

### Requirement: Ngoại lệ phải được duyệt
Ngoại lệ (ví dụ thành phần có license kép, hoặc license thương mại đã mua) SHALL được ghi trong một file cấu hình nằm trong kho mã, kèm lý do, người duyệt và ngày duyệt. Ngoại lệ thiếu một trong các thông tin đó MUST làm CI thất bại.

#### Scenario: Ngoại lệ thiếu người duyệt
- **WHEN** file ngoại lệ có một mục không ghi người duyệt
- **THEN** bước kiểm tra license của CI thất bại

### Requirement: Ghi công và thông báo license
Bản phát hành SHALL kèm file `THIRD_PARTY_NOTICES` liệt kê từng thành phần, phiên bản, license và toàn văn license hoặc thông báo bản quyền theo yêu cầu. Ứng dụng MUST có mục "Giới thiệu → Giấy phép bên thứ ba" hiện nội dung đó, và CLI MUST có `vpdf --licenses`.

#### Scenario: Xem giấy phép trong ứng dụng
- **WHEN** người dùng mở Giới thiệu → Giấy phép bên thứ ba
- **THEN** danh sách hiện đủ các thành phần có trong bản phát hành, kèm license

#### Scenario: Danh sách khớp bản phát hành
- **WHEN** CI dựng bản phát hành
- **THEN** `THIRD_PARTY_NOTICES` được tạo tự động từ cây phụ thuộc thực tế và không thiếu thành phần nào

### Requirement: Danh mục thành phần phần mềm
Mỗi bản phát hành SHALL kèm một SBOM dạng CycloneDX hoặc SPDX, liệt kê mọi thành phần cùng phiên bản và license.

#### Scenario: Có SBOM
- **WHEN** xem tài sản của một bản phát hành
- **THEN** có file SBOM khớp với phiên bản đó

### Requirement: Không dùng tài sản của sản phẩm khác
Mã nguồn, biểu tượng, hình ảnh, tên gọi và văn bản giao diện SHALL do dự án tự tạo hoặc lấy từ nguồn có license cho phép. Ứng dụng MUST không dùng tên, logo hay tài sản thiết kế của các trình đọc PDF thương mại.

#### Scenario: Rà soát tài sản trước phát hành
- **WHEN** rà soát các file biểu tượng và hình ảnh trong bản phát hành
- **THEN** mỗi file đều ghi được nguồn gốc là do dự án tạo hoặc có license cho phép
