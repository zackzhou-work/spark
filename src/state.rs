#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TimeFilter {
    #[default]
    Today,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    /// 任务进行中：灰度呼吸动效 (#D1D5DB ↔ #374151, 2.0s 周期)
    Running,
    /// 等待开发者确认/授权：实心金黄色 (#F59E0B)
    Waiting,
    /// 任务已完成：实心深蓝色 (#3B82F6)
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionItem {
    pub id: String,
    pub title: String,
    pub state: TaskState,
    pub pid: Option<i32>,
    pub working_dir: String,
    #[serde(default)]
    pub is_today: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectGroup {
    pub project_name: String,
    pub sessions: Vec<SessionItem>,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub is_pinned: bool,
    pub filter: TimeFilter,
    pub prev_filter: Option<TimeFilter>,
    pub project_groups: Vec<ProjectGroup>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            is_pinned: true,
            filter: TimeFilter::Today,
            prev_filter: None,
            project_groups: Vec::new(),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn demo_groups() -> Vec<ProjectGroup> {
        vec![
            ProjectGroup {
                project_name: "openpaper".to_string(),
                sessions: vec![
                    SessionItem {
                        id: "s1".to_string(),
                        title: "Refactor Auth & Token Refresh".to_string(),
                        state: TaskState::Running,
                        pid: Some(1024),
                        working_dir: "/Users/zhouzhou74/Desktop/work/openpaper".to_string(),
                        is_today: true,
                    },
                    SessionItem {
                        id: "s2".to_string(),
                        title: "Fix SQLite Read/Write Deadlock".to_string(),
                        state: TaskState::Completed,
                        pid: Some(1025),
                        working_dir: "/Users/zhouzhou74/Desktop/work/openpaper".to_string(),
                        is_today: false,
                    },
                ],
            },
            ProjectGroup {
                project_name: "claude-desktop-ext".to_string(),
                sessions: vec![
                    SessionItem {
                        id: "s3".to_string(),
                        title: "Support macOS Sequoia Deep Link Activation".to_string(),
                        state: TaskState::Waiting,
                        pid: Some(1026),
                        working_dir: "/Users/zhouzhou74/Desktop/work/claude-desktop-ext".to_string(),
                        is_today: true,
                    },
                ],
            },
        ]
    }

    pub fn today_groups(&self) -> Vec<ProjectGroup> {
        self.project_groups
            .iter()
            .filter_map(|g| {
                let today_sessions: Vec<SessionItem> = g
                    .sessions
                    .iter()
                    .filter(|s| s.is_today)
                    .cloned()
                    .collect();
                if today_sessions.is_empty() {
                    None
                } else {
                    Some(ProjectGroup {
                        project_name: g.project_name.clone(),
                        sessions: today_sessions,
                    })
                }
            })
            .collect()
    }

    pub fn all_groups(&self) -> Vec<ProjectGroup> {
        self.project_groups.clone()
    }

    pub fn filtered_groups(&self) -> Vec<ProjectGroup> {
        match self.filter {
            TimeFilter::Today => self.today_groups(),
            TimeFilter::All => self.all_groups(),
        }
    }

    pub fn total_sessions(&self) -> usize {
        self.project_groups.iter().map(|g| g.sessions.len()).sum()
    }

    pub fn active_sessions(&self) -> usize {
        self.project_groups
            .iter()
            .flat_map(|g| &g.sessions)
            .filter(|s| s.state != TaskState::Completed)
            .count()
    }
}
