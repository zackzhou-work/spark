#!/usr/bin/env bash
# 把 spark-monitor 打成 spark.app，带上图标。
#   ./scripts/bundle-mac.sh              -> target/spark.app
#   ./scripts/bundle-mac.sh /Applications/spark.app
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="${1:-$ROOT/target/spark.app}"
BIN="spark-monitor"

cargo build --release --manifest-path "$ROOT/Cargo.toml"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$ROOT/target/release/$BIN" "$APP/Contents/MacOS/$BIN"
cp "$ROOT/resources/Info.plist" "$APP/Contents/Info.plist"

# 装了 Xcode 命令行工具就从 .iconset 现生成，否则用仓库里那份 .icns
if command -v iconutil >/dev/null 2>&1 && [ -d "$ROOT/resources/icon/spark.iconset" ]; then
	iconutil -c icns "$ROOT/resources/icon/spark.iconset" \
		-o "$APP/Contents/Resources/AppIcon.icns"
else
	cp "$ROOT/resources/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"
fi

# ad-hoc 签名，省得 gpui 的窗口权限在某些机器上被拦
codesign --force --sign - "$APP" >/dev/null 2>&1 || true

# 碰一下让 Finder / Dock 丢掉旧图标缓存
touch "$APP"

echo "built: $APP"
