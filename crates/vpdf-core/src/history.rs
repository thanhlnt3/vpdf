//! Lịch sử điều hướng cho lệnh Quay lại (sau khi bấm link nội bộ, mục lục,
//! ảnh thu nhỏ hoặc nhảy trang).

use crate::view::ViewPos;

/// Số vị trí tối đa được nhớ.
pub const MAX_HISTORY: usize = 100;

#[derive(Debug, Clone, Default)]
pub struct NavHistory {
    back: Vec<ViewPos>,
    forward: Vec<ViewPos>,
}

impl NavHistory {
    /// Ghi lại vị trí trước khi nhảy.
    pub fn push(&mut self, before_jump: ViewPos) {
        if self.back.last() == Some(&before_jump) {
            return;
        }
        self.back.push(before_jump);
        if self.back.len() > MAX_HISTORY {
            self.back.remove(0);
        }
        self.forward.clear();
    }

    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// Quay lại: trả về vị trí cần khôi phục; `current` được đưa vào danh sách Tiếp.
    pub fn back(&mut self, current: ViewPos) -> Option<ViewPos> {
        let p = self.back.pop()?;
        self.forward.push(current);
        Some(p)
    }

    pub fn forward(&mut self, current: ViewPos) -> Option<ViewPos> {
        let p = self.forward.pop()?;
        self.back.push(current);
        Some(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{FitMode, PageSize, ViewState};

    /// Scenario: Link nội bộ — Quay lại đưa về vị trí trước khi bấm link.
    #[test]
    fn back_restores_position_before_link() {
        let mut v = ViewState::new(vec![PageSize::A4; 100]);
        v.set_viewport(900.0, 700.0);
        v.goto_page(4);
        v.scroll_by(0.0, 150.0);
        let before = v.pos();
        let mut h = NavHistory::default();
        h.push(v.pos());
        v.goto_page(29);
        assert_eq!(v.current_page(), 29);
        let target = h.back(v.pos()).unwrap();
        v.restore(target);
        assert_eq!(v.current_page(), before.page);
        assert!((v.pos().y_in_page_pt - before.y_in_page_pt).abs() < 0.5);
        assert_eq!(v.fit(), FitMode::Free);
        assert!(h.can_go_forward());
        assert!(!h.can_go_back());
    }
}
