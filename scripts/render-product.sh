#!/usr/bin/env bash
# render-product.sh —— 把「一个产品」注入壳工程，产出可构建的 src-tauri/resources + tauri.conf.json。
#
# 这是启动器 packaging 服务与 CI 共用的装配脚本（ADR-0004 步骤③④⑤）：
#   1) 把攒好的自包含快照（node + dsh 完整依赖树 + 虚拟 DSH_HOME）staging 进 src-tauri/resources/dsh-snapshot/；
#   2) 生成 product.manifest.json（format=1，见 docs/contract.md）；
#   3) 改写 tauri.conf.json 的构建期身份（productName / identifier / 窗口标题）。
#
# 自包含是硬指标（ADR-0004 三条硬指标之二）：`--dsh-pkg` 传入的必须是**完整**的
# @deepseek-ai/dsh 运行时依赖树根（node_modules，pnpm 布局），由打包侧物化，本脚本不触网不取 store。
#
# 用法：
#   scripts/render-product.sh \
#     --node      <path-to-node-bin>    # 目标平台 node 可执行文件（node / node.exe）
#     --dsh-runtime <path-to-node-modules> # 完整运行时依赖树根（pnpm 布局，含 .pnpm/ 与 @deepseek-ai/）
#     --dsh-home  <path-to-virtual-home># 装配好的虚拟 $DSH_HOME（含 profiles/<profile>）
#     --profile   <profile>             # 要 boot 的 profile
#     --name      "产品名"              # 构建期身份：productName + 窗口标题
#     --id        com.example.dshdesktop # 构建期身份：identifier
#     [--icons-dir <dir>]               # 可选：覆盖 src-tauri/icons
#     [--out <src-tauri>]               # 可选：默认 src-tauri
#
# 幂等：重复执行会覆盖 resources 与 tauri.conf.json 相关字段，其余不动。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC_TAURI="$ROOT/src-tauri"

# ---- 解析参数 ----
NODE_BIN=""; DSH_RUNTIME=""; DSH_HOME=""; PROFILE="default"; NAME="DSH Dock"; ID="dev.deepseek.dsh-dock"; ICONS_DIR=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --node) NODE_BIN="$2"; shift 2;;
    --dsh-runtime) DSH_RUNTIME="$2"; shift 2;;
    --dsh-home) DSH_HOME="$2"; shift 2;;
    --profile) PROFILE="$2"; shift 2;;
    --name) NAME="$2"; shift 2;;
    --id) ID="$2"; shift 2;;
    --icons-dir) ICONS_DIR="$2"; shift 2;;
    *) echo "未知参数: $1" >&2; exit 2;;
  esac
done

for v in NODE_BIN DSH_RUNTIME DSH_HOME; do
  [[ -n "${!v}" ]] || { echo "缺少 --${v,,} 参数" >&2; exit 2; }
done
[[ -f "$NODE_BIN" ]] || { echo "--node 不是文件: $NODE_BIN" >&2; exit 2; }
[[ -f "$DSH_RUNTIME/@deepseek-ai/dsh/lib/bin.js" ]] || { echo "--dsh-runtime 缺少 @deepseek-ai/dsh/lib/bin.js（应传入运行时 node_modules 根）" >&2; exit 2; }
[[ -d "$DSH_HOME/profiles" ]] || { echo "--dsh-home 缺少 profiles/（应是虚拟 $DSH_HOME）" >&2; exit 2; }

# ---- 1) staging 自包含快照（全新重建，幂等） ----
snap="$SRC_TAURI/resources/dsh-snapshot"
rm -rf "$snap"
mkdir -p "$snap/node/bin" "$snap/dsh" "$snap/home"

cp -f "$NODE_BIN" "$snap/node/bin/dsh-node"
chmod +x "$snap/node/bin/dsh-node" 2>/dev/null || true
cp -fR "$DSH_RUNTIME"/. "$snap/dsh/"
cp -fR "$DSH_HOME"/. "$snap/home/"

# 兜底校验：三件套就位
[[ -f "$snap/dsh/@deepseek-ai/dsh/lib/bin.js" ]] || { echo "错误：快照未含 dsh 入口" >&2; exit 1; }
[[ -f "$snap/home/profiles/$PROFILE/package.json" || -d "$snap/home/profiles/$PROFILE" ]] || {
  echo "警告：快照 home 下找不到 profile '$PROFILE'，请核对 --profile" >&2
}

# ---- 2) product.manifest.json（contract v3：snapshot 三件套 = 快照档，离线可用；
# resolution/fallback 语义已废止——见 docs/contract.md「运行时策略 v3」。打包侧
# 须与本壳 MANIFEST_FORMAT=3 同步升版，旧 format 由壳兼容迁移。） ----
cat > "$SRC_TAURI/resources/product.manifest.json" <<JSON
{
  "format": 3,
  "productName": "$NAME",
  "terminal": {
    "defaultProfile": "$PROFILE"
  },
  "snapshot": {
    "nodeBin": "dsh-snapshot/node/bin/dsh-node",
    "dshBinJs": "dsh-snapshot/dsh/@deepseek-ai/dsh/lib/bin.js",
    "dshHome": "dsh-snapshot/home",
    "profile": "$PROFILE"
  }
}
JSON

# ---- 3) 构建期身份 ----
python3 - "$SRC_TAURI/tauri.conf.json" "$NAME" "$ID" <<'PY'
import json, sys
path, name, ident = sys.argv[1], sys.argv[2], sys.argv[3]
data = json.load(open(path))
data["productName"] = name
data["identifier"] = ident
for w in data.get("app", {}).get("windows", []):
    if w.get("label") == "main":
        w["title"] = name
json.dump(data, open(path, "w"), ensure_ascii=False, indent=2)
print(f"tauri.conf.json 已更新：productName={name} identifier={ident}")
PY

[[ -n "$ICONS_DIR" ]] && rm -rf "$SRC_TAURI/icons" && cp -fR "$ICONS_DIR" "$SRC_TAURI/icons"

echo "✅ 已装配：$NAME"
echo "   快照 → ${snap#$ROOT/}"
echo "   如下一步：cd src-tauri && cargo tauri build"
