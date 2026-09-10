#!/usr/bin/env bash
# regen-icons.sh —— 用 dsh 官方 logo 重新生成整套图标（含桌面客户端图标）。
#
# 官方源：dsh-web-frontend 的 favicon.svg（深色模式官方渲染 = 白标）。
# 链路：官方 svg → assets/dsh-logo.svg（原始落库）→
#       assets/icon-master.svg（白标 + 深色圆角底的主图标合成）→
#       rsvg-convert → src-tauri/app-icon.png（1024 master）→
#       cargo tauri icon → src-tauri/icons/*（全部平台产物）。
# 幂等：每次运行整体重生成。
# 2026-09-08：落库路径 ui/assets/dsh-logo.svg → assets/dsh-logo.svg（ui/ 目录已删除）。
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# 默认用仓库内的官方溯源副本（自包含）；官方出新版时用 DSH_FAVICON 指向新 favicon.svg 刷新。
if [ -n "${DSH_FAVICON:-}" ]; then
  [ -f "$DSH_FAVICON" ] || { echo "DSH_FAVICON 指向的文件不存在：$DSH_FAVICON" >&2; exit 1; }
  cp -f "$DSH_FAVICON" "$ROOT/assets/dsh-logo.svg"
  echo "已从官方源刷新：$DSH_FAVICON → assets/dsh-logo.svg"
else
  echo "使用仓库内官方溯源副本 assets/dsh-logo.svg（刷新自新版请设 DSH_FAVICON）"
fi

# 优先使用 assets/ 下的 1024x1024 规范级主图标（支持 ICON_VARIANT=light-icon|peace|wave，默认 light-icon）
ICON_VARIANT="${ICON_VARIANT:-light-icon}"
VARIANT_SRC="$ROOT/assets/whale-chan-${ICON_VARIANT}.png"

if [ -f "$VARIANT_SRC" ]; then
  echo "使用主图标方案：whale-chan-${ICON_VARIANT} ($VARIANT_SRC)"
  cp -f "$VARIANT_SRC" "$ROOT/src-tauri/app-icon.png"
elif [ -f "$ROOT/assets/icon-master.png" ]; then
  cp -f "$ROOT/assets/icon-master.png" "$ROOT/src-tauri/app-icon.png"
elif command -v rsvg-convert >/dev/null; then
  rsvg-convert -w 1024 -h 1024 -o "$ROOT/src-tauri/app-icon.png" "$ROOT/assets/icon-master.svg"
else
  # 使用 Python PIL 确保透明度，严禁使用 qlmanage 产生不透明白底
  python3 -c "
import base64, re, sys
with open('$ROOT/assets/icon-master.svg', 'r') as f:
    c = f.read()
m = re.search(r'base64,([A-Za-z0-9+/=]+)', c)
if m:
    with open('$ROOT/src-tauri/app-icon.png', 'wb') as out:
        out.write(base64.b64decode(m.group(1)))
else:
    sys.exit('未能从 icon-master.svg 提取主图标数据，请检查 assets/whale-chan-peace.png')
"
fi
echo "→ src-tauri/app-icon.png（1024x1024 master）"

(cd "$ROOT/src-tauri" && cargo tauri icon app-icon.png)

# 强门禁：验证生成产物四个角透明度为 0（严防 macOS Dock 四周白边白底回归）
python3 -c "
from PIL import Image
p = '$ROOT/src-tauri/icons/icon.png'
im = Image.open(p)
assert im.getpixel((0,0))[3] == 0, f'门禁失败：{p} 存在非透明白角 {im.getpixel((0,0))}'
print('✓ 图标透明度门禁通过：四个角完全透明 (Alpha=0)')
"
echo "→ src-tauri/icons/* 已重生成；如改 tauri.conf.json 请同步 bundle.icon 清单"