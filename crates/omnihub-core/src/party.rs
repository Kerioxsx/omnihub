//! Party queue: friends scan a QR code and add songs from their own phones,
//! no pairing. Songs are searched in the same public catalogue as the music
//! hub, and played one after another in your music app the way the music hub
//! does it (YouTube Music opens the song itself; Spotify and others open on
//! the song's search). With "auto", the next song starts when the one
//! playing ends.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

pub const MAX_QUEUE: usize = 200;
pub const PER_GUEST: usize = 15;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PartySong {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub cover: String,
    /// The guest's name.
    pub by: String,
    pub at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PartyState {
    pub code: Option<String>,
    pub queue: Vec<PartySong>,
    pub playing: Option<PartySong>,
    pub auto: bool,
    pub played: usize,
}

#[derive(Default)]
struct Inner {
    code: Option<String>,
    queue: Vec<PartySong>,
    playing: Option<PartySong>,
    /// The playing song was seen in the music app (so its end can be noticed).
    started: bool,
    auto: bool,
    played: usize,
}

#[derive(Default)]
pub struct Party {
    inner: Mutex<Inner>,
}

fn norm(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Whether the music app's song is the queued one.
pub fn same_song(song: &PartySong, title: &str, artist: &str) -> bool {
    let (a, b) = (norm(&song.title), norm(title));
    !a.is_empty() && (b.contains(&a) || a.contains(&b)) && (artist.is_empty() || norm(artist).contains(&norm(&song.artist)) || norm(&song.artist).contains(&norm(artist)))
}

impl Party {
    pub fn state(&self) -> PartyState {
        let i = self.inner.lock();
        PartyState { code: i.code.clone(), queue: i.queue.clone(), playing: i.playing.clone(), auto: i.auto, played: i.played }
    }

    /// Start (or keep) a party; returns its code.
    pub fn start(&self) -> String {
        let mut i = self.inner.lock();
        i.code.get_or_insert_with(|| uuid::Uuid::new_v4().simple().to_string()[..10].to_string()).clone()
    }

    pub fn stop(&self) {
        *self.inner.lock() = Inner::default();
    }

    pub fn code_ok(&self, code: &str) -> bool {
        self.inner.lock().code.as_deref() == Some(code)
    }

    pub fn set_auto(&self, on: bool) {
        self.inner.lock().auto = on;
    }

    /// A guest adds a song.
    pub fn add(&self, code: &str, mut song: PartySong, now: i64) -> Result<usize, String> {
        let mut i = self.inner.lock();
        if i.code.as_deref() != Some(code) {
            return Err("This party has ended".into());
        }
        song.title = song.title.chars().take(200).collect();
        song.artist = song.artist.chars().take(200).collect();
        song.by = song.by.trim().chars().take(30).collect();
        if song.by.is_empty() {
            song.by = "A friend".into();
        }
        if song.title.trim().is_empty() {
            return Err("Pick a song".into());
        }
        if i.queue.len() >= MAX_QUEUE {
            return Err("The queue is full".into());
        }
        if i.queue.iter().filter(|s| s.by == song.by).count() >= PER_GUEST {
            return Err(format!("You have {PER_GUEST} songs waiting already"));
        }
        if i.queue.iter().any(|s| norm(&s.title) == norm(&song.title) && norm(&s.artist) == norm(&song.artist)) {
            return Err("That song is already in the queue".into());
        }
        song.id = uuid::Uuid::new_v4().simple().to_string()[..10].to_string();
        song.at = now;
        i.queue.push(song);
        Ok(i.queue.len())
    }

    pub fn remove(&self, id: &str) {
        self.inner.lock().queue.retain(|s| s.id != id);
    }

    /// Move a song to the front.
    pub fn bump(&self, id: &str) {
        let mut i = self.inner.lock();
        if let Some(at) = i.queue.iter().position(|s| s.id == id) {
            let s = i.queue.remove(at);
            i.queue.insert(0, s);
        }
    }

    /// Take the next song (to hand to the music app).
    pub fn take_next(&self) -> Option<PartySong> {
        let mut i = self.inner.lock();
        if i.queue.is_empty() {
            i.playing = None;
            return None;
        }
        let s = i.queue.remove(0);
        i.playing = Some(s.clone());
        i.started = false;
        i.played += 1;
        Some(s)
    }

    /// With auto on: whether the playing song has just ended, given what the
    /// music app shows now (title, artist, playing, position, duration).
    pub fn song_ended(&self, title: &str, artist: &str, playing: bool, pos_ms: u64, dur_ms: u64) -> bool {
        let mut i = self.inner.lock();
        if !i.auto || i.code.is_none() {
            return false;
        }
        let Some(song) = i.playing.clone() else { return !i.queue.is_empty() && !playing };
        let same = same_song(&song, title, artist);
        if same && playing {
            i.started = true;
            return false;
        }
        // Ended: it was playing, and now another song plays, or it stopped at the end.
        i.started && (!same || (!playing && dur_ms > 0 && pos_ms + 3000 >= dur_ms))
    }
}

/// The page guests open (no pairing; only the party code).
pub const PAGE: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Party queue</title><style>
*{box-sizing:border-box}body{margin:0;font:16px/1.4 system-ui,sans-serif;background:#0b0b12;color:#eee}main{max-width:560px;margin:auto;padding:18px}
h1{font-size:22px;margin:6px 0 2px}p.s{color:#9a9aae;margin:0 0 14px}input{width:100%;padding:12px 14px;border-radius:12px;border:1px solid #2a2a3a;background:#14141f;color:#eee;font-size:16px}
.row{display:flex;gap:10px;align-items:center;padding:10px;border-radius:12px;background:#14141f;margin-top:8px}.row img{width:48px;height:48px;border-radius:8px;object-fit:cover;background:#222}
.row div{flex:1;min-width:0}.t{font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.a{color:#9a9aae;font-size:14px}button{border:0;border-radius:10px;padding:9px 14px;background:#8b5cf6;color:#fff;font-weight:600}
button:disabled{opacity:.5}h2{font-size:15px;color:#9a9aae;margin:22px 0 4px}.msg{margin-top:10px;color:#34d399;min-height:20px}.err{color:#f87171}
</style></head><body><main><h1>🎉 Party queue</h1><p class="s">Add songs to what's playing.</p>
<input id="name" placeholder="Your name" maxlength="30" style="margin-bottom:10px"><input id="q" placeholder="Search a song or artist" autocomplete="off">
<div id="msg" class="msg"></div><div id="res"></div><h2>Up next</h2><div id="queue"></div></main><script>
var code=location.pathname.split('/')[2];var $=function(i){return document.getElementById(i)};
var nm=localStorage.getItem('partyName')||'';$('name').value=nm;$('name').oninput=function(){localStorage.setItem('partyName',this.value)};
function esc(s){return String(s).replace(/[&<>"]/g,function(c){return{'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]})}
function row(s,btn){return '<div class="row"><img src="'+esc(s.cover||'')+'" alt=""><div><div class="t">'+esc(s.title)+'</div><div class="a">'+esc(s.artist)+(s.by?' · '+esc(s.by):'')+'</div></div>'+(btn||'')+'</div>'}
function load(){fetch('/party/'+code+'/queue').then(function(r){if(!r.ok)throw 0;return r.json()}).then(function(d){var h='';if(d.playing)h+=row(d.playing,'<span>▶︎</span>');d.queue.forEach(function(s){h+=row(s)});$('queue').innerHTML=h||'<p class="s">Nothing yet — add the first song!</p>'}).catch(function(){$('queue').innerHTML='<p class="s err">This party has ended.</p>'})}
var t;$('q').oninput=function(){clearTimeout(t);var q=this.value.trim();if(q.length<2){$('res').innerHTML='';return}t=setTimeout(function(){fetch('/party/'+code+'/search?q='+encodeURIComponent(q)).then(function(r){return r.json()}).then(function(d){window._r=d.tracks||[];$('res').innerHTML=window._r.slice(0,8).map(function(s,i){return row(s,'<button onclick="add('+i+',this)">Add</button>')}).join('')})},300)};
function add(i,b){var s=window._r[i];b.disabled=true;fetch('/party/'+code+'/add',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({title:s.title,artist:s.artist,album:s.album||'',cover:s.cover||'',by:$('name').value})}).then(function(r){return r.json().then(function(j){if(!r.ok)throw j.error||'Couldn\'t add it';return j})}).then(function(){$('msg').className='msg';$('msg').textContent='Added: '+s.title;load()}).catch(function(e){$('msg').className='msg err';$('msg').textContent=e;b.disabled=false})}
load();setInterval(load,5000);
</script></body></html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn song(t: &str, a: &str, by: &str) -> PartySong {
        PartySong { id: String::new(), title: t.into(), artist: a.into(), album: String::new(), cover: String::new(), by: by.into(), at: 0 }
    }

    #[test]
    fn guests_add_and_songs_play_in_order() {
        let p = Party::default();
        assert!(p.add("x", song("Song", "Artist", "Sam"), 0).is_err(), "no party yet");
        let code = p.start();
        assert_eq!(p.start(), code, "the same party");
        p.add(&code, song("Blinding Lights", "The Weeknd", "Sam"), 1).unwrap();
        p.add(&code, song("Levitating", "Dua Lipa", ""), 2).unwrap();
        assert!(p.add(&code, song("blinding lights", "the weeknd", "Alex"), 3).is_err(), "no doubles");
        assert_eq!(p.state().queue[1].by, "A friend");
        assert!(p.add("wrong", song("X", "Y", "Z"), 4).is_err());

        p.set_auto(true);
        let first = p.take_next().unwrap();
        assert_eq!(first.title, "Blinding Lights");
        assert!(!p.song_ended("Blinding Lights", "The Weeknd", true, 1000, 200_000), "playing now");
        assert!(!p.song_ended("Blinding Lights", "The Weeknd", true, 100_000, 200_000));
        assert!(p.song_ended("Blinding Lights", "The Weeknd", false, 199_000, 200_000), "stopped at its end");
        let second = p.take_next().unwrap();
        assert!(!p.song_ended("Something Else", "Other", true, 0, 1), "not started yet");
        assert!(!p.song_ended("Levitating", "Dua Lipa", true, 0, 1));
        assert!(p.song_ended("Something Else", "Other", true, 0, 1), "the music app moved on");
        assert_eq!(second.title, "Levitating");
        assert!(p.take_next().is_none());
        p.stop();
        assert!(p.state().code.is_none());
    }
}
