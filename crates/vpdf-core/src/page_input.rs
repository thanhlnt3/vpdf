//! Ô số trang: hiển thị "trang / tổng" và phân tích số người dùng gõ vào.

/// Chuỗi hiển thị trong ô số trang, ví dụ "125 / 500" (trang tính từ 1).
pub fn display(current_index: usize, total: usize) -> String {
    format!("{} / {}", current_index + 1, total)
}

/// Phân tích nội dung người dùng gõ. Trả về chỉ số trang (từ 0) nếu hợp lệ.
///
/// Chấp nhận "125", " 125 ", và cả "125 / 500" (khi người dùng chỉ sửa phần đầu).
/// Số 0, số lớn hơn tổng số trang, số âm và chữ đều không hợp lệ.
pub fn parse(input: &str, total: usize) -> Option<usize> {
    let head = input.split('/').next()?.trim();
    if head.is_empty() || !head.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let n: usize = head.parse().ok()?;
    if n >= 1 && n <= total {
        Some(n - 1)
    } else {
        None
    }
}

/// Trạng thái ô số trang trong lúc người dùng gõ.
#[derive(Debug, Clone, Default)]
pub struct PageBox {
    /// Nội dung đang gõ; `None` khi ô không được chỉnh sửa (hiện trang hiện tại).
    pub editing: Option<String>,
}

impl PageBox {
    /// Nội dung cần hiển thị trong ô.
    pub fn text(&self, current_index: usize, total: usize) -> String {
        match &self.editing {
            Some(s) => s.clone(),
            None => display(current_index, total),
        }
    }

    /// Người dùng nhấn Enter: trả về trang cần nhảy tới (nếu hợp lệ) và luôn thoát
    /// chế độ sửa, nên ô số trang trở về hiển thị trang hiện tại.
    pub fn commit(&mut self, total: usize) -> Option<usize> {
        let s = self.editing.take()?;
        parse(&s, total)
    }

    pub fn cancel(&mut self) {
        self.editing = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scenario: Nhảy tới trang.
    #[test]
    fn jump_to_125() {
        let mut b = PageBox { editing: Some("125".into()) };
        assert_eq!(b.commit(500), Some(124));
        assert_eq!(b.text(124, 500), "125 / 500");
    }

    /// Scenario: Số trang không hợp lệ.
    #[test]
    fn invalid_input_reverts() {
        for bad in ["501", "0", "abc", "-3", "", "12a", "1.5", "  "] {
            let mut b = PageBox { editing: Some(bad.into()) };
            assert_eq!(b.commit(500), None, "{bad:?} phải bị từ chối");
            assert_eq!(b.text(39, 500), "40 / 500", "ô phải trở về trang hiện tại");
        }
    }

    #[test]
    fn accepts_whitespace_and_suffix() {
        assert_eq!(parse(" 7 ", 10), Some(6));
        assert_eq!(parse("7 / 10", 10), Some(6));
        assert_eq!(parse("10", 10), Some(9));
    }
}
