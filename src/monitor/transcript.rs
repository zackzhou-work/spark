use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Deserialize;

/// transcript 末尾记录反映的回合进度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnEvent {
    /// 模型正在生成：用户刚提交、工具结果已回填、或消息尚未收尾
    Generating,
    /// 模型发出工具调用后尚无结果：工具执行中，或在等用户授权
    ToolPending,
    /// 工具本身就是在向用户提问（AskUserQuestion / ExitPlanMode）
    AwaitingUser,
    /// 回合已结束：end_turn、stop hook 已执行、或被用户打断
    Ended,
}

#[derive(Debug, Clone, Copy)]
pub struct TranscriptActivity {
    pub last_event: TurnEvent,
    pub last_modified_ms: u64,
}

const TAIL_BYTES: u64 = 64 * 1024;
const USER_FACING_TOOLS: &[&str] = &["AskUserQuestion", "ExitPlanMode"];
const INTERRUPT_MARK: &str = "[Request interrupted by user";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    #[serde(rename = "type")]
    kind: String,
    subtype: Option<String>,
    #[serde(default)]
    is_sidechain: bool,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    stop_reason: Option<String>,
    content: Option<serde_json::Value>,
}

/// Claude Code 把 cwd 里所有非字母数字字符换成 '-' 作为项目目录名
pub fn locate(projects_root: &Path, cli_session_id: &str, cwd_candidates: &[&str]) -> Option<PathBuf> {
    cwd_candidates
        .iter()
        .filter(|c| !c.is_empty())
        .map(|c| projects_root.join(encode_cwd(c)).join(format!("{cli_session_id}.jsonl")))
        .find(|p| p.is_file())
}

fn encode_cwd(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

pub fn read_activity(transcript: &Path) -> Option<TranscriptActivity> {
    let last_event = last_turn_event(transcript)?;
    let last_modified_ms = mtime_ms(transcript)
        .max(newest_subagent_mtime_ms(transcript))
        .unwrap_or(0);
    Some(TranscriptActivity {
        last_event,
        last_modified_ms,
    })
}

fn last_turn_event(transcript: &Path) -> Option<TurnEvent> {
    let mut file = File::open(transcript).ok()?;
    let len = file.metadata().ok()?.len();
    let start = len.saturating_sub(TAIL_BYTES);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut raw = Vec::with_capacity((len - start) as usize);
    file.read_to_end(&mut raw).ok()?;
    let text = String::from_utf8_lossy(&raw);

    let mut lines = text.lines();
    if start > 0 {
        lines.next();
    }
    lines.rev().find_map(classify)
}

fn classify(line: &str) -> Option<TurnEvent> {
    let record: Record = serde_json::from_str(line).ok()?;
    if record.is_sidechain {
        return None;
    }
    match record.kind.as_str() {
        "system" => (record.subtype.as_deref() == Some("stop_hook_summary")).then_some(TurnEvent::Ended),
        "user" => {
            let interrupted = record.message.as_ref().map_or(false, |m| has_text_prefix(m, INTERRUPT_MARK));
            Some(if interrupted { TurnEvent::Ended } else { TurnEvent::Generating })
        }
        "assistant" => {
            let message = record.message?;
            Some(match message.stop_reason.as_deref() {
                Some("end_turn") | Some("stop_sequence") => TurnEvent::Ended,
                Some("tool_use") if uses_tool(&message, USER_FACING_TOOLS) => TurnEvent::AwaitingUser,
                Some("tool_use") => TurnEvent::ToolPending,
                _ => TurnEvent::Generating,
            })
        }
        _ => None,
    }
}

fn content_blocks(message: &Message) -> impl Iterator<Item = &serde_json::Value> {
    message
        .content
        .as_ref()
        .and_then(|c| c.as_array())
        .into_iter()
        .flatten()
}

fn has_text_prefix(message: &Message, prefix: &str) -> bool {
    content_blocks(message)
        .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
        .any(|t| t.starts_with(prefix))
}

fn uses_tool(message: &Message, names: &[&str]) -> bool {
    content_blocks(message)
        .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
        .filter_map(|b| b.get("name").and_then(|n| n.as_str()))
        .any(|n| names.contains(&n))
}

fn mtime_ms(path: &Path) -> Option<u64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    Some(modified.duration_since(UNIX_EPOCH).ok()?.as_millis() as u64)
}

/// 子 agent 的 transcript 放在 <目录>/<会话ID>/subagents/ 下，主文件在它们运行期间不会更新
fn newest_subagent_mtime_ms(transcript: &Path) -> Option<u64> {
    let session_dir = transcript.with_extension("").join("subagents");
    fs::read_dir(session_dir)
        .ok()?
        .flatten()
        .filter_map(|entry| mtime_ms(&entry.path()))
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event_of(lines: &[&str]) -> Option<TurnEvent> {
        lines.iter().rev().find_map(|l| classify(l))
    }

    const END_TURN: &str = r#"{"type":"assistant","message":{"role":"assistant","stop_reason":"end_turn","content":[{"type":"text","text":"done"}]}}"#;
    const TOOL_USE: &str = r#"{"type":"assistant","message":{"role":"assistant","stop_reason":"tool_use","content":[{"type":"tool_use","name":"Bash","input":{}}]}}"#;
    const ASK_USER: &str = r#"{"type":"assistant","message":{"role":"assistant","stop_reason":"tool_use","content":[{"type":"tool_use","name":"AskUserQuestion","input":{}}]}}"#;
    const TOOL_RESULT: &str = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","content":"ok"}]}}"#;
    const PROMPT: &str = r#"{"type":"user","message":{"role":"user","content":"hello"}}"#;
    const INTERRUPTED: &str = r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"[Request interrupted by user for tool use]"}]}}"#;
    const STOP_HOOK: &str = r#"{"type":"system","subtype":"stop_hook_summary","hookCount":1}"#;
    const BRIDGE: &str = r#"{"type":"bridge-session","sessionId":"x"}"#;
    const LAST_PROMPT: &str = r#"{"type":"last-prompt","lastPrompt":"hi"}"#;
    const SIDECHAIN: &str = r#"{"type":"assistant","isSidechain":true,"message":{"role":"assistant","stop_reason":"tool_use","content":[]}}"#;

    #[test]
    fn end_turn_is_ended_even_with_trailing_metadata() {
        assert_eq!(event_of(&[TOOL_RESULT, END_TURN, STOP_HOOK, BRIDGE, LAST_PROMPT]), Some(TurnEvent::Ended));
        assert_eq!(event_of(&[TOOL_RESULT, END_TURN]), Some(TurnEvent::Ended));
    }

    #[test]
    fn pending_tool_call() {
        assert_eq!(event_of(&[PROMPT, TOOL_USE]), Some(TurnEvent::ToolPending));
        assert_eq!(event_of(&[PROMPT, ASK_USER]), Some(TurnEvent::AwaitingUser));
    }

    #[test]
    fn generating_after_prompt_or_tool_result() {
        assert_eq!(event_of(&[END_TURN, PROMPT]), Some(TurnEvent::Generating));
        assert_eq!(event_of(&[TOOL_USE, TOOL_RESULT]), Some(TurnEvent::Generating));
    }

    #[test]
    fn user_interrupt_ends_turn() {
        assert_eq!(event_of(&[TOOL_USE, INTERRUPTED]), Some(TurnEvent::Ended));
    }

    #[test]
    fn sidechain_and_unknown_records_are_skipped() {
        assert_eq!(event_of(&[END_TURN, SIDECHAIN, BRIDGE]), Some(TurnEvent::Ended));
        assert_eq!(event_of(&[BRIDGE, "not json"]), None);
    }

    #[test]
    fn read_activity_from_file_tail() {
        let dir = std::env::temp_dir().join(format!("spark-transcript-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("abc.jsonl");
        let mut body = String::new();
        for _ in 0..2000 {
            body.push_str(TOOL_RESULT);
            body.push('\n');
        }
        body.push_str(END_TURN);
        body.push('\n');
        fs::write(&path, body).unwrap();

        let activity = read_activity(&path).unwrap();
        assert_eq!(activity.last_event, TurnEvent::Ended);
        assert!(activity.last_modified_ms > 0);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cwd_encoding_matches_claude_code() {
        assert_eq!(
            encode_cwd("/Users/me/work/spark/.claude/worktrees/x-1"),
            "-Users-me-work-spark--claude-worktrees-x-1"
        );
    }
}
