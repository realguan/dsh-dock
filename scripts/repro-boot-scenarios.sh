#!/usr/bin/env bash
# repro-boot-scenarios.sh —— 引擎引导多场景复现（ADR-0010 boot 链调试用）。
#
# 背景：engines/ 引擎目录登记于 AGENTS §6 运行时持久化例外册——「可丢失可重建，
# 缺失走引导」。本脚本通过裁剪 engines/ 内**壳自有资产**构造不同就绪状态，
# 重启应用观察 boot:step 五步链（01 环境检测 → 02 准备引擎 → 03 启动工作台 →
# 04 等待就绪 → 05 进入工作台）与引导日志。
#
# 红线守护（脚本自身约束）：
#   - 只动 <数据目录>/engines/ 内壳资产（pnpm/node/dsh 布局见
#     src-tauri/src/engines.rs pnpm_home / engine_bin_dir）；
#   - 不碰 settings.json / node-map.json*（--deep 明确指定才清缓存类），
#     绝不碰 dsh_home（默认 ~/.dsh）与三件套；
#   - 场景变更前先退出运行中的壳（单实例锁会吞掉二次启动）。
#
# 用法：
#   ./repro-boot-scenarios.sh status                 # 引擎三件版本 + 磁盘占用
#   ./repro-boot-scenarios.sh kill                   # 退出 dev 壳与已安装应用
#   ./repro-boot-scenarios.sh clean [--with-cache|--deep]
#                                 # 删 engines/；--with-cache 连探测缓存，
#                                 # --deep 再连 node-map 缓存（离线降级实验用）
#   ./repro-boot-scenarios.sh scenario <名> [--dry]  # 构造场景（--dry 只预览）
#   ./repro-boot-scenarios.sh watch                  # tail shell.log + dsh-shell.log
#   ./repro-boot-scenarios.sh full <名>              # kill → scenario → 后台 dev → 跟日志
#
# 场景一览（引导分支锚点 = engines.rs::bootstrap / resolve.rs::resolve_launch）：
#   fresh        删除整个 engines/          → 完整首启链：重铺 pnpm → runtime set
#                                           node（下载 ~52MB，boot:progress）→
#                                           pnpm add -g dsh。需联网。
#   no-pnpm      只删 bin/pnpm              → 每次 boot 重铺捆绑 pnpm 的自愈路径。
#   no-node      删 bin/node + node 运行时  → 只重下 node（runtime set）。
#   stale-node   bin/node 换成假 v24.17.0   → 「版本不符 → 幂等切换」分支。
#   corrupt-node bin/node 换成退出非 0 假体 → 探测失败按缺失处理的分支。
#   no-dsh       删 global/ + bin/dsh       → 只重装 dsh（dist-tags 惰性解析）。
#   ready        不做任何改动（对照组）     → 就绪引擎幂等快启、零网络。
#
# 离线语义实验（脚本不断网，手动关 Wi-Fi 后跑）：
#   ready + --deep（连 node-map 缓存一起清）→ node-map 解析失败 → 用已装 node
#   降级启动（bootstrap 警告「用已装引擎 node 继续启动」）；
#   fresh + 断网 → node 缺失且期望版本无从解析 → 硬错误卡（首启必须联网）。

set -u

# ---------- 布局锚点（与 tauri.conf.json identifier / engines.rs 对齐） ----------
IDENTIFIER="io.github.realguan.dsh-dock"
REPO="$(cd "$(dirname "$0")/.." && pwd)"

case "$(uname -s)" in
  Darwin) DATA_DIR="$HOME/Library/Application Support/$IDENTIFIER" ;;
  Linux)  DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/$IDENTIFIER" ;;
  *)
    echo "本脚本仅覆盖 macOS/Linux；Windows 数据目录为 %APPDATA%\\${IDENTIFIER}，请手动操作" >&2
    exit 1
    ;;
esac

ENGINES="$DATA_DIR/engines"
BIN="$ENGINES/bin"
SHELL_LOG="$DATA_DIR/shell.log"
DSH_LOG="$DATA_DIR/dsh-shell.log"

if [ "$(uname -s)" = "Darwin" ]; then
  DEV_BIN="$REPO/src-tauri/target/debug/dsh-dock"
else
  DEV_BIN="$REPO/src-tauri/target/debug/dsh-dock"
fi

die() { echo "错误：$*" >&2; exit 1; }
info() { echo "==> $*"; }

# 引擎 bin 内工具版本（探测口径对齐 engines.rs::probe_engine：--version + env 注入）。
engine_version() {
  local tool="$1"
  [ -x "$BIN/$tool" ] || { echo "缺失"; return; }
  local v
  v=$(PNPM_HOME="$ENGINES" PATH="$BIN:$PATH" "$BIN/$tool" --version 2>/dev/null | head -1)
  echo "${v:-不可执行}"
}

app_running() {
  pgrep -f "$DEV_BIN" >/dev/null 2>&1 || pgrep -x "dsh-dock" >/dev/null 2>&1
}

# 退出壳：已安装应用 + dev 构建 + 本仓库的 cargo tauri dev / vite（1420）。
# 单实例插件会让二次启动静默唤起既有实例——不先退出则场景构造后看不到新 boot。
kill_app() {
  info "退出运行中的 DSH Dock…"
  if [ "$(uname -s)" = "Darwin" ]; then
    osascript -e 'tell application "DSH Dock" to quit' >/dev/null 2>&1 || true
  fi
  pkill -f "$DEV_BIN" 2>/dev/null && echo "  已退出 dev 壳" || true
  # 本仓库专属的 tauri dev 监护与 vite dev server（端口 1420）
  pgrep -fl "tauri dev" 2>/dev/null | grep -i "dsh-dock" | awk '{print $1}' | xargs kill 2>/dev/null || true
  lsof -ti :1420 2>/dev/null | xargs kill 2>/dev/null || true
  sleep 1
}

cmd_status() {
  echo "仓库：      $REPO"
  echo "数据目录：  $DATA_DIR"
  [ -d "$DATA_DIR" ] || { echo "  （不存在——应用尚未首次运行）"; return; }
  echo "引擎目录：  $ENGINES $([ -d "$ENGINES" ] && du -sh "$ENGINES" 2>/dev/null | awk '{print "（" $1 "）"}')"
  echo
  echo "引擎三件（engines/bin 实测 --version，对齐 probe_engine）："
  printf "  pnpm: %s\n  node: %s\n  dsh:  %s\n" \
    "$(engine_version pnpm)" "$(engine_version node)" "$(engine_version dsh)"
  echo
  echo "node 运行时目录："
  ls -d "$ENGINES/node_modules/.pnpm/"node@runtime* 2>/dev/null | sed 's/^/  /' || echo "  （无）"
  echo "dsh 全局虚拟 store："
  ls -d "$ENGINES/global/"*/ 2>/dev/null | sed 's/^/  /' || echo "  （无——dsh 未装或场景已构造）"
  echo
  echo "缓存/日志："
  for f in settings.json node-map.json probe-cache.json; do
    [ -f "$DATA_DIR/$f" ] && echo "  $f"
  done
  [ -f "$SHELL_LOG" ] && echo "  shell.log $(du -h "$SHELL_LOG" | awk '{print $1}')"
}

cmd_clean() {
  local with_cache=0 deep=0
  while [ $# -gt 0 ]; do
    case "$1" in
      --with-cache) with_cache=1 ;;
      --deep) deep=1; with_cache=1 ;;
      *) die "未知选项：${1}（可选：--with-cache | --deep）" ;;
    esac
    shift
  done
  [ -d "$ENGINES" ] || die "$ENGINES 不存在，无需清理"
  app_running && die "应用正在运行，先执行 kill 再清理"
  info "删除 ${ENGINES}（例外册：可丢失可重建，缺失走引导）"
  rm -rf "$ENGINES"
  if [ "$with_cache" = 1 ]; then
    info "删除探测缓存 probe-cache.json"
    rm -f "$DATA_DIR/probe-cache.json"
  fi
  if [ "$deep" = 1 ]; then
    info "删除 node-map 缓存 node-map.json(.sig)（下次 boot 需重新拉取映射包）"
    rm -f "$DATA_DIR/node-map.json" "$DATA_DIR/node-map.json.sig"
  fi
  info "完成。settings.json / ~/.dsh 未动。"
}

# 预览/执行一批删除（参数 = 路径列表）；dry 模式只打印。
apply_rm() {
  local dry="$1"; shift
  for p in "$@"; do
    [ -e "$p" ] || [ -L "$p" ] || continue
    if [ "$dry" = 1 ]; then
      echo "  [dry] 将删除：$p"
    else
      rm -rf "$p" && echo "  已删除：$p"
    fi
  done
}

# 在 bin 内落一个假工具（stale / corrupt 场景用）。
write_fake_bin() {
  local dry="$1" name="$2" body="$3"
  if [ "$dry" = 1 ]; then
    echo "  [dry] 将写入假二进制：$BIN/$name"
    return
  fi
  rm -f "$BIN/$name"
  printf '%s\n' "$body" > "$BIN/$name"
  chmod 755 "$BIN/$name"
  echo "  已写入假二进制：$BIN/$name"
}

cmd_scenario() {
  local name="${1:-}"
  local dry=0
  [ "${2:-}" = "--dry" ] && dry=1
  [ -n "$name" ] || die "用法：scenario <fresh|no-pnpm|no-node|stale-node|corrupt-node|no-dsh|ready> [--dry]"
  [ -d "$ENGINES" ] || die "$ENGINES 不存在——先用 scenario fresh（即等价于 clean）"
  if [ "$dry" = 0 ] && [ "$name" != "ready" ]; then
    app_running && die "应用正在运行，先执行 kill 再构造场景"
  fi

  case "$name" in
    fresh)
      info "场景 fresh：删除整个 engines/ → 完整首启引导链（需联网）"
      if [ "$dry" = 1 ]; then
        echo "  [dry] 将删除：$ENGINES"
      else
        app_running && die "应用正在运行，先执行 kill"
        rm -rf "$ENGINES" && echo "  已删除：$ENGINES"
      fi
      ;;
    no-pnpm)
      info "场景 no-pnpm：删 bin/pnpm → 每次 boot 重铺捆绑 pnpm 的自愈路径"
      apply_rm "$dry" "$BIN/pnpm"
      ;;
    no-node)
      info "场景 no-node：删 node 运行时与 bin/node → 只重下 node（runtime set，需联网）"
      apply_rm "$dry" "$BIN/node" "$ENGINES/node_modules/node"
      for d in "$ENGINES/node_modules/.pnpm/"node@runtime*; do
        apply_rm "$dry" "$d"
      done
      ;;
    stale-node)
      info "场景 stale-node：bin/node 假装 v24.17.0 → 版本不符走幂等切换（node-map 期望 v24.18.0）"
      write_fake_bin "$dry" node '#!/bin/sh
echo "v24.17.0"'
      ;;
    corrupt-node)
      info "场景 corrupt-node：bin/node 退出非 0 → 探测失败按缺失处理"
      write_fake_bin "$dry" node '#!/bin/sh
echo "simulated corrupted node binary" >&2
exit 1'
      ;;
    no-dsh)
      info "场景 no-dsh：删 global/ 与 bin/dsh → 只重装 dsh（dist-tags 解析 + pnpm add -g，需联网）"
      apply_rm "$dry" "$ENGINES/global" "$BIN/dsh"
      ;;
    ready)
      info "场景 ready：不做任何改动（对照组：就绪引擎幂等快启、零网络）"
      ;;
    *)
      die "未知场景：${name}（可选：fresh|no-pnpm|no-node|stale-node|corrupt-node|no-dsh|ready）"
      ;;
  esac

  if [ "$dry" = 0 ]; then
    echo
    info "场景已就绪。启动方式二选一："
    echo "  ./repro-boot-scenarios.sh full $name   # 后台起 cargo tauri dev 并跟日志"
    echo "  或手动：cd $REPO && cargo tauri dev"
  fi
}

cmd_watch() {
  [ -f "$SHELL_LOG" ] || touch "$SHELL_LOG"
  info "跟踪 $SHELL_LOG 与 dsh-shell.log（Ctrl-C 退出）"
  tail -F "$SHELL_LOG" "$DSH_LOG" 2>/dev/null
}

# kill → scenario → 后台 cargo tauri dev → 跟 dev stdout（debug 构建日志双写
# stdout，init_tracing TeeWriter：dev 终端/本日志即 boot 第一现场）。
cmd_full() {
  local name="${1:-}"
  [ -n "$name" ] || die "用法：full <场景名>"
  kill_app
  cmd_scenario "$name"
  local out="/tmp/dsh-dock-dev-$name.log"
  info "后台启动 cargo tauri dev（日志：${out}）…"
  ( cd "$REPO" && nohup cargo tauri dev > "$out" 2>&1 & )
  sleep 2
  [ -s "$out" ] || echo "  （dev 日志暂时为空，Rust 编译可能需要一段时间）"
  info "跟日志中（Ctrl-C 只退出 tail，dev 继续跑）："
  tail -F "$out"
}

case "${1:-}" in
  status)   cmd_status ;;
  kill)     kill_app ;;
  clean)    shift; cmd_clean "$@" ;;
  scenario) shift; cmd_scenario "$@" ;;
  watch)    cmd_watch ;;
  full)     shift; cmd_full "$@" ;;
  ""|-h|--help|help)
    # 打印头部说明：到「布局锚点」分隔线或首个非注释行为止（不依赖行号）。
    awk 'NR==1{next} /^# ----------/{exit} !/^#/{exit} {sub(/^# ?/,""); print}' "$0"
    ;;
  *) die "未知命令：$1（可选：status|kill|clean|scenario|watch|full，help 看用法）" ;;
esac
