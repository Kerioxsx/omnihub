//! "Capri, clip that": voice commands with Windows' own speech recognition.
//!
//! The recognizer is given the exact phrases below (a list grammar), not
//! free dictation: it runs on the PC, sends nothing anywhere, costs next to
//! nothing while it listens, and only reacts to sentences that start with
//! "Capri". A name for "Capri, call me …" must be in the list of names
//! (OmniHub's own, plus the ones you add).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
pub enum Command {
    /// Save the usual clip length.
    Clip,
    /// Save the last so many seconds.
    SaveLast(u32),
    StartRecording,
    StopRecording,
    CallMe(String),
    Fps,
    Screenshot,
    Hello,
}

/// Names offered for "Capri, call me …" besides the ones you add.
pub const NICKNAMES: &[&str] = &["boss", "champ", "chief", "captain", "king", "queen", "legend", "daddy", "mommy", "bro", "dude", "partner", "commander", "master chief", "goat"];

const WAKE: &[&str] = &["capri", "hey capri", "okay capri"];
const NUMBERS: &[&str] = &["one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen", "twenty"];
const SECONDS: &[(&str, u32)] = &[("fifteen", 15), ("twenty", 20), ("thirty", 30), ("forty five", 45), ("sixty", 60), ("ninety", 90)];
const SAVE_VERBS: &[&str] = &["record", "clip", "save"];

/// Every sentence the recognizer listens for, with what it means.
pub fn phrases(nicknames: &[String]) -> Vec<(String, Command)> {
    let mut body: Vec<(String, Command)> = Vec::new();
    for p in ["clip that", "clip it", "clip this", "save that", "save it"] {
        body.push((p.into(), Command::Clip));
    }
    for verb in SAVE_VERBS {
        for (i, n) in NUMBERS.iter().enumerate() {
            let mins = (i as u32 + 1) * 60;
            let unit = if i == 0 { "minute" } else { "minutes" };
            body.push((format!("{verb} the last {n} {unit}"), Command::SaveLast(mins)));
            body.push((format!("{verb} last {n} {unit}"), Command::SaveLast(mins)));
        }
        body.push((format!("{verb} the last minute"), Command::SaveLast(60)));
        for (words, secs) in SECONDS {
            body.push((format!("{verb} the last {words} seconds"), Command::SaveLast(*secs)));
        }
    }
    body.push(("start recording".into(), Command::StartRecording));
    body.push(("stop recording".into(), Command::StopRecording));
    let mut names: Vec<String> = NICKNAMES.iter().map(|s| s.to_string()).collect();
    names.extend(nicknames.iter().map(|n| clean(n)).filter(|n| !n.is_empty()));
    names.sort();
    names.dedup();
    for n in names {
        body.push((format!("call me {n}"), Command::CallMe(n)));
    }
    for p in ["what's my fps", "how many fps", "what is my fps", "fps check"] {
        body.push((p.into(), Command::Fps));
    }
    for p in ["take a screenshot", "screenshot", "screenshot that"] {
        body.push((p.into(), Command::Screenshot));
    }
    for p in ["are you there", "hello", "say hi"] {
        body.push((p.into(), Command::Hello));
    }
    WAKE.iter().flat_map(|w| body.iter().map(move |(p, c)| (format!("{w} {p}"), c.clone()))).collect()
}

/// Lower case, letters, digits and single spaces ("Capri, clip that!" → "capri clip that").
pub fn clean(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '\'' { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// What a recognized sentence asks for.
pub fn parse(text: &str, nicknames: &[String]) -> Option<Command> {
    let t = clean(text);
    phrases(nicknames).into_iter().find(|(p, _)| *p == t).map(|(_, c)| c)
}

#[cfg(windows)]
pub use win::Listener;

#[cfg(windows)]
mod win {
    use std::sync::mpsc;

    use windows::core::HSTRING;
    use windows_collections::IIterable;
    use windows::Foundation::{TimeSpan, TypedEventHandler};
    use windows::Media::SpeechRecognition::{SpeechContinuousRecognitionCompletedEventArgs, SpeechContinuousRecognitionResultGeneratedEventArgs, SpeechContinuousRecognitionSession, SpeechRecognitionConfidence, SpeechRecognitionListConstraint, SpeechRecognitionResultStatus, SpeechRecognizer};
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

    /// Listening on its own thread until dropped.
    pub struct Listener {
        stop: mpsc::Sender<()>,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    fn why(e: windows::core::Error) -> String {
        match e.code().0 as u32 {
            0x8004_5509 => "Windows speech recognition is off: Settings → Privacy & security → Speech → turn on Online speech recognition, or install a speech language in Settings → Time & language → Speech.".into(),
            0x8007_0005 => "Windows won't let OmniHub use the microphone: Settings → Privacy & security → Microphone → let desktop apps use it.".into(),
            0x8004_5500..=0x8004_55ff => format!("Windows speech recognition isn't ready ({}). Check Settings → Time & language → Speech.", e.message()),
            _ => format!("Windows speech recognition: {}", e.message()),
        }
    }

    impl Listener {
        /// Listen for `phrases`; `heard` gets each one recognized with enough confidence.
        pub fn start(phrases: Vec<String>, heard: impl Fn(String) + Send + Sync + 'static) -> Result<Listener, String> {
            let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
            let (stop_tx, stop_rx) = mpsc::channel::<()>();
            let thread = std::thread::Builder::new()
                .name("voice".into())
                .spawn(move || {
                    unsafe {
                        let _ = RoInitialize(RO_INIT_MULTITHREADED);
                    }
                    let setup = || -> windows::core::Result<(SpeechRecognizer, SpeechContinuousRecognitionSession)> {
                        let rec = SpeechRecognizer::new()?;
                        let list: Vec<HSTRING> = phrases.iter().map(HSTRING::from).collect();
                        let iterable: IIterable<HSTRING> = list.into();
                        rec.Constraints()?.Append(&SpeechRecognitionListConstraint::CreateWithTag(&iterable, &HSTRING::from("omnihub"))?)?;
                        let compiled = rec.CompileConstraintsAsync()?.join()?;
                        if compiled.Status()? != SpeechRecognitionResultStatus::Success {
                            return Err(windows::core::Error::new(windows::core::HRESULT(0x8004_5500u32 as i32), "the phrases weren't accepted"));
                        }
                        let session = rec.ContinuousRecognitionSession()?;
                        // Don't stop after a quiet spell (a day of silence, then it starts again below).
                        session.SetAutoStopSilenceTimeout(TimeSpan { Duration: 24 * 3600 * 10_000_000 })?;
                        session.ResultGenerated(&TypedEventHandler::<SpeechContinuousRecognitionSession, SpeechContinuousRecognitionResultGeneratedEventArgs>::new(move |_, args| {
                            if let Ok(args) = args.ok() {
                                let r = args.Result()?;
                                let conf = r.Confidence()?;
                                if conf == SpeechRecognitionConfidence::High || conf == SpeechRecognitionConfidence::Medium {
                                    heard(r.Text()?.to_string());
                                }
                            }
                            Ok(())
                        }))?;
                        // It ends now and then (a device change, a long silence): start it again.
                        session.Completed(&TypedEventHandler::<SpeechContinuousRecognitionSession, SpeechContinuousRecognitionCompletedEventArgs>::new(|s, _| {
                            if let Ok(s) = s.ok() {
                                let _ = s.StartAsync();
                            }
                            Ok(())
                        }))?;
                        session.StartAsync()?.join()?;
                        Ok((rec, session))
                    };
                    match setup() {
                        Ok((rec, session)) => {
                            let _ = ready_tx.send(Ok(()));
                            let _ = stop_rx.recv();
                            let _ = session.StopAsync().and_then(|a| a.join());
                            drop(rec);
                        }
                        Err(e) => {
                            let _ = ready_tx.send(Err(why(e)));
                        }
                    }
                })
                .map_err(|e| e.to_string())?;
            match ready_rx.recv_timeout(std::time::Duration::from_secs(20)) {
                Ok(Ok(())) => Ok(Listener { stop: stop_tx, thread: Some(thread) }),
                Ok(Err(e)) => Err(e),
                Err(_) => Err("Windows speech recognition didn't start.".into()),
            }
        }
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            let _ = self.stop.send(());
            if let Some(t) = self.thread.take() {
                let _ = t.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn understands_the_commands() {
        let mine = vec!["Sir Fluffy".to_string()];
        assert_eq!(parse("Capri, clip that!", &mine), Some(Command::Clip));
        assert_eq!(parse("Hey Capri record the last ten minutes", &mine), Some(Command::SaveLast(600)));
        assert_eq!(parse("capri save last one minute", &mine), Some(Command::SaveLast(60)));
        assert_eq!(parse("capri clip the last thirty seconds", &mine), Some(Command::SaveLast(30)));
        assert_eq!(parse("Capri call me daddy", &mine), Some(Command::CallMe("daddy".into())));
        assert_eq!(parse("Capri, call me sir fluffy", &mine), Some(Command::CallMe("sir fluffy".into())));
        assert_eq!(parse("Capri what's my FPS", &mine), Some(Command::Fps));
        assert_eq!(parse("Capri start recording", &mine), Some(Command::StartRecording));
        // Without her name, or a name not in the list: nothing.
        assert_eq!(parse("clip that", &mine), None);
        assert_eq!(parse("capri call me bob", &mine), None);
        // No sentence twice (the recognizer would refuse the list).
        let all = phrases(&mine);
        let mut seen: Vec<&String> = all.iter().map(|(p, _)| p).collect();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), all.len());
        assert!(all.len() < 1500, "{} phrases", all.len());
    }
}
