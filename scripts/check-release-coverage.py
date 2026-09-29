#!/usr/bin/env python3
"""
check-release-coverage.py —— 发布日志「账目必须平」闸门（2026-09-29 立）。

# 为什么需要它

v1.3.4 的发版暴露了一个结构性缺陷：发布日志由作者**凭记忆**写，而不是从
`git log <上个 tag>..<tag>` **推导**。结果是区间内 10 个提交里有 4 个用户可感知的
改动（会话维护模块下线、安全模式删除、实验能力裁剪、插件操作体验四连）**一条都没写**——
而且 CI 照样绿：既有门禁只 `grep` 标题在不在，**内容漏一半它查不出来**。

更糟的是，那 4 个提交是 9-24 合入 master 的，到 9-29 还没发版，**五天里没有任何东西
提示「这些尚未发布」**。所以这不是态度问题——只要作者看的是「最近的工作」，就必然漏。

# 本闸门的判据（刻意做成 fail-closed）

发布日志对应版本的小节里必须带一条**对用户不可见**的覆盖清单：

    ## [v1.3.4] - 2026-09-29
    <!-- coverage: 930d6a0 94d5e7d d4ba4c0 ... -->

要求 **集合完全相等** 于 `git log <上个tag>..<tag>` 的短哈希集合：
多列、少列、写错哈希、漏写整条清单，全部红。

三个刻意的设计选择：

1. **列全部提交，不做「内部 / 用户可见」分类。** 分类判断留给正文（那是人的事），
   机器只管**账目齐不齐**。这样将来出现新的提交类型（比如某次 `chore` 其实动了用户可见
   行为）**不可能**被静默放过——而按 type 猜是否用户可见必然误判
   （`refactor(session)!` 也是 refactor）。
2. **集合相等而非包含。** 防止「清单是上一版抄来的、没更新」这类形态。
3. **HTML 注释承载。** 渲染后完全不可见，不污染发行正文。

# 用法

    scripts/check-release-coverage.py v1.3.4            # 校验（tag 或任意 rev 均可）
    scripts/check-release-coverage.py v1.3.4 --print    # 只打印区间与应填清单，不判定
    scripts/check-release-coverage.py v1.3.4 --prev v1.3.3   # 手工指定上个 tag

无上个 tag（首次发版）时跳过并说明——不制造假红。
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys

COVERAGE_RE = re.compile(r"<!--\s*coverage:\s*([0-9a-fA-F\s]+?)\s*-->")


def parse_coverage(notes_text: str, version: str) -> list[str] | None:
    """从发布日志里取出该版本的覆盖清单。

    返回 None = 小节里没有清单（与「清单为空」区分：后者是作者写了个空的注释）。
    """
    for section in split_sections(notes_text):
        header = section.split("\n", 1)[0]
        if not re.search(rf"\[v?{re.escape(version)}\]", header):
            continue
        m = COVERAGE_RE.search(section)
        return m.group(1).split() if m else None
    return None


def split_sections(notes_text: str) -> list[str]:
    """按 `## ` 切分版本小节（与 extract-release-notes.py 同口径）。"""
    return [s for s in re.split(r"\n(?=## )", notes_text) if s.startswith("## ")]


def compare_coverage(expected: list[str], actual: list[str]) -> tuple[list[str], list[str]]:
    """返回 (缺失, 多余)。两侧都按集合语义比较，便于给出可操作的报错。"""
    e, a = set(expected), set(actual)
    missing = sorted(e - a)
    extra = sorted(a - e)
    return missing, extra


VERSION_RE = re.compile(r"\d+(\.\d+)*([-+].*)?$")


def resolve_version(tag: str, override: str | None) -> str | None:
    """发布日志里的版本号；推不出来返回 None。

    为什么需要 override：发版流程里本脚本常在 **tag 建立之前** 被跑（先补清单再打 tag），
    此时入参是 `HEAD` 或某个 sha——从它推版本号不可能，必须显式给 `--version`。
    反过来若默默取 `tag.lstrip("v")`，就会去读 `[vHEAD]` 小节，报出「缺少覆盖清单」
    这种**指向错误原因**的错，把人带沟里（本次实现时就先踩了一次）。
    """
    if override:
        return override.lstrip("v")
    candidate = tag.lstrip("v")
    return candidate if VERSION_RE.fullmatch(candidate) else None


def commit_range(rev: str, prev: str | None, repo: str) -> list[str] | None:
    """区间内的短哈希；rev 不是合法版本对象时返回 None（由调用方给可读报错）。

    为什么要容错：发版流程里本脚本常在 **tag 建立之前** 被跑一次（先补清单再打 tag），
    此时 `v1.3.4` 还不是合法 rev。抛 subprocess 栈对作者毫无信息量，
    故这里转成一条可操作的提示。
    """
    spec = f"{prev}..{rev}" if prev else rev
    try:
        out = subprocess.check_output(
            ["git", "-C", repo, "log", "--format=%h", spec],
            text=True,
            stderr=subprocess.DEVNULL,
        )
    except subprocess.CalledProcessError:
        return None
    return [h for h in out.split() if h]


NOTES_ONLY_PATH = "docs/RELEASE_NOTES.md"


def touches_only_release_notes(sha: str, repo: str) -> bool:
    """该提交是否**只**改了 docs/RELEASE_NOTES.md。

    为什么需要这条豁免：覆盖清单本身写在发布日志里，而**承载清单的那个提交无法列出自己**
    （哈希在提交后才存在）。这不是 hacks——只改发布日志的提交按定义不可能改变用户可见行为，
    所以豁免它是**可靠的**，而不是为了方便开后门。

    对应的流程约定（写进 docs/prompts/release-notes.md）：
    覆盖清单**单独一个提交**（或作为最后一个提交），不要和内容改动混在同一个提交里——
    混在一起时那个提交既改了内容又改不了账，会被本闸门正确地判红。
    """
    out = subprocess.check_output(
        ["git", "-C", repo, "show", "--pretty=format:", "--name-only", sha],
        text=True,
    )
    files = [f for f in out.split() if f]
    return bool(files) and set(files) <= {NOTES_ONLY_PATH}


def previous_tag(rev: str, repo: str) -> str | None:
    """`rev` 之前最近的一个 tag；没有则 None（首次发版）。"""
    try:
        return subprocess.check_output(
            ["git", "-C", repo, "describe", "--tags", "--abbrev=0", f"{rev}^"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip() or None
    except subprocess.CalledProcessError:
        return None


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description="发布日志覆盖清单闸门（账目必须平）")
    p.add_argument("tag", help="要校验的 tag 或任意 rev")
    p.add_argument("--prev", default=None, help="手工指定上个 tag（默认自动取最近一个）")
    p.add_argument(
        "--version",
        default=None,
        help="发布日志里的版本号（如 1.3.4）。tag 未建、用 HEAD 预演时**必须**给——"
        "否则无从知道该读哪个小节。",
    )
    p.add_argument("--repo", default=".", help="仓库路径（默认当前目录）")
    p.add_argument("--notes", default=None, help="RELEASE_NOTES.md 路径（默认 <repo>/docs/RELEASE_NOTES.md）")
    p.add_argument("--print", dest="print_only", action="store_true", help="只打印区间与应填清单，不判定")
    return p


def main() -> int:
    args = build_parser().parse_args()
    repo = os.path.abspath(args.repo)
    notes_path = args.notes or os.path.join(repo, "docs", "RELEASE_NOTES.md")
    version = resolve_version(args.tag, args.version)
    if version is None:
        print(
            f"::error::无法从 `{args.tag}` 推出发布日志的版本号（它不像 vX.Y.Z）。\n"
            f"tag 未建、用 HEAD/某个 sha 预演时请显式给版本：\n"
            f"  scripts/check-release-coverage.py {args.tag} --prev {args.prev or '<上个 tag>'} --version 1.3.4",
            file=sys.stderr,
        )
        return 1

    prev = args.prev or previous_tag(args.tag, repo)
    if prev is None and not args.prev:
        print(
            f"::notice::未找到 {args.tag} 之前的 tag（首次发版？）——覆盖清单闸门跳过。",
        )
        return 0

    commits = commit_range(args.tag, prev, repo)
    if commits is None:
        print(
            f"::error::{args.tag} 不是合法的版本对象（git log {prev}..{args.tag} 失败）。\n"
            f"tag 尚未创建时属正常——用 --print 配合一个真实 rev 预演，例如：\n"
            f"  scripts/check-release-coverage.py HEAD --prev {prev} --print",
            file=sys.stderr,
        )
        return 1
    line = " ".join(sorted(commits))

    if args.print_only:
        excused = [h for h in commits if touches_only_release_notes(h, repo)]
        print(f"区间: {prev}..{args.tag}（{len(commits)} 个提交）")
        print(f"应填：<!-- coverage: {line} -->")
        if excused:
            print(f"（其中 {len(excused)} 个只动发布日志、闸门会自动豁免：{' '.join(excused)}）")
        return 0

    if not os.path.isfile(notes_path):
        print(f"::error::找不到发布日志：{notes_path}", file=sys.stderr)
        return 1

    with open(notes_path, "r", encoding="utf-8") as f:
        notes = f.read()

    actual = parse_coverage(notes, version)
    if actual is None:
        print(
            f"::error::docs/RELEASE_NOTES.md 的 [v{version}] 小节缺少覆盖清单。\n"
            f"区间 {prev}..{args.tag} 共 {len(commits)} 个提交，请在小节标题下方加一行：\n"
            f"<!-- coverage: {line} -->\n"
            f"（该注释渲染后不可见，只供本闸门核对「账目是否平」。逐条读一遍区间提交，"
            f"把用户可感知的改动写进正文——这正是 v1.3.4 漏掉 4 条的那个坑。）",
            file=sys.stderr,
        )
        return 1

    missing, extra = compare_coverage(commits, actual)
    # 豁免「只动发布日志」的提交（承载清单的那个提交无法列出自己）
    excused = [h for h in missing if touches_only_release_notes(h, repo)]
    missing = [h for h in missing if h not in excused]
    if missing or extra:
        detail = []
        if missing:
            detail.append(f"漏列 {len(missing)} 个：{' '.join(missing)}")
        if extra:
            detail.append(f"多列 {len(extra)} 个：{' '.join(extra)}")
        print(
            f"::error::[v{version}] 覆盖清单与区间 {prev}..{args.tag} 不一致——"
            + "；".join(detail)
            + f"\n正确清单：<!-- coverage: {line} -->",
            file=sys.stderr,
        )
        return 1

    note = f"（另有 {len(excused)} 个「只动发布日志」的提交已豁免）" if excused else ""
    print(
        f"✅ [v{version}] 覆盖清单与区间 {prev}..{args.tag} 完全一致"
        f"（{len(commits)} 个提交）{note}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
