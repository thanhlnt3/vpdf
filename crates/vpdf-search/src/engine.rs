//! Luồng tìm chạy nền của một tài liệu.
//!
//! - Nhận lớp chữ từng trang (do worker PDFium trích ra, chạy nền) và giữ chỉ mục
//!   gọn: chữ gốc, hộp ký tự và chuỗi đã chuẩn hóa mặc định.
//! - Mỗi lượt tìm quét các trang **theo thứ tự trang**, gửi kết quả từng trang ngay khi
//!   có. Trang chưa được trích thì báo `need` để worker trích trang đó trước.
//! - Lượt tìm mới (hoặc Hủy) thay lượt cũ: luồng kiểm tra hộp thư giữa mỗi nhóm
//!   trang nhỏ (≤ 8 ms), nên lượt cũ dừng gần như ngay lập tức.

use crate::normalize::{normalize_query, NormText, SearchOptions};
use crate::page::{hits_on_page, Hit, PageSource};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use vpdf_core::text::PageText;

/// Kết quả gửi về giao diện. `id` là mã của lượt tìm.
#[derive(Debug, Clone)]
pub enum Output {
    /// Kết quả trên một trang (các trang đến theo thứ tự tăng dần).
    Hits { id: u64, hits: Vec<Hit> },
    /// Đã quét xong `scanned` trang đầu.
    Progress { id: u64, scanned: u32 },
    /// Đã quét hết tài liệu.
    Done { id: u64, total: usize, no_text_pages: usize },
}

enum Input {
    Page(u32, PageText),
    Search { id: u64, query: String, opts: SearchOptions },
    Cancel,
    Shutdown,
}

/// Yêu cầu gửi cho worker trích chữ: `Some(p)` = cần gấp các trang từ `p` trở đi,
/// `None` = không có lượt tìm nào đang chờ (trích nền bình thường).
pub type NeedFn = Arc<dyn Fn(Option<u32>) + Send + Sync>;
pub type NotifyFn = Arc<dyn Fn() + Send + Sync>;

pub struct SearchEngine {
    tx: Sender<Input>,
    out: Receiver<Output>,
    indexed: Arc<AtomicUsize>,
    page_count: u32,
    thread: Option<JoinHandle<()>>,
}

impl SearchEngine {
    pub fn new(page_count: u32, need: NeedFn, notify: NotifyFn) -> Self {
        let (tx, rx) = channel();
        let (otx, orx) = channel();
        let indexed = Arc::new(AtomicUsize::new(0));
        let ix = indexed.clone();
        let thread = std::thread::Builder::new()
            .name("vpdf-search".into())
            .spawn(move || run(page_count, rx, otx, need, notify, ix))
            .expect("không tạo được luồng tìm");
        SearchEngine { tx, out: orx, indexed, page_count, thread: Some(thread) }
    }

    /// Hàm nhận lớp chữ của trang, đưa cho worker PDFium.
    pub fn page_sink(&self) -> Box<dyn Fn(u32, PageText) + Send> {
        let tx = self.tx.clone();
        Box::new(move |p, t| {
            let _ = tx.send(Input::Page(p, t));
        })
    }

    pub fn add_page(&self, page: u32, text: PageText) {
        let _ = self.tx.send(Input::Page(page, text));
    }

    pub fn search(&self, id: u64, query: &str, opts: SearchOptions) {
        let _ = self.tx.send(Input::Search { id, query: query.to_string(), opts });
    }

    pub fn cancel(&self) {
        let _ = self.tx.send(Input::Cancel);
    }

    pub fn try_recv(&self) -> Option<Output> {
        self.out.try_recv().ok()
    }

    pub fn recv_timeout(&self, d: Duration) -> Option<Output> {
        self.out.recv_timeout(d).ok()
    }

    /// Số trang đã có trong chỉ mục.
    pub fn indexed_pages(&self) -> usize {
        self.indexed.load(Ordering::Relaxed)
    }

    pub fn page_count(&self) -> u32 {
        self.page_count
    }
}

impl Drop for SearchEngine {
    fn drop(&mut self) {
        let _ = self.tx.send(Input::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

struct Entry {
    src: PageSource,
    has_text: bool,
    forms: [Option<NormText>; 4],
}

impl Entry {
    fn ensure_form(&mut self, opts: SearchOptions) -> usize {
        let k = opts.form_key();
        if self.forms[k].is_none() {
            // Giữ dạng mặc định và dạng đang dùng; bỏ các dạng khác để tiết kiệm bộ nhớ.
            for (i, f) in self.forms.iter_mut().enumerate() {
                if i != 0 && i != k {
                    *f = None;
                }
            }
            self.forms[k] = Some(self.src.normalize(opts));
        }
        k
    }
}

struct Active {
    id: u64,
    query: String,
    opts: SearchOptions,
    next: u32,
    total: usize,
    no_text: usize,
    done: bool,
}

/// Thời gian tối đa cho một nhóm trang trước khi kiểm tra lại hộp thư.
const SLICE: Duration = Duration::from_millis(8);

fn run(n: u32, rx: Receiver<Input>, out: Sender<Output>, need: NeedFn, notify: NotifyFn, indexed: Arc<AtomicUsize>) {
    let mut pages: Vec<Option<Entry>> = (0..n).map(|_| None).collect();
    let mut active: Option<Active> = None;
    let mut last_need: Option<Option<u32>> = None;
    let send = |o: Output| {
        let _ = out.send(o);
        notify();
    };
    let set_need = |v: Option<u32>, last: &mut Option<Option<u32>>| {
        if *last != Some(v) {
            *last = Some(v);
            need(v);
        }
    };
    loop {
        let runnable = active.as_ref().is_some_and(|a| !a.done && (a.next >= n || pages[a.next as usize].is_some()));
        let msg = if runnable {
            match rx.try_recv() {
                Ok(m) => Some(m),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => return,
            }
        } else {
            match rx.recv() {
                Ok(m) => Some(m),
                Err(_) => return,
            }
        };
        if let Some(m) = msg {
            match m {
                Input::Shutdown => return,
                Input::Page(p, t) => {
                    if let Some(slot) = pages.get_mut(p as usize) {
                        let src = PageSource::from_page_text(&t);
                        let has_text = src.has_text();
                        let default_form = if has_text { Some(src.normalize(SearchOptions::default())) } else { None };
                        if slot.is_none() {
                            indexed.fetch_add(1, Ordering::Relaxed);
                        }
                        *slot = Some(Entry { src, has_text, forms: [default_form, None, None, None] });
                    }
                }
                Input::Search { id, query, opts } => {
                    let q = normalize_query(&query, opts);
                    if q.is_empty() {
                        active = None;
                        set_need(None, &mut last_need);
                    } else {
                        active = Some(Active { id, query: q, opts, next: 0, total: 0, no_text: 0, done: false });
                    }
                }
                Input::Cancel => {
                    active = None;
                    set_need(None, &mut last_need);
                }
            }
            continue;
        }
        // Quét một nhóm trang của lượt tìm hiện tại.
        let Some(a) = active.as_mut() else { continue };
        let t0 = Instant::now();
        let start = a.next;
        loop {
            if a.next >= n {
                a.done = true;
                if a.next > start {
                    send(Output::Progress { id: a.id, scanned: a.next });
                }
                send(Output::Done { id: a.id, total: a.total, no_text_pages: a.no_text });
                set_need(None, &mut last_need);
                break;
            }
            let Some(entry) = pages[a.next as usize].as_mut() else {
                // Trang chưa được trích: nhờ worker trích các trang từ đây trước.
                if a.next > start {
                    send(Output::Progress { id: a.id, scanned: a.next });
                }
                set_need(Some(a.next), &mut last_need);
                break;
            };
            if entry.has_text {
                let opts = a.opts;
                let k = entry.ensure_form(opts);
                let form = entry.forms[k].as_ref().unwrap();
                let hits = hits_on_page(a.next, &entry.src, form, &a.query, opts);
                if !hits.is_empty() {
                    a.total += hits.len();
                    send(Output::Hits { id: a.id, hits });
                }
            } else {
                a.no_text += 1;
            }
            a.next += 1;
            if t0.elapsed() > SLICE {
                send(Output::Progress { id: a.id, scanned: a.next });
                break;
            }
        }
    }
}
