use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use sysinfo::System;

use crate::state::{ProjectGroup, SessionItem, TaskState};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
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
    last_focused_at: u64,
    #[serde(default)]
    created_at: u64,
    #[serde(default)]
    latest_user_frame_at: Option<u64>,
    #[serde(default)]
    completed_turns: Option<u32>,
    pending_system_reminder: Option<String>,
}

pub struct ProcessScanner {
    sys: System,
}

impl ProcessScanner {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(),
        }
    }

    /// 扫描系统中正在执行的 Claude 核心进程工作目录及 PID
    fn get_running_cwds(&mut self) -> HashMap<String, u32> {
        self.sys.refresh_processes();
        let mut active_cwds = HashMap::new();

        for (pid, process) in self.sys.processes() {
            let name = process.name();
            // 过滤外层包装器 (如 macOS disclaimer 辅助进程)
            if name.eq_ignore_ascii_case("disclaimer") {
                continue;
            }

            let is_claude = name.eq_ignore_ascii_case("claude")
                || process.cmd().iter().any(|arg| {
                    arg.contains("claude-code") || arg.contains("claude.app/Contents/MacOS/claude")
                });

            if is_claude {
                if let Some(cwd) = process.cwd() {
                    let cwd_str = cwd.to_string_lossy().trim_end_matches('/').to_string();
                    if !cwd_str.is_empty() {
                        // 优先保留真正名为 claude 的核心工作进程
                        let is_exact_claude = name.eq_ignore_ascii_case("claude");
                        if !active_cwds.contains_key(&cwd_str) || is_exact_claude {
                            active_cwds.insert(cwd_str, pid.as_u32());
                        }
                    }
                }
            }
        }

        active_cwds
    }

    /// 扫描真实 Claude Desktop 的所有未归档会话
    pub fn scan_claude_sessions(&mut self) -> Vec<ProjectGroup> {
        let active_cwds = self.get_running_cwds();

        let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/zhouzhou74".to_string());
        let sessions_dir = PathBuf::from(home)
            .join("Library/Application Support/Claude/claude-code-sessions");

        if !sessions_dir.exists() {
            return Vec::new();
        }

        let mut discovered = Vec::new();

        Self::find_json_files(&sessions_dir, &mut |file_path| {
            let Ok(content) = fs::read_to_string(file_path) else {
                return;
            };
            let Ok(data) = serde_json::from_str::<RawSessionJson>(&content) else {
                return;
            };

            // 过滤已归档会话
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

            // 检查当前会话是否匹配活动进程
            let mut matched_pid = None;
            if let Some(ref wt) = data.worktree_path {
                let wt_trim = wt.trim_end_matches('/');
                if let Some(&p) = active_cwds.get(wt_trim) {
                    matched_pid = Some(p as i32);
                }
            }
            if matched_pid.is_none() {
                if let Some(ref cwd) = data.cwd {
                    let cwd_trim = cwd.trim_end_matches('/');
                    if let Some(&p) = active_cwds.get(cwd_trim) {
                        matched_pid = Some(p as i32);
                    }
                }
            }
            if matched_pid.is_none() && !origin_cwd.is_empty() {
                let orig_trim = origin_cwd.trim_end_matches('/');
                if let Some(&p) = active_cwds.get(orig_trim) {
                    matched_pid = Some(p as i32);
                }
            }

            let now_ms = chrono::Local::now().timestamp_millis() as u64;

            let state = if let Some(pid_val) = matched_pid {
                let pid_sys = sysinfo::Pid::from_u32(pid_val as u32);
                let has_children = self
                    .sys
                    .processes()
                    .values()
                    .any(|p| p.parent() == Some(pid_sys));
                let proc_cpu = self
                    .sys
                    .process(pid_sys)
                    .map(|p| p.cpu_usage())
                    .unwrap_or(0.0);

                Self::determine_session_state(
                    true,
                    has_children,
                    proc_cpu,
                    now_ms,
                    data.last_activity_at,
                    data.last_focused_at,
                    data.latest_user_frame_at,
                    data.pending_system_reminder.is_some(),
                )
            } else {
                TaskState::Completed
            };

            let session_id = data.session_id.unwrap_or_else(|| "session".to_string());
            let activity_ts = data.last_activity_at.max(data.created_at);

            let today = chrono::Local::now().date_naive();
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
                    pid: matched_pid,
                    working_dir: origin_cwd,
                    is_today,
                },
                activity_ts,
            ));
        });

        if discovered.is_empty() {
            return Vec::new();
        }

        // 按项目分组聚合
        let mut groups_map: BTreeMap<String, Vec<(SessionItem, u64)>> = BTreeMap::new();
        for (proj, item, last_act) in discovered {
            groups_map.entry(proj).or_default().push((item, last_act));
        }

        let mut groups = Vec::new();
        for (proj, mut items) in groups_map {
            // 项目内会话：Running/Waiting 优先，其余按最后活动时间倒序排列
            items.sort_by(|a, b| {
                let a_prio = match a.0.state {
                    TaskState::Running => 2,
                    TaskState::Waiting => 1,
                    TaskState::Completed => 0,
                };
                let b_prio = match b.0.state {
                    TaskState::Running => 2,
                    TaskState::Waiting => 1,
                    TaskState::Completed => 0,
                };
                b_prio.cmp(&a_prio).then_with(|| b.1.cmp(&a.1))
            });
            let sessions = items.into_iter().map(|(item, _)| item).collect();
            groups.push(ProjectGroup {
                project_name: proj,
                sessions,
            });
        }

        // 排序项目：有进行中/待处理任务的项目优先置顶显示，其余按字母排序
        groups.sort_by(|a, b| {
            let a_prio = a
                .sessions
                .iter()
                .map(|s| match s.state {
                    TaskState::Running => 2,
                    TaskState::Waiting => 1,
                    TaskState::Completed => 0,
                })
                .max()
                .unwrap_or(0);
            let b_prio = b
                .sessions
                .iter()
                .map(|s| match s.state {
                    TaskState::Running => 2,
                    TaskState::Waiting => 1,
                    TaskState::Completed => 0,
                })
                .max()
                .unwrap_or(0);
            b_prio
                .cmp(&a_prio)
                .then_with(|| a.project_name.to_lowercase().cmp(&b.project_name.to_lowercase()))
        });

        groups
    }

    /// 核心状态判决机：
    /// 1. Running (灰度呼吸): 活跃执行中（存在子进程/CPU占用/新Prompt刚发出/15秒内有活动）
    /// 2. Waiting (金黄色常亮): 进程存活但当前处于等待（5分钟内刚结束等待用户查看，或处于权限授权/用户输入确认等待）
    /// 3. Completed (蓝色常亮): 进程已关闭，或已闲置超过5分钟/用户已查看完毕
    pub fn determine_session_state(
        has_matched_process: bool,
        has_children: bool,
        proc_cpu: f32,
        now_ms: u64,
        last_activity_at: u64,
        last_focused_at: u64,
        latest_user_frame_at: Option<u64>,
        pending_system_reminder: bool,
    ) -> TaskState {
        if !has_matched_process {
            return TaskState::Completed;
        }

        let elapsed_act = now_ms.saturating_sub(last_activity_at);
        let user_frame_after_act = latest_user_frame_at.map_or(false, |uf| uf > last_activity_at);

        if has_children || proc_cpu > 1.0 || user_frame_after_act || elapsed_act < 15_000 {
            TaskState::Running
        } else {
            let has_focused_after = last_focused_at >= last_activity_at;
            if (elapsed_act < 300_000 && !has_focused_after)
                || (pending_system_reminder && elapsed_act < 300_000)
            {
                TaskState::Waiting
            } else {
                TaskState::Completed
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

    #[test]
    fn test_state_evaluation() {
        let now_ms = 1_000_000_000;

        // 1. Process dead -> Completed
        assert_eq!(
            ProcessScanner::determine_session_state(false, false, 0.0, now_ms, now_ms, 0, None, false),
            TaskState::Completed
        );

        // 2. Active child process -> Running
        assert_eq!(
            ProcessScanner::determine_session_state(true, true, 0.0, now_ms, now_ms - 50_000, 0, None, false),
            TaskState::Running
        );

        // 3. High CPU usage -> Running
        assert_eq!(
            ProcessScanner::determine_session_state(true, false, 5.0, now_ms, now_ms - 50_000, 0, None, false),
            TaskState::Running
        );

        // 4. User sent frame after last activity (prompt submitted, waiting for output) -> Running
        assert_eq!(
            ProcessScanner::determine_session_state(true, false, 0.0, now_ms, now_ms - 20_000, 0, Some(now_ms - 5_000), false),
            TaskState::Running
        );

        // 5. Recent activity within 15 seconds -> Running
        assert_eq!(
            ProcessScanner::determine_session_state(true, false, 0.0, now_ms, now_ms - 10_000, 0, None, false),
            TaskState::Running
        );

        // 6. Finished within 5 mins, user has NOT focused tab -> Waiting (prompt/review reminder)
        assert_eq!(
            ProcessScanner::determine_session_state(true, false, 0.0, now_ms, now_ms - 60_000, now_ms - 120_000, None, false),
            TaskState::Waiting
        );

        // 7. Pending system reminder / permission within 5 mins -> Waiting
        assert_eq!(
            ProcessScanner::determine_session_state(true, false, 0.0, now_ms, now_ms - 60_000, now_ms - 10_000, None, true),
            TaskState::Waiting
        );

        // 8. Finished within 5 mins, user HAS focused tab (reviewed) -> Completed
        assert_eq!(
            ProcessScanner::determine_session_state(true, false, 0.0, now_ms, now_ms - 60_000, now_ms - 30_000, None, false),
            TaskState::Completed
        );

        // 9. Idle for > 5 minutes (> 300s) -> Completed
        assert_eq!(
            ProcessScanner::determine_session_state(true, false, 0.0, now_ms, now_ms - 400_000, 0, None, false),
            TaskState::Completed
        );
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

    #[test]
    fn test_filter_today() {
        let mut scanner = ProcessScanner::new();
        let groups = scanner.scan_claude_sessions();
        let app_state = crate::state::AppState {
            is_pinned: true,
            filter: crate::state::TimeFilter::Today,
            prev_filter: None,
            project_groups: groups.clone(),
        };
        let today_groups = app_state.filtered_groups();
        println!("Today groups count: {}", today_groups.len());
        for g in &today_groups {
            println!("Today Project: {} ({} sessions)", g.project_name, g.sessions.len());
            for s in &g.sessions {
                println!("  - [{:?}] {} (is_today: {})", s.state, s.title, s.is_today);
                assert!(s.is_today);
            }
        }
    }
}
