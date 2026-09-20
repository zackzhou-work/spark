use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sysinfo::{ProcessRefreshKind, System};

use super::hooks::{self, HookKind, HookSignal};
use super::transcript::{self, TranscriptActivity, TurnEvent};
use crate::platform::host_session_id;
use crate::state::{ProjectGroup, SessionItem, TaskState};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSessionJson {
    session_id: Option<String>,
    cli_session_id: Option<String>,
    title: Option<String>,
    origin_cwd: Option<String>,
    cwd: Option<String>,
    worktree_path: Option<String>,
    #[serde(default)]
    is_archived: bool,
    #[serde(default)]
    last_activity_at: u64,
    #[serde(default)]
    created_at: u64,
}

/// 与某个桌面端会话对应的存活 Claude 进程
#[derive(Debug, Clone, Copy)]
pub struct LiveProcess {
    pub pid: u32,
    pub has_children: bool,
    pub cpu_usage: f32,
}

pub struct ProcessScanner {
    sys: System,
    sessions_dir: PathBuf,
    projects_root: PathBuf,
    hooks_dir: PathBuf,
}

/// 工具调用挂起且 transcript 无更新超过此时长，视为在等用户授权
const TOOL_PENDING_IDLE_MS: u64 = 45_000;
/// transcript 尚未生成时，刚提交的会话仍按运行中处理
const FRESH_SESSION_MS: u64 = 60_000;

impl ProcessScanner {
    pub fn new() -> Self {
        let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/Users/zhouzhou74".to_string()));
        Self {
            sys: System::new(),
            sessions_dir: home.join("Library/Application Support/Claude/claude-code-sessions"),
            projects_root: home.join(".claude/projects"),
            hooks_dir: hooks::hooks_dir(&home),
        }
    }

    /// 需要监听变化的目录；hooks 目录先建好，否则无法被监听
    pub fn watch_dirs(&self) -> Vec<PathBuf> {
        let _ = fs::create_dir_all(&self.hooks_dir);
        vec![
            self.sessions_dir.clone(),
            self.projects_root.clone(),
            self.hooks_dir.clone(),
        ]
    }

    /// 通过进程环境变量把每个存活的 Claude 核心进程精确归到宿主会话 ID
    fn get_live_processes(&mut self) -> HashMap<String, LiveProcess> {
        self.sys
            .refresh_processes_specifics(ProcessRefreshKind::new().with_cpu());
        let processes = self.sys.processes();
        let mut live = HashMap::new();

        for (pid, process) in processes {
            if !process.name().eq_ignore_ascii_case("claude") {
                continue;
            }
            let Some(session_id) = host_session_id(pid.as_u32()) else {
                continue;
            };
            let has_children = processes.values().any(|p| p.parent() == Some(*pid));
            live.insert(
                session_id,
                LiveProcess {
                    pid: pid.as_u32(),
                    has_children,
                    cpu_usage: process.cpu_usage(),
                },
            );
        }

        live
    }

    /// 扫描真实 Claude Desktop 的所有未归档会话
    pub fn scan_claude_sessions(&mut self) -> Vec<ProjectGroup> {
        let live = self.get_live_processes();

        if !self.sessions_dir.exists() {
            return Vec::new();
        }

        let now_ms = chrono::Local::now().timestamp_millis() as u64;
        let today = chrono::Local::now().date_naive();
        let projects_root = self.projects_root.clone();
        let hooks_dir = self.hooks_dir.clone();
        let mut discovered = Vec::new();

        Self::find_json_files(&self.sessions_dir, &mut |file_path| {
            let Ok(content) = fs::read_to_string(file_path) else {
                return;
            };
            let Ok(data) = serde_json::from_str::<RawSessionJson>(&content) else {
                return;
            };
            if data.is_archived {
                return;
            }

            let origin_cwd = data.origin_cwd.or(data.cwd.clone()).unwrap_or_default();
            let project_name = if !origin_cwd.is_empty() {
                Path::new(&origin_cwd)
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Unknown".to_string())
            } else {
                "Unknown".to_string()
            };

            let title = data.title.unwrap_or_else(|| "Untitled Session".to_string());
            if project_name == "Unknown" && title == "Untitled Session" {
                return;
            }

            let session_id = data.session_id.unwrap_or_else(|| "session".to_string());
            let process = live.get(&session_id).copied();

            let cli_session_id = process.and(data.cli_session_id.as_deref());
            let hook = cli_session_id.and_then(|id| hooks::read_signal(&hooks_dir, id));
            let activity = cli_session_id.and_then(|cli_session_id| {
                let candidates = [
                    data.cwd.as_deref().unwrap_or(""),
                    data.worktree_path.as_deref().unwrap_or(""),
                    origin_cwd.as_str(),
                ];
                let path = transcript::locate(&projects_root, cli_session_id, &candidates)?;
                transcript::read_activity(&path)
            });

            let state = Self::determine_session_state(
                process.as_ref(),
                activity.as_ref(),
                hook.as_ref(),
                data.last_activity_at,
                now_ms,
            );

            let activity_ts = data.last_activity_at.max(data.created_at);
            let is_today = if state == TaskState::Running || state == TaskState::Waiting {
                true
            } else if activity_ts > 0 {
                let naive_date = if activity_ts > 100_000_000_000 {
                    chrono::DateTime::from_timestamp_millis(activity_ts as i64)
                        .map(|utc| utc.with_timezone(&chrono::Local).date_naive())
                } else {
                    chrono::DateTime::from_timestamp(activity_ts as i64, 0)
                        .map(|utc| utc.with_timezone(&chrono::Local).date_naive())
                };
                naive_date == Some(today)
            } else {
                false
            };

            discovered.push((
                project_name,
                SessionItem {
                    id: session_id,
                    title,
                    state,
                    pid: process.map(|p| p.pid as i32),
                    working_dir: origin_cwd,
                    is_today,
                },
                activity_ts,
            ));
        });

        if discovered.is_empty() {
            return Vec::new();
        }

        Self::group_by_project(discovered)
    }

    fn group_by_project(discovered: Vec<(String, SessionItem, u64)>) -> Vec<ProjectGroup> {
        let priority = |state: TaskState| match state {
            TaskState::Running => 2,
            TaskState::Waiting => 1,
            TaskState::Completed => 0,
        };

        let mut groups_map: BTreeMap<String, Vec<(SessionItem, u64)>> = BTreeMap::new();
        for (proj, item, last_act) in discovered {
            groups_map.entry(proj).or_default().push((item, last_act));
        }

        let mut groups = Vec::new();
        for (proj, mut items) in groups_map {
            // 项目内会话：Running/Waiting 优先，其余按最后活动时间倒序排列
            items.sort_by(|a, b| {
                priority(b.0.state)
                    .cmp(&priority(a.0.state))
                    .then_with(|| b.1.cmp(&a.1))
            });
            let sessions = items.into_iter().map(|(item, _)| item).collect();
            groups.push(ProjectGroup {
                project_name: proj,
                sessions,
            });
        }

        // 排序项目：有进行中/待处理任务的项目优先置顶显示，其余按字母排序
        let group_priority = |g: &ProjectGroup| g.sessions.iter().map(|s| priority(s.state)).max().unwrap_or(0);
        groups.sort_by(|a, b| {
            group_priority(b)
                .cmp(&group_priority(a))
                .then_with(|| a.project_name.to_lowercase().cmp(&b.project_name.to_lowercase()))
        });

        groups
    }

    /// 状态判决：
    /// - Completed: 没有存活进程，或 transcript 显示回合已结束
    /// - Waiting: hook 报告正在等授权且 transcript 之后没有新动静；工具在向用户提问；
    ///   没装 hook 时，工具调用挂起且进程、transcript 都长时间无动静也视为等授权
    /// - Running: 其余存活会话；transcript 尚未生成时按提交时间兜底
    pub fn determine_session_state(
        process: Option<&LiveProcess>,
        activity: Option<&TranscriptActivity>,
        hook: Option<&HookSignal>,
        last_activity_at: u64,
        now_ms: u64,
    ) -> TaskState {
        let Some(process) = process else {
            return TaskState::Completed;
        };
        let transcript_modified_ms = activity.map_or(0, |a| a.last_modified_ms);
        if let Some(hook) = hook {
            if hook.kind == HookKind::PermissionPrompt && hook.at_ms >= transcript_modified_ms {
                return TaskState::Waiting;
            }
        }
        let Some(activity) = activity else {
            return if now_ms.saturating_sub(last_activity_at) < FRESH_SESSION_MS {
                TaskState::Running
            } else {
                TaskState::Completed
            };
        };

        match activity.last_event {
            TurnEvent::Ended => TaskState::Completed,
            TurnEvent::Generating => TaskState::Running,
            TurnEvent::AwaitingUser => TaskState::Waiting,
            // 装了 hook 的会话，等授权会由 hook 明确报告，不再靠超时猜
            TurnEvent::ToolPending if hook.is_some() => TaskState::Running,
            TurnEvent::ToolPending => {
                let idle_ms = now_ms.saturating_sub(activity.last_modified_ms);
                let busy = process.has_children || process.cpu_usage > 1.0 || idle_ms < TOOL_PENDING_IDLE_MS;
                if busy {
                    TaskState::Running
                } else {
                    TaskState::Waiting
                }
            }
        }
    }

    fn find_json_files(dir: &Path, callback: &mut dyn FnMut(&Path)) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::find_json_files(&path, callback);
                } else if path.extension().map_or(false, |ext| ext == "json") {
                    callback(&path);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_000_000_000;

    fn idle_process() -> LiveProcess {
        LiveProcess { pid: 1, has_children: false, cpu_usage: 0.0 }
    }

    fn activity(event: TurnEvent, modified_ago_ms: u64) -> TranscriptActivity {
        TranscriptActivity { last_event: event, last_modified_ms: NOW - modified_ago_ms }
    }

    #[test]
    fn no_process_is_completed() {
        let act = activity(TurnEvent::Generating, 0);
        assert_eq!(ProcessScanner::determine_session_state(None, Some(&act), None, NOW, NOW), TaskState::Completed);
    }

    #[test]
    fn turn_ended_is_completed_regardless_of_process_activity() {
        let busy = LiveProcess { pid: 1, has_children: true, cpu_usage: 50.0 };
        let act = activity(TurnEvent::Ended, 0);
        assert_eq!(ProcessScanner::determine_session_state(Some(&busy), Some(&act), None, NOW, NOW), TaskState::Completed);
    }

    #[test]
    fn generating_is_running_even_when_process_looks_idle() {
        let act = activity(TurnEvent::Generating, 600_000);
        assert_eq!(ProcessScanner::determine_session_state(Some(&idle_process()), Some(&act), None, NOW - 600_000, NOW), TaskState::Running);
    }

    #[test]
    fn user_facing_tool_is_waiting() {
        let act = activity(TurnEvent::AwaitingUser, 0);
        assert_eq!(ProcessScanner::determine_session_state(Some(&idle_process()), Some(&act), None, NOW, NOW), TaskState::Waiting);
    }

    #[test]
    fn pending_tool_is_running_while_busy_and_waiting_when_stale() {
        let fresh = activity(TurnEvent::ToolPending, 5_000);
        assert_eq!(ProcessScanner::determine_session_state(Some(&idle_process()), Some(&fresh), None, NOW, NOW), TaskState::Running);

        let stale = activity(TurnEvent::ToolPending, 120_000);
        assert_eq!(ProcessScanner::determine_session_state(Some(&idle_process()), Some(&stale), None, NOW, NOW), TaskState::Waiting);

        let with_child = LiveProcess { pid: 1, has_children: true, cpu_usage: 0.0 };
        assert_eq!(ProcessScanner::determine_session_state(Some(&with_child), Some(&stale), None, NOW, NOW), TaskState::Running);
    }

    #[test]
    fn missing_transcript_falls_back_to_submit_time() {
        assert_eq!(ProcessScanner::determine_session_state(Some(&idle_process()), None, None, NOW - 10_000, NOW), TaskState::Running);
        assert_eq!(ProcessScanner::determine_session_state(Some(&idle_process()), None, None, NOW - 120_000, NOW), TaskState::Completed);
    }

    fn hook(kind: HookKind, at_ago_ms: u64) -> HookSignal {
        HookSignal { kind, at_ms: NOW - at_ago_ms }
    }

    #[test]
    fn hook_permission_prompt_wins_until_transcript_moves_on() {
        let pending = activity(TurnEvent::ToolPending, 10_000);
        let asked = hook(HookKind::PermissionPrompt, 5_000);
        assert_eq!(ProcessScanner::determine_session_state(Some(&idle_process()), Some(&pending), Some(&asked), NOW, NOW), TaskState::Waiting);

        // 用户授权后工具结果写入 transcript，比 hook 事件更新
        let resumed = activity(TurnEvent::Generating, 1_000);
        assert_eq!(ProcessScanner::determine_session_state(Some(&idle_process()), Some(&resumed), Some(&asked), NOW, NOW), TaskState::Running);

        // 没有进程时 hook 不起作用
        assert_eq!(ProcessScanner::determine_session_state(None, Some(&pending), Some(&asked), NOW, NOW), TaskState::Completed);
    }

    #[test]
    fn hooked_session_does_not_guess_waiting_from_idle_tool() {
        let stale = activity(TurnEvent::ToolPending, 120_000);
        let started = hook(HookKind::Busy, 100_000);
        assert_eq!(ProcessScanner::determine_session_state(Some(&idle_process()), Some(&stale), Some(&started), NOW, NOW), TaskState::Running);
    }

    #[test]
    fn test_scan_real_sessions() {
        let mut scanner = ProcessScanner::new();
        let groups = scanner.scan_claude_sessions();
        println!("Discovered {} project groups:", groups.len());
        for g in &groups {
            println!("Project: {} ({} sessions)", g.project_name, g.sessions.len());
            for s in &g.sessions {
                println!("  - [{:?}] {} (PID: {:?})", s.state, s.title, s.pid);
            }
        }
        assert!(!groups.is_empty(), "Should discover project groups if claude sessions exist");
    }
}
