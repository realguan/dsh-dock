#!/usr/bin/env python3
"""AppImage 内部权限闸门：owner-only 权限**对构建者本人永远不可见**。

## 为什么需要（2026-09-28，AppImageHub 目录站拒收 v1.3.3 的真因）

Tauri bundler（上游 `tauri-apps/tauri#16155`，dev 分支至今未修）用
`write_and_make_executable()` 把下载的工具按 `0o770` 落盘，其中包含
`AppRun-<arch>`；`linuxdeploy.rs` 再用 `fs::copy` **保权**复制进 `<AppDir>/AppRun`，
linuxdeploy 装了 GTK 插件后把它改名为 `AppRun.wrapped` ⇒ 镜像里出现**唯一一个
「other 不可读、owner 可执行但 other 不可执行」的文件**（本仓 v1.3.3 实测
352 条中恰 1 条：`-rwxrwx--- AppRun.wrapped`）。

普通用户走 FUSE 挂载时文件属主 = 启动者，770 也够执行 ⇒ **构建者本机、CI、
任何"自己跑一遍"的验证都测不出来**；只有「root 挂载、他人 uid 运行」的场景
（firejail、AppImageHub 目录站测试、root 解包后他人运行）会在 `AppRun` 第 12 行
`exec .../AppRun.wrapped` 处 `Permission denied`，应用 11 秒内退出、无窗口 ——
整包被判定"不可用"。这正是「本机绿灯 ≠ 对外可用」的典型，必须机器兜底。

## 判据（与上游 issue 的建议一致）

① 任何条目 other 不可读 → 红
② owner 可执行但 other 不可执行 → 红
③ 目录 other 不可进入（缺 +x，可读也没用）→ 红
④ 符号链接豁免（权限恒 `lrwxrwxrwx`，不承载执行语义）
⑤ 解析不出任何条目同样判红（闸门不接受"静默绿"）

## 用法（Linux）

    offset=$("./DSH.Dock_x.y.z_amd64.AppImage" --appimage-offset)
    unsquashfs -lln -o "$offset" ./DSH.Dock_x.y.z_amd64.AppImage > listing.txt
    python3 scripts/check-appimage-perms.py listing.txt

读的是 **squashfs 元数据**（`unsquashfs -lln`）而非解包后的文件系统：不受 umask
影响，正是挂载者在 firejail 里看到的那一份。listing 是纯文本，因此**没有 Linux
机器也能复核**（由 CI 或他人产出 listing 后在本机跑本脚本）。
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path

# `unsquashfs -lln` 每行 = 5 个字段 + 文件名（文件名可含空格 ⇒ 不能按空白全切）：
#   -rwxr-xr-x 0/0 31552 2026-09-24 01:31 squashfs-root/AppRun.wrapped
# 其余行（`Parallel unsquashfs: ...`、`created 300 files`、空行）一律忽略。
_LINE = re.compile(
    r"^(?P<mode>[-dlbcps][rwxsStT-]{9})"
    r"\s+\S+"  # uid/gid（-n 下形如 0/0）
    r"\s+\S+"  # size
    r"\s+\S+"  # date
    r"\s+\S+"  # time
    r"\s+(?P<name>.*\S)\s*$"
)

OTHER_READ = 0o004
OTHER_EXEC = 0o001
OWNER_EXEC = 0o100

# `rwx` 三元组 → 位：三组分别是 owner / group / other。
_READ_BITS = (0o400, 0o040, 0o004)
_WRITE_BITS = (0o200, 0o020, 0o002)
_EXEC_BITS = (0o100, 0o010, 0o001)


@dataclass(frozen=True)
class Entry:
    """listing 里的一行：路径 + 原始 mode 字符串（10 字符，含类型位）。"""

    path: str
    mode: str
    bits: int

    @property
    def kind(self) -> str:
        """`-` 普通文件 · `d` 目录 · `l` 符号链接 · `c`/`b`/`p`/`s` 设备与 IPC。"""
        return self.mode[0]


def mode_bits(mode: str) -> int:
    """9 字符权限串 `rwxrwx---` → `0o770`（不含类型位，调用方用 `mode[1:]` 传入）。

    setuid/setgid/sticky 的 `s`/`t`（带 x 位）按可执行算，`S`/`T`（无 x 位）按不可执行算。
    """
    perms = mode
    bits = 0
    for triple in range(3):  # owner / group / other
        chunk = perms[triple * 3 : triple * 3 + 3]
        if chunk[0] == "r":
            bits |= _READ_BITS[triple]
        if chunk[1] == "w":
            bits |= _WRITE_BITS[triple]
        if chunk[2] in ("x", "s", "t"):
            bits |= _EXEC_BITS[triple]
    # setuid / setgid / sticky：`S`/`T` = 位已置但 x 未置（故与 `s`/`t` 同样计入特殊位）
    if perms[2] in ("s", "S"):
        bits |= 0o4000
    if perms[5] in ("s", "S"):
        bits |= 0o2000
    if perms[8] in ("t", "T"):
        bits |= 0o1000
    return bits


def parse_listing(text: str) -> list[Entry]:
    """解析 `unsquashfs -lln` 输出；无法识别的行直接跳过。"""
    entries: list[Entry] = []
    for line in text.splitlines():
        match = _LINE.match(line)
        if match is None:
            continue
        mode = match.group("mode")
        # 符号链接行尾部是 `-> target`：只保留链接自身路径（target 另有自己的条目）
        name = match.group("name").split(" -> ", 1)[0].rstrip()
        if not name:
            continue
        entries.append(Entry(path=name, mode=mode, bits=mode_bits(mode[1:])))
    return entries


def find_violations(entries: list[Entry]) -> list[tuple[Entry, str]]:
    """返回 (条目, 原因) 列表；空列表 = 全通过。

    目录先判「能否进入」——缺 +x 比缺 r 更致命（连下层条目都 stat 不到）；文件先判
    「owner 可执行而 other 不可执行」——正是本闸门要防的那一类（`AppRun.wrapped`）。
    """
    violations: list[tuple[Entry, str]] = []
    for entry in entries:
        if entry.kind == "l":
            continue
        if entry.kind == "d":
            if not entry.bits & OTHER_EXEC:
                violations.append((entry, "目录 other 不可进入（缺 +x）"))
            elif not entry.bits & OTHER_READ:
                violations.append((entry, "目录 other 不可读"))
            continue
        if entry.bits & OWNER_EXEC and not entry.bits & OTHER_EXEC:
            violations.append((entry, "owner 可执行但 other 不可执行"))
        elif not entry.bits & OTHER_READ:
            violations.append((entry, "other 不可读"))
    return violations


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("用法: check-appimage-perms.py <unsquashfs -lln 输出文件>", file=sys.stderr)
        return 2

    listing = Path(argv[1])
    if not listing.is_file():
        print(f"::error::listing 文件不存在：{listing}", file=sys.stderr)
        return 2

    entries = parse_listing(listing.read_text(encoding="utf-8", errors="replace"))
    if not entries:
        print(
            f"::error::{listing} 中解析不出任何条目 —— 闸门拒绝静默通过"
            "（unsquashfs 输出格式变了，或 offset 给错导致列出空表）",
            file=sys.stderr,
        )
        return 1

    violations = find_violations(entries)
    for entry, reason in violations:
        print(f"::error::{entry.mode} {entry.path}：{reason}")
    if violations:
        print(
            f"::error::AppImage 权限闸门未通过：{len(violations)}/{len(entries)} 条违规。"
            "owner-only 权限在日常使用（FUSE 挂载、属主=启动者）不可见，"
            "但 root 挂载 / 他人 uid 运行（firejail、目录站）会 Permission denied —— "
            "参见 tauri-apps/tauri#16155。",
            file=sys.stderr,
        )
        return 1

    print(f"AppImage 权限闸门通过：{len(entries)} 条全部 other 可读/可执行")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
