use std::process::Command;

/// 桌面端会话 ID 的格式：local_ 前缀加 1..=64 个字母数字或连字符
fn is_desktop_session_id(session_id: &str) -> bool {
    let Some(rest) = session_id.strip_prefix("local_") else {
        return false;
    };
    (1..=64).contains(&rest.len()) && rest.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// Claude Desktop 私有深链，用会话 ID 直接定位到 Code 标签页里的那个会话
pub fn session_deeplink(session_id: &str) -> Option<String> {
    is_desktop_session_id(session_id)
        .then(|| format!("claude://code/continue?session={session_id}&source=spark"))
}

/// 唤起 Claude Desktop 并切到指定会话；窗口的置前由应用自己处理
pub fn jump_to_session(session_id: &str) {
    #[cfg(target_os = "macos")]
    {
        // ID 格式对不上多半意味着桌面端换了格式，深链整体已经失效
        let Some(url) = session_deeplink(session_id) else {
            println!("[Jump] 不是桌面端会话 ID，跳过：{}", session_id);
            return;
        };
        let _ = Command::new("open").arg(&url).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_session_id_builds_url() {
        assert_eq!(
            session_deeplink("local_1f079f2a-8c74-449d-b254-1be4a5d9b0d8").as_deref(),
            Some("claude://code/continue?session=local_1f079f2a-8c74-449d-b254-1be4a5d9b0d8&source=spark")
        );
    }

    #[test]
    fn rejects_ids_the_desktop_app_would_not_match() {
        // 没有 local_ 前缀：cliSessionId 和 bridgeSessionId 都走不了这条链
        assert_eq!(session_deeplink("1f079f2a-8c74-449d-b254-1be4a5d9b0d8"), None);
        assert_eq!(session_deeplink("session_01NSKvUJpEpTmqnEih8Y3g8R"), None);
        // 前缀后为空
        assert_eq!(session_deeplink("local_"), None);
        // 下划线不在允许的字符集里
        assert_eq!(session_deeplink("local_has_underscore"), None);
        // 超过 64 个字符
        assert_eq!(session_deeplink(&format!("local_{}", "a".repeat(65))), None);
        assert!(session_deeplink(&format!("local_{}", "a".repeat(64))).is_some());
    }

    #[test]
    fn scanner_fallback_id_is_rejected() {
        // 会话 JSON 缺 sessionId 时 scanner 会填这个占位值
        assert_eq!(session_deeplink("session"), None);
    }
}
