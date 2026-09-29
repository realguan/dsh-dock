"""check-release-coverage.py 的单元回归测试。

只测纯逻辑（解析 / 比对），不碰 git —— git 侧由 CI 在真 tag 上跑。
"""

from __future__ import annotations

import importlib.util
from pathlib import Path
import unittest

_SCRIPT_PATH = Path(__file__).parents[1] / "check-release-coverage.py"
_SPEC = importlib.util.spec_from_file_location("check_release_coverage", _SCRIPT_PATH)
assert _SPEC is not None and _SPEC.loader is not None
_MODULE = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_MODULE)

parse_coverage = _MODULE.parse_coverage
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
