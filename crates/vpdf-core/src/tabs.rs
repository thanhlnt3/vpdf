//! Danh sách tab. Mỗi tab giữ trạng thái riêng của nó (vị trí đọc, mức phóng,
//! lịch sử…), nên chuyển tab không làm thay đổi tab khác.

#[derive(Debug, Clone)]
pub struct TabSet<T> {
    tabs: Vec<T>,
    active: usize,
}

impl<T> Default for TabSet<T> {
    fn default() -> Self {
        Self { tabs: Vec::new(), active: 0 }
    }
}

impl<T> TabSet<T> {
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Thêm tab mới và chuyển sang tab đó. Trả về chỉ số tab.
    pub fn push(&mut self, tab: T) -> usize {
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.active
    }

    pub fn active_index(&self) -> Option<usize> {
        if self.tabs.is_empty() {
            None
        } else {
            Some(self.active)
        }
    }

    pub fn active(&self) -> Option<&T> {
        self.tabs.get(self.active)
    }

    pub fn active_mut(&mut self) -> Option<&mut T> {
        self.tabs.get_mut(self.active)
    }

    pub fn select(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            self.active = index;
            true
        } else {
            false
        }
    }

    /// Đóng tab, trả lại nội dung tab để lưu vị trí đọc.
    pub fn close(&mut self, index: usize) -> Option<T> {
        if index >= self.tabs.len() {
            return None;
        }
        let t = self.tabs.remove(index);
        if self.active > index || (self.active == index && self.active >= self.tabs.len()) {
            self.active = self.active.saturating_sub(1);
        }
        Some(t)
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.tabs.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.tabs.iter_mut()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.tabs.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.tabs.get_mut(index)
    }

    pub fn position(&self, f: impl Fn(&T) -> bool) -> Option<usize> {
        self.tabs.iter().position(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{PageSize, ViewState};

    /// Scenario: Chuyển tab giữ vị trí.
    #[test]
    fn switching_tabs_keeps_position_and_zoom() {
        let mut tabs = TabSet::default();
        let a = tabs.push(ViewState::new(vec![PageSize::A4; 100]));
        let b = tabs.push(ViewState::new(vec![PageSize::A4; 10]));
        tabs.select(a);
        {
            let v = tabs.active_mut().unwrap();
            v.set_viewport(1000.0, 700.0);
            v.set_zoom(1.5);
            v.goto_page(39);
        }
        tabs.select(b);
        {
            let v = tabs.active_mut().unwrap();
            v.set_viewport(1000.0, 700.0);
            v.set_zoom(0.5);
            v.goto_page(3);
        }
        tabs.select(a);
        let v = tabs.active().unwrap();
        assert_eq!(v.current_page() + 1, 40);
        assert_eq!(v.zoom_percent(), 150);
    }

    #[test]
    fn closing_tabs_keeps_active_valid() {
        let mut t = TabSet::default();
        t.push(1);
        t.push(2);
        t.push(3);
        t.select(2);
        assert_eq!(t.close(2), Some(3));
        assert_eq!(t.active(), Some(&2));
        t.select(0);
        t.close(1);
        assert_eq!(t.active(), Some(&1));
        t.close(0);
        assert!(t.active().is_none());
    }
}
