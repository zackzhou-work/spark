# spark

监控 Claude Desktop（Code 标签页）里各个会话的任务状态。

## 状态来源

- **进程**：每个 claude 核心进程的环境变量 `CLAUDE_CODE_HOST_SESSION_ID` 对应桌面端的会话 ID，没有进程即 Completed。
- **transcript**：读 `~/.claude/projects/<cwd>/<cliSessionId>.jsonl` 末尾。end_turn 或用户打断为 Completed；工具调用挂起为 Running；AskUserQuestion / ExitPlanMode 为 Waiting。
- **hooks（可选）**：装上后"等待授权"由 Claude Code 直接报告，否则只能靠"工具挂起 45 秒无动静"推测。

扫描由文件监听驱动（会话目录、transcript 目录、hooks 目录），另有 5 秒定时兜底处理进程退出等无文件事件的变化。

## 点击跳转

单击任意一行（包括已完成的会话），Claude Desktop 会切到对应的 Code 会话并置前，用的是它自己的私有深链：

```
claude://code/continue?session=<sessionId>&source=spark
```

`sessionId` 就是会话 JSON 里的那个 `local_<uuid>`，不需要任何映射。窗口的唤起和置前由 Claude Desktop 自己完成，spark 只负责 `open` 这条 URL，不判断跳转是否成功——成功与否由目标应用自己在屏幕上宣告。

**这条链没有公开契约。** 它是从 Claude.app 2.2553.1 的 `app.asar` 路由代码里读出来的，随时可能变：

- 应用侧的校验正则是 `^local_[A-Za-z0-9-]{1,64}$`，`src/platform/mac_deeplink.rs` 的单测钉住了同一套规则。会话 ID 对不上格式时不会发出请求，只打一行 `[Jump]` 日志——那通常就是格式变了的信号。
- 应用侧按 `sessionId` 匹配前会先过滤掉已归档的会话，spark 本来也不显示归档会话，两边一致。
- 深链整体受一个远端开关和 `~/Library/Application Support/Claude/config.json` 里的 `disableDeepLinks` 管控，被关掉时点击不会有任何反应。
- 如果哪天 `code/continue` 不灵了，还有一条 `claude://resume?session=<cliSessionId>`（裸 UUID）可以试。它走的是"导入 CLI 会话"的路径，语义和聚焦已有桌面会话不同，可能产生重复会话，所以没有拿来做自动兜底。

**多账号未处理**：spark 会扫描会话目录下所有 `<账号>/<组织>` 的会话，而深链只能命中当前登录账号的那些。切换账号后，旧账号的会话点了不会有反应。

## 安装 hooks

先把脚本复制到固定位置，再把下面内容合并进 `~/.claude/settings.json`。脚本只把事件写到 `~/.config/spark/hooks/<session_id>.json`，不影响 Claude Code 的任何决策。

```bash
cp hooks/spark-hook.sh ~/.config/spark/spark-hook.sh && chmod +x ~/.config/spark/spark-hook.sh
```

```json
{
  "hooks": {
    "UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": "/Users/<you>/.config/spark/spark-hook.sh", "timeout": 5 }] }],
    "PreToolUse": [{ "hooks": [{ "type": "command", "command": "/Users/<you>/.config/spark/spark-hook.sh", "timeout": 5 }] }],
    "PostToolUse": [{ "hooks": [{ "type": "command", "command": "/Users/<you>/.config/spark/spark-hook.sh", "timeout": 5 }] }],
    "PermissionRequest": [{ "hooks": [{ "type": "command", "command": "/Users/<you>/.config/spark/spark-hook.sh", "timeout": 5 }] }],
    "Notification": [{ "hooks": [{ "type": "command", "command": "/Users/<you>/.config/spark/spark-hook.sh", "timeout": 5 }] }],
    "Stop": [{ "hooks": [{ "type": "command", "command": "/Users/<you>/.config/spark/spark-hook.sh", "timeout": 5 }] }],
    "SessionEnd": [{ "hooks": [{ "type": "command", "command": "/Users/<you>/.config/spark/spark-hook.sh", "timeout": 5 }] }]
  }
}
```

把 `<you>` 换成你的用户名。
