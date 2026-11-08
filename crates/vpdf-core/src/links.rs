//! Link trong tài liệu và quy tắc khi bấm vào link.

use crate::RectPt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkTarget {
    /// Link nội bộ: trang đích (từ 0) và vị trí y tính từ mép trên trang (point).
    Page { index: usize, y_from_top_pt: Option<f32> },
    /// Link ra ngoài.
    Uri(String),
    /// Loại link chưa hỗ trợ (mở file khác, chạy chương trình…): không làm gì.
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageLink {
    pub rect: RectPt,
    pub target: LinkTarget,
}

/// Việc giao diện cần làm sau khi người dùng bấm link.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkAction {
    /// Ghi vị trí hiện tại vào lịch sử rồi chuyển tới đích.
    Navigate { page: usize, y_from_top_pt: Option<f32> },
    /// Hỏi "Mở liên kết này trong trình duyệt?"; chỉ mở khi người dùng đồng ý.
    ConfirmOpenUrl(String),
    Ignore,
}

/// Chỉ các giao thức an toàn mới được đề nghị mở bằng trình duyệt.
pub fn is_browser_url(uri: &str) -> bool {
    let lower = uri.trim().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")
}

pub fn on_click(target: &LinkTarget, page_count: usize) -> LinkAction {
    match target {
        LinkTarget::Page { index, y_from_top_pt } if *index < page_count => {
            LinkAction::Navigate { page: *index, y_from_top_pt: *y_from_top_pt }
        }
        LinkTarget::Uri(u) if is_browser_url(u) => LinkAction::ConfirmOpenUrl(u.trim().to_string()),
        _ => LinkAction::Ignore,
    }
}

/// Link nằm dưới điểm (x, y) của trang, theo tọa độ PDF.
pub fn link_at(links: &[PageLink], x: f32, y: f32) -> Option<&PageLink> {
    links.iter().find(|l| l.rect.contains(x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_link_navigates() {
        let t = LinkTarget::Page { index: 29, y_from_top_pt: Some(10.0) };
        assert_eq!(on_click(&t, 500), LinkAction::Navigate { page: 29, y_from_top_pt: Some(10.0) });
        assert_eq!(on_click(&t, 10), LinkAction::Ignore);
    }

    /// Scenario: Link URL — phải hỏi xác nhận, không mở thẳng.
    #[test]
    fn url_link_requires_confirmation() {
        let t = LinkTarget::Uri("https://example.com".into());
        assert_eq!(on_click(&t, 1), LinkAction::ConfirmOpenUrl("https://example.com".into()));
        assert_eq!(on_click(&LinkTarget::Uri("javascript:alert(1)".into()), 1), LinkAction::Ignore);
        assert_eq!(on_click(&LinkTarget::Uri("file:///C:/x.exe".into()), 1), LinkAction::Ignore);
    }
}
