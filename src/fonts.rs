//! Font giao diện: Noto Sans (OFL-1.1), đủ chữ tiếng Việt.
//!
//! Font mặc định của egui (Ubuntu-Light, giấy phép Ubuntu Font Licence) không nằm
//! trong danh sách license được phép, nên tính năng `default_fonts` bị tắt và chỉ
//! dùng Noto Sans đi kèm dự án.

use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

static NOTO_REGULAR: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");
static NOTO_BOLD: &[u8] = include_bytes!("../assets/fonts/NotoSans-Bold.ttf");

pub fn definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("noto".into(), Arc::new(FontData::from_static(NOTO_REGULAR)));
    fonts.font_data.insert("noto-bold".into(), Arc::new(FontData::from_static(NOTO_BOLD)));
    fonts.families.insert(FontFamily::Proportional, vec!["noto".into()]);
    fonts.families.insert(FontFamily::Monospace, vec!["noto".into()]);
    fonts.families.insert(FontFamily::Name("bold".into()), vec!["noto-bold".into()]);
    fonts
}

pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(definitions());
}
