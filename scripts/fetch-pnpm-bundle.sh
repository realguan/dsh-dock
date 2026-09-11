#!/usr/bin/env bash
# fetch-pnpm-bundle.sh —— 取 pnpm 平台二进制压缩包入 resources/pnpm/（ADR-0010
# 边界 A 裁定：安装包内压缩存储；boot 期 stage_pnpm_from_bundle 经系统 tar 解包
# 落 engines/bin/pnpm）。
#
# 契约对齐（改一侧必须改另一侧）：
#   - 落位路径 = src-tauri/src/updates.rs engine_pnpm_bundle()（宿主平台份）
#     与 guest_pnpm_bundle()（WSL 客体投递份 linux-x64，ADR-0010 台账）；
#   - 版本 = src-tauri/src/updates.rs PINNED_PNPM_VERSION（本脚本从源码推导，
#     防两处漂移）。升级 pnpm 必过 ADR-0010 升级清单（runtime set / add -g /
#     spawnSync 可达 / 三平台 boot 冒烟）。
# 完整性 = packument dist.shasum（sha1）比对，不符即弃换镜像。
#
# 用法：scripts/fetch-pnpm-bundle.sh [platform ...]
#   无参 = 取当前平台份（darwin-arm64 / darwin-x64 / linux-x64 / linux-arm64 /
#   win32-x64 / win32-arm64）；显式传参 = 取指定平台份（Windows 包需 win32-x64 +
#   linux-x64 两份：宿主引擎 + WSL 客体投递）。pack 期调用，幂等覆盖；需 node + curl。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST_DIR="$ROOT/src-tauri/resources/pnpm"
mkdir -p "$DEST_DIR"

command -v node >/dev/null 2>&1 || { echo "需要 node（解析 packument）" >&2; exit 2; }
command -v curl >/dev/null 2>&1 || { echo "需要 curl" >&2; exit 2; }

# 版本单一真相源：从 updates.rs 常量推导
VERSION=$(sed -n 's/.*PINNED_PNPM_VERSION: &str = "\([^"]*\)".*/\1/p' \
  "$ROOT/src-tauri/src/updates.rs" | head -1)
[[ -n "$VERSION" ]] || { echo "无法从 updates.rs 推导 PINNED_PNPM_VERSION" >&2; exit 2; }

# 无参 = 当前平台（落位命名与 engine_pnpm_bundle 契约一致：<platform>.tgz）
# Windows 分支按 `uname -m` 区分 x64 / ARM64：Git Bash 在 Windows ARM64 上报告
# `aarch64`（Git for Windows 的 MSYS2 运行时）与 `x86_64`，两者都要有落位名，
# 否则在 ARM64 机器上会掉进 `*)` 直接退出。
if [[ $# -eq 0 ]]; then
  case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) set -- darwin-arm64 ;;
    Darwin-x86_64) set -- darwin-x64 ;;
    Linux-x86_64) set -- linux-x64 ;;
    Linux-aarch64) set -- linux-arm64 ;;
    MINGW*-aarch64|MSYS*-aarch64) set -- win32-arm64 ;;
    MINGW*|MSYS*) set -- win32-x64 ;;
    *) echo "不支持的平台：$(uname -s)-$(uname -m)" >&2; exit 2 ;;
  esac
fi

sha1_of() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 1 "$1" | cut -d' ' -f1
  else
    sha1sum "$1" | cut -d' ' -f1
  fi
}

# 取单个平台（$1 = 落位名，映射 @pnpm/exe.<platform> 包）
#
# **白名单必须覆盖 engine_pnpm_bundle() 能求出的全部平台**（否则构建到那个平台时才
# 会红，且是「内置 pnpm 从未随包发出」的运行时故障）。曾漏 `win32-arm64`：上游
# `@pnpm/exe.win32-arm64` 一直存在，而此处直接 `return 2` ⇒ 本仓脚本缺口，非上游缺口
# （2026-09-11 实测 registry 六个平台全 200）。该不变式由
# `scripts/tests/test_release_platform_matrix.py` 机器守住——**加平台时改这里，
# 那个测试会告诉你 rm 侧是否也齐**。
fetch_one() {
  local PLATFORM="$1" PKG DEST
  case "$PLATFORM" in
    darwin-arm64|darwin-x64|linux-x64|linux-arm64|win32-x64|win32-arm64)
      PKG="exe.$PLATFORM" ;;
    *) echo "不支持的平台名：$PLATFORM" >&2; return 2 ;;
  esac
  DEST="$DEST_DIR/$PLATFORM.tgz"
  local TMP REG meta parsed tarball expected actual
  TMP="$DEST_DIR/.fetch-tmp-$PLATFORM.tgz"
  for REG in https://registry.npmmirror.com https://registry.npmjs.org; do
    echo "fetch @pnpm/$PKG@${VERSION}（${REG}）…"
    meta=$(curl -fsSL --max-time 60 "$REG/@pnpm/$PKG" 2>/dev/null) || {
      echo "  packument 失败，换下一镜像"; continue;
    }
    parsed=$(printf '%s' "$meta" | node -e '
      let d="";process.stdin.on("data",c=>d+=c);process.stdin.on("end",()=>{
        try{
          const v=JSON.parse(d).versions[process.argv[1]];
          if(!v){process.exit(3)}
          process.stdout.write((v.dist?.tarball||"")+" "+(v.dist?.shasum||""));
        }catch(e){process.exit(4)}
      })' "$VERSION") || { echo "  packument 解析失败，换下一镜像"; continue; }
    tarball=${parsed% *}; expected=${parsed#* }
    if [[ -z "$tarball" || -z "$expected" ]]; then
      echo "  dist 元数据缺失，换下一镜像"; continue
    fi
    if ! curl -fsSL --max-time 300 -o "$TMP" "$tarball"; then
      echo "  tarball 下载失败，换下一镜像"; continue;
    fi
    actual=$(sha1_of "$TMP")
    if [[ "$actual" != "$expected" ]]; then
      echo "  sha1 不符（$actual != $expected），弃包换下一镜像"
      rm -f "$TMP"
      continue
    fi
    mv -f "$TMP" "$DEST"
    echo "✅ pnpm $VERSION 平台二进制已就位：$DEST"
    return 0
  done
  rm -f "$TMP"
  echo "错误：所有镜像均不可达或校验失败（@pnpm/$PKG@$VERSION）" >&2
  return 1
}

FAILED=0
for P in "$@"; do
  fetch_one "$P" || FAILED=1
done
exit "$FAILED"
