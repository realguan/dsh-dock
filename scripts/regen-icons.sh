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

FAV=$(find "${DSH_RUNTIME:-$HOME/.dsh/.dsh-launcher/runtimes}" \
  -name favicon.svg -path "*dsh-web-frontend*" 2>/dev/null | head -1)
[ -n "$FAV" ] || { echo "未找到官方 favicon.svg（可用 DSH_RUNTIME 指定运行时根）" >&2; exit 1; }
echo "官方源：$FAV"

cp -f "$FAV" "$ROOT/ui/assets/dsh-logo.svg"
echo "→ ui/assets/dsh-logo.svg"

command -v rsvg-convert >/dev/null || { echo "需要 rsvg-convert（brew install librsvg）" >&2; exit 1; }
rsvg-convert -w 1024 -h 1024 -o "$ROOT/src-tauri/app-icon.png" "$ROOT/assets/icon-master.svg"
echo "→ src-tauri/app-icon.png（1024x1024 master）"

(cd "$ROOT/src-tauri" && cargo tauri icon app-icon.png)
echo "→ src-tauri/icons/* 已重生成；如改 tauri.conf.json 请同步 bundle.icon 清单"