use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Deserialize;

/// Claude Code hook 落盘的最近一次事件，文件名为 <cliSessionId>.json，后写覆盖先写
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookKind {
    /// 正在等用户授权或回答（PermissionRequest / Notification permission_prompt）
    PermissionPrompt,
    /// 用户提交了 prompt，或工具开始/结束执行
    Busy,
    /// 回合结束或会话退出
    Stopped,
    Other,
}

#[derive(Debug, Clone, Copy)]
pub struct HookSignal {
    pub kind: HookKind,
    pub at_ms: u64,
}

#[derive(Deserialize)]
struct HookPayload {
    hook_event_name: String,
    notification_type: Option<String>,
}

pub fn hooks_dir(home: &Path) -> PathBuf {
    home.join(".config/spark/hooks")
}

pub fn read_signal(dir: &Path, cli_session_id: &str) -> Option<HookSignal> {
    let path = dir.join(format!("{cli_session_id}.json"));
    let content = fs::read_to_string(&path).ok()?;
    let modified = fs::metadata(&path).ok()?.modified().ok()?;
    Some(HookSignal {
        kind: classify(&content)?,
        at_ms: modified.duration_since(UNIX_EPOCH).ok()?.as_millis() as u64,
    })
}

fn classify(payload: &str) -> Option<HookKind> {
    let payload: HookPayload = serde_json::from_str(payload).ok()?;
    Some(match payload.hook_event_name.as_str() {
        "PermissionRequest" => HookKind::PermissionPrompt,
        "Notification" => match payload.notification_type.as_deref() {
            Some("permission_prompt") | Some("elicitation_dialog") => HookKind::PermissionPrompt,
            _ => HookKind::Other,
        },
        "UserPromptSubmit" | "PreToolUse" | "PostToolUse" => HookKind::Busy,
        "Stop" | "SessionEnd" => HookKind::Stopped,
        _ => HookKind::Other,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_hook_events() {
        assert_eq!(classify(r#"{"session_id":"s","hook_event_name":"PermissionRequest","tool_name":"Bash"}"#), Some(HookKind::PermissionPrompt));
        assert_eq!(classify(r#"{"hook_event_name":"Notification","notification_type":"permission_prompt"}"#), Some(HookKind::PermissionPrompt));
        assert_eq!(classify(r#"{"hook_event_name":"Notification","notification_type":"idle_prompt"}"#), Some(HookKind::Other));
        assert_eq!(classify(r#"{"hook_event_name":"PreToolUse","tool_name":"Bash"}"#), Some(HookKind::Busy));
        assert_eq!(classify(r#"{"hook_event_name":"Stop"}"#), Some(HookKind::Stopped));
        assert_eq!(classify("garbage"), None);
    }

    #[test]
    fn reads_signal_from_session_file() {
        let dir = std::env::temp_dir().join(format!("spark-hooks-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("abc.json"), r#"{"hook_event_name":"UserPromptSubmit","session_id":"abc"}"#).unwrap();

        let signal = read_signal(&dir, "abc").unwrap();
        assert_eq!(signal.kind, HookKind::Busy);
        assert!(signal.at_ms > 0);
        assert!(read_signal(&dir, "missing").is_none());
        fs::remove_dir_all(&dir).unwrap();
    }
}
