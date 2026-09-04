#!/usr/bin/env bash
# fetch-pnpm-bundle.sh —— 取 pnpm 平台二进制压缩包入 resources/pnpm/（ADR-0010
# 边界 A 裁定：安装包内压缩存储；boot 期 stage_pnpm_from_bundle 经系统 tar 解包
# 落 engines/bin/pnpm）。
#
# 契约对齐（改一侧必须改另一侧）：
#   - 落位路径 = src-tauri/src/updates.rs engine_pnpm_bundle()
#     （resources/pnpm/<platform>.tgz，platform 命名见下方 case）；
#   - 版本 = src-tauri/src/updates.rs PINNED_PNPM_VERSION（本脚本从源码推导，
#     防两处漂移）。升级 pnpm 必过 ADR-0010 升级清单（runtime set / add -g /
#     spawnSync 可达 / 三平台 boot 冒烟）。
# 完整性 = packument dist.shasum（sha1）比对，不符即弃换镜像。
#
# 用法：scripts/fetch-pnpm-bundle.sh（pack 期调用，幂等覆盖；需 node + curl）
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

# 目标平台命名（与 engine_pnpm_bundle 契约一致）
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) PKG=exe.darwin-arm64 ;;
  Darwin-x86_64) PKG=exe.darwin-x64 ;;
  Linux-x86_64) PKG=exe.linux-x64 ;;
  Linux-aarch64) PKG=exe.linux-arm64 ;;
  MINGW*|MSYS*) PKG=exe.win32-x64 ;;
  *) echo "不支持的平台：$(uname -s)-$(uname -m)" >&2; exit 2 ;;
esac
# 落位名不带 exe. 前缀（engine_pnpm_bundle 契约：<platform>.tgz）
DEST="$DEST_DIR/${PKG#exe.}.tgz"

sha1_of() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 1 "$1" | cut -d' ' -f1
  else
    sha1sum "$1" | cut -d' ' -f1
  fi
}

TMP="$DEST_DIR/.fetch-tmp.tgz"
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
  curl -fsSL --max-time 300 -o "$TMP" "$tarball" || {
    echo "  tarball 下载失败，换下一镜像"; continue;
  }
  actual=$(sha1_of "$TMP")
  if [[ "$actual" != "$expected" ]]; then
    echo "  sha1 不符（$actual != $expected），弃包换下一镜像"
    rm -f "$TMP"
    continue
  fi
  mv -f "$TMP" "$DEST"
  echo "✅ pnpm $VERSION 平台二进制已就位：$DEST"
  exit 0
done

rm -f "$TMP"
echo "错误：所有镜像均不可达或校验失败（@pnpm/$PKG@$VERSION）" >&2
exit 1
