//! Counters on a stream: a small web page for OBS (Browser source) at
//! `http://127.0.0.1:<port>/counter/<id>`, with a see-through background.
//! It listens on this PC only (never the network) and only while turned on.
//! The page asks for the counter twice a second.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;

use crate::core::AppCore;
use crate::counters::Counter;

pub const DEFAULT_PORT: u16 = 47823;

pub struct OverlayServer {
    stop: Arc<AtomicBool>,
    pub port: u16,
}

impl Drop for OverlayServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// Listen on 127.0.0.1 only.
pub fn start(port: u16, core: Weak<AppCore>) -> Result<OverlayServer, String> {
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| format!("Port {port} is taken ({e}); pick another"))?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    std::thread::Builder::new()
        .name("counters-overlay".into())
        .spawn(move || {
            while !flag.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let core = core.clone();
                        std::thread::spawn(move || {
                            let _ = handle(stream, &core);
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(100)),
                    Err(_) => std::thread::sleep(Duration::from_millis(200)),
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(OverlayServer { stop, port })
}

fn respond(mut s: TcpStream, status: &str, kind: &str, body: &[u8]) -> std::io::Result<()> {
    write!(s, "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n", body.len())?;
    s.write_all(body)
}

/// What the page shows for a counter.
pub fn counter_json(c: &Counter, now: i64) -> serde_json::Value {
    let hunt = c.hunt.as_ref();
    let p = hunt.map(|h| h.chance());
    serde_json::json!({
        "name": c.name,
        "count": c.count,
        "phaseCount": c.count - c.phase_base,
        "phases": c.phases,
        "timerMs": c.elapsed_ms(now),
        "running": c.timer_since.is_some(),
        "odds": p.map(|p| format!("1/{}", (1.0 / p).round())),
        "chance": p.map(|p| 1.0 - (1.0 - p).powf(c.count as f64)),
        "sprite": hunt.filter(|h| h.species > 0).map(|h| format!("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/shiny/{}.png", h.species)),
        "image": c.image.is_some(),
    })
}

fn handle(stream: TcpStream, core: &Weak<AppCore>) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line)?;
    let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    let (route, _query) = path.split_once('?').unwrap_or((&path, ""));
    let Some(core) = core.upgrade() else { return respond(stream, "503 Service Unavailable", "text/plain", b"OmniHub is closing") };
    let id_of = |s: &str| s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>();
    if route == "/" {
        let list: String = core
            .counters
            .state()
            .counters
            .iter()
            .map(|c| format!("<li><a href=\"/counter/{0}\">{1}</a> <code>http://127.0.0.1:{2}/counter/{0}</code></li>", id_of(&c.id), html_escape(&c.name), stream.local_addr().map(|a| a.port()).unwrap_or(DEFAULT_PORT)))
            .collect();
        let body = format!("<!doctype html><meta charset=utf-8><title>OmniHub counters</title><body style=\"font:15px system-ui;background:#111;color:#eee;padding:24px\"><h1>Counters for OBS</h1><p>Add one as a Browser source (width 400, height 300).</p><ul>{list}</ul>");
        return respond(stream, "200 OK", "text/html; charset=utf-8", body.as_bytes());
    }
    let Some(rest) = route.strip_prefix("/counter/") else { return respond(stream, "404 Not Found", "text/plain", b"not found") };
    if let Some(id) = rest.strip_suffix(".json") {
        let Some(c) = core.counters.counter(&id_of(id)) else { return respond(stream, "404 Not Found", "application/json", b"{}") };
        let body = counter_json(&c, chrono::Utc::now().timestamp_millis()).to_string();
        return respond(stream, "200 OK", "application/json", body.as_bytes());
    }
    if let Some(id) = rest.strip_suffix("/image") {
        use base64::Engine;
        let Some(url) = core.counters.image(&id_of(id)) else { return respond(stream, "404 Not Found", "text/plain", b"no picture") };
        let (head, data) = url.split_once(',').unwrap_or(("", ""));
        let mime = head.trim_start_matches("data:").trim_end_matches(";base64");
        let bytes = base64::engine::general_purpose::STANDARD.decode(data).unwrap_or_default();
        return respond(stream, "200 OK", mime, &bytes);
    }
    let id = id_of(rest);
    if core.counters.counter(&id).is_none() {
        return respond(stream, "404 Not Found", "text/plain", b"no such counter");
    }
    respond(stream, "200 OK", "text/html; charset=utf-8", PAGE.replace("__ID__", &id).as_bytes())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The overlay page: options in the address (?size=96&color=ffffff&picture=0&timer=0&odds=0&name=0).
const PAGE: &str = r#"<!doctype html><html><head><meta charset="utf-8"><title>Counter</title>
<style>
html,body{margin:0;background:transparent;overflow:hidden}
body{font-family:"Inter","Segoe UI",system-ui,sans-serif;color:var(--c,#fff);display:flex;align-items:center;gap:.4em;padding:8px;font-size:var(--s,72px);text-shadow:0 2px 6px rgba(0,0,0,.6)}
img{height:1.4em;image-rendering:pixelated;filter:drop-shadow(0 3px 4px rgba(0,0,0,.5))}
.n{font-size:.32em;opacity:.9;font-weight:600}.c{font-weight:800;line-height:1;font-variant-numeric:tabular-nums}
.m{font-size:.26em;opacity:.85;font-variant-numeric:tabular-nums}.hide{display:none}
</style></head><body>
<img id="p" class="hide" alt=""><div><div class="n" id="n"></div><div class="c" id="c">0</div><div class="m" id="m"></div></div>
<script>
const q=new URLSearchParams(location.search),b=document.body;
if(q.get('size'))b.style.setProperty('--s',(+q.get('size')||72)+'px');
if(q.get('color'))b.style.setProperty('--c','#'+q.get('color').replace(/[^0-9a-f]/gi,''));
const show=k=>q.get(k)!=='0',fmt=ms=>{const s=Math.floor(ms/1e3),h=Math.floor(s/3600),m=Math.floor(s%3600/60),x=s%60;return(h?h+':'+String(m).padStart(2,'0'):m)+':'+String(x).padStart(2,'0')};
let last=null,img='';
async function tick(){try{const r=await fetch('/counter/__ID__.json',{cache:'no-store'});last=await r.json();last.at=Date.now()}catch(e){}draw()}
function draw(){if(!last)return;document.getElementById('n').textContent=show('name')?last.name:'';
document.getElementById('c').textContent=Number(last.count).toLocaleString();
const bits=[];if(last.phases)bits.push('Phase '+(last.phases+1)+': '+Number(last.phaseCount).toLocaleString());
if(show('odds')&&last.odds)bits.push(last.odds+' · '+(last.chance*100).toFixed(1)+'%');
if(show('timer')&&(last.timerMs||last.running))bits.push(fmt(last.timerMs+(last.running?Date.now()-last.at:0)));
document.getElementById('m').textContent=bits.join('  ·  ');
const p=document.getElementById('p'),src=last.image?'/counter/__ID__/image':(last.sprite||'');
if(show('picture')&&src){if(img!==src){p.src=src;img=src}p.classList.remove('hide')}else p.classList.add('hide')}
tick();setInterval(tick,500);setInterval(draw,250);
</script></body></html>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counters::Hunt;

    #[test]
    fn json_for_a_hunt() {
        let c = Counter { name: "Shiny Mudkip".into(), count: 4096, phase_base: 96, phases: 1, hunt: Some(Hunt { species: 258, ..Default::default() }), ..Default::default() };
        let j = counter_json(&c, 0);
        assert_eq!(j["phaseCount"], 4000);
        assert_eq!(j["odds"], "1/4096");
        assert!(j["sprite"].as_str().unwrap().ends_with("/shiny/258.png"));
        assert!((j["chance"].as_f64().unwrap() - 0.632).abs() < 0.01);
    }

    #[test]
    fn escapes() {
        assert_eq!(html_escape("<a&\"b\">"), "&lt;a&amp;&quot;b&quot;&gt;");
    }
}
