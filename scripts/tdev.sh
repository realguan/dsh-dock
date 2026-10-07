#!/usr/bin/env bash
# tdev.sh —— 本地开发「隔离测试档」：**一条命令**起前端 dev server + 应用，
# 用与正式数据物理隔离的配置目录（不碰用户真实的 ~/.dsh）。
#
#   app 数据目录   io.github.realguan.dsh-dock.dev（tauri.dev.conf.json 的 identifier）
#   dsh home       $HOME/.dsh-dock-dev
#   前端           tauri dev 自动执行 beforeDevCommand（vite → http://localhost:1420）
#
# 与 `cargo tdev` 的关系：同一条启动链（`cargo tauri dev --config tauri.dev.conf.json`），
# 本脚本**额外把 DSH_HOME 钉死**在隔离目录上——`DSH_HOME` 是 `resolve.rs::user_dsh_home`
# 里优先级最高的输入（「用户主权」），调用环境里只要有人导出过它（例如从 dsh 会话里
# 起的终端），debug 默认的隔离档就会被**静默绕过**、直接写正式 `~/.dsh`。
# 本脚本把这条不确定性删掉：一次命令 = 一个确定结果。
#
# 想要正式配置目录请用 ./scripts/dev.sh。
#
# 用法：
#   ./scripts/tdev.sh              # 起隔离档开发（Ctrl-C 结束）
#   ./scripts/tdev.sh --print      # 只打印本次用到的目录与命令，不启动
#   ./scripts/tdev.sh <tauri 参数> # 其余参数原样透传给 `cargo tauri dev`
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IDENTIFIER="io.github.realguan.dsh-dock.dev"

# 隔离 dsh home：显式设置（压过环境里可能已导出的 DSH_HOME，见文件头）
export DSH_HOME="$HOME/.dsh-dock-dev"

# 仅供展示：数据目录由 identifier 决定，路径按平台推导（不参与任何功能逻辑）
case "$(uname -s)" in
    Darwin) APP_DATA="$HOME/Library/Application Support/$IDENTIFIER" ;;
    Linux) APP_DATA="${XDG_DATA_HOME:-$HOME/.local/share}/$IDENTIFIER" ;;
    *) APP_DATA="%APPDATA%\\$IDENTIFIER" ;;
esac

if [[ "${1:-}" == "--print" ]]; then
    echo "mode=tdev"
    echo "identifier=$IDENTIFIER"
    echo "app_data=$APP_DATA"
    echo "DSH_HOME=$DSH_HOME"
    echo "command=cargo tauri dev --config tauri.dev.conf.json"
    exit 0
fi

echo "▶ tdev（隔离档）  dsh home=$DSH_HOME"
echo "                  app 数据目录=$APP_DATA"
cd "$ROOT/src-tauri"
exec cargo tauri dev --config tauri.dev.conf.json "$@"
