//! Logins from other password managers' exports: Bitwarden (CSV or JSON),
//! Chrome / Edge / Brave / Opera, Firefox, 1Password, KeePass / KeePassXC,
//! LastPass, and any CSV whose columns have recognisable names. The file is
//! only read here; nothing leaves the PC.

use super::EntryKind;

/// One login (or secure note) read from an export.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Imported {
    pub kind: EntryKind,
    pub title: String,
    pub username: String,
    pub password: String,
    pub url: String,
    pub notes: String,
    pub totp: String,
    pub tags: Vec<String>,
    pub favorite: bool,
}

/// Read an export (CSV, or Bitwarden's JSON).
pub fn parse(bytes: &[u8]) -> Result<Vec<Imported>, String> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_start_matches('\u{feff}');
    if text.trim_start().starts_with('{') {
        return bitwarden_json(text);
    }
    let rows = csv(text);
    let (header, rows) = rows.split_first().ok_or("The file is empty.")?;
    let names: Vec<String> = header.iter().map(|h| h.trim().to_lowercase()).collect();
    let find = |keys: &[&str]| names.iter().position(|n| keys.contains(&n.as_str()));
    let title = find(&["name", "title", "account"]);
    let url = find(&["url", "uri", "login_uri", "website", "web site", "login url", "origin"]);
    let user = find(&["username", "login_username", "login name", "user name", "user", "login", "email"]);
    let pass = find(&["password", "login_password"]);
    let notes = find(&["notes", "note", "comments", "extra"]);
    let totp = find(&["totp", "login_totp", "otpauth", "otp", "one-time password"]);
    let kind = find(&["type"]);
    let fav = find(&["favorite", "fav"]);
    let group = find(&["folder", "grouping", "group", "tags"]);
    if pass.is_none() || (title.is_none() && url.is_none() && user.is_none()) {
        return Err("This file doesn't look like a password export (no password column).".into());
    }
    let get = |r: &[String], i: Option<usize>| i.and_then(|i| r.get(i)).map(|s| s.trim().to_string()).unwrap_or_default();
    let mut out = Vec::new();
    for r in rows {
        let t = get(r, kind).to_lowercase();
        let kind = match t.as_str() {
            "" | "login" => EntryKind::Login,
            "note" | "securenote" | "secure note" => EntryKind::Note,
            _ => continue, // cards and identities have other fields
        };
        let mut e = Imported { kind, title: get(r, title), username: get(r, user), password: get(r, pass), url: get(r, url), notes: get(r, notes), totp: get(r, totp), ..Default::default() };
        e.favorite = matches!(get(r, fav).to_lowercase().as_str(), "1" | "true" | "yes");
        e.tags = get(r, group).split(['/', '\\', ',']).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        if e.title.is_empty() {
            e.title = host(&e.url).unwrap_or_else(|| e.username.clone());
        }
        if e.password.is_empty() && e.notes.is_empty() && e.username.is_empty() {
            continue;
        }
        out.push(e);
    }
    Ok(out)
}

/// The site's name in a URL ("https://www.github.com/login" → "github.com").
fn host(url: &str) -> Option<String> {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let h = rest.split(['/', '?', '#', ':']).next()?.trim_start_matches("www.");
    (!h.is_empty()).then(|| h.to_string())
}

fn bitwarden_json(text: &str) -> Result<Vec<Imported>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|_| "This JSON file isn't a Bitwarden export.".to_string())?;
    if v["encrypted"].as_bool() == Some(true) {
        return Err("This Bitwarden export is encrypted: export it again as unencrypted JSON or CSV.".into());
    }
    let items = v["items"].as_array().ok_or("This JSON file isn't a Bitwarden export.")?;
    let folders: std::collections::HashMap<String, String> = v["folders"].as_array().into_iter().flatten().filter_map(|f| Some((f["id"].as_str()?.to_string(), f["name"].as_str()?.to_string()))).collect();
    let s = |x: &serde_json::Value| x.as_str().unwrap_or_default().trim().to_string();
    let mut out = Vec::new();
    for it in items {
        let kind = match it["type"].as_i64() {
            Some(1) => EntryKind::Login,
            Some(2) => EntryKind::Note,
            _ => continue,
        };
        let login = &it["login"];
        let url = login["uris"].as_array().and_then(|u| u.first()).map(|u| s(&u["uri"])).unwrap_or_default();
        let mut e = Imported { kind, title: s(&it["name"]), username: s(&login["username"]), password: s(&login["password"]), url, notes: s(&it["notes"]), totp: s(&login["totp"]), favorite: it["favorite"].as_bool().unwrap_or(false), ..Default::default() };
        if let Some(f) = it["folderId"].as_str().and_then(|id| folders.get(id)) {
            e.tags.push(f.clone());
        }
        if e.title.is_empty() {
            e.title = host(&e.url).unwrap_or_else(|| e.username.clone());
        }
        out.push(e);
    }
    Ok(out)
}

/// RFC 4180 CSV: quoted fields with "" inside, commas and line breaks in quotes.
fn csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let (mut row, mut field, mut quoted) = (Vec::new(), String::new(), false);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            (true, '"') => quoted = false,
            (true, c) => field.push(c),
            (false, '"') if field.is_empty() => quoted = true,
            (false, ',') => row.push(std::mem::take(&mut field)),
            (false, '\r') => {}
            (false, '\n') => {
                row.push(std::mem::take(&mut field));
                if row.iter().any(|f| !f.is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            (false, c) => field.push(c),
        }
    }
    row.push(field);
    if row.iter().any(|f| !f.is_empty()) {
        rows.push(row);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_csv() {
        let e = parse(b"name,url,username,password,note\r\ngithub.com,https://github.com/login,me,\"p,a\"\"ss\",\r\n,https://www.example.com/,you,x,multi\nline\r\n").unwrap();
        // The second row's note ran onto another line without quotes: it's a new (dropped) row.
        assert_eq!(e[0].title, "github.com");
        assert_eq!(e[0].password, "p,a\"ss");
        assert_eq!(e[1].title, "example.com");
    }

    #[test]
    fn bitwarden_csv_and_json() {
        let csv = "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp\nWork,1,login,Mail,,,,https://mail.example,me@example.com,pw,JBSWY3DPEHPK3PXP\n,,card,Visa,,,,,,,\n,,note,Wifi codes,1234,,,,,,\n";
        let e = parse(csv.as_bytes()).unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!((e[0].title.as_str(), e[0].username.as_str(), e[0].totp.as_str(), e[0].favorite, e[0].tags.clone()), ("Mail", "me@example.com", "JBSWY3DPEHPK3PXP", true, vec!["Work".to_string()]));
        assert_eq!(e[1].kind, EntryKind::Note);
        let json = r#"{"encrypted":false,"folders":[{"id":"f1","name":"Games"}],"items":[{"type":1,"name":"Steam","folderId":"f1","favorite":false,"notes":null,"login":{"username":"gamer","password":"pw2","totp":null,"uris":[{"uri":"https://store.steampowered.com"}]}},{"type":3,"name":"Card"}]}"#;
        let e = parse(json.as_bytes()).unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!((e[0].title.as_str(), e[0].password.as_str(), e[0].tags.clone()), ("Steam", "pw2", vec!["Games".to_string()]));
        assert!(parse(br#"{"encrypted":true,"items":[]}"#).unwrap_err().contains("encrypted"));
    }

    #[test]
    fn keepass_and_firefox_and_junk() {
        let kp = parse("\"Account\",\"Login Name\",\"Password\",\"Web Site\",\"Comments\"\n\"Bank\",\"me\",\"s3cret\",\"https://bank.example\",\"\"\n".as_bytes()).unwrap();
        assert_eq!((kp[0].title.as_str(), kp[0].username.as_str(), kp[0].url.as_str()), ("Bank", "me", "https://bank.example"));
        let ff = parse(b"\"url\",\"username\",\"password\",\"httpRealm\"\n\"https://news.example\",\"u\",\"p\",\"\"\n").unwrap();
        assert_eq!(ff[0].title, "news.example");
        assert!(parse(b"a,b,c\n1,2,3\n").is_err());
        assert!(parse(b"").is_err());
    }
}
