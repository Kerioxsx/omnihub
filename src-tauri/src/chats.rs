//! Chats in the shell: OmniHub's own chats with Claude and ChatGPT's
//! models, Claude Code and Codex conversations on this PC, ideas, usage and
//! the OpenAI key.

use std::sync::Arc;

use omnihub_core::chats::openai::OpenAiSettings;
use omnihub_core::chats::usage::Report;
use omnihub_core::chats::{save_idea, ChatHub, ChatSummary, ChatView, Folder, NewKind, Source, Tools};
use omnihub_core::core::AppCore;
use omnihub_core::notes::{Note, NoteFilter, NoteKind};
use tauri::State;

type Core<'a> = State<'a, Arc<AppCore>>;
type Res<T> = Result<T, String>;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Res<T> + Send + 'static) -> Res<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

macro_rules! run {
    ($core:expr, |$c:ident| $body:expr) => {{
        let $c = $core.inner().clone();
        blocking(move || $body).await
    }};
}

fn source(s: &str) -> Res<Source> {
    Source::parse(s).ok_or_else(|| "Unknown kind of chat.".to_string())
}

#[tauri::command]
pub async fn chats_list(core: Core<'_>) -> Res<Vec<ChatSummary>> {
    run!(core, |c| Ok(c.chats.list()))
}

#[tauri::command]
pub async fn chats_view(core: Core<'_>, source: String, id: String) -> Res<ChatView> {
    run!(core, |c| c.chats.view(self::source(&source)?, &id))
}

#[tauri::command]
pub async fn chats_send(core: Core<'_>, source: String, id: String, text: String, mode: String) -> Res<()> {
    run!(core, |c| {
        let s = self::source(&source)?;
        let r = c.chats.send(s, &id, &text, &mode);
        if s != Source::Omni {
            c.audit.record("desktop", "chats.agent", &format!("{} {mode}", s.label()), r.is_ok());
        }
        r
    })
}

#[tauri::command]
pub async fn chats_create(core: Core<'_>, kind: NewKind, folder: Option<String>, text: String, mode: String) -> Res<ChatSummary> {
    run!(core, |c| {
        let r = c.chats.create(kind, folder.as_deref(), &text, &mode);
        if matches!(kind, NewKind::ClaudeCode | NewKind::Codex) {
            c.audit.record("desktop", "chats.agent", &format!("new in {} ({mode})", folder.unwrap_or_default()), r.is_ok());
        }
        r
    })
}

#[tauri::command]
pub async fn chats_stop(core: Core<'_>, source: String, id: String) -> Res<()> {
    core.chats.stop(self::source(&source)?, &id);
    Ok(())
}

#[tauri::command]
pub async fn chats_rename(core: Core<'_>, id: String, title: String) -> Res<()> {
    run!(core, |c| c.chats.rename(&id, &title))
}

#[tauri::command]
pub async fn chats_delete(core: Core<'_>, id: String) -> Res<()> {
    run!(core, |c| c.chats.delete(&id))
}

#[tauri::command]
pub async fn chats_folders(core: Core<'_>) -> Res<Vec<Folder>> {
    run!(core, |c| Ok(c.chats.folders()))
}

#[tauri::command]
pub async fn chats_tools(core: Core<'_>) -> Res<Tools> {
    run!(core, |c| Ok(c.chats.tools()))
}

#[tauri::command]
pub async fn chats_usage(core: Core<'_>) -> Res<Report> {
    run!(core, |c| Ok(c.chats.usage_report(ChatHub::maker_tokens(&c.mods.maker.projects(None)))))
}

#[tauri::command]
pub async fn ideas_list(core: Core<'_>) -> Res<Vec<Note>> {
    run!(core, |c| c.notes.list(&NoteFilter { kind: Some(NoteKind::Idea), limit: Some(300), ..Default::default() }).map_err(|e| e.to_string()))
}

#[tauri::command]
pub async fn ideas_save(core: Core<'_>, text: String, from: Option<String>) -> Res<Note> {
    run!(core, |c| save_idea(&c, &text, from.as_deref()))
}

#[tauri::command]
pub async fn openai_settings(core: Core<'_>) -> Res<OpenAiSettings> {
    Ok(core.chats.openai.settings())
}

#[tauri::command]
pub async fn openai_update(core: Core<'_>, patch: serde_json::Value) -> Res<OpenAiSettings> {
    core.chats.openai.update(&patch)
}

/// The OpenAI API key: kept encrypted for this Windows account; empty removes it.
#[tauri::command]
pub async fn openai_set_key(core: Core<'_>, key: String) -> Res<()> {
    run!(core, |c| {
        let had = c.chats.openai.has_key();
        let r = c.chats.openai.set_key(&key);
        c.audit.record("desktop", "openai.key", if key.trim().is_empty() { "removed" } else if had { "replaced" } else { "added" }, r.is_ok());
        r
    })
}

#[tauri::command]
pub async fn openai_models(core: Core<'_>) -> Res<Vec<String>> {
    run!(core, |c| c.chats.openai.models())
}
