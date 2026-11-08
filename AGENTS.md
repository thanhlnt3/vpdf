# AGENTS.md

Nguồn yêu cầu duy nhất của dự án là OpenSpec trong `openspec/`.

- Đọc `openspec/changes/add-pdf-reader-app/proposal.md`, `design.md`, rồi từng file `specs/*/spec.md`.
- Mỗi `### Requirement` là yêu cầu bắt buộc (SHALL/MUST); mỗi `#### Scenario` là một tiêu chí chấp nhận và cần có test tương ứng.
- Làm theo thứ tự trong `tasks.md`, đánh dấu `- [x]` khi task đã được xác minh.
- Chỉ dùng thư viện có license trong danh sách của `specs/dependency-licensing/spec.md`. Không dùng MuPDF, Poppler, Ghostscript, iText hay bất kỳ thành phần GPL/AGPL nào.
- Không dùng tên, logo, biểu tượng hay văn bản giao diện của sản phẩm PDF thương mại nào.
- Chỗ nào spec chưa rõ thì ghi câu hỏi lại, không tự đoán.
