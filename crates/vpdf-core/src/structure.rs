//! Thao tác cấu trúc PDF bằng lopdf (design.md, D3). Lượt này chỉ cần mở và ghi
//! lại; gộp, tách, chèn, xóa trang thuộc nhóm 4–5.

use std::path::Path;

/// Mở bằng lopdf và ghi lại thành file mới. Trả về số trang.
pub fn rewrite_with_lopdf(input: &Path, output: &Path) -> Result<usize, String> {
    let mut doc = lopdf::Document::load(input).map_err(|e| format!("lopdf không mở được {}: {e}", input.display()))?;
    let pages = doc.get_pages().len();
    doc.save(output).map_err(|e| format!("lopdf không ghi được {}: {e}", output.display()))?;
    Ok(pages)
}

/// Số trang theo lopdf.
pub fn page_count(input: &Path) -> Result<usize, String> {
    let doc = lopdf::Document::load(input).map_err(|e| e.to_string())?;
    Ok(doc.get_pages().len())
}
