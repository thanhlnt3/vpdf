//! CLI `vpdf` (`specs/batch-cli`). Các lệnh merge/split/compress/ocr thuộc nhóm task 11;
//! lượt này chỉ có `--version` và `--help` để kiểm tra workspace dựng được.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version") | Some("-V") => {
            println!("vpdf {}", vpdf_core::VERSION);
            ExitCode::SUCCESS
        }
        None | Some("--help") | Some("-h") => {
            println!("vpdf {} — công cụ dòng lệnh của {}", vpdf_core::VERSION, vpdf_core::PRODUCT_NAME);
            println!();
            println!("Cách dùng: vpdf [--version | --help]");
            println!("Các lệnh merge, split, compress, ocr sẽ có ở phiên bản sau.");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("vpdf: lệnh chưa hỗ trợ: {other}");
            ExitCode::from(2)
        }
    }
}
