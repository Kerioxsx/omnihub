//! Chats end to end: OmniHub's own chats against pretend Claude and OpenAI
//! servers, and Claude Code and Codex conversations from real-shaped
//! transcripts, continued through pretend `claude` and `codex` programs.

#[cfg(unix)]
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::*;
use crate::claude::api::tests::sse;
use crate::mods::tests::serve;

#[cfg(unix)]
fn write(p: &Path, b: &[u8]) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, b).unwrap();
}

#[cfg(unix)]
fn lines(v: &[Value]) -> String {
    v.iter().map(|l| format!("{l}\n")).collect()
}

#[cfg(unix)]
fn iso(at: i64) -> String {
    chrono::DateTime::from_timestamp(at, 0).unwrap().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Wait until nothing is running for this conversation.
fn settle(hub: &ChatHub, source: Source, id: &str) {
    let start = Instant::now();
    while hub.running(&(source, id.to_string())) {
        assert!(start.elapsed() < Duration::from_secs(20), "still running");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn claude_reply(text: &str, stop: &str) -> String {
    sse(&[
        json!({"type": "message_start", "message": {"model": "claude-opus-5-5", "usage": {"input_tokens": 50, "cache_read_input_tokens": 0}}}),
        json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
        json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": text}}),
        json!({"type": "content_block_stop", "index": 0}),
        json!({"type": "message_delta", "delta": {"stop_reason": stop}, "usage": {"output_tokens": 20}}),
        json!({"type": "message_stop"}),
    ])
}

fn texts(v: &ChatView) -> Vec<(Role, String)> {
    v.messages.iter().map(|m| (m.role, if m.role == Role::Tools { m.tools.join(" | ") } else { m.text.clone() })).collect()
}

#[test]
fn own_chats_with_claude_and_openai_stream_and_count() {
    let t = tempfile::tempdir().unwrap();
    let server = serve(Box::new(|req| {
        if req.path.starts_with("/v1/responses") {
            let body: Value = serde_json::from_slice(&req.body).unwrap();
            let n = body["input"].as_array().map_or(0, |a| a.len());
            let s: String = [
                json!({"type": "response.created", "response": {"model": "gpt-6-astra"}}),
                json!({"type": "response.output_text.delta", "delta": format!("GPT saw {n} message(s).")}),
                json!({"type": "response.completed", "response": {"model": "gpt-6-astra", "usage": {"input_tokens": 40, "input_tokens_details": {"cached_tokens": 10}, "output_tokens": 30}}}),
            ]
            .iter()
            .map(|e| format!("event: {}\ndata: {e}\n\n", e["type"].as_str().unwrap()))
            .collect();
            return Some((200, "text/event-stream", s.into_bytes()));
        }
        None
    }));
    let events = EventBus::new();
    let mut rx = events.subscribe();
    let claude = Arc::new(Claude::with_base(t.path(), &server.base));
    let openai = Arc::new(OpenAi::with_base(t.path(), &server.base));
    let hub = ChatHub::with(t.path(), events, claude.clone(), openai.clone(), Places::default());

    assert!(hub.create(NewKind::Claude, None, "Hello", "").unwrap_err().contains("Anthropic API key"));
    claude.set_test_key("sk-ant-test");
    openai.set_test_key("sk-proj-test");
    assert!(hub.create(NewKind::Claude, None, "   ", "").is_err());

    server.claude.lock().push_back(claude_reply("Hi there!", "end_turn"));
    let s = hub.create(NewKind::Claude, None, "Hello Claude\nsecond line", "").unwrap();
    assert_eq!((s.title.as_str(), s.provider.as_deref()), ("Hello Claude", Some("claude")));
    settle(&hub, Source::Omni, &s.id);
    let v = hub.view(Source::Omni, &s.id).unwrap();
    assert_eq!(texts(&v), vec![(Role::User, "Hello Claude\nsecond line".into()), (Role::Assistant, "Hi there!".into())]);
    let req: Value = serde_json::from_slice(&server.seen.lock().iter().find(|r| r.path == "/v1/messages").unwrap().body).unwrap();
    assert_eq!(req["model"], "claude-opus-5-5");
    assert_eq!(req["messages"], json!([{"role": "user", "content": "Hello Claude\nsecond line"}]));
    assert!(req["system"][0]["text"].as_str().unwrap().contains("OmniHub"));
    assert!(req["tools"].as_array().unwrap().iter().any(|t| t["name"] == "web_search"));

    // Streamed text reached the event bus; the second turn carries the history.
    let mut got = String::new();
    while let Ok(ev) = rx.try_recv() {
        if ev.topic == "chats:delta" && ev.payload["kind"] == "text" {
            got.push_str(ev.payload["text"].as_str().unwrap());
        }
    }
    assert_eq!(got, "Hi there!");
    server.claude.lock().push_back(claude_reply("A long answer", "max_tokens"));
    hub.send(Source::Omni, &s.id, "Tell me more", "").unwrap();
    assert!(hub.send(Source::Omni, &s.id, "again", "").unwrap_err().contains("still answering"));
    settle(&hub, Source::Omni, &s.id);
    let req: Value = serde_json::from_slice(&server.seen.lock().iter().rev().find(|r| r.path == "/v1/messages").unwrap().body).unwrap();
    assert_eq!(req["messages"].as_array().unwrap().len(), 3);
    let v = hub.view(Source::Omni, &s.id).unwrap();
    assert_eq!(v.messages.last().unwrap().role, Role::Notice, "the length limit is explained");

    // A refused request leaves the question and says why.
    server.claude.lock().push_back(sse(&[json!({"type": "error", "error": {"type": "invalid_request_error", "message": "prompt is too long"}})]));
    hub.send(Source::Omni, &s.id, "and this?", "").unwrap();
    settle(&hub, Source::Omni, &s.id);
    let v = hub.view(Source::Omni, &s.id).unwrap();
    let last = v.messages.last().unwrap();
    assert!(last.role == Role::Notice && last.tone.as_deref() == Some("bad") && last.text.contains("prompt is too long"));
    assert_eq!(v.messages[v.messages.len() - 2].text, "and this?");

    let g = hub.create(NewKind::Openai, None, "Hi GPT", "").unwrap();
    settle(&hub, Source::Omni, &g.id);
    assert_eq!(texts(&hub.view(Source::Omni, &g.id).unwrap())[1], (Role::Assistant, "GPT saw 1 message(s).".into()));
    let seen = server.seen.lock().iter().find(|r| r.path == "/v1/responses").cloned().unwrap();
    assert!(seen.headers.iter().any(|(k, v)| k == "authorization" && v == "Bearer sk-proj-test"));
    let req: Value = serde_json::from_slice(&seen.body).unwrap();
    assert_eq!((req["store"].clone(), req["stream"].clone()), (json!(false), json!(true)));

    let list = hub.list();
    assert_eq!(list.iter().filter(|c| c.source == Some(Source::Omni)).count(), 2);
    assert!(list.windows(2).all(|w| w[0].updated >= w[1].updated), "newest first");
    hub.rename(&g.id, "GPT test").unwrap();
    assert_eq!(hub.view(Source::Omni, &g.id).unwrap().summary.title, "GPT test");

    let u = hub.usage_report(usage::Tokens::default());
    assert_eq!(u.omni.claude.today.output, 40);
    assert_eq!((u.omni.openai.today.input, u.omni.openai.today.cache_read, u.omni.openai.today.output), (30, 10, 30));
    assert!(u.omni.claude.today.cost > 0.0);

    hub.delete(&g.id).unwrap();
    assert!(hub.view(Source::Omni, &g.id).is_err());
    assert!(hub.view(Source::Omni, "../../etc").is_err());
}

#[cfg(unix)]
fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let p = dir.join(name);
    std::fs::write(&p, format!("#!/bin/sh\n{body}")).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

#[cfg(unix)]
#[test]
fn claude_code_conversations_list_read_continue_and_count() {
    let t = tempfile::tempdir().unwrap();
    let home = t.path().join(".claude");
    let proj = t.path().join("work").join("shop");
    std::fs::create_dir_all(&proj).unwrap();
    let id = "5d5e2c1e-8f39-4c39-9a71-0b6d2a6e9f10";
    let now = crate::db::now();
    let file = home.join("projects").join("-work-shop").join(format!("{id}.jsonl"));
    let cwd = proj.to_string_lossy().to_string();
    let base = json!({"cwd": cwd, "sessionId": id, "gitBranch": "main"});
    let with = |mut v: Value, at: i64| {
        for (k, val) in base.as_object().unwrap() {
            v[k] = val.clone();
        }
        v["timestamp"] = json!(iso(at));
        v
    };
    let usage = |i: u64, o: u64| json!({"input_tokens": i, "output_tokens": o, "cache_read_input_tokens": 1000, "cache_creation_input_tokens": 0});
    write(
        &file,
        lines(&[
            with(json!({"type": "user", "message": {"role": "user", "content": "Fix the login bug"}, "origin": {"kind": "human"}}), now - 7200),
            with(json!({"type": "assistant", "message": {"id": "msg_1", "model": "claude-opus-5-5", "content": [{"type": "thinking", "thinking": "secret thoughts"}], "usage": usage(10, 5)}}), now - 7190),
            with(json!({"type": "assistant", "message": {"id": "msg_1", "model": "claude-opus-5-5", "content": [{"type": "text", "text": "Looking."}, {"type": "tool_use", "name": "Bash", "input": {"command": "npm test", "description": "Run the tests"}}], "usage": usage(10, 5)}}), now - 7189),
            with(json!({"type": "user", "message": {"role": "user", "content": [{"type": "tool_result", "content": "ok"}]}}), now - 7180),
            with(json!({"type": "user", "isMeta": true, "message": {"role": "user", "content": "hidden context"}}), now - 7179),
            with(json!({"type": "user", "origin": {"kind": "task-notification"}, "message": {"role": "user", "content": "<task-notification>done</task-notification>"}}), now - 7178),
            with(json!({"type": "assistant", "message": {"id": "msg_2", "model": "claude-opus-5-5", "content": [{"type": "tool_use", "name": "Edit", "input": {"file_path": "/x/src/login.ts"}}], "usage": usage(20, 10)}}), now - 7170),
            with(json!({"type": "user", "message": {"role": "user", "content": "<command-name>/model</command-name>\n<command-args>sonnet</command-args>"}}), now - 3600),
            with(json!({"type": "user", "message": {"role": "user", "content": "Thanks<system-reminder>be nice</system-reminder>"}}), now - 3500),
            with(json!({"type": "assistant", "quotaLimits": {"status": "rejected", "rateLimitType": "five_hour", "resetsAt": now + 1800}, "message": {"id": "msg_3", "model": "claude-sonnet-5-5", "content": [{"type": "text", "text": "Fixed it."}], "usage": usage(30, 15)}}), now - 3400),
            with(json!({"type": "system", "subtype": "compact_boundary"}), now - 3300),
            json!({"type": "custom-title", "customTitle": "Login fix", "sessionId": id}),
        ])
        .as_bytes(),
    );

    let bin = t.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let log = t.path().join("claude-args.txt");
    let input = t.path().join("claude-stdin.txt");
    let reply = lines(&[
        json!({"type": "system", "subtype": "init", "session_id": id}),
        json!({"type": "stream_event", "parent_tool_use_id": null, "event": {"type": "content_block_delta", "delta": {"type": "text_delta", "text": "Adding "}}}),
        json!({"type": "stream_event", "parent_tool_use_id": null, "event": {"type": "content_block_delta", "delta": {"type": "text_delta", "text": "a test."}}}),
        json!({"type": "stream_event", "parent_tool_use_id": "toolu_x", "event": {"type": "content_block_delta", "delta": {"type": "text_delta", "text": "helper's words"}}}),
        json!({"type": "assistant", "parent_tool_use_id": null, "message": {"content": [{"type": "text", "text": "Adding a test."}, {"type": "tool_use", "name": "Write", "input": {"file_path": "/x/src/login.test.ts"}}]}}),
        json!({"type": "result", "subtype": "success", "is_error": false, "result": "Adding a test.", "total_cost_usd": 0.12, "permission_denials": [{"tool_name": "Bash", "tool_input": {"command": "rm -rf build"}}]}),
    ]);
    let appended = lines(&[
        with(json!({"type": "user", "message": {"role": "user", "content": "Now add a test"}}), now - 10),
        with(json!({"type": "assistant", "message": {"id": "msg_4", "model": "claude-opus-5-5", "content": [{"type": "text", "text": "Adding a test."}], "usage": usage(40, 20)}}), now - 5),
    ]);
    let exe = script(
        &bin,
        "claude",
        &format!("echo \"$@\" >> '{}'\ncat > '{}'\ncat <<'EOF'\n{reply}EOF\ncase \"$*\" in *--resume*) cat >> '{}' <<'EOF'\n{appended}EOF\n;; esac\n", log.display(), input.display(), file.display()),
    );

    let events = EventBus::new();
    let mut rx = events.subscribe();
    let claude = Arc::new(Claude::new(t.path()));
    let hub = ChatHub::with(t.path(), events, claude, Arc::new(OpenAi::new(t.path())), Places { claude_home: Some(home.clone()), claude_exe: Some(exe), ..Default::default() });

    let list = hub.list();
    let s = list.iter().find(|c| c.source == Some(Source::ClaudeCode)).unwrap();
    assert_eq!((s.id.as_str(), s.title.as_str(), s.folder.as_deref(), s.branch.as_deref(), s.model.as_deref()), (id, "Login fix", Some(cwd.as_str()), Some("main"), Some("claude-sonnet-5-5")));
    assert_eq!(s.preview, "Fixed it.");
    let v = hub.view(Source::ClaudeCode, id).unwrap();
    assert_eq!(
        texts(&v),
        vec![
            (Role::User, "Fix the login bug".into()),
            (Role::Assistant, "Looking.".into()),
            (Role::Tools, "Run the tests | Edited login.ts".into()),
            (Role::User, "/model sonnet".into()),
            (Role::User, "Thanks".into()),
            (Role::Assistant, "Fixed it.".into()),
            (Role::Notice, "The conversation was summarized here to make room.".into()),
        ]
    );
    assert!(!v.messages.iter().any(|m| m.text.contains("secret") || m.text.contains("hidden") || m.text.contains("be nice")));
    assert!(hub.folders().iter().any(|f| f.path == cwd && f.claude_code));
    assert!(hub.known_folder(&format!("{cwd}/")));

    let u = hub.usage_report(usage::Tokens::default()).claude_code;
    assert!(u.found);
    assert_eq!((u.period.month.input, u.period.month.output), (60, 30), "msg_1 counted once");
    assert!(u.window.is_some(), "used in the last 5 hours");
    assert_eq!(u.limits.len(), 1);
    assert!(u.limits[0].reached && u.limits[0].name == "5-hour limit" && u.limits[0].resets_at == Some(now + 1800));

    hub.send(Source::ClaudeCode, id, "Now add a test --dangerously-skip-permissions", "acceptEdits").unwrap();
    settle(&hub, Source::ClaudeCode, id);
    let args = std::fs::read_to_string(&log).unwrap();
    assert!(args.contains(&format!("-p --output-format stream-json --verbose --include-partial-messages --permission-mode acceptEdits --resume {id}")), "{args}");
    assert_eq!(std::fs::read_to_string(&input).unwrap(), "Now add a test --dangerously-skip-permissions", "the message goes on stdin, never as flags");
    let (mut text, mut tools, mut notice, mut done) = (String::new(), Vec::new(), String::new(), false);
    while let Ok(ev) = rx.try_recv() {
        if ev.topic != "chats:delta" {
            continue;
        }
        let t = ev.payload["text"].as_str().unwrap_or_default().to_string();
        match ev.payload["kind"].as_str().unwrap() {
            "text" => text.push_str(&t),
            "tool" => tools.push(t),
            "notice" => notice = t,
            "done" => done = true,
            other => panic!("unexpected {other}: {t}"),
        }
    }
    assert_eq!(text, "Adding a test.", "a helper's words stay out");
    assert_eq!(tools, vec!["Wrote login.test.ts"]);
    assert!(notice.contains("Ran rm -rf build") && done);
    let v = hub.view(Source::ClaudeCode, id).unwrap();
    assert_eq!(v.messages.last().unwrap().text, "Adding a test.");
    assert!(!v.busy_elsewhere, "OmniHub's own change");
    let u2 = hub.usage_report(usage::Tokens::default()).claude_code;
    assert_eq!((u2.period.month.input, u2.period.month.output), (100, 50), "only the new lines were added");

    std::fs::write(&log, "").unwrap();
    hub.send(Source::ClaudeCode, id, "go", "bypassPermissions").unwrap();
    settle(&hub, Source::ClaudeCode, id);
    assert!(std::fs::read_to_string(&log).unwrap().contains("--permission-mode default"), "never skips permissions");

    std::fs::write(&log, "").unwrap();
    let n = hub.create(NewKind::ClaudeCode, Some(&cwd), "Start something", "plan").unwrap();
    assert!(uuid::Uuid::parse_str(&n.id).is_ok());
    settle(&hub, Source::ClaudeCode, &n.id);
    assert!(std::fs::read_to_string(&log).unwrap().contains(&format!("--permission-mode plan --session-id {}", n.id)));
    assert!(hub.create(NewKind::ClaudeCode, Some("relative/dir"), "x", "plan").is_err());
    assert!(hub.create(NewKind::ClaudeCode, Some(&t.path().join("missing").to_string_lossy()), "x", "plan").is_err());
}

#[cfg(unix)]
#[test]
fn codex_sessions_list_read_continue_and_rate_limits() {
    let t = tempfile::tempdir().unwrap();
    let home = t.path().join(".codex");
    let proj = t.path().join("site");
    std::fs::create_dir_all(&proj).unwrap();
    let cwd = proj.to_string_lossy().to_string();
    let id = "0199a213-81c0-7800-8aa1-bbab2a035a53";
    let now = crate::db::now();
    let at = |s: i64| json!(iso(now - s));
    let file = home.join("sessions/2026/10/19").join(format!("rollout-2026-10-19T10-00-00-{id}.jsonl"));
    let tc = |i: u64, c: u64, o: u64, s: i64| json!({"timestamp": at(s), "type": "event_msg", "payload": {"type": "token_count", "info": {"total_token_usage": {"input_tokens": i, "cached_input_tokens": c, "output_tokens": o, "reasoning_output_tokens": 0, "total_tokens": i + o}}, "rate_limits": {"plan_type": "plus", "primary": {"used_percent": 42.0, "window_minutes": 300, "resets_at": now + 7200}, "secondary": {"used_percent": 12.5, "window_minutes": 10080, "resets_at": now + 3 * 86400}}}});
    write(
        &file,
        lines(&[
            json!({"timestamp": at(600), "type": "session_meta", "payload": {"id": id, "timestamp": at(600), "cwd": cwd, "git": {"branch": "dev"}}}),
            json!({"timestamp": at(600), "type": "turn_context", "payload": {"cwd": cwd, "model": "gpt-6-codex", "approval_policy": "on-request"}}),
            json!({"timestamp": at(599), "type": "response_item", "payload": {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "<environment_context>cwd</environment_context>"}]}}),
            json!({"timestamp": at(598), "type": "response_item", "payload": {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "Add dark mode"}]}}),
            json!({"timestamp": at(598), "type": "event_msg", "payload": {"type": "user_message", "message": "Add dark mode"}}),
            json!({"timestamp": at(590), "type": "response_item", "payload": {"type": "function_call", "name": "shell", "arguments": "{\"command\":[\"bash\",\"-lc\",\"npm run build\"]}", "call_id": "c1"}}),
            json!({"timestamp": at(580), "type": "response_item", "payload": {"type": "custom_tool_call", "name": "apply_patch", "input": "*** Begin Patch\n*** Update File: src/theme.ts\n@@\n*** End Patch", "call_id": "c2"}}),
            json!({"timestamp": at(570), "type": "response_item", "payload": {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "Dark mode is in."}]}}),
            json!({"timestamp": at(570), "type": "event_msg", "payload": {"type": "agent_message", "message": "Dark mode is in."}}),
            tc(1000, 600, 200, 560),
            tc(1000, 600, 200, 559),
            tc(1500, 900, 260, 550),
        ])
        .as_bytes(),
    );
    write(&home.join("session_index.jsonl"), lines(&[json!({"id": id, "thread_name": "Old name", "updated_at": "x"}), json!({"id": id, "thread_name": "Dark mode", "updated_at": "y"})]).as_bytes());
    let old = "11111111-2222-4333-8444-555555555555";
    let archived = lines(&[
        json!({"timestamp": "2026-09-01T08:00:00Z", "type": "session_meta", "payload": {"id": old, "cwd": cwd}}),
        json!({"timestamp": "2026-09-01T08:00:01Z", "type": "event_msg", "payload": {"type": "user_message", "message": "An old question"}}),
    ]);
    write(&home.join("archived_sessions").join(format!("rollout-2026-09-01T08-00-00-{old}.jsonl.zst")), &ruzstd::encoding::compress_to_vec(archived.as_bytes(), ruzstd::encoding::CompressionLevel::Fastest));

    let log = t.path().join("codex-args.txt");
    let input = t.path().join("codex-stdin.txt");
    let new_id = "0199b000-0000-7000-8000-000000000001";
    let reply = |thread: &str| {
        lines(&[
            json!({"type": "thread.started", "thread_id": thread}),
            json!({"type": "turn.started"}),
            json!({"type": "item.completed", "item": {"id": "i0", "type": "reasoning", "text": "**Planning the change**"}}),
            json!({"type": "item.started", "item": {"id": "i1", "type": "command_execution", "command": "npm test", "status": "in_progress"}}),
            json!({"type": "item.completed", "item": {"id": "i2", "type": "file_change", "changes": [{"path": "src/theme.ts", "kind": "update"}], "status": "completed"}}),
            json!({"type": "item.completed", "item": {"id": "i3", "type": "agent_message", "text": "More contrast now."}}),
            json!({"type": "turn.completed", "usage": {"input_tokens": 10, "cached_input_tokens": 0, "output_tokens": 5}}),
        ])
    };
    let exe = script(
        t.path(),
        "codex",
        &format!("echo \"$@\" >> '{}'\ncat > '{}'\ncase \"$*\" in *resume*) cat <<'EOF'\n{}EOF\n;; *) cat <<'EOF'\n{}EOF\n;; esac\n", log.display(), input.display(), reply(id), reply(new_id)),
    );
    let events = EventBus::new();
    let mut rx = events.subscribe();
    let hub = ChatHub::with(t.path(), events, Arc::new(Claude::new(t.path())), Arc::new(OpenAi::new(t.path())), Places { codex_home: Some(home.clone()), codex_exe: Some(exe), ..Default::default() });

    let list = hub.list();
    let s = list.iter().find(|c| c.id == id).unwrap();
    assert_eq!((s.title.as_str(), s.folder.as_deref(), s.branch.as_deref(), s.model.as_deref(), s.preview.as_str()), ("Dark mode", Some(cwd.as_str()), Some("dev"), Some("gpt-6-codex"), "Dark mode is in."));
    assert_eq!(list.iter().find(|c| c.id == old).unwrap().title, "An old question", "compressed sessions are read too");
    let v = hub.view(Source::Codex, id).unwrap();
    assert_eq!(texts(&v), vec![(Role::User, "Add dark mode".into()), (Role::Tools, "Ran npm run build | Edited theme.ts".into()), (Role::Assistant, "Dark mode is in.".into())]);

    let u = hub.usage_report(usage::Tokens::default()).codex;
    assert!(u.found && u.plan.as_deref() == Some("plus"));
    assert_eq!((u.period.today.input, u.period.today.cache_read, u.period.today.output), (600, 900, 260), "each report counted once");
    let names: Vec<(String, Option<f64>)> = u.limits.iter().map(|l| (l.name.clone(), l.used_percent)).collect();
    assert_eq!(names, vec![("5-hour limit".into(), Some(42.0)), ("Weekly limit".into(), Some(12.5))]);

    hub.send(Source::Codex, id, "More contrast", "workspace-write").unwrap();
    settle(&hub, Source::Codex, id);
    assert_eq!(std::fs::read_to_string(&log).unwrap().trim(), format!("exec --json --skip-git-repo-check --sandbox workspace-write resume {id} -"));
    assert_eq!(std::fs::read_to_string(&input).unwrap(), "More contrast");
    let mut seen = Vec::new();
    while let Ok(ev) = rx.try_recv() {
        if ev.topic == "chats:delta" {
            seen.push(format!("{}:{}", ev.payload["kind"].as_str().unwrap(), ev.payload["text"].as_str().unwrap()));
        }
    }
    assert_eq!(seen, vec!["note:Planning the change", "tool:Ran npm test", "tool:Edited theme.ts", "text:More contrast now.", "done:"]);

    std::fs::write(&log, "").unwrap();
    let n = hub.create(NewKind::Codex, Some(&cwd), "Start fresh", "danger-full-access").unwrap();
    assert_eq!(n.id, new_id, "the id Codex gave");
    settle(&hub, Source::Codex, new_id);
    assert_eq!(std::fs::read_to_string(&log).unwrap().trim(), "exec --json --skip-git-repo-check --sandbox read-only -", "never full access");

    // A Codex that can't run says why.
    let broken = script(t.path(), "codex-broken", "cat > /dev/null\necho 'Error: Not logged in. Run codex login.' >&2\nexit 1\n");
    hub.places.lock().codex_exe = Some(broken);
    let err = hub.create(NewKind::Codex, Some(&cwd), "Hi", "read-only").unwrap_err();
    assert!(err.contains("Not logged in"), "{err}");
    hub.places.lock().codex_exe = Some(t.path().join("gone"));
    assert!(hub.send(Source::Codex, id, "x", "read-only").unwrap_err().contains("isn't installed"));
}

#[test]
fn ideas_are_notes_that_say_where_they_came_from() {
    let t = tempfile::tempdir().unwrap();
    let core = crate::core::AppCore::new(crate::paths::AppPaths::at(t.path()).unwrap(), crate::core::CoreOptions { vault_dpapi: Some(false), ..Default::default() }).unwrap();
    let n = save_idea(&core, "## A shop for mods\nSell Balatro jokers.", Some("Claude Code · Login fix")).unwrap();
    assert_eq!((n.kind, n.title.as_str()), (crate::notes::NoteKind::Idea, "A shop for mods"));
    assert!(n.body.ends_with("— from Claude Code · Login fix") && n.tags.contains(&"chat".to_string()));
    assert!(save_idea(&core, "  ", None).is_err());
}
