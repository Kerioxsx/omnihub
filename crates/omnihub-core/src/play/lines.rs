//! What Capri says about your games. `{name}` is what she calls you,
//! `{game}` the game; other `{…}` are filled in where they're used.

use rand::seq::SliceRandom;

pub const GAME_START: &[&str] = &[
    "{game}? Let's go, {name}.",
    "Booting up {game}. Say “Capri, clip that” if something cool happens.",
    "Good luck in {game}, {name}. I've got your back.",
    "{game} time. Show me what you've got, {name}.",
];
pub const GAME_START_REPLAY: &[&str] = &["I'm keeping the last {minutes} minutes, {name}. Just say “Capri, clip that”.", "Instant replay is on: the last {minutes} minutes are safe with me."];
pub const GAME_END: &[&str] = &["GG, {name}. {duration} of {game}.", "{game} closed after {duration}. Nice session, {name}.", "That's {duration} of {game}. Take a breather, {name}."];
pub const GAME_END_CLIPS: &[&str] = &["GG, {name}. {duration} of {game} and {clips} saved.", "{duration} of {game}, {clips} in the bag."];
pub const CLIP_SAVED: &[&str] = &["Clipped! The last {secs} seconds are saved, {name}.", "Got it, {name}. That one's in your clips.", "Saved! That's going in the highlight reel."];
pub const LONG_SAVED: &[&str] = &["Saved the last {what}, {name}.", "Done: the last {what} are in your clips."];
pub const CLIP_FAILED: &[&str] = &["I couldn't save that one, {name}: {why}"];
pub const NO_REPLAY: &[&str] = &["I'm not recording right now, {name}. Turn on Instant replay in OmniHub → Games → Clips."];
pub const NO_GAME: &[&str] = &["I record while a game is running, {name}. Start one and I'll keep the last few minutes."];
pub const ONLY_KEEP: &[&str] = &["I only keep {minutes} minutes, {name}, so I saved all of that."];
pub const RECORDING: &[&str] = &["Recording, {name}. Say “Capri, stop recording” when you're done."];
pub const RECORDING_SAVED: &[&str] = &["Saved your recording, {name}: {duration}."];
pub const NOT_RECORDING: &[&str] = &["I wasn't recording, {name}. Say “Capri, start recording” first."];
pub const CALL_ME: &[&str] = &["Okay, {name}.", "{name} it is.", "Got it. From now on you're {name}.", "Sure thing, {name}."];
pub const FPS: &[&str] = &["You're at {fps} FPS, {name}. 1% low: {low}.", "{fps} FPS right now, {name}."];
pub const NO_FPS: &[&str] = &["Turn on the FPS meter in OmniHub → Games and start the game from there, {name}, then I can tell you."];
pub const FPS_DROP: &[&str] = &["Your FPS dropped to {fps}, {name}.", "Frames are down to {fps}, {name}. Something's eating your PC."];
pub const SCREENSHOT: &[&str] = &["Screenshot saved, {name}.", "Snap! Got it."];
pub const HELLO: &[&str] = &["I'm here, {name}.", "Right here, {name}. Want me to clip something?", "Hi, {name}!"];

// Counter-Strike 2
pub const CS_TRIPLE: &[&str] = &["Three down! {name}, that's a clip.", "Triple kill, {name}!"];
pub const CS_QUAD: &[&str] = &["FOUR kills, {name}! Clipping that.", "Quad kill! Who even are you?"];
pub const CS_ACE: &[&str] = &["ACE! {name}, that was insane. Clipped.", "An ace! I'm saving that forever."];
pub const CS_ROUND_WON: &[&str] = &["Round's ours. {ours}–{theirs}.", "Nice round, {name}. {ours}–{theirs}."];
pub const CS_ROUND_LOST: &[&str] = &["We'll get the next one, {name}. {ours}–{theirs}.", "Shake it off. {ours}–{theirs}."];
pub const CS_MVP: &[&str] = &["MVP, {name}! Of course.", "Round MVP. I knew it."];
pub const CS_BOMB: &[&str] = &["Bomb's planted!", "Bomb's down, {name}!"];
pub const CS_WON: &[&str] = &["We won, {name}! {ours}–{theirs}. GG!", "Victory! {ours}–{theirs}."];
pub const CS_LOST: &[&str] = &["That one's gone, {ours}–{theirs}. Queue again, {name}?", "GG anyway, {name}. {ours}–{theirs}."];

// League of Legends
pub const LOL_KILL: &[&str] = &["Nice kill, {name}.", "Got 'em!", "One down, {name}."];
pub const LOL_DOUBLE: &[&str] = &["Double kill!"];
pub const LOL_TRIPLE: &[&str] = &["Triple kill, {name}! Clipped."];
pub const LOL_QUADRA: &[&str] = &["QUADRA KILL! Clipping it, {name}."];
pub const LOL_PENTA: &[&str] = &["PENTAKILL! {name}, I'm framing that one."];
pub const LOL_DIED: &[&str] = &["Ouch. Back in it soon, {name}.", "They got you. Revenge time."];
pub const LOL_DRAGON: &[&str] = &["Dragon's ours!", "We took the dragon, {name}."];
pub const LOL_DRAGON_LOST: &[&str] = &["They took the dragon."];
pub const LOL_BARON: &[&str] = &["Baron secured! Push, {name}!"];
pub const LOL_BARON_LOST: &[&str] = &["They've got Baron. Careful, {name}."];
pub const LOL_ACE: &[&str] = &["Ace! Their whole team is down."];
pub const LOL_WIN: &[&str] = &["Victory, {name}!", "We won! GG, {name}."];
pub const LOL_LOSS: &[&str] = &["Defeat. GG though, {name}.", "We'll get them next game, {name}."];

/// One of `lines` with `{key}`s filled in.
pub fn pick(lines: &[&str], fill: &[(&str, String)]) -> String {
    let line = lines.choose(&mut rand::thread_rng()).copied().unwrap_or_default();
    fill.iter().fold(line.to_string(), |s, (k, v)| s.replace(&format!("{{{k}}}"), v))
}

/// "1 h 12 min", "34 min", "45 s".
pub fn duration(secs: u64) -> String {
    match secs {
        0..=59 => format!("{secs} s"),
        60..=3599 => format!("{} min", secs / 60),
        _ => format!("{} h {} min", secs / 3600, secs % 3600 / 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_in_names() {
        let s = pick(&["GG, {name}. {duration} of {game}."], &[("name", "daddy".into()), ("duration", duration(4320)), ("game", "Fortnite".into())]);
        assert_eq!(s, "GG, daddy. 1 h 12 min of Fortnite.");
        assert_eq!(duration(45), "45 s");
        assert_eq!(duration(600), "10 min");
        // Every line set has something, and nothing is left unfilled by mistake.
        for set in [GAME_START, GAME_END, CLIP_SAVED, CALL_ME, CS_ACE, LOL_PENTA] {
            assert!(!set.is_empty());
            assert!(!pick(set, &[("name", "x".into()), ("game", "g".into()), ("secs", "1".into()), ("duration", "1".into())]).contains("{name}"));
        }
    }
}
