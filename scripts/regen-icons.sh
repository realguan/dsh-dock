#!/usr/bin/env bash
# regen-icons.sh —— 用 dsh 官方 logo 重新生成整套图标（含桌面客户端图标）。
#
# 官方源：dsh-web-frontend 的 favicon.svg（深色模式官方渲染 = 白标）。
# 链路：官方 svg → ui/assets/dsh-logo.svg（原始落库）→
#       assets/icon-master.svg（白标 + 深色圆角底的主图标合成）→
#       rsvg-convert → src-tauri/app-icon.png（1024 master）→
#       cargo tauri icon → src-tauri/icons/*（全部平台产物）。
# 幂等：每次运行整体重生成。
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# 默认用仓库内的官方溯源副本（自包含）；官方出新版时用 DSH_FAVICON 指向新 favicon.svg 刷新。
if [ -n "${DSH_FAVICON:-}" ]; then
  [ -f "$DSH_FAVICON" ] || { echo "DSH_FAVICON 指向的文件不存在：$DSH_FAVICON" >&2; exit 1; }
  cp -f "$DSH_FAVICON" "$ROOT/ui/assets/dsh-logo.svg"
  echo "已从官方源刷新：$DSH_FAVICON → ui/assets/dsh-logo.svg"
else
  echo "使用仓库内官方溯源副本 ui/assets/dsh-logo.svg（刷新自新版请设 DSH_FAVICON）"
fi

command -v rsvg-convert >/dev/null || { echo "需要 rsvg-convert（brew install librsvg）" >&2; exit 1; }
rsvg-convert -w 1024 -h 1024 -o "$ROOT/src-tauri/app-icon.png" "$ROOT/assets/icon-master.svg"
echo "→ src-tauri/app-icon.png（1024x1024 master）"

(cd "$ROOT/src-tauri" && cargo tauri icon app-icon.png)
echo "→ src-tauri/icons/* 已重生成；如改 tauri.conf.json 请同步 bundle.icon 清单"