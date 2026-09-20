use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

use super::ProcessScanner;
use crate::state::ProjectGroup;

/// 文件事件到达后再等这么久，把一串连续写入合并成一次扫描
const DEBOUNCE: Duration = Duration::from_millis(300);
/// 进程退出、工具挂起超时这类变化不产生文件事件，靠定时兜底
const FALLBACK_WATCHING: Duration = Duration::from_secs(5);
const FALLBACK_POLLING: Duration = Duration::from_secs(2);

/// 在后台线程监听会话目录、transcript 目录和 hooks 目录，有变化就重新扫描并把结果发给 UI
pub fn spawn(mut scanner: ProcessScanner) -> UnboundedReceiver<Vec<ProjectGroup>> {
    let (tx, rx) = unbounded_channel();

    thread::Builder::new()
        .name("session-watcher".into())
        .spawn(move || {
            let (fs_tx, fs_rx) = mpsc::channel();
            let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if event.is_ok() {
                    let _ = fs_tx.send(());
                }
            });
            let mut watcher = watcher.ok();
            let mut watching = false;
            if let Some(w) = watcher.as_mut() {
                for dir in scanner.watch_dirs() {
                    watching |= w.watch(&dir, RecursiveMode::Recursive).is_ok();
                }
            }
            let fallback = if watching { FALLBACK_WATCHING } else { FALLBACK_POLLING };

            loop {
                if tx.send(scanner.scan_claude_sessions()).is_err() {
                    return;
                }
                match fs_rx.recv_timeout(fallback) {
                    Ok(()) => while fs_rx.recv_timeout(DEBOUNCE).is_ok() {},
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => thread::sleep(fallback),
                }
            }
        })
        .expect("failed to spawn session watcher thread");

    rx
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 往真实 hooks 目录写一个临时文件，验证文件事件会在兜底间隔之前触发一次扫描
    #[test]
    fn file_change_triggers_rescan_before_fallback() {
        let scanner = ProcessScanner::new();
        let probe = scanner.watch_dirs()[2].join("spark-watcher-test.json");
        let mut rx = spawn(scanner);

        assert!(rx.blocking_recv().is_some(), "initial scan");
        thread::sleep(Duration::from_millis(500));
        std::fs::write(&probe, "{}").unwrap();

        let started = std::time::Instant::now();
        let got = rx.blocking_recv();
        let _ = std::fs::remove_file(&probe);
        assert!(got.is_some());
        assert!(started.elapsed() < FALLBACK_WATCHING, "rescan came from fallback timer, not the file event");
    }
}
