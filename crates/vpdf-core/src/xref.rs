//! Kiểm tra nhanh cấu trúc file trước khi mở, để phân biệt:
//! - file không phải PDF (báo "Không đọc được file PDF"),
//! - PDF có bảng xref hỏng: PDFium vẫn khôi phục được trang, nhưng ứng dụng phải
//!   cảnh báo rằng file đã được sửa tạm khi mở.
//!
//! Chỉ đọc phần đầu, phần cuối và bảng xref cuối cùng của file (không đọc cả file),
//! nên không làm chậm việc mở file lớn.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XrefHealth {
    /// Bảng xref trỏ đúng vào các đối tượng.
    Ok,
    /// Có dấu hiệu hỏng; nội dung là mô tả ngắn để ghi nhật ký.
    Damaged(String),
    /// Không có tiêu đề `%PDF-`.
    NotPdf,
}

/// Số mục xref được lấy mẫu để kiểm tra.
const SAMPLE_ENTRIES: usize = 24;

pub fn check_file(path: &Path) -> std::io::Result<XrefHealth> {
    let mut f = File::open(path)?;
    let len = f.metadata()?.len();
    let mut head = vec![0u8; 1024.min(len as usize)];
    f.read_exact(&mut head)?;
    if find(&head, b"%PDF-").is_none() {
        return Ok(XrefHealth::NotPdf);
    }
    let tail_len = 4096.min(len);
    f.seek(SeekFrom::Start(len - tail_len))?;
    let mut tail = vec![0u8; tail_len as usize];
    f.read_exact(&mut tail)?;
    let Some(sx) = rfind(&tail, b"startxref") else {
        return Ok(XrefHealth::Damaged("thiếu startxref".into()));
    };
    let after = &tail[sx + 9..];
    let num: String = after
        .iter()
        .skip_while(|b| b.is_ascii_whitespace())
        .take_while(|b| b.is_ascii_digit())
        .map(|&b| b as char)
        .collect();
    let Ok(offset) = num.parse::<u64>() else {
        return Ok(XrefHealth::Damaged("startxref không có số".into()));
    };
    if offset >= len {
        return Ok(XrefHealth::Damaged(format!("startxref {offset} vượt quá kích thước file")));
    }
    let mut probe = vec![0u8; 64.min((len - offset) as usize)];
    f.seek(SeekFrom::Start(offset))?;
    f.read_exact(&mut probe)?;
    let trimmed = trim_ws(&probe);
    if trimmed.starts_with(b"xref") {
        check_classic_table(&mut f, offset, len)
    } else if looks_like_obj_header(trimmed) {
        // Xref stream (PDF 1.5+): đích của startxref là một đối tượng, coi là hợp lệ.
        Ok(XrefHealth::Ok)
    } else {
        Ok(XrefHealth::Damaged(format!("startxref {offset} không trỏ vào bảng xref")))
    }
}

fn check_classic_table(f: &mut File, offset: u64, len: u64) -> std::io::Result<XrefHealth> {
    // Đọc tối đa 4 MB của bảng xref.
    let read_len = (4 * 1024 * 1024).min(len - offset) as usize;
    let mut buf = vec![0u8; read_len];
    f.seek(SeekFrom::Start(offset))?;
    f.read_exact(&mut buf)?;
    let text = String::from_utf8_lossy(&buf);
    let mut lines = text.split(['\n', '\r']).filter(|l| !l.trim().is_empty());
    lines.next(); // "xref"
    let mut entries: Vec<(u32, u64)> = Vec::new();
    'outer: loop {
        let Some(header) = lines.next() else { break };
        let header = header.trim();
        if header.starts_with("trailer") {
            break;
        }
        let mut it = header.split_whitespace();
        let (Some(start), Some(count)) = (it.next(), it.next()) else {
            return Ok(XrefHealth::Damaged(format!("dòng xref lạ: {header}")));
        };
        let (Ok(start), Ok(count)) = (start.parse::<u32>(), count.parse::<u32>()) else {
            return Ok(XrefHealth::Damaged(format!("dòng xref lạ: {header}")));
        };
        for i in 0..count {
            let Some(e) = lines.next() else { break 'outer };
            let parts: Vec<&str> = e.split_whitespace().collect();
            if parts.len() < 3 {
                return Ok(XrefHealth::Damaged(format!("mục xref lạ: {e}")));
            }
            if parts[2] == "n" {
                if let Ok(o) = parts[0].parse::<u64>() {
                    entries.push((start + i, o));
                }
            }
        }
    }
    if entries.is_empty() {
        return Ok(XrefHealth::Damaged("bảng xref rỗng".into()));
    }
    let step = (entries.len() / SAMPLE_ENTRIES).max(1);
    for &(num, off) in entries.iter().step_by(step) {
        if off >= len {
            return Ok(XrefHealth::Damaged(format!("đối tượng {num} trỏ ra ngoài file")));
        }
        let mut b = vec![0u8; 32.min((len - off) as usize)];
        f.seek(SeekFrom::Start(off))?;
        f.read_exact(&mut b)?;
        let t = trim_ws(&b);
        let want = format!("{num} ");
        if !t.starts_with(want.as_bytes()) || find(t, b"obj").is_none() {
            return Ok(XrefHealth::Damaged(format!("xref của đối tượng {num} lệch vị trí")));
        }
    }
    Ok(XrefHealth::Ok)
}

/// File có từ điển `/Encrypt` trong trailer (hoặc trong từ điển xref stream) không.
pub fn has_encrypt_entry(path: &Path) -> std::io::Result<bool> {
    let mut f = File::open(path)?;
    let len = f.metadata()?.len();
    let tail_len = 8192.min(len);
    f.seek(SeekFrom::Start(len - tail_len))?;
    let mut tail = vec![0u8; tail_len as usize];
    f.read_exact(&mut tail)?;
    if find(&tail, b"/Encrypt").is_some() {
        return Ok(true);
    }
    // Xref stream ở xa cuối file: đọc từ điển tại vị trí startxref.
    if let Some(sx) = rfind(&tail, b"startxref") {
        let num: String = tail[sx + 9..]
            .iter()
            .skip_while(|b| b.is_ascii_whitespace())
            .take_while(|b| b.is_ascii_digit())
            .map(|&b| b as char)
            .collect();
        if let Ok(off) = num.parse::<u64>() {
            if off < len {
                let mut b = vec![0u8; 4096.min((len - off) as usize)];
                f.seek(SeekFrom::Start(off))?;
                f.read_exact(&mut b)?;
                return Ok(find(&b, b"/Encrypt").is_some());
            }
        }
    }
    Ok(false)
}

fn looks_like_obj_header(b: &[u8]) -> bool {
    let s = String::from_utf8_lossy(&b[..b.len().min(32)]);
    let mut it = s.split_whitespace();
    matches!((it.next(), it.next(), it.next()), (Some(a), Some(g), Some(o))
        if a.chars().all(|c| c.is_ascii_digit()) && g.chars().all(|c| c.is_ascii_digit()) && o.starts_with("obj"))
}

fn trim_ws(b: &[u8]) -> &[u8] {
    let start = b.iter().position(|c| !c.is_ascii_whitespace()).unwrap_or(b.len());
    &b[start..]
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).rposition(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_pdf() -> Vec<u8> {
        let objs = [
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>\nendobj\n",
        ];
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offs = Vec::new();
        for o in objs {
            offs.push(out.len());
            out.extend_from_slice(o.as_bytes());
        }
        let xref = out.len();
        out.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
        for o in offs {
            out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
        out
    }

    fn write(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("vpdf-xref-{}-{name}", std::process::id()));
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn healthy_file_is_ok() {
        let p = write("ok.pdf", &tiny_pdf());
        assert_eq!(check_file(&p).unwrap(), XrefHealth::Ok);
    }

    #[test]
    fn not_a_pdf() {
        let p = write("txt.pdf", b"Day khong phai la PDF\n");
        assert_eq!(check_file(&p).unwrap(), XrefHealth::NotPdf);
    }

    #[test]
    fn shifted_objects_are_damaged() {
        let mut b = tiny_pdf();
        // Chèn rác sau tiêu đề: mọi offset trong xref bị lệch.
        b.splice(9..9, b"%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%\n".iter().copied());
        let p = write("shift.pdf", &b);
        assert!(matches!(check_file(&p).unwrap(), XrefHealth::Damaged(_)));
    }

    #[test]
    fn missing_startxref_is_damaged() {
        let mut b = tiny_pdf();
        let n = b.len();
        b.truncate(n - 30);
        let p = write("trunc.pdf", &b);
        assert!(matches!(check_file(&p).unwrap(), XrefHealth::Damaged(_)));
    }
}
