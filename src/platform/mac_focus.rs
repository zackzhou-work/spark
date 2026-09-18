use std::process::Command;

/// 根据进程 PID 唤起并置前对应的终端或应用窗口 (Click-to-Focus)
pub fn focus_session_window(pid: Option<i32>) {
    #[cfg(target_os = "macos")]
    {
        if let Some(pid) = pid {
            println!("[Focus] Focusing window for process PID: {}", pid);
            let script = format!(
                r#"
                tell application "System Events"
                    set procList to every process whose unix id is {pid}
                    if (count of procList) > 0 then
                        set targetProc to item 1 of procList
                        set frontmost of targetProc to true
                        return
                    end if
                end tell
                "#,
                pid = pid
            );
            if let Ok(status) = Command::new("osascript").arg("-e").arg(&script).status() {
                if status.success() {
                    return;
                }
            }
        }

        // Fallback: 唤醒并置前 Claude Desktop 应用
        println!("[Focus] Activating Claude application");
        let fallback_script = r#"tell application "Claude" to activate"#;
        let _ = Command::new("osascript").arg("-e").arg(fallback_script).spawn();
    }
}
