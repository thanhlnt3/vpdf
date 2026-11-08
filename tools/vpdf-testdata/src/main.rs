//! Bộ tài liệu tham chiếu (tasks.md 1.5; specs/performance "Máy và tài liệu tham chiếu").
//!
//! Lệnh:
//!   vpdf-testdata generate [--dir D]   tạo toàn bộ file và ghi MANIFEST
//!   vpdf-testdata fetch    [--dir D]   tải về từ $VPDF_TESTDATA_URL rồi kiểm tra mã băm;
//!                                      nếu chưa có URL thì tạo tại chỗ rồi kiểm tra
//!   vpdf-testdata verify   [--dir D]   kiểm tra mã băm theo MANIFEST
//!
//! Mọi nội dung đều do dự án tự sinh (chữ, ảnh minh họa, ảnh "scan"), font là Noto
//! Sans (OFL-1.1). File không mã hóa được tạo xác định (chạy lại cho ra cùng mã băm);
//! file mã hóa dùng muối ngẫu nhiên của qpdf nên được kiểm tra bằng chức năng
//! (mở được bằng đúng mật khẩu) thay vì mã băm khi tạo tại chỗ.

use lopdf::{dictionary, Document, Object, ObjectId, Stream, StringFormat};
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use vpdf_core::qpdf::{self, Encryption, ObjectStreams, PrintPermission};

pub const USER_PASSWORD: &str = "matkhau123";
pub const OWNER_PASSWORD: &str = "chusohuu456";

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Entry {
    name: String,
    sha256: String,
    bytes: u64,
    deterministic: bool,
    purpose: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Manifest {
    version: u32,
    note: String,
    files: Vec<Entry>,
}

fn app_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn default_dir() -> PathBuf {
    app_root().join("testdata").join("reference")
}

fn manifest_path() -> PathBuf {
    app_root().join("testdata").join("MANIFEST.json")
}

fn font_bytes() -> Vec<u8> {
    std::fs::read(app_root().join("assets/fonts/NotoSans-Regular.ttf")).expect("thiếu assets/fonts/NotoSans-Regular.ttf")
}

// ---------------------------------------------------------------------------
// Bộ sinh số giả ngẫu nhiên xác định (xorshift64*)

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

const WORDS: &[&str] = &[
    "Điều", "khoản", "hợp đồng", "Nghị định", "Đà Nẵng", "Hà Nội", "thành phố", "Hồ Chí Minh", "quy định",
    "trách nhiệm", "các bên", "thực hiện", "nghĩa vụ", "quyền lợi", "văn bản", "thời hạn", "thanh toán",
    "bảo đảm", "ủy ban", "nhân dân", "tỉnh", "huyện", "xã", "phường", "công ty", "trách nhiệm hữu hạn",
    "người lao động", "người sử dụng", "báo cáo", "kế hoạch", "năm", "tháng", "ngày", "số", "theo",
    "và", "của", "cho", "được", "không", "những", "trong", "khi", "với", "đã", "sẽ", "phải", "có",
    "Quốc hội", "Chính phủ", "Bộ Tài chính", "thông tư", "hướng dẫn", "điều chỉnh", "bổ sung",
    "sửa đổi", "hiệu lực", "kể từ", "ký", "đóng dấu", "xác nhận", "phụ lục", "đính kèm", "bản sao",
    "chứng thực", "giấy phép", "đăng ký", "kinh doanh", "đầu tư", "xây dựng", "đất đai", "nhà ở",
    "thuê", "giá", "đồng", "Việt Nam", "tiếng Việt", "dữ liệu", "điện tử", "chữ ký số", "mật khẩu",
];

const CHAPTERS: &[&str] = &[
    "Những quy định chung", "Giải thích từ ngữ", "Quyền và nghĩa vụ của các bên", "Giá thuê và phương thức thanh toán",
    "Thời hạn và gia hạn", "Bảo trì, sửa chữa", "Chấm dứt hợp đồng", "Giải quyết tranh chấp", "Bồi thường thiệt hại",
    "Bất khả kháng", "Bảo mật thông tin", "Chuyển nhượng", "Thông báo", "Luật áp dụng", "Hiệu lực thi hành",
    "Điều khoản chuyển tiếp", "Phụ lục kỹ thuật", "Biểu mẫu", "Danh mục tài liệu", "Tổ chức thực hiện",
];

fn sentence(rng: &mut Rng) -> String {
    let n = 8 + rng.below(10) as usize;
    let mut s = String::new();
    for i in 0..n {
        let w = WORDS[rng.below(WORDS.len() as u64) as usize];
        if i == 0 {
            let mut c = w.chars();
            let first = c.next().unwrap().to_uppercase().collect::<String>();
            s.push_str(&first);
            s.push_str(c.as_str());
        } else {
            s.push(' ');
            s.push_str(w);
        }
    }
    s.push('.');
    s
}

fn wrap(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for w in text.split(' ') {
        if !cur.is_empty() && cur.chars().count() + 1 + w.chars().count() > max_chars {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(w);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

// ---------------------------------------------------------------------------
// Ảnh

fn jpeg_rgb(w: u32, h: u32, seed: u64, quality: u8) -> Vec<u8> {
    let mut rng = Rng(seed | 1);
    let mut img = image::RgbImage::new(w, h);
    let cx = (rng.below(w as u64)) as f32;
    let cy = (rng.below(h as u64)) as f32;
    let hue = rng.below(255) as f32;
    for (x, y, p) in img.enumerate_pixels_mut() {
        let d = (((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / 40.0).sin();
        let n = (rng.below(48)) as f32;
        let r = (120.0 + 80.0 * d + n + hue * 0.2).clamp(0.0, 255.0) as u8;
        let g = (90.0 + 60.0 * (x as f32 / w as f32) * 2.0 + n).clamp(0.0, 255.0) as u8;
        let b = (160.0 - 70.0 * (y as f32 / h as f32) + n + (255.0 - hue) * 0.2).clamp(0.0, 255.0) as u8;
        *p = image::Rgb([r, g, b]);
    }
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality).encode_image(&img).unwrap();
    out
}

/// Trang "scan": nền giấy có nhiễu, các dòng chữ là vệt tối.
fn jpeg_scan(w: u32, h: u32, seed: u64) -> Vec<u8> {
    let mut rng = Rng(seed | 1);
    let mut img = image::GrayImage::new(w, h);
    let line_h = 26u32;
    let tilt = (rng.below(7) as f32 - 3.0) / 1000.0;
    for (x, y, p) in img.enumerate_pixels_mut() {
        let yy = y as f32 + x as f32 * tilt;
        let row = (yy / line_h as f32) as i64;
        let in_text_band = row > 4 && row < 62 && (yy as i64 % line_h as i64) < 13 && x > 120 && x < w - 120;
        let word_gap = ((x / 37 + row as u32 * 7) % 9) == 0;
        let base: i32 = if in_text_band && !word_gap { 40 } else { 236 };
        let noise = rng.below(22) as i32 - 11;
        *p = image::Luma([(base + noise).clamp(0, 255) as u8]);
    }
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 64).encode_image(&img).unwrap();
    out
}

// ---------------------------------------------------------------------------
// Tài liệu chữ (PDFium), rồi thêm mục lục và link bằng lopdf

struct TextDocSpec {
    pages: usize,
    pages_per_chapter: usize,
    image_quality: u8,
    image_size: (u32, u32),
    link_page: usize,
}

fn make_text_doc(spec: &TextDocSpec, out: &Path) -> Result<(), String> {
    let pdfium = vpdf_render::pdfium()?;
    let font_data = font_bytes();
    let mut doc = pdfium.create_new_pdf().map_err(|e| format!("{e:?}"))?;
    let font = doc.fonts_mut().load_true_type_from_bytes(&font_data, true).map_err(|e| format!("{e:?}"))?;
    let mut rng = Rng(0x5eed_0001);
    for i in 0..spec.pages {
        let mut page = doc.pages_mut().create_page_at_end(PdfPagePaperSize::a4()).map_err(|e| format!("{e:?}"))?;
        // Sinh lại content stream một lần cho cả trang thay vì sau mỗi đối tượng.
        page.set_content_regeneration_strategy(PdfPageContentRegenerationStrategy::Manual);
        let mut y = 842.0 - 60.0;
        let chapter = i / spec.pages_per_chapter;
        let objs = page.objects_mut();
        let line = |objs: &mut PdfPageObjects, text: &str, x: f32, y: f32, size: f32| -> Result<(), String> {
            objs.create_text_object(PdfPoints::new(x), PdfPoints::new(y), text, font, PdfPoints::new(size))
                .map(|_| ())
                .map_err(|e| format!("{e:?}"))
        };
        if i == 0 {
            line(objs, "Cộng hòa xã hội chủ nghĩa Việt Nam", 56.0, y, 14.0)?;
            y -= 22.0;
            line(objs, "Độc lập - Tự do - Hạnh phúc", 56.0, y, 12.0)?;
            y -= 26.0;
            line(objs, &format!("Xem trang {}", spec.link_page), 56.0, y, 11.0)?;
            y -= 18.0;
            line(objs, "Trang web: https://example.com", 56.0, y, 11.0)?;
            y -= 24.0;
        }
        if i % spec.pages_per_chapter == 0 {
            // Tiêu đề chương ở cách mép trên 100 pt: đích của mục lục trỏ đúng vị trí này.
            let title = format!("Chương {}: {}", chapter + 1, CHAPTERS[chapter % CHAPTERS.len()]);
            let ty = if i == 0 { y } else { 842.0 - 100.0 };
            line(objs, &title, 56.0, ty - 14.0, 14.0)?;
            y = ty - 40.0;
        }
        let image_h = 483.0 * spec.image_size.1 as f32 / spec.image_size.0 as f32;
        let text_bottom = 60.0 + image_h + 20.0;
        while y > text_bottom {
            let para = (0..3).map(|_| sentence(&mut rng)).collect::<Vec<_>>().join(" ");
            for l in wrap(&para, 84) {
                if y <= text_bottom {
                    break;
                }
                line(objs, &l, 56.0, y, 10.5)?;
                y -= 15.0;
            }
            y -= 6.0;
        }
        line(objs, &format!("Trang {} / {}", i + 1, spec.pages), 270.0, 30.0, 9.0)?;
        let jpg = jpeg_rgb(spec.image_size.0, spec.image_size.1, 1000 + i as u64, spec.image_quality);
        let mut img = PdfPageImageObject::new_from_jpeg_reader(&doc, Cursor::new(jpg)).map_err(|e| format!("{e:?}"))?;
        img.scale(483.0, image_h).map_err(|e| format!("{e:?}"))?;
        img.translate(PdfPoints::new(56.0), PdfPoints::new(60.0)).map_err(|e| format!("{e:?}"))?;
        page.objects_mut().add_image_object(img).map_err(|e| format!("{e:?}"))?;
        page.regenerate_content().map_err(|e| format!("{e:?}"))?;
    }
    let tmp = out.with_extension("pdfium.tmp");
    doc.save_to_file(&tmp).map_err(|e| format!("{e:?}"))?;
    drop(doc);
    add_outline_and_links(&tmp, out, spec)?;
    let _ = std::fs::remove_file(&tmp);
    Ok(())
}

fn utf16_title(s: &str) -> Object {
    let mut b = vec![0xFE, 0xFF];
    for u in s.encode_utf16() {
        b.extend_from_slice(&u.to_be_bytes());
    }
    Object::String(b, StringFormat::Hexadecimal)
}

fn xyz(page: ObjectId, top: f32) -> Object {
    Object::Array(vec![Object::Reference(page), "XYZ".into(), Object::Integer(0), Object::Real(top), Object::Null])
}

fn finalize(doc: &mut Document) {
    // /ID và ngày tạo cố định để file sinh ra xác định.
    let id = Object::String(b"vpdf-reference-0001".to_vec(), StringFormat::Hexadecimal);
    doc.trailer.set("ID", Object::Array(vec![id.clone(), id]));
    let info = dictionary! {
        "Creator" => Object::string_literal("vpdf-testdata"),
        "CreationDate" => Object::string_literal("D:20260101000000Z"),
    };
    match doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
        Ok(id) => {
            doc.objects.insert(id, Object::Dictionary(info));
        }
        Err(_) => {
            let id = doc.add_object(info);
            doc.trailer.set("Info", id);
        }
    }
    doc.max_id = doc.objects.keys().map(|k| k.0).max().unwrap_or(0);
}

fn add_outline_and_links(input: &Path, out: &Path, spec: &TextDocSpec) -> Result<(), String> {
    let mut doc = Document::load(input).map_err(|e| e.to_string())?;
    let pages: Vec<ObjectId> = doc.get_pages().values().copied().collect();
    let catalog_id = doc.trailer.get(b"Root").and_then(|o| o.as_reference()).map_err(|e| e.to_string())?;

    // Mục lục: mỗi chương một mục, mỗi chương có 2 mục con.
    let outlines_id = doc.new_object_id();
    let chapters = spec.pages.div_ceil(spec.pages_per_chapter);
    let chapter_ids: Vec<ObjectId> = (0..chapters).map(|_| doc.new_object_id()).collect();
    for (c, &cid) in chapter_ids.iter().enumerate() {
        let first_page = c * spec.pages_per_chapter;
        let title = format!("Chương {}: {}", c + 1, CHAPTERS[c % CHAPTERS.len()]);
        let top = if c == 0 { 842.0 - 60.0 } else { 842.0 - 100.0 };
        let kids: Vec<ObjectId> = (0..2).map(|_| doc.new_object_id()).collect();
        for (k, &kid) in kids.iter().enumerate() {
            let p = (first_page + 1 + k * (spec.pages_per_chapter / 2).max(1)).min(spec.pages - 1);
            let mut d = dictionary! {
                "Title" => utf16_title(&format!("Mục {}.{} — Đề mục số {}", c + 1, k + 1, k + 1)),
                "Parent" => cid,
                "Dest" => xyz(pages[p], 842.0 - 60.0),
            };
            if k == 0 {
                d.set("Next", kids[1]);
            } else {
                d.set("Prev", kids[0]);
            }
            doc.objects.insert(kid, Object::Dictionary(d));
        }
        let mut d = dictionary! {
            "Title" => utf16_title(&title),
            "Parent" => outlines_id,
            "Dest" => xyz(pages[first_page], top),
            "First" => kids[0],
            "Last" => kids[1],
            "Count" => -2,
        };
        if c > 0 {
            d.set("Prev", chapter_ids[c - 1]);
        }
        if c + 1 < chapters {
            d.set("Next", chapter_ids[c + 1]);
        }
        doc.objects.insert(cid, Object::Dictionary(d));
    }
    doc.objects.insert(
        outlines_id,
        Object::Dictionary(dictionary! {
            "Type" => "Outlines",
            "First" => chapter_ids[0],
            "Last" => *chapter_ids.last().unwrap(),
            "Count" => chapters as i64,
        }),
    );
    {
        let cat = doc.get_object_mut(catalog_id).and_then(|o| o.as_dict_mut()).map_err(|e| e.to_string())?;
        cat.set("Outlines", outlines_id);
        cat.set("PageMode", "UseOutlines");
    }

    // Link trên trang 1: tới trang `link_page` và tới https://example.com.
    let y_link = 842.0 - 60.0 - 22.0 - 26.0;
    let goto = doc.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Link",
        "Rect" => vec![54.into(), (y_link - 4.0).into(), 160.into(), (y_link + 12.0).into()],
        "Border" => vec![0.into(), 0.into(), 0.into()],
        "Dest" => xyz(pages[spec.link_page - 1], 842.0),
    });
    let y_uri = y_link - 18.0;
    let uri = doc.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Link",
        "Rect" => vec![120.into(), (y_uri - 4.0).into(), 230.into(), (y_uri + 12.0).into()],
        "Border" => vec![0.into(), 0.into(), 0.into()],
        "A" => dictionary! { "S" => "URI", "URI" => Object::string_literal("https://example.com") },
    });
    {
        let p1 = doc.get_object_mut(pages[0]).and_then(|o| o.as_dict_mut()).map_err(|e| e.to_string())?;
        p1.set("Annots", vec![Object::Reference(goto), Object::Reference(uri)]);
    }
    finalize(&mut doc);
    doc.save(out).map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tài liệu scan (lopdf, mỗi trang một ảnh JPEG xám, không có lớp chữ)

fn make_scan_doc(pages: usize, out: &Path) -> Result<(), String> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::new();
    let (w, h) = (1240u32, 1754u32); // A4 ở 150 dpi
    for i in 0..pages {
        let jpg = jpeg_scan(w, h, 77 + i as u64);
        let img = Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Image", "Width" => w as i64, "Height" => h as i64,
                "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8, "Filter" => "DCTDecode",
            },
            jpg,
        );
        let img_id = doc.add_object(img);
        let content = doc.add_object(Stream::new(dictionary! {}, b"q 595 0 0 842 0 0 cm /Im0 Do Q".to_vec()));
        let page = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => img_id } },
            "Contents" => content,
        });
        kids.push(Object::Reference(page));
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => pages as i64 }),
    );
    let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", cat);
    finalize(&mut doc);
    doc.save(out).map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Form AcroForm

fn make_form_doc(out: &Path) -> Result<(), String> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.new_object_id();
    let helv = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let content = doc.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 14 Tf 56 780 Td (Form mau - VPdf) Tj ET BT /F1 11 Tf 56 730 Td (Ho ten:) Tj ET BT /F1 11 Tf 56 690 Td (Dong y:) Tj ET"
            .to_vec(),
    ));
    let off = doc.add_object(Stream::new(dictionary! { "BBox" => vec![0.into(), 0.into(), 14.into(), 14.into()] }, b"".to_vec()));
    let yes = doc.add_object(Stream::new(
        dictionary! { "BBox" => vec![0.into(), 0.into(), 14.into(), 14.into()] },
        b"q 0 g 2 2 10 10 re f Q".to_vec(),
    ));
    let name = doc.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Widget", "FT" => "Tx", "T" => Object::string_literal("ho_ten"),
        "V" => Object::string_literal(""), "Rect" => vec![120.into(), 722.into(), 400.into(), 742.into()],
        "F" => 4, "P" => page_id, "DA" => Object::string_literal("/Helv 11 Tf 0 g"),
        "MK" => dictionary! { "BC" => vec![0.into(), 0.into(), 0.into()] },
    });
    let agree = doc.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Widget", "FT" => "Btn", "T" => Object::string_literal("dong_y"),
        "V" => "Off", "AS" => "Off", "Rect" => vec![120.into(), 684.into(), 134.into(), 698.into()],
        "F" => 4, "P" => page_id,
        "AP" => dictionary! { "N" => dictionary! { "Yes" => yes, "Off" => off } },
        "MK" => dictionary! { "BC" => vec![0.into(), 0.into(), 0.into()] },
    });
    doc.objects.insert(
        page_id,
        Object::Dictionary(dictionary! {
            "Type" => "Page", "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => helv } },
            "Contents" => content, "Annots" => vec![name.into(), agree.into()],
        }),
    );
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 }),
    );
    let cat = doc.add_object(dictionary! {
        "Type" => "Catalog", "Pages" => pages_id,
        "AcroForm" => dictionary! {
            "Fields" => vec![name.into(), agree.into()],
            "DA" => Object::string_literal("/Helv 0 Tf 0 g"),
            "DR" => dictionary! { "Font" => dictionary! { "Helv" => helv } },
            "NeedAppearances" => true,
        },
    });
    doc.trailer.set("Root", cat);
    finalize(&mut doc);
    doc.save(out).map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// File hỏng

fn make_broken_xref(src: &Path, out: &Path) -> Result<(), String> {
    let mut b = std::fs::read(src).map_err(|e| e.to_string())?;
    let nl = b.iter().position(|&c| c == b'\n').unwrap_or(8) + 1;
    // Chèn 64 byte rác sau dòng tiêu đề: mọi vị trí trong bảng xref đều lệch.
    let junk = [b'%'; 63].iter().copied().chain(std::iter::once(b'\n'));
    b.splice(nl..nl, junk);
    std::fs::write(out, b).map_err(|e| e.to_string())
}

fn make_not_pdf(out: &Path) -> Result<(), String> {
    std::fs::write(out, "Đây không phải là file PDF, chỉ là chữ thường có đuôi .pdf\n".repeat(20)).map_err(|e| e.to_string())
}

fn encrypt(src: &Path, out: &Path, user: &str, allow_extract: bool) -> Result<(), String> {
    let q = qpdf::Qpdf::open(src, None)?;
    q.write(
        out,
        ObjectStreams::Preserve,
        Some(&Encryption {
            user_password: user.into(),
            owner_password: OWNER_PASSWORD.into(),
            allow_extract,
            print: if allow_extract { PrintPermission::Full } else { PrintPermission::None },
        }),
    )
}

// ---------------------------------------------------------------------------

fn sha256_file(p: &Path) -> std::io::Result<(String, u64)> {
    use std::io::Read;
    let mut f = std::fs::File::open(p)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut n = 0u64;
    loop {
        let k = f.read(&mut buf)?;
        if k == 0 {
            break;
        }
        n += k as u64;
        h.update(&buf[..k]);
    }
    Ok((h.finalize().iter().map(|b| format!("{b:02x}")).collect(), n))
}

struct Plan {
    name: &'static str,
    purpose: &'static str,
    deterministic: bool,
}

const PLAN: &[Plan] = &[
    Plan { name: "text-500.pdf", purpose: "500 trang chữ tiếng Việt có ảnh minh họa, mục lục UTF-16, link nội bộ và URL", deterministic: true },
    Plan { name: "scan-500.pdf", purpose: "500 trang scan (ảnh JPEG xám), không có lớp chữ", deterministic: true },
    Plan { name: "small-vi.pdf", purpose: "40 trang chữ tiếng Việt có mục lục và link (mẫu nhỏ cho kiểm thử giao diện)", deterministic: true },
    Plan { name: "form.pdf", purpose: "Form AcroForm: trường chữ ho_ten và ô đánh dấu dong_y", deterministic: true },
    Plan { name: "password.pdf", purpose: "Mật khẩu mở AES-256 R6 (matkhau123), mật khẩu chủ sở hữu chusohuu456", deterministic: false },
    Plan { name: "no-copy.pdf", purpose: "Mở không cần mật khẩu nhưng cấm sao chép và in; mật khẩu chủ sở hữu chusohuu456", deterministic: false },
    Plan { name: "broken-xref.pdf", purpose: "Bảng xref lệch vị trí nhưng các trang vẫn khôi phục được", deterministic: true },
    Plan { name: "not-a-pdf.pdf", purpose: "Đuôi .pdf nhưng nội dung không phải PDF", deterministic: true },
];

fn generate(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let t = std::time::Instant::now();
    let step = |name: &str| eprintln!("[{:>6.1}s] {name}", t.elapsed().as_secs_f32());
    step("small-vi.pdf");
    make_text_doc(
        &TextDocSpec { pages: 40, pages_per_chapter: 5, image_quality: 60, image_size: (480, 200), link_page: 30 },
        &dir.join("small-vi.pdf"),
    )?;
    step("text-500.pdf");
    make_text_doc(
        &TextDocSpec { pages: 500, pages_per_chapter: 25, image_quality: 80, image_size: (900, 380), link_page: 30 },
        &dir.join("text-500.pdf"),
    )?;
    step("scan-500.pdf");
    make_scan_doc(500, &dir.join("scan-500.pdf"))?;
    step("form.pdf");
    make_form_doc(&dir.join("form.pdf"))?;
    step("password.pdf / no-copy.pdf");
    encrypt(&dir.join("small-vi.pdf"), &dir.join("password.pdf"), USER_PASSWORD, true)?;
    encrypt(&dir.join("small-vi.pdf"), &dir.join("no-copy.pdf"), "", false)?;
    step("broken-xref.pdf / not-a-pdf.pdf");
    make_broken_xref(&dir.join("small-vi.pdf"), &dir.join("broken-xref.pdf"))?;
    make_not_pdf(&dir.join("not-a-pdf.pdf"))?;
    step("xong");
    Ok(())
}

fn write_manifest(dir: &Path) -> Result<Manifest, String> {
    let mut m = Manifest {
        version: 1,
        note: "Mã băm SHA-256 của bộ tài liệu tham chiếu. File deterministic=false (mã hóa) có muối ngẫu nhiên; \
               khi tạo tại chỗ chúng được kiểm tra bằng cách mở với đúng mật khẩu."
            .into(),
        files: Vec::new(),
    };
    for p in PLAN {
        let (sha, bytes) = sha256_file(&dir.join(p.name)).map_err(|e| format!("{}: {e}", p.name))?;
        m.files.push(Entry { name: p.name.into(), sha256: sha, bytes, deterministic: p.deterministic, purpose: p.purpose.into() });
    }
    Ok(m)
}

/// Kiểm tra theo MANIFEST. `strict` = cả file mã hóa cũng phải khớp mã băm (bộ file tải về).
fn verify(dir: &Path, strict: bool) -> Result<(), String> {
    let m: Manifest = serde_json::from_slice(&std::fs::read(manifest_path()).map_err(|e| format!("thiếu MANIFEST: {e}"))?)
        .map_err(|e| e.to_string())?;
    let mut bad = Vec::new();
    for e in &m.files {
        let p = dir.join(&e.name);
        match sha256_file(&p) {
            Err(err) => bad.push(format!("{}: {err}", e.name)),
            Ok((sha, bytes)) => {
                if sha == e.sha256 && bytes == e.bytes {
                    println!("OK      {:<16} {:>11} byte  {}", e.name, bytes, &sha[..16]);
                } else if !e.deterministic && !strict {
                    // Kiểm tra chức năng thay cho mã băm.
                    let pw = if e.name == "password.pdf" { Some(USER_PASSWORD) } else { None };
                    match qpdf::check_file(&p, pw) {
                        Ok(r) if r.ok && r.pages > 0 => {
                            println!("OK*     {:<16} {:>11} byte  (mã hóa, mở được: {} trang)", e.name, bytes, r.pages)
                        }
                        other => bad.push(format!("{}: không mở được bằng mật khẩu mẫu: {other:?}", e.name)),
                    }
                } else {
                    bad.push(format!("{}: mã băm {} ≠ {}", e.name, &sha[..16], &e.sha256[..16]));
                }
            }
        }
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!("Bộ tài liệu tham chiếu không khớp MANIFEST:\n  {}", bad.join("\n  ")))
    }
}

fn fetch(dir: &Path) -> Result<(), String> {
    match std::env::var("VPDF_TESTDATA_URL") {
        Ok(base) if !base.is_empty() => {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            for p in PLAN {
                let url = format!("{}/{}", base.trim_end_matches('/'), p.name);
                eprintln!("tải {url}");
                let st = std::process::Command::new("curl")
                    .args(["-fsSL", "--retry", "3", "-o"])
                    .arg(dir.join(p.name))
                    .arg(&url)
                    .status()
                    .map_err(|e| format!("không chạy được curl: {e}"))?;
                if !st.success() {
                    return Err(format!("tải {url} thất bại"));
                }
            }
            verify(dir, true)
        }
        _ => {
            eprintln!("Chưa đặt VPDF_TESTDATA_URL: tạo bộ tài liệu tại chỗ.");
            generate(dir)?;
            verify(dir, false)
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let dir = args.iter().position(|a| a == "--dir").and_then(|i| args.get(i + 1)).map(PathBuf::from).unwrap_or_else(default_dir);
    let r = match cmd {
        "generate" => generate(&dir).and_then(|_| {
            let m = write_manifest(&dir)?;
            std::fs::write(manifest_path(), serde_json::to_vec_pretty(&m).unwrap()).map_err(|e| e.to_string())?;
            for f in &m.files {
                println!("{:<16} {:>11} byte  {}", f.name, f.bytes, f.sha256);
            }
            Ok(())
        }),
        "verify" => verify(&dir, args.iter().any(|a| a == "--strict")),
        "fetch" => fetch(&dir),
        _ => {
            println!("vpdf-testdata generate|fetch|verify [--dir DIR] [--strict]");
            Ok(())
        }
    };
    if let Err(e) = r {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
