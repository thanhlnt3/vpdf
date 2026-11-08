//! qpdf qua C API (`qpdf-c.h`), nạp động lúc chạy (design.md, D3).
//!
//! Lượt này chỉ cần: đọc, kiểm tra cấu trúc (`qpdf_check_pdf`, tương đương
//! `qpdf --check`), ghi lại (object streams, nén luồng) và đặt mật khẩu AES-256 R6
//! cho bộ tài liệu kiểm thử. Giao diện đặt/gỡ mật khẩu thuộc nhóm 6.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::Path;
use std::sync::OnceLock;

type QpdfData = *mut c_void;
type QpdfError = *mut c_void;

const QPDF_ERRORS: c_int = 1 << 1;

/// Chế độ object streams khi ghi.
#[derive(Debug, Clone, Copy)]
pub enum ObjectStreams {
    Disable = 0,
    Preserve = 1,
    Generate = 2,
}

/// Quyền in cho mã hóa R6.
#[derive(Debug, Clone, Copy)]
pub enum PrintPermission {
    Full = 0,
    Low = 1,
    None = 2,
}

/// Tham số mã hóa AES-256 (R6).
#[derive(Debug, Clone)]
pub struct Encryption {
    pub user_password: String,
    pub owner_password: String,
    pub allow_extract: bool,
    pub print: PrintPermission,
}

#[derive(Debug, Clone)]
pub struct CheckReport {
    pub ok: bool,
    pub pages: usize,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

struct Api {
    _lib: Library,
    init: unsafe extern "C" fn() -> QpdfData,
    cleanup: unsafe extern "C" fn(*mut QpdfData),
    read: unsafe extern "C" fn(QpdfData, *const c_char, *const c_char) -> c_int,
    check_pdf: unsafe extern "C" fn(QpdfData) -> c_int,
    has_error: unsafe extern "C" fn(QpdfData) -> c_int,
    get_error: unsafe extern "C" fn(QpdfData) -> QpdfError,
    more_warnings: unsafe extern "C" fn(QpdfData) -> c_int,
    next_warning: unsafe extern "C" fn(QpdfData) -> QpdfError,
    error_text: unsafe extern "C" fn(QpdfData, QpdfError) -> *const c_char,
    set_suppress_warnings: unsafe extern "C" fn(QpdfData, c_int),
    init_write: unsafe extern "C" fn(QpdfData, *const c_char) -> c_int,
    write: unsafe extern "C" fn(QpdfData) -> c_int,
    set_object_stream_mode: unsafe extern "C" fn(QpdfData, c_int),
    set_compress_streams: unsafe extern "C" fn(QpdfData, c_int),
    set_deterministic_id: unsafe extern "C" fn(QpdfData, c_int),
    num_pages: unsafe extern "C" fn(QpdfData) -> c_int,
    is_encrypted: unsafe extern "C" fn(QpdfData) -> c_int,
    allow_extract_all: unsafe extern "C" fn(QpdfData) -> c_int,
    allow_print_low_res: unsafe extern "C" fn(QpdfData) -> c_int,
    #[allow(clippy::type_complexity)]
    set_r6: unsafe extern "C" fn(
        QpdfData,
        *const c_char,
        *const c_char,
        c_int,
        c_int,
        c_int,
        c_int,
        c_int,
        c_int,
        c_int,
        c_int,
    ),
}

static API: OnceLock<Result<Api, String>> = OnceLock::new();

fn load() -> Result<Api, String> {
    let mut tried = Vec::new();
    for dir in crate::native::qpdf_dirs() {
        for name in crate::native::qpdf_library_names() {
            let p = dir.join(name);
            if !p.is_file() {
                continue;
            }
            tried.push(p.display().to_string());
            // SAFETY: nạp thư viện qpdf chính thức; các hàm có chữ ký đúng theo qpdf-c.h.
            unsafe {
                let lib = match open_library(&p) {
                    Ok(l) => l,
                    Err(e) => {
                        tried.push(format!("({e})"));
                        continue;
                    }
                };
                macro_rules! sym {
                    ($n:literal) => {{
                        let s: Symbol<_> = lib.get($n).map_err(|e| format!("{}: {e}", String::from_utf8_lossy($n)))?;
                        *s
                    }};
                }
                let api = Api {
                    init: sym!(b"qpdf_init"),
                    cleanup: sym!(b"qpdf_cleanup"),
                    read: sym!(b"qpdf_read"),
                    check_pdf: sym!(b"qpdf_check_pdf"),
                    has_error: sym!(b"qpdf_has_error"),
                    get_error: sym!(b"qpdf_get_error"),
                    more_warnings: sym!(b"qpdf_more_warnings"),
                    next_warning: sym!(b"qpdf_next_warning"),
                    error_text: sym!(b"qpdf_get_error_full_text"),
                    set_suppress_warnings: sym!(b"qpdf_set_suppress_warnings"),
                    init_write: sym!(b"qpdf_init_write"),
                    write: sym!(b"qpdf_write"),
                    set_object_stream_mode: sym!(b"qpdf_set_object_stream_mode"),
                    set_compress_streams: sym!(b"qpdf_set_compress_streams"),
                    set_deterministic_id: sym!(b"qpdf_set_deterministic_ID"),
                    num_pages: sym!(b"qpdf_get_num_pages"),
                    is_encrypted: sym!(b"qpdf_is_encrypted"),
                    allow_extract_all: sym!(b"qpdf_allow_extract_all"),
                    allow_print_low_res: sym!(b"qpdf_allow_print_low_res"),
                    set_r6: sym!(b"qpdf_set_r6_encryption_parameters2"),
                    _lib: lib,
                };
                return Ok(api);
            }
        }
    }
    Err(format!("không tìm thấy thư viện qpdf (đã thử: {})", tried.join(", ")))
}

/// Nạp thư viện; trên Windows tìm các DLL phụ thuộc (C++ runtime) trong cùng thư mục.
unsafe fn open_library(p: &Path) -> Result<Library, libloading::Error> {
    #[cfg(windows)]
    {
        const LOAD_WITH_ALTERED_SEARCH_PATH: u32 = 0x8;
        libloading::os::windows::Library::load_with_flags(p, LOAD_WITH_ALTERED_SEARCH_PATH).map(Library::from)
    }
    #[cfg(not(windows))]
    {
        Library::new(p)
    }
}

fn api() -> Result<&'static Api, String> {
    API.get_or_init(load).as_ref().map_err(|e| e.clone())
}

/// qpdf có dùng được không (thư viện đã nạp được).
pub fn available() -> bool {
    api().is_ok()
}

/// Một phiên qpdf (`qpdf_data`). Không dùng chung giữa các luồng.
pub struct Qpdf {
    api: &'static Api,
    h: QpdfData,
}

impl Drop for Qpdf {
    fn drop(&mut self) {
        // SAFETY: h do qpdf_init tạo và chỉ được giải phóng một lần.
        unsafe { (self.api.cleanup)(&mut self.h) }
    }
}

fn cpath(p: &Path) -> Result<CString, String> {
    // qpdf nhận đường dẫn UTF-8 (trên Windows được đổi sang UTF-16 bên trong qpdf).
    CString::new(p.to_string_lossy().as_bytes()).map_err(|e| e.to_string())
}

impl Qpdf {
    /// Mở file. `password` có thể là mật khẩu người dùng hoặc chủ sở hữu.
    pub fn open(path: &Path, password: Option<&str>) -> Result<Self, String> {
        let api = api()?;
        // SAFETY: gọi theo đúng trình tự init → read của qpdf-c.h.
        unsafe {
            let h = (api.init)();
            let q = Qpdf { api, h };
            (api.set_suppress_warnings)(h, 1);
            let file = cpath(path)?;
            let pw = password.map(|p| CString::new(p).map_err(|e| e.to_string())).transpose()?;
            let rc = (api.read)(h, file.as_ptr(), pw.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()));
            if rc & QPDF_ERRORS != 0 {
                return Err(q.last_error().unwrap_or_else(|| "qpdf_read thất bại".into()));
            }
            Ok(q)
        }
    }

    fn last_error(&self) -> Option<String> {
        // SAFETY: con trỏ lỗi chỉ dùng ngay trong lời gọi này.
        unsafe {
            if (self.api.has_error)(self.h) == 0 {
                return None;
            }
            let e = (self.api.get_error)(self.h);
            Some(cstr((self.api.error_text)(self.h, e)))
        }
    }

    fn drain_warnings(&self) -> Vec<String> {
        let mut out = Vec::new();
        // SAFETY: như trên.
        unsafe {
            while (self.api.more_warnings)(self.h) != 0 {
                let w = (self.api.next_warning)(self.h);
                out.push(cstr((self.api.error_text)(self.h, w)));
            }
        }
        out
    }

    pub fn page_count(&self) -> usize {
        // SAFETY: h hợp lệ.
        unsafe { (self.api.num_pages)(self.h).max(0) as usize }
    }

    pub fn is_encrypted(&self) -> bool {
        unsafe { (self.api.is_encrypted)(self.h) != 0 }
    }

    pub fn allows_extract(&self) -> bool {
        unsafe { (self.api.allow_extract_all)(self.h) != 0 }
    }

    pub fn allows_print(&self) -> bool {
        unsafe { (self.api.allow_print_low_res)(self.h) != 0 }
    }

    /// Kiểm tra cấu trúc như `qpdf --check`.
    pub fn check(&self) -> CheckReport {
        // SAFETY: h hợp lệ.
        let rc = unsafe { (self.api.check_pdf)(self.h) };
        let warnings = self.drain_warnings();
        let error = if rc & QPDF_ERRORS != 0 { self.last_error() } else { None };
        CheckReport { ok: rc & QPDF_ERRORS == 0 && warnings.is_empty(), pages: self.page_count(), warnings, error }
    }

    /// Ghi ra file mới (chuẩn hóa: object streams, nén lại luồng, /ID xác định).
    pub fn write(self, out: &Path, object_streams: ObjectStreams, encryption: Option<&Encryption>) -> Result<(), String> {
        let file = cpath(out)?;
        // SAFETY: trình tự init_write → set_* → write của qpdf-c.h; CString sống tới hết lời gọi.
        unsafe {
            let rc = (self.api.init_write)(self.h, file.as_ptr());
            if rc & QPDF_ERRORS != 0 {
                return Err(self.last_error().unwrap_or_else(|| "qpdf_init_write thất bại".into()));
            }
            (self.api.set_object_stream_mode)(self.h, object_streams as c_int);
            (self.api.set_compress_streams)(self.h, 1);
            let pw = if let Some(e) = encryption {
                let u = CString::new(e.user_password.as_str()).map_err(|e| e.to_string())?;
                let o = CString::new(e.owner_password.as_str()).map_err(|e| e.to_string())?;
                let ex = e.allow_extract as c_int;
                // accessibility, extract, assemble, annotate+form, form filling, modify other, print, metadata
                (self.api.set_r6)(self.h, u.as_ptr(), o.as_ptr(), 1, ex, 1, 1, 1, 1, e.print as c_int, 1);
                Some((u, o))
            } else {
                // /ID xác định chỉ dùng được cho file không mã hóa (qpdf từ chối với file mã hóa).
                if (self.api.is_encrypted)(self.h) == 0 {
                    (self.api.set_deterministic_id)(self.h, 1);
                }
                None
            };
            let rc = (self.api.write)(self.h);
            drop(pw);
            if rc & QPDF_ERRORS != 0 {
                return Err(self.last_error().unwrap_or_else(|| "qpdf_write thất bại".into()));
            }
        }
        Ok(())
    }
}

fn cstr(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        // SAFETY: qpdf trả về chuỗi C kết thúc bằng NUL, còn hiệu lực trong phiên.
        unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() }
    }
}

/// Kiểm tra nhanh một file bằng qpdf.
pub fn check_file(path: &Path, password: Option<&str>) -> Result<CheckReport, String> {
    Ok(Qpdf::open(path, password)?.check())
}
