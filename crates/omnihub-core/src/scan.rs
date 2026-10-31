//! The phone as a scanner: pages photographed and straightened on the phone
//! arrive here as JPEGs, become a PDF (or a picture for one page) in your
//! scans folder, and Windows' own text recognition reads them. Receipts and
//! business cards are read into fields (shop, date, total; name, phone,
//! email…) for a note, and a card also becomes a contact file (.vcf).
//! Nothing leaves the PC.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    #[default]
    Document,
    Receipt,
    Card,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Target {
    /// The scans folder (a PDF, or a picture for one page, plus the text).
    #[default]
    Folder,
    /// A note with the text and fields (the file is still kept).
    Notes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Scanned {
    pub title: String,
    /// What was saved (PDF or picture, text, contact card).
    pub files: Vec<String>,
    pub text: String,
    /// Read from a receipt or card ("Total" → "12.40").
    pub fields: Vec<(String, String)>,
    pub note_id: Option<String>,
}

// ---------- JPEG and PDF ----------

/// Width and height from a JPEG's frame header.
pub fn jpeg_size(b: &[u8]) -> Option<(u32, u32)> {
    if b.len() < 4 || b[0] != 0xFF || b[1] != 0xD8 {
        return None;
    }
    let mut i = 2;
    while i + 9 < b.len() {
        if b[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = b[i + 1];
        // Standalone markers have no length.
        if marker == 0xD8 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            i += 2;
            continue;
        }
        let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        // SOF0–SOF15, except DHT (C4), JPG (C8) and DAC (CC).
        if (0xC0..=0xCF).contains(&marker) && ![0xC4, 0xC8, 0xCC].contains(&marker) {
            let h = u16::from_be_bytes([b[i + 5], b[i + 6]]) as u32;
            let w = u16::from_be_bytes([b[i + 7], b[i + 8]]) as u32;
            return (w > 0 && h > 0).then_some((w, h));
        }
        i += 2 + len;
    }
    None
}

/// A PDF with one JPEG per page, each page sized to its picture (at 150 dpi,
/// so an A4 photo comes out about A4).
pub fn pdf_from_jpegs(pages: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets: Vec<usize> = Vec::new();
    // Objects: 1 catalog, 2 pages, then per page: page, image, contents.
    let n = pages.len();
    if n == 0 {
        return Err("no pages".into());
    }
    let obj = |out: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
        offsets.push(out.len());
        let id = offsets.len();
        out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    };
    obj(&mut out, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 3 + i * 3)).collect();
    obj(&mut out, &mut offsets, format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).as_bytes());
    for (i, jpg) in pages.iter().enumerate() {
        let (w, h) = jpeg_size(jpg).ok_or_else(|| format!("page {} isn't a JPEG picture", i + 1))?;
        let (pw, ph) = (w as f64 * 72.0 / 150.0, h as f64 * 72.0 / 150.0);
        let (page_id, img_id, content_id) = (3 + i * 3, 4 + i * 3, 5 + i * 3);
        obj(&mut out, &mut offsets, format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {pw:.2} {ph:.2}] /Resources << /XObject << /Im{page_id} {img_id} 0 R >> >> /Contents {content_id} 0 R >>").as_bytes());
        let mut img = format!("<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n", jpg.len()).into_bytes();
        img.extend_from_slice(jpg);
        img.extend_from_slice(b"\nendstream");
        obj(&mut out, &mut offsets, &img);
        let draw = format!("q {pw:.2} 0 0 {ph:.2} 0 0 cm /Im{page_id} Do Q");
        obj(&mut out, &mut offsets, format!("<< /Length {} >>\nstream\n{draw}\nendstream", draw.len()).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for o in &offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len() + 1).as_bytes());
    Ok(out)
}

// ---------- reading receipts and cards ----------

fn lines(text: &str) -> Vec<String> {
    text.lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|l| !l.is_empty()).collect()
}

/// Amounts on a line ("12.40", "1,299.00", "8,50").
fn amounts(line: &str) -> Vec<f64> {
    let mut out = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == ',') {
                i += 1;
            }
            let raw: String = chars[start..i].iter().collect::<String>().trim_end_matches(['.', ',']).to_string();
            // The last separator followed by exactly two digits is the decimal point.
            let norm = match raw.rfind(['.', ',']) {
                Some(p) if raw.len() - p == 3 => format!("{}.{}", raw[..p].replace(['.', ','], ""), &raw[p + 1..]),
                _ => continue,
            };
            if let Ok(v) = norm.parse::<f64>() {
                out.push(v);
            }
        } else {
            i += 1;
        }
    }
    out
}

/// A date in the text, as written ("2026-10-31", "31/10/2026", "Oct 31, 2026").
pub fn find_date(text: &str) -> Option<String> {
    let months = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
    for raw in text.split(|c: char| c.is_whitespace() || c == '|') {
        let w = raw.trim_matches(|c: char| !c.is_alphanumeric());
        let parts: Vec<&str> = w.split(['/', '-', '.']).collect();
        if parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.len() <= 4 && p.chars().all(|c| c.is_ascii_digit())) {
            let nums: Vec<u32> = parts.iter().filter_map(|p| p.parse().ok()).collect();
            let ok = if parts[0].len() == 4 { (1..=12).contains(&nums[1]) && (1..=31).contains(&nums[2]) } else { nums[0] <= 31 && nums[1] <= 31 && (parts[2].len() == 2 || parts[2].len() == 4) && nums[0] > 0 && nums[1] > 0 };
            if ok {
                return Some(w.to_string());
            }
        }
    }
    let low = text.to_lowercase();
    for (i, m) in months.iter().enumerate() {
        if let Some(p) = low.find(m) {
            // "Oct 31, 2026" or "31 Oct 2026": take the words around it.
            let start = low[..p].rfind('\n').map_or(0, |x| x + 1);
            let line = &text[start..];
            let line = line.lines().next().unwrap_or("");
            let words: Vec<&str> = line.split_whitespace().collect();
            if let Some(k) = words.iter().position(|w| w.to_lowercase().starts_with(months[i])) {
                let lo = k.saturating_sub(1);
                let hi = (k + 3).min(words.len());
                let piece: Vec<&str> = words[lo..hi].iter().copied().filter(|w| w.chars().any(|c| c.is_ascii_digit()) || w.to_lowercase().starts_with(months[i])).collect();
                if piece.iter().any(|w| w.chars().any(|c| c.is_ascii_digit())) {
                    return Some(piece.join(" ").trim_end_matches(',').to_string());
                }
            }
        }
    }
    None
}

/// Shop, date, total and tax from a receipt's text.
pub fn parse_receipt(text: &str) -> Vec<(String, String)> {
    let ls = lines(text);
    let mut out = Vec::new();
    // The shop: the first line that is mostly letters.
    if let Some(shop) = ls.iter().find(|l| l.chars().filter(|c| c.is_alphabetic()).count() >= 3 && l.chars().filter(|c| c.is_ascii_digit()).count() * 2 < l.len()) {
        out.push(("Shop".to_string(), shop.clone()));
    }
    if let Some(d) = find_date(text) {
        out.push(("Date".into(), d));
    }
    let total_words = ["grand total", "total due", "amount due", "balance due", "total", "to pay", "importe", "gesamt", "montant", "totale"];
    let total = total_words
        .iter()
        .find_map(|w| ls.iter().rev().find(|l| l.to_lowercase().contains(w) && !l.to_lowercase().contains("subtotal") && !l.to_lowercase().contains("sub total")).and_then(|l| amounts(l).last().copied()))
        .or_else(|| ls.iter().flat_map(|l| amounts(l)).fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.max(v)))));
    if let Some(t) = total {
        let currency = ["€", "$", "£", "¥", "₹", "CHF", "MXN", "USD", "EUR"].iter().find(|c| text.contains(*c)).map(|c| c.to_string()).unwrap_or_default();
        out.push(("Total".into(), format!("{currency}{t:.2}").trim().to_string()));
    }
    if let Some(tax) = ls.iter().find(|l| ["tax", "vat", "iva", "mwst", "tva", "gst"].iter().any(|w| l.to_lowercase().split(|c: char| !c.is_alphabetic()).any(|x| x == *w))).and_then(|l| amounts(l).last().copied()) {
        out.push(("Tax".into(), format!("{tax:.2}")));
    }
    out
}

fn email_in(l: &str) -> Option<String> {
    l.split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '@' && c != '.' && c != '_' && c != '-' && c != '+')).find(|w| {
        let at = w.find('@');
        at.is_some_and(|a| a > 0 && w[a + 1..].contains('.') && !w.ends_with('.'))
    }).map(str::to_string)
}

fn phone_in(l: &str) -> Option<String> {
    let digits = l.chars().filter(|c| c.is_ascii_digit()).count();
    if !(7..=15).contains(&digits) || email_in(l).is_some() {
        return None;
    }
    // From the first digit or '+' to the last digit.
    let start = l.find(|c: char| c.is_ascii_digit() || c == '+' || c == '(')?;
    let end = l.rfind(|c: char| c.is_ascii_digit())? + 1;
    let p = l[start..end].trim();
    p.chars().all(|c| c.is_ascii_digit() || " +-().".contains(c)).then(|| p.to_string())
}

fn website_in(l: &str) -> Option<String> {
    l.split_whitespace().find(|w| {
        let w = w.to_lowercase();
        !w.contains('@') && (w.starts_with("www.") || w.starts_with("http") || [".com", ".net", ".org", ".io", ".co", ".dev", ".app", ".mx", ".es", ".uk", ".de", ".fr"].iter().any(|t| w.trim_end_matches('/').ends_with(t)))
    }).map(|w| w.trim_end_matches(['/', ',']).to_string())
}

const TITLES: &[&str] = &["manager", "director", "engineer", "developer", "designer", "ceo", "cto", "cfo", "coo", "founder", "president", "consultant", "sales", "marketing", "officer", "lead", "head", "partner", "owner", "specialist", "analyst", "architect", "agent", "attorney", "lawyer", "doctor", "dr.", "advisor", "associate", "assistant", "coordinator", "representative", "gerente", "director general", "ingeniero"];
const COMPANY: &[&str] = &["inc", "llc", "ltd", "gmbh", "s.a.", "sa de cv", "corp", "co.", "company", "group", "studio", "labs", "solutions", "technologies", "services", "agency", "bank"];

/// Name, title, company, phones, email and website from a business card's text.
pub fn parse_card(text: &str) -> Vec<(String, String)> {
    let ls = lines(text);
    let mut out: Vec<(String, String)> = Vec::new();
    let mut used = vec![false; ls.len()];
    for (i, l) in ls.iter().enumerate() {
        if let Some(e) = email_in(l) {
            out.push(("Email".into(), e));
            used[i] = true;
        } else if let Some(p) = phone_in(l) {
            out.push(("Phone".into(), p));
            used[i] = true;
        } else if let Some(w) = website_in(l) {
            out.push(("Website".into(), w));
            used[i] = true;
        }
    }
    let low = |l: &str| l.to_lowercase();
    if let Some(i) = (0..ls.len()).find(|&i| !used[i] && COMPANY.iter().any(|c| low(&ls[i]).split(|x: char| x.is_whitespace() || x == ',').any(|w| w == c.trim_end_matches('.') || w == *c))) {
        out.push(("Company".into(), ls[i].clone()));
        used[i] = true;
    }
    if let Some(i) = (0..ls.len()).find(|&i| !used[i] && TITLES.iter().any(|t| low(&ls[i]).contains(t))) {
        out.push(("Title".into(), ls[i].clone()));
        used[i] = true;
    }
    // The name: 2–4 words, letters only, each starting with a capital.
    if let Some(i) = (0..ls.len()).find(|&i| {
        let words: Vec<&str> = ls[i].split_whitespace().collect();
        !used[i] && (2..=4).contains(&words.len()) && words.iter().all(|w| w.chars().next().is_some_and(char::is_uppercase) && w.chars().all(|c| c.is_alphabetic() || c == '.' || c == '-' || c == '\''))
    }) {
        out.insert(0, ("Name".into(), ls[i].clone()));
        used[i] = true;
    }
    if !out.iter().any(|(k, _)| k == "Company") {
        if let Some(i) = (0..ls.len()).find(|&i| !used[i] && ls[i].chars().filter(|c| c.is_alphabetic()).count() >= 3) {
            out.push(("Company".into(), ls[i].clone()));
        }
    }
    out
}

/// A contact file (vCard 3.0) for the card's fields.
pub fn vcard(fields: &[(String, String)]) -> String {
    let get = |k: &str| fields.iter().find(|(f, _)| f == k).map(|(_, v)| v.replace([';', ','], " "));
    let esc = |s: String| s.replace('\n', " ");
    let mut v = String::from("BEGIN:VCARD\r\nVERSION:3.0\r\n");
    let name = get("Name").unwrap_or_else(|| get("Company").unwrap_or_else(|| "Contact".into()));
    let mut parts: Vec<&str> = name.split_whitespace().collect();
    let last = if parts.len() > 1 { parts.pop().unwrap_or("") } else { "" };
    v.push_str(&format!("N:{};{};;;\r\nFN:{}\r\n", esc(last.to_string()), esc(parts.join(" ")), esc(name.clone())));
    if let Some(c) = get("Company") {
        v.push_str(&format!("ORG:{}\r\n", esc(c)));
    }
    if let Some(t) = get("Title") {
        v.push_str(&format!("TITLE:{}\r\n", esc(t)));
    }
    for (k, val) in fields {
        match k.as_str() {
            "Phone" => v.push_str(&format!("TEL;TYPE=WORK,VOICE:{}\r\n", esc(val.clone()))),
            "Email" => v.push_str(&format!("EMAIL;TYPE=INTERNET:{}\r\n", esc(val.clone()))),
            "Website" => v.push_str(&format!("URL:{}\r\n", esc(val.clone()))),
            _ => {}
        }
    }
    v.push_str("END:VCARD\r\n");
    v
}

/// A file name from a title: what Windows allows, and not empty.
pub fn file_title(title: &str, kind: Kind) -> String {
    let base = if title.trim().is_empty() {
        match kind {
            Kind::Document => "Scan",
            Kind::Receipt => "Receipt",
            Kind::Card => "Business card",
        }
        .to_string()
    } else {
        // A title, not a path: slashes stay part of the name.
        crate::remote::guest::safe_name(&title.trim().replace(['/', '\\'], "_"))
    };
    format!("{} {}", base, chrono::Local::now().format("%Y-%m-%d %H-%M"))
}

/// Save the pages in `dir`: a PDF for several, the picture for one. Returns the paths.
pub fn save_pages(dir: &Path, name: &str, pages: &[Vec<u8>]) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("couldn't make {}: {e}", dir.display()))?;
    if pages.len() == 1 {
        let p = crate::remote::guest::unique_path(dir, &format!("{name}.jpg"));
        std::fs::write(&p, &pages[0]).map_err(|e| e.to_string())?;
        Ok(p)
    } else {
        let p = crate::remote::guest::unique_path(dir, &format!("{name}.pdf"));
        std::fs::write(&p, pdf_from_jpegs(pages)?).map_err(|e| e.to_string())?;
        Ok(p)
    }
}

/// A note's body: the fields first, then the text.
pub fn note_body(kind: Kind, fields: &[(String, String)], text: &str, file: &Path) -> String {
    let mut b = String::new();
    for (k, v) in fields {
        b.push_str(&format!("**{k}:** {v}\n"));
    }
    if !fields.is_empty() {
        b.push('\n');
    }
    let label = match kind {
        Kind::Document => "Scanned",
        Kind::Receipt => "Receipt",
        Kind::Card => "Card",
    };
    b.push_str(&format!("{label}: `{}`\n", file.display()));
    if !text.trim().is_empty() {
        b.push_str("\n---\n\n");
        b.push_str(text.trim());
        b.push('\n');
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb([240, 240, 240]));
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 80).encode_image(&img).unwrap();
        out
    }

    #[test]
    fn pdf_and_sizes() {
        let a = tiny_jpeg(300, 450);
        assert_eq!(jpeg_size(&a), Some((300, 450)));
        assert_eq!(jpeg_size(b"not a jpeg"), None);
        let pdf = pdf_from_jpegs(&[a.clone(), tiny_jpeg(200, 100)]).unwrap();
        let s = String::from_utf8_lossy(&pdf);
        assert!(s.starts_with("%PDF-1.4") && s.trim_end().ends_with("%%EOF"));
        assert!(s.contains("/Count 2") && s.contains("/Width 300 /Height 450"));
        // The cross-reference table points at each object (byte offsets).
        let xref = pdf.windows(6).rposition(|w| w == b"\nxref\n").unwrap() + 1;
        let table = String::from_utf8(pdf[xref..].to_vec()).unwrap();
        for (n, line) in table.lines().skip(3).take(8).enumerate() {
            let off: usize = line[..10].parse().unwrap();
            assert!(pdf[off..].starts_with(format!("{} 0 obj", n + 1).as_bytes()), "object {}", n + 1);
        }
        assert!(pdf_from_jpegs(&[]).is_err());
        let dir = tempfile::tempdir().unwrap();
        let one = save_pages(dir.path(), "Scan", std::slice::from_ref(&a)).unwrap();
        assert_eq!(one.extension().unwrap(), "jpg");
        let two = save_pages(dir.path(), "Scan", &[a.clone(), a]).unwrap();
        assert_eq!(two.extension().unwrap(), "pdf");
    }

    #[test]
    fn reads_receipts() {
        let text = "CORNER CAFE\n123 Main St\n31/10/2026 08:14\nLatte 4.50\nCroissant 3,20\nSubtotal 7.70\nVAT 21% 1.62\nTOTAL € 9.32\nThank you";
        let f = parse_receipt(text);
        let get = |k: &str| f.iter().find(|(x, _)| x == k).map(|(_, v)| v.clone());
        assert_eq!(get("Shop").as_deref(), Some("CORNER CAFE"));
        assert_eq!(get("Date").as_deref(), Some("31/10/2026"));
        assert_eq!(get("Total").as_deref(), Some("€9.32"));
        assert_eq!(get("Tax").as_deref(), Some("1.62"));
        assert_eq!(find_date("Paid on Oct 31, 2026 at noon").as_deref(), Some("Oct 31, 2026"));
        assert_eq!(find_date("2026-10-31").as_deref(), Some("2026-10-31"));
        assert_eq!(amounts("1,299.00 and 8,50"), vec![1299.0, 8.5]);
    }

    #[test]
    fn reads_business_cards() {
        let text = "Jordan Rivera\nSenior Product Designer\nNorthwind Labs Inc.\n+1 (555) 010-2030\njordan@northwind.example\nwww.northwind.example";
        let f = parse_card(text);
        let get = |k: &str| f.iter().find(|(x, _)| x == k).map(|(_, v)| v.clone());
        assert_eq!(get("Name").as_deref(), Some("Jordan Rivera"));
        assert_eq!(get("Title").as_deref(), Some("Senior Product Designer"));
        assert_eq!(get("Company").as_deref(), Some("Northwind Labs Inc."));
        assert_eq!(get("Phone").as_deref(), Some("+1 (555) 010-2030"));
        assert_eq!(get("Email").as_deref(), Some("jordan@northwind.example"));
        assert_eq!(get("Website").as_deref(), Some("www.northwind.example"));
        let v = vcard(&f);
        assert!(v.contains("FN:Jordan Rivera") && v.contains("N:Rivera;Jordan;;;") && v.contains("TEL;TYPE=WORK,VOICE:+1 (555) 010-2030") && v.ends_with("END:VCARD\r\n"));
        assert!(file_title("", Kind::Card).starts_with("Business card 20"));
        assert!(file_title("Lease p1/2", Kind::Document).starts_with("Lease p1_2 "));
    }
}
