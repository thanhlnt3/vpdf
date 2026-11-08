//! Một phiên bản duy nhất: khi người dùng nhấp đúp file PDF (liên kết loại file của hệ
//! điều hành) lúc ứng dụng đang chạy, file được chuyển sang cửa sổ đang chạy và mở
//! thành tab mới (task 2.3; specs/platform-support "Mở bằng nhấp đúp").
//!
//! Cách làm: phiên bản đầu tiên nghe trên 127.0.0.1 (cổng ngẫu nhiên) và ghi cổng cùng
//! một mã bí mật ngẫu nhiên vào `instance.json` trong thư mục cấu hình của người dùng.
//! Phiên bản sau đọc file đó, gửi mã bí mật kèm danh sách file rồi thoát.

use serde::{Deserialize, Serialize};
use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

#[derive(Serialize, Deserialize)]
struct Lock {
    port: u16,
    token: String,
}

#[derive(Serialize, Deserialize)]
struct Message {
    token: String,
    files: Vec<PathBuf>,
}

fn lock_path(dir: &Path) -> PathBuf {
    dir.join("instance.json")
}

fn random_token() -> String {
    let mut s = String::new();
    for i in 0..4u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(i ^ std::process::id() as u64);
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
        s.push_str(&format!("{:016x}", h.finish()));
    }
    s
}

/// Gửi file sang phiên bản đang chạy. Trả về true nếu đã gửi được (khi đó nên thoát).
pub fn forward(dir: &Path, files: &[PathBuf]) -> bool {
    let Ok(text) = std::fs::read_to_string(lock_path(dir)) else { return false };
    let Ok(lock) = serde_json::from_str::<Lock>(&text) else { return false };
    let addr = SocketAddr::from(([127, 0, 0, 1], lock.port));
    let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_millis(400)) else { return false };
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    let files: Vec<PathBuf> = files.iter().map(|f| std::fs::canonicalize(f).unwrap_or_else(|_| f.clone())).collect();
    let msg = serde_json::to_string(&Message { token: lock.token, files }).unwrap();
    if writeln!(s, "{msg}").is_err() {
        return false;
    }
    let mut reply = String::new();
    BufReader::new(s).read_line(&mut reply).is_ok() && reply.trim() == "ok"
}

/// Giữ cổng nghe; xóa `instance.json` khi ứng dụng thoát.
pub struct Listener {
    dir: PathBuf,
    pub files: Receiver<Vec<PathBuf>>,
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(lock_path(&self.dir));
    }
}

/// Bắt đầu nghe. `notify` được gọi khi có file mới (để giao diện vẽ lại và đưa cửa sổ lên trước).
pub fn listen(dir: &Path, notify: impl Fn() + Send + 'static) -> Option<Listener> {
    std::fs::create_dir_all(dir).ok()?;
    let listener = TcpListener::bind(("127.0.0.1", 0)).ok()?;
    let port = listener.local_addr().ok()?.port();
    let token = random_token();
    std::fs::write(lock_path(dir), serde_json::to_vec(&Lock { port, token: token.clone() }).ok()?).ok()?;
    let (tx, rx) = channel();
    std::thread::Builder::new()
        .name("single-instance".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let mut line = String::new();
                let mut reader = BufReader::new(&stream);
                if reader.read_line(&mut line).is_err() {
                    continue;
                }
                match serde_json::from_str::<Message>(&line) {
                    Ok(m) if m.token == token => {
                        let _ = (&stream).write_all(b"ok\n");
                        if tx.send(m.files).is_err() {
                            return;
                        }
                        notify();
                    }
                    _ => {
                        let _ = (&stream).write_all(b"denied\n");
                    }
                }
            }
        })
        .ok()?;
    Some(Listener { dir: dir.to_path_buf(), files: rx })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scenario "Mở bằng nhấp đúp" khi ứng dụng đang chạy: file được chuyển sang phiên bản đang chạy.
    #[test]
    fn second_instance_forwards_files() {
        let dir = std::env::temp_dir().join(format!("vpdf-single-{}", std::process::id()));
        let l = listen(&dir, || {}).expect("nghe được");
        let f = dir.join("Hợp đồng thuê nhà.pdf");
        std::fs::write(&f, b"%PDF-1.4").unwrap();
        assert!(forward(&dir, &[f.clone()]));
        let got = l.files.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(got.len(), 1);
        assert!(got[0].to_string_lossy().contains("Hợp đồng thuê nhà.pdf"));
        drop(l);
        assert!(!lock_path(&dir).exists());
        assert!(!forward(&dir, &[f]), "không còn phiên bản nào đang chạy");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn wrong_token_is_rejected() {
        let dir = std::env::temp_dir().join(format!("vpdf-single-bad-{}", std::process::id()));
        let l = listen(&dir, || {}).unwrap();
        let lock: Lock = serde_json::from_str(&std::fs::read_to_string(lock_path(&dir)).unwrap()).unwrap();
        let mut s = TcpStream::connect(("127.0.0.1", lock.port)).unwrap();
        writeln!(s, "{}", serde_json::to_string(&Message { token: "sai".into(), files: vec![PathBuf::from("x.pdf")] }).unwrap()).unwrap();
        let mut reply = String::new();
        BufReader::new(s).read_line(&mut reply).unwrap();
        assert_eq!(reply.trim(), "denied");
        assert!(l.files.recv_timeout(Duration::from_millis(300)).is_err());
        drop(l);
        let _ = std::fs::remove_dir_all(dir);
    }
}
