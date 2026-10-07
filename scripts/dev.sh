#!/usr/bin/env bash
# dev.sh —— 本地开发「正式档」：**一条命令**起前端 dev server + 应用，用正式配置目录。
#
#   app 数据目录   io.github.realguan.dsh-dock（settings.json / engines / 日志 / 单实例锁）
#                  —— 与已安装的 DSH Dock.app 是同一份：跑之前先退出它
#   dsh home       $HOME/.dsh（profiles / sessions / .credentials.yaml）
#   前端           tauri dev 自动执行 beforeDevCommand（vite → http://localhost:1420）
#
# 为什么必须显式设 DSH_HOME（本脚本存在的唯一理由）：`resolve.rs::user_dsh_home` 的
# 判据是 `cfg!(debug_assertions)` —— **任何 debug 构建**（`cargo run` / `cargo tauri dev`
# 都一样）默认都落到隔离档 `~/.dsh-dock-dev`；`DSH_HOME` 是代码里明确保留的
# 「用户主权」逃生门（显式设置最高优先）。本脚本把它**写死为 `$HOME/.dsh`**，
# 且**不受调用环境里已有的 `DSH_HOME` 影响**（一次命令 = 一个确定结果）。
#
# 想要隔离测试目录请用 ./scripts/tdev.sh。
#
# 用法：
#   ./scripts/dev.sh              # 起开发（Ctrl-C 结束；tauri dev 一并收掉子进程）
#   ./scripts/dev.sh --print      # 只打印本次用到的目录与命令，不启动
#   ./scripts/dev.sh <tauri 参数> # 其余参数原样透传给 `cargo tauri dev`
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IDENTIFIER="io.github.realguan.dsh-dock"

# 正式 dsh home：显式设置（覆盖环境里可能已导出的 DSH_HOME，见文件头）
export DSH_HOME="$HOME/.dsh"

# 仅供展示：数据目录由 identifier 决定，路径按平台推导（不参与任何功能逻辑）
case "$(uname -s)" in
    Darwin) APP_DATA="$HOME/Library/Application Support/$IDENTIFIER" ;;
    Linux) APP_DATA="${XDG_DATA_HOME:-$HOME/.local/share}/$IDENTIFIER" ;;
    *) APP_DATA="%APPDATA%\\$IDENTIFIER" ;;
esac

if [[ "${1:-}" == "--print" ]]; then
    echo "mode=dev"
    echo "identifier=$IDENTIFIER"
    echo "app_data=$APP_DATA"
    echo "DSH_HOME=$DSH_HOME"
    echo "command=cargo tauri dev"
    exit 0
fi

# 同 identifier + 同 dsh home 的另一个宿主正在跑：单实例锁会吞掉本次启动，
# 或两边抢同一份 profile / 会话写锁（ADR-0015 记的正是这类互锁事故）。
if [[ "$(uname -s)" == "Darwin" ]] && pgrep -f "DSH Dock.app/Contents/MacOS/" >/dev/null 2>&1; then
    echo "⚠️  已安装的 DSH Dock.app 正在运行：二者同 identifier 且共用 $DSH_HOME。" >&2
    echo "    建议先退出它，否则本次启动可能被单实例锁吞掉 / 与会话写锁互抢。" >&2
fi

echo "▶ dev（正式档）  dsh home=$DSH_HOME"
echo "                 app 数据目录=$APP_DATA"
cd "$ROOT/src-tauri"
exec cargo tauri dev "$@"
