#!/usr/bin/env bash
# Spike 0003 CI 复验（ADR-0010 Spike①②）：三平台 pnpm12 引导链实机验证。
# 维护者仅 macOS 一台实机——Windows/Linux 由 CI runner（真实系统）顶替。
# 覆盖：平台二进制取材（registry 镜像链）/ 镜像 env 通道接管（本地坏源决定性
# 路由）/ 非 TTY runtime set / 字节进度 / shim 激活 / npm 缺位 / add -g。
# 结果判读见 docs/spikes/0003-pnpm12-engine-bootstrap.md。
set -u
FAIL=0
pass() { echo "PASS: $1"; }
fail() { echo "FAIL: $1"; FAIL=1; }

WORK="$(mktemp -d)"
cd "$WORK" || exit 1

# ① 取材：pnpm12 平台二进制（@pnpm/exe.* 普通 npm tarball，镜像链可达）
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) PKG=exe.darwin-arm64 ;;
  Darwin-x86_64) PKG=exe.darwin-x64 ;;
  Linux-x86_64) PKG=exe.linux-x64 ;;
  MINGW*|MSYS*) PKG=exe.win32-x64 ;;
  *) fail "未识别平台 $(uname -s)-$(uname -m)"; exit 1 ;;
esac
GOT_TGZ=""
for REG in https://registry.npmmirror.com https://registry.npmjs.org; do
  if curl -fsSL -o exe.tgz "$REG/@pnpm/$PKG/-/$PKG-12.3.1.tgz"; then
    echo "取材成功：$REG/@pnpm/$PKG"
    GOT_TGZ=1
    break
  fi
done
[ -n "$GOT_TGZ" ] || { fail "pnpm 二进制取材失败"; exit 1; }
tar -xzf exe.tgz
PNPM="$WORK/package/pnpm"
[ -f "$WORK/package/pnpm.exe" ] && PNPM="$WORK/package/pnpm.exe"
"$PNPM" --version >/dev/null 2>&1 || { fail "pnpm 二进制不可执行"; exit 1; }
pass "pnpm 12.3.1 平台二进制可执行: ${PKG}"

# PNPM_HOME：Windows 原生 pnpm.exe 只认 Windows 路径（git-bash 互操作）
PNPM_HOME_POSIX="$WORK/engines"
mkdir -p "$PNPM_HOME_POSIX/bin"
if command -v cygpath >/dev/null 2>&1; then
  export PNPM_HOME="$(cygpath -w "$PNPM_HOME_POSIX")"
  BIN_ENTRY="$(cygpath -w "$PNPM_HOME_POSIX/bin")"
else
  export PNPM_HOME="$PNPM_HOME_POSIX"
  BIN_ENTRY="$PNPM_HOME_POSIX/bin"
fi
export PATH="$BIN_ENTRY:$PATH"

# ② 镜像 env 通道决定性路由：键=release 指向本地坏源，必须被打到
PY="$(command -v python3 || command -v python)"
"$PY" -m http.server 8799 --bind 127.0.0.1 >/dev/null 2>&1 &
SRV=$!
sleep 2
if PNPM_CONFIG_NODE_DOWNLOAD_MIRRORS='{"release":"http://127.0.0.1:8799/"}' \
  "$PNPM" runtime set node 22.20.0 >badmirror.log 2>&1; then
  fail "坏镜像未生效（下载成功 = env 通道被忽略）"
else
  if grep -q "127.0.0.1:8799" badmirror.log; then
    pass "镜像 env 通道被接管（PNPM_CONFIG_NODE_DOWNLOAD_MIRRORS 键=release）"
  else
    fail "下载失败但未指向本地坏源：$(head -c 300 badmirror.log)"
  fi
fi
kill $SRV 2>/dev/null

# ③ 生产镜像非 TTY runtime set + 字节进度 + 激活 + npm 缺位
if PNPM_CONFIG_NODE_DOWNLOAD_MIRRORS='{"release":"https://npmmirror.com/mirrors/node/"}' \
  "$PNPM" runtime set node 24.18.0 >runtime.log 2>&1; then
  pass "非 TTY runtime set node 24.18.0 成功（npmmirror）"
else
  fail "runtime set 失败：$(tail -c 300 runtime.log)"
fi
grep -q "Downloading node@runtime" runtime.log &&
  pass "字节级进度行可解析（boot:progress 映射可行）" ||
  fail "未见字节级进度行"

"$PNPM" shim add node >/dev/null 2>&1 || fail "shim add node 失败"
NODE_SHIM="$(ls "$PNPM_HOME_POSIX"/bin/node* 2>/dev/null | head -1)"
if [ -n "$NODE_SHIM" ] && "$NODE_SHIM" -v >/dev/null 2>&1; then
  pass "引擎 node 已激活（PNPM_HOME/bin）"
else
  fail "引擎 node 未激活"
fi
REAL="$("$NODE_SHIM" -p process.execPath 2>/dev/null)"
[ -n "$REAL" ] || REAL="$NODE_SHIM"
# npm 缺位判定：node 树内不得有 npm/npx/corepack CLI（Windows 布局多出
# 文档与 package.json 属正常，不参与判定；node_modules 内容如实打印供记录）
NDIR="$(dirname "$REAL")"
echo "---- 引擎 node 树根：$(ls "$NDIR" | tr '\n' ' ')"
echo "---- node_modules：$(ls "$NDIR/node_modules" 2>/dev/null | tr '\n' ' ')"
BAD="$( { ls "$NDIR"; ls "$NDIR/node_modules" 2>/dev/null; } | grep -iE '^(npm|npx|corepack)' | tr '\n' ' ')"
if [ -z "$BAD" ]; then
  pass "npm/npx/corepack 缺位（引擎 node 树内无 CLI）"
else
  fail "引擎 node 树内发现包管理器 CLI：$BAD"
fi

# ④ 引擎 pnpm add -g（global bin 目录 + shim 链 + registry 镜像）
if "$PNPM" add -g semver@7.7.2 --registry=https://registry.npmmirror.com >addg.log 2>&1; then
  SEMVER="$(ls "$PNPM_HOME_POSIX"/bin/semver* 2>/dev/null | head -1)"
  if [ -n "$SEMVER" ] && { [ -x "$SEMVER" ] && "$SEMVER" -h >/dev/null 2>&1 ||
    sh "$SEMVER" -h >/dev/null 2>&1; }; then
    pass "引擎 pnpm add -g 可用（semver shim 可执行）"
  else
    fail "semver shim 不可执行"
  fi
else
  fail "add -g 失败：$(tail -c 300 addg.log)"
fi

echo "=========================================="
if [ "$FAIL" = "0" ]; then
  echo "SPIKE-0003 三平台 CI 复验：全部通过"
  exit 0
fi
echo "SPIKE-0003 三平台 CI 复验：存在失败"
exit 1
