"""check-release-coverage.py 的单元回归测试。

只测纯逻辑（解析 / 比对），不碰 git —— git 侧由 CI 在真 tag 上跑。
"""

from __future__ import annotations

import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

_SCRIPT_PATH = Path(__file__).parents[1] / "check-release-coverage.py"
_SPEC = importlib.util.spec_from_file_location("check_release_coverage", _SCRIPT_PATH)
assert _SPEC is not None and _SPEC.loader is not None
_MODULE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_MODULE)

parse_coverage = _MODULE.parse_coverage
resolve_version = _MODULE.resolve_version
previous_tag = _MODULE.previous_tag
all_tags = _MODULE.all_tags
compare_coverage = _MODULE.compare_coverage
split_sections = _MODULE.split_sections


NOTES = """# Release Notes

## [v1.3.4] - 2026-09-29
<!-- coverage: aaa111 bbb222 ccc333 -->

### 🌟 核心亮点 (Highlights)
- 这一版做了什么

---

## [v1.3.3] - 2026-09-23

### 🌟 核心亮点 (Highlights)
- 上一版
"""


class ParseCoverageTest(unittest.TestCase):
    def test_reads_hashes_of_the_matching_section(self) -> None:
        self.assertEqual(parse_coverage(NOTES, "1.3.4"), ["aaa111", "bbb222", "ccc333"])

    def test_accepts_v_prefixed_version_argument(self) -> None:
        self.assertEqual(parse_coverage(NOTES, "v1.3.4"), ["aaa111", "bbb222", "ccc333"])

    def test_absent_list_returns_none_not_empty(self) -> None:
        """「没写清单」与「写了个空清单」必须区分：前者是漏写，后者是作者明确写了空。"""
        self.assertIsNone(parse_coverage(NOTES, "1.3.3"))
        self.assertIsNone(parse_coverage(NOTES, "9.9.9"))

    def test_empty_comment_reads_as_empty_list(self) -> None:
        notes = "## [v2.0.0] - 2026-01-01\n<!-- coverage:  -->\n"
        self.assertEqual(parse_coverage(notes, "2.0.0"), [])

    def test_does_not_leak_across_sections(self) -> None:
        """清单只在小节内生效——否则上一版的清单会被当成这一版的。"""
        notes = "## [v1.0.0] - 2026-01-01\n<!-- coverage: aaa111 -->\n\n## [v1.0.1] - 2026-01-02\n- 无清单\n"
        self.assertIsNone(parse_coverage(notes, "1.0.1"))

    def test_exact_version_match_does_not_catch_substring(self) -> None:
        """[v1.3.4] 不得被 [v1.3.40] 命中（与 extract-release-notes 同款防误伤）。"""
        notes = "## [v1.3.40] - 2026-01-01\n<!-- coverage: aaa111 -->\n"
        self.assertIsNone(parse_coverage(notes, "1.3.4"))


class CompareCoverageTest(unittest.TestCase):
    def test_equal_sets_pass(self) -> None:
        self.assertEqual(compare_coverage(["a", "b"], ["b", "a"]), ([], []))

    def test_reports_missing_and_extra(self) -> None:
        missing, extra = compare_coverage(["a", "b", "c"], ["a", "x"])
        self.assertEqual(missing, ["b", "c"])
        self.assertEqual(extra, ["x"])

    def test_order_does_not_matter(self) -> None:
        self.assertEqual(compare_coverage(["c", "b", "a"], ["a", "b", "c"]), ([], []))

    def test_duplicates_do_not_mask_a_missing_entry(self) -> None:
        """清单里重复写同一个哈希，不能因此掩盖另一个缺失。"""
        missing, extra = compare_coverage(["a", "b"], ["a", "a"])
        self.assertEqual(missing, ["b"])
        self.assertEqual(extra, [])


class ResolveVersionTest(unittest.TestCase):
    """发版前用 HEAD 预演是常态，版本推导必须可靠——推不出来就要报错，不能猜。"""

    def test_tag_form_yields_version(self) -> None:
        self.assertEqual(resolve_version("v1.3.4", None), "1.3.4")
        self.assertEqual(resolve_version("1.3.4", None), "1.3.4")

    def test_override_wins_and_accepts_v_prefix(self) -> None:
        self.assertEqual(resolve_version("HEAD", "1.3.4"), "1.3.4")
        self.assertEqual(resolve_version("HEAD", "v1.3.4"), "1.3.4")

    def test_non_version_rev_yields_none_instead_of_guessing(self) -> None:
        """`HEAD` / sha 绝不能变成 `[vHEAD]` 这种假版本号——那会报出指向错误原因的错。"""
        self.assertIsNone(resolve_version("HEAD", None))
        self.assertIsNone(resolve_version("af4f77e", None))
        self.assertIsNone(resolve_version("master", None))

    def test_prerelease_tag_is_accepted(self) -> None:
        self.assertEqual(resolve_version("v1.4.0-rc.1", None), "1.4.0-rc.1")


class PreviousTagFailClosedTest(unittest.TestCase):
    """**不要让 v1.3.4 那次「CI 静默跳过」重演。**

    事故形态：CI 的 actions/checkout 默认浅克隆且不带 tag ⇒ `git describe` 找不到
    上个 tag ⇒ 旧实现把「找不到」当成「首次发版」放行 ⇒ 闸门报绿但什么都没查。
    这正是本闸门要消灭的那类假绿，所以必须在「有 tag 却取不到」时**报错而非跳过**。

    这里用真实 git 仓库复现「浅克隆」：clone 出 depth=1 且不带 tag 的副本。
    """

    def _git(self, *args: str, cwd: str) -> None:
        subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True)

    def test_lightweight_clone_without_tags_must_not_be_treated_as_first_release(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            origin = os.path.join(tmp, "origin")
            light = os.path.join(tmp, "light")
            os.makedirs(origin)
            self._git("init", "-q", cwd=origin)
            self._git("config", "user.email", "t@t", cwd=origin)
            self._git("config", "user.name", "t", cwd=origin)
            Path(origin, "f.txt").write_text("a")
            self._git("add", ".", cwd=origin)
            self._git("commit", "-qm", "c1", cwd=origin)
            self._git("tag", "v1.0.0", cwd=origin)
            Path(origin, "f.txt").write_text("b")
            self._git("commit", "-qam", "c2", cwd=origin)
            self._git("tag", "v1.1.0", cwd=origin)

            # 模拟 CI：浅克隆且不带 tag
            self._git("clone", "-q", "--depth", "1", "--no-tags", f"file://{origin}", light, cwd=tmp)

            self.assertEqual(all_tags(light), [], "前提：浅克隆确实没取到 tag")
            tag, reason = previous_tag("v1.1.0", light)
            self.assertIsNone(tag)
            self.assertIsNotNone(
                reason,
                "仓库有 tag 却取不到时必须给出原因（fail-closed），"
                "不得当作『首次发版』放行——v1.3.4 就是这样在 CI 里静默跳过的",
            )
            self.assertIn("fetch-depth", reason or "")

    def test_full_clone_finds_previous_tag(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            origin = os.path.join(tmp, "origin")
            full = os.path.join(tmp, "full")
            os.makedirs(origin)
            self._git("init", "-q", cwd=origin)
            self._git("config", "user.email", "t@t", cwd=origin)
            self._git("config", "user.name", "t", cwd=origin)
            Path(origin, "f.txt").write_text("a")
            self._git("add", ".", cwd=origin)
            self._git("commit", "-qm", "c1", cwd=origin)
            self._git("tag", "v1.0.0", cwd=origin)
            Path(origin, "f.txt").write_text("b")
            self._git("commit", "-qam", "c2", cwd=origin)
            self._git("tag", "v1.1.0", cwd=origin)

            self._git("clone", "-q", f"file://{origin}", full, cwd=tmp)
            self._git("fetch", "-q", "--tags", cwd=full)
            tag, reason = previous_tag("v1.1.0", full)
            self.assertEqual(tag, "v1.0.0")
            self.assertIsNone(reason)


class NotesOnlyPathTest(unittest.TestCase):
    """豁免判据的常量必须与发布日志真实路径一致。

    这条看着像废话，但它是整条豁免逻辑的**唯一锚点**：写错了（比如少个 docs/）
    就会静默地把所有提交都判成「非豁免」——那是 fail-closed 方向，会吵但不会漏；
    反过来若写成空串或前缀匹配，就会把内容提交也豁免掉，**那才是真漏**。
    """

    def test_exemption_path_is_the_real_notes_path(self) -> None:
        self.assertEqual(_MODULE.NOTES_ONLY_PATH, "docs/RELEASE_NOTES.md")

    def test_path_is_repo_relative_not_absolute_and_not_prefix(self) -> None:
        path = _MODULE.NOTES_ONLY_PATH
        self.assertFalse(path.startswith("/"), "必须是仓库相对路径（CI 在工作区根跑）")
        self.assertTrue(path.endswith(".md"), "必须是具体文件，不能是目录前缀")
        self.assertEqual(path.count("/"), 1, "只应有一级目录，避免前缀匹配误伤其他 docs 文件")


class SplitSectionsTest(unittest.TestCase):
    def test_only_top_level_version_sections(self) -> None:
        sections = split_sections(NOTES)
        self.assertEqual(len(sections), 2)
        self.assertTrue(all(s.startswith("## ") for s in sections))


if __name__ == "__main__":
    unittest.main()
