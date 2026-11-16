//! Guest uploads: a QR code anyone nearby can scan to send you files,
//! without pairing their phone. Each link works for a short time and a set
//! amount of data, only uploads (a guest sees nothing on the PC), and lands
//! in its own folder (Incoming\Guests\<date>). You can end it any time.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::Serialize;

/// The most a guest may send in one link (and the default).
pub const MAX_BYTES: u64 = 20 << 30;
pub const DEFAULT_BYTES: u64 = 4 << 30;
pub const MAX_FILES: u32 = 500;

#[derive(Debug, Clone)]
struct Link {
    label: String,
    created: i64,
    expires: Instant,
    expires_at: i64,
    max_bytes: u64,
    used_bytes: u64,
    files: u32,
    /// Bytes promised to uploads still running (so two at once can't overshoot).
    reserved: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GuestLink {
    pub code: String,
    pub label: String,
    pub created: i64,
    /// Unix seconds.
    pub expires_at: i64,
    pub max_bytes: u64,
    pub used_bytes: u64,
    pub files: u32,
}

#[derive(Default)]
pub struct Guests {
    links: Mutex<HashMap<String, Link>>,
}

fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

impl Guests {
    /// A new link for `minutes` (5–240) and up to `max_bytes`.
    pub fn create(&self, label: &str, minutes: u32, max_bytes: u64) -> GuestLink {
        let minutes = minutes.clamp(5, 240);
        let code = crate::remote::auth::random_token(18);
        let label: String = label.trim().chars().take(40).collect();
        let link = Link {
            label: if label.is_empty() { "Guest".into() } else { label },
            created: now_unix(),
            expires: Instant::now() + Duration::from_secs(minutes as u64 * 60),
            expires_at: now_unix() + minutes as i64 * 60,
            max_bytes: max_bytes.clamp(1 << 20, MAX_BYTES),
            used_bytes: 0,
            files: 0,
            reserved: 0,
        };
        let mut links = self.links.lock();
        links.retain(|_, l| l.expires > Instant::now());
        links.insert(code.clone(), link);
        Self::view(&code, &links[&code])
    }

    fn view(code: &str, l: &Link) -> GuestLink {
        GuestLink { code: code.to_string(), label: l.label.clone(), created: l.created, expires_at: l.expires_at, max_bytes: l.max_bytes, used_bytes: l.used_bytes, files: l.files }
    }

    pub fn list(&self) -> Vec<GuestLink> {
        let mut links = self.links.lock();
        links.retain(|_, l| l.expires > Instant::now());
        let mut v: Vec<GuestLink> = links.iter().map(|(c, l)| Self::view(c, l)).collect();
        v.sort_by_key(|l| l.created);
        v
    }

    pub fn get(&self, code: &str) -> Option<GuestLink> {
        let links = self.links.lock();
        links.get(code).filter(|l| l.expires > Instant::now()).map(|l| Self::view(code, l))
    }

    pub fn stop(&self, code: &str) -> bool {
        self.links.lock().remove(code).is_some()
    }

    pub fn stop_all(&self) {
        self.links.lock().clear();
    }

    /// Room for a file of `size` bytes: reserved until `finish`/`release`.
    pub fn reserve(&self, code: &str, size: u64) -> Result<(), String> {
        let mut links = self.links.lock();
        let l = links.get_mut(code).filter(|l| l.expires > Instant::now()).ok_or("This link has ended. Ask for a new QR code.")?;
        if l.files >= MAX_FILES {
            return Err("That's as many files as this link takes.".into());
        }
        if l.used_bytes + l.reserved + size > l.max_bytes {
            return Err(format!("That's more than this link takes ({} left).", crate::storage::format_bytes(l.max_bytes.saturating_sub(l.used_bytes + l.reserved))));
        }
        l.reserved += size;
        Ok(())
    }

    /// The upload of a reserved file ended: `written` bytes were kept (0 = failed).
    pub fn finish(&self, code: &str, reserved: u64, written: u64) {
        if let Some(l) = self.links.lock().get_mut(code) {
            l.reserved = l.reserved.saturating_sub(reserved);
            if written > 0 {
                l.used_bytes += written;
                l.files += 1;
            }
        }
    }
}

/// A file name that is safe on Windows: no folders, no reserved names or characters.
pub fn safe_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let mut s: String = base.chars().map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { '_' } else { c }).collect();
    s = s.trim().trim_end_matches(['.', ' ']).to_string();
    if s.chars().count() > 150 {
        let ext = Path::new(&s).extension().map(|e| e.to_string_lossy().chars().take(12).collect::<String>()).unwrap_or_default();
        let stem: String = s.chars().take(130).collect();
        s = if ext.is_empty() { stem } else { format!("{stem}.{ext}") };
    }
    let stem = s.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str()) || ((stem.starts_with("COM") || stem.starts_with("LPT")) && stem.len() == 4 && stem.as_bytes()[3].is_ascii_digit());
    if s.is_empty() || reserved {
        s = format!("file {}", if s.is_empty() { String::new() } else { s });
        s = s.trim().to_string();
    }
    s
}

/// `dir/name`, or `dir/name (2).ext` … when the name is taken.
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    if !p.exists() {
        return p;
    }
    let path = Path::new(name);
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    (2..10_000).map(|i| dir.join(format!("{stem} ({i}){ext}"))).find(|p| !p.exists()).unwrap_or_else(|| dir.join(format!("{stem} ({}){ext}", now_unix())))
}

/// Today's folder for guest files.
pub fn folder(incoming: &Path) -> PathBuf {
    incoming.join("Guests").join(chrono::Local::now().format("%Y-%m-%d").to_string())
}

/// The page a guest's phone opens (its script is [`PAGE_JS`]; the server's
/// content policy allows no inline script).
pub const PAGE_HTML: &str = r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
<meta name="robots" content="noindex"><title>Send files</title>
<style>
:root{color-scheme:dark;--bg:#0b0b12;--card:#15151f;--line:#2a2a38;--fg:#f4f4f8;--dim:#9a9ab0;--accent:#8b5cf6;--good:#22c55e;--bad:#f43f5e}
*{box-sizing:border-box}body{margin:0;min-height:100vh;background:radial-gradient(80% 50% at 50% 0%,#2a1a55,transparent),var(--bg);color:var(--fg);font:16px/1.45 system-ui,-apple-system,Segoe UI,Roboto,sans-serif;padding:24px 16px env(safe-area-inset-bottom)}
main{max-width:520px;margin:0 auto}h1{font-size:24px;margin:8px 0 4px}p{color:var(--dim);margin:0 0 16px}
.drop{display:flex;flex-direction:column;align-items:center;gap:10px;border:2px dashed var(--line);border-radius:22px;background:var(--card);padding:34px 16px;text-align:center;cursor:pointer}
.drop.on{border-color:var(--accent)}.btn{background:var(--accent);color:#fff;border:0;border-radius:999px;padding:12px 22px;font-weight:600;font-size:16px}
ul{list-style:none;padding:0;margin:18px 0 0}li{background:var(--card);border:1px solid var(--line);border-radius:14px;padding:10px 12px;margin-bottom:8px}
.row{display:flex;justify-content:space-between;gap:10px}.name{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.small{font-size:13px;color:var(--dim)}
.bar{height:5px;border-radius:5px;background:var(--line);margin-top:8px;overflow:hidden}.bar i{display:block;height:100%;width:0;background:var(--accent);transition:width .2s}
.ok{color:var(--good)}.err{color:var(--bad)}.ended{text-align:center;padding:60px 10px}
</style></head><body><main id="app"><h1>Send files</h1><p>Loading…</p></main><script src="/guest.js"></script></body></html>"#;

pub const PAGE_JS: &str = r#"(function(){
var code=location.pathname.split('/')[2]||'';var app=document.getElementById('app');
function fmt(n){var u=['B','KB','MB','GB','TB'],i=0;while(n>=1000&&i<4){n/=1000;i++}return (i?n.toFixed(1):n)+' '+u[i]}
function el(t,c,x){var e=document.createElement(t);if(c)e.className=c;if(x!=null)e.textContent=x;return e}
function ended(msg){app.innerHTML='';var d=el('div','ended');d.appendChild(el('h1',null,'This link has ended'));d.appendChild(el('p',null,msg||'Ask for a new QR code.'));app.appendChild(d)}
fetch('/guest/'+code+'/info').then(function(r){if(!r.ok)throw 0;return r.json()}).then(function(info){
 app.innerHTML='';app.appendChild(el('h1',null,'Send files to '+info.pc));
 var left=el('p',null,'');app.appendChild(left);
 function upd(){var mins=Math.max(0,Math.round((info.expiresAt*1000-Date.now())/60000));left.textContent='Up to '+fmt(info.maxBytes-info.usedBytes)+' more · this link works for '+mins+' more minute'+(mins==1?'':'s')+'. The person at the PC sees what you send.'}
 upd();setInterval(upd,15000);
 var drop=el('label','drop');var inp=el('input');inp.type='file';inp.multiple=true;inp.style.display='none';
 drop.appendChild(el('div',null,'📤'));drop.appendChild(el('strong',null,'Choose photos, videos or files'));var b=el('span','btn','Choose');drop.appendChild(b);drop.appendChild(inp);app.appendChild(drop);
 var list=el('ul');app.appendChild(list);var queue=[],busy=false;
 function add(files){for(var i=0;i<files.length;i++)queue.push(files[i]);next()}
 inp.onchange=function(){add(inp.files);inp.value=''};
 drop.ondragover=function(e){e.preventDefault();drop.classList.add('on')};drop.ondragleave=function(){drop.classList.remove('on')};
 drop.ondrop=function(e){e.preventDefault();drop.classList.remove('on');add(e.dataTransfer.files)};
 function next(){if(busy||!queue.length)return;busy=true;var f=queue.shift();var li=el('li');var row=el('div','row');row.appendChild(el('span','name',f.name));var st=el('span','small',fmt(f.size));row.appendChild(st);li.appendChild(row);var bar=el('div','bar');var fill=el('i');bar.appendChild(fill);li.appendChild(bar);list.insertBefore(li,list.firstChild);
  var x=new XMLHttpRequest();x.open('PUT','/guest/'+code+'/file?name='+encodeURIComponent(f.name)+'&size='+f.size);
  x.upload.onprogress=function(e){if(e.lengthComputable)fill.style.width=(e.loaded/e.total*100)+'%'};
  x.onload=function(){var ok=x.status>=200&&x.status<300;if(ok){st.textContent='Sent ✓';st.className='small ok';fill.style.width='100%';info.usedBytes+=f.size;upd()}else{var m='Not sent';try{m=JSON.parse(x.responseText).error||m}catch(_){}st.textContent=m;st.className='small err';if(x.status==404||x.status==410){queue=[];ended(m)}}busy=false;next()};
  x.onerror=function(){st.textContent='Not sent (connection lost)';st.className='small err';busy=false;next()};x.send(f)}
}).catch(function(){ended()});
})();"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_names_and_limits() {
        let g = Guests::default();
        let l = g.create("  Sam's phone ", 15, 10 << 20);
        assert_eq!(l.label, "Sam's phone");
        assert_eq!(g.list().len(), 1);
        assert!(g.reserve(&l.code, 6 << 20).is_ok());
        assert!(g.reserve(&l.code, 6 << 20).is_err(), "two uploads can't overshoot");
        g.finish(&l.code, 6 << 20, 6 << 20);
        assert_eq!(g.get(&l.code).unwrap().files, 1);
        assert!(g.reserve(&l.code, 5 << 20).is_err());
        assert!(g.reserve(&l.code, 4 << 20).is_ok());
        g.finish(&l.code, 4 << 20, 0);
        assert_eq!(g.get(&l.code).unwrap().files, 1, "a failed upload doesn't count");
        assert!(g.reserve("nope", 1).is_err());
        assert!(g.stop(&l.code));
        assert!(g.get(&l.code).is_none());

        assert_eq!(safe_name("../../evil/../x.txt"), "x.txt");
        assert_eq!(safe_name(r"C:\Windows\system32\a:b?.png"), "a_b_.png");
        assert_eq!(safe_name("CON.txt"), "file CON.txt");
        assert_eq!(safe_name("   "), "file");
        assert_eq!(safe_name("trailing. . "), "trailing");
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "1").unwrap();
        std::fs::write(dir.path().join("a (2).txt"), "1").unwrap();
        assert_eq!(unique_path(dir.path(), "a.txt").file_name().unwrap(), "a (3).txt");
        assert_eq!(unique_path(dir.path(), "b.txt").file_name().unwrap(), "b.txt");
    }
}
