# spark

监控 Claude Desktop（Code 标签页）里各个会话的任务状态。

## 状态来源

- **进程**：每个 claude 核心进程的环境变量 `CLAUDE_CODE_HOST_SESSION_ID` 对应桌面端的会话 ID，没有进程即 Completed。
- **transcript**：读 `~/.claude/projects/<cwd>/<cliSessionId>.jsonl` 末尾。end_turn 或用户打断为 Completed；工具调用挂起为 Running；AskUserQuestion / ExitPlanMode 为 Waiting。
- **hooks（可选）**：装上后"等待授权"由 Claude Code 直接报告，否则只能靠"工具挂起 45 秒无动静"推测。

扫描由文件监听驱动（会话目录、transcript 目录、hooks 目录），另有 5 秒定时兜底处理进程退出等无文件事件的变化。

## 安装 hooks

把下面内容合并进 `~/.claude/settings.json`。脚本只把事件写到 `~/.config/spark/hooks/<session_id>.json`，不影响 Claude Code 的任何决策。

```json
{
  "hooks": {
    "UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": "/path/to/spark/hooks/spark-hook.sh", "timeout": 5 }] }],
    "PreToolUse": [{ "hooks": [{ "type": "command", "command": "/path/to/spark/hooks/spark-hook.sh", "timeout": 5 }] }],
    "PostToolUse": [{ "hooks": [{ "type": "command", "command": "/path/to/spark/hooks/spark-hook.sh", "timeout": 5 }] }],
    "PermissionRequest": [{ "hooks": [{ "type": "command", "command": "/path/to/spark/hooks/spark-hook.sh", "timeout": 5 }] }],
    "Notification": [{ "hooks": [{ "type": "command", "command": "/path/to/spark/hooks/spark-hook.sh", "timeout": 5 }] }],
    "Stop": [{ "hooks": [{ "type": "command", "command": "/path/to/spark/hooks/spark-hook.sh", "timeout": 5 }] }],
    "SessionEnd": [{ "hooks": [{ "type": "command", "command": "/path/to/spark/hooks/spark-hook.sh", "timeout": 5 }] }]
  }
}
```

路径按实际 clone 位置替换。
